"""J-02: an own kana CTC model for the mora check.

Base: reazon-research/japanese-wav2vec2-base or -large (wav2vec 2.0, pre-trained on ReazonSpeech,
Apache-2.0). The prepared data (data.json, audio.npy) can be linked into another --work directory.
Data: Common Voice 19.0 Japanese (CC0): train, dev and the clips of `other` that at least one person
voted for and nobody against. Speakers of the test split are left out completely (real.py tests on
them), and so are 40 dev speakers, whose clips measure the error rate while training.

Targets are the pronounced kana from OpenJTalk (pyopenjtalk 'pron'): ー for long vowels (とうきょう ->
とーきょー), は as わ, っ and ん as spoken. So the model learns what is said, not how it is written:
the hiragana model of spike 0.6 heard げんいん as げにん because of the spelling.

  python -I finetune.py --cv ../../target/cv19-ja --work ../../target/kana-ctc --steps 12000
"""

import argparse
import csv
import json
import math
import random
import re
import sys
import time
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).parent))  # -I leaves the script's directory out
import ctc  # noqa: E402

# Pre-trained on ReazonSpeech, Apache-2.0 each: name -> revision
BASES = {
    "reazon-research/japanese-wav2vec2-base": "021e23f88a4fe70548049063d172ad2ba0c636f7",
    "reazon-research/japanese-wav2vec2-large": "55969d3700533972fcc4ff0e3747f0bc3c21e4c1",
}
SR = 16000
MAX_S = 12.0
HELD_OUT_SPEAKERS = 40
SPECIAL = ["[PAD]", "[UNK]", "|"]
KANA = sorted(set(ctc.VOWEL) | set("んっー"))


def label(sentence):
    """Pronounced hiragana, or None when the sentence has something else (latin, digits …)."""
    import pyopenjtalk
    if re.search(r"[0-9０-９A-Za-zＡ-Ｚａ-ｚ]", sentence):
        return None  # how the speaker read digits and latin letters is unknown
    p = "".join(n["pron"] for n in pyopenjtalk.run_frontend(sentence))
    p = ctc.kata_to_hira(p.replace("’", ""))
    p = "".join(c for c in p if c not in "、。！？!?「」『』・…　 ")
    return p if p and all(c in KANA for c in p) else None


def rows_of(path):
    return list(csv.DictReader(open(path, encoding="utf-8"), delimiter="\t", quoting=csv.QUOTE_NONE))


def decode(args):
    path, = args
    import librosa
    try:
        a, _ = librosa.load(path, sr=SR)
        a, _ = librosa.effects.trim(a, top_db=35)
    except Exception:
        return None
    if not 0.8 <= len(a) / SR <= MAX_S:
        return None
    return (np.clip(a, -1, 1) * 32767).astype(np.int16)


def prepare(cv, work, splits):
    """Labels and 16 kHz audio of every usable clip, cached as one int16 array with offsets."""
    if (work / "data.json").exists():
        return json.loads((work / "data.json").read_text())
    test_speakers = {r["client_id"] for r in rows_of(cv / "test.tsv")}
    rows = []
    for split in splits:
        for r in rows_of(cv / f"{split}.tsv"):
            if r["client_id"] in test_speakers:
                continue
            if split == "other" and not (int(r["up_votes"] or 0) >= 1 and int(r["down_votes"] or 0) == 0):
                continue
            r["split"] = split
            rows.append(r)
    dev_speakers = sorted({r["client_id"] for r in rows if r["split"] == "dev"})
    random.Random(0).shuffle(dev_speakers)
    held = set(dev_speakers[:HELD_OUT_SPEAKERS])
    labels = {}
    for r in rows:
        if r["sentence"] not in labels:
            labels[r["sentence"]] = label(r["sentence"])
    where = {f.name: f for d in (cv / "audio").iterdir() for f in d.iterdir()}
    items = []
    for r in rows:
        lab = labels[r["sentence"]]
        mp3 = where.get(r["path"])
        if lab and mp3:
            items.append({"path": str(mp3), "label": lab, "held_out": r["client_id"] in held})
    print(len(items), "clips with labels", flush=True)
    chunks, offsets, keep, pos = [], [], [], 0
    with ProcessPoolExecutor(20) as ex:
        for it, a in zip(items, ex.map(decode, [(i["path"],) for i in items], chunksize=64)):
            if a is None or len(it["label"]) * 2 > len(a) / SR * 50:  # CTC needs frames for the labels
                continue
            chunks.append(a)
            offsets.append([pos, len(a)])
            pos += len(a)
            keep.append(it)
    np.save(work / "audio.npy", np.concatenate(chunks))
    data = {"items": keep, "offsets": offsets}
    (work / "data.json").write_text(json.dumps(data, ensure_ascii=False))
    return data


def batches(lengths, idx, seconds, rng):
    """Batches of similar length, at most `seconds` of audio (padding included)."""
    idx = sorted(idx, key=lambda i: lengths[i])
    out, cur = [], []
    for i in idx:
        if cur and (len(cur) + 1) * lengths[i] > seconds * SR:
            out.append(cur)
            cur = []
        cur.append(i)
    if cur:
        out.append(cur)
    rng.shuffle(out)
    return out


def greedy(logits, inv, blank):
    ids = logits.argmax(-1).tolist()
    out, prev = [], None
    for i in ids:
        if i != prev and i != blank:
            out.append(inv.get(i, ""))
        prev = i
    return "".join(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cv", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--steps", type=int, default=12000)
    ap.add_argument("--batch-seconds", type=float, default=200)
    ap.add_argument("--lr", type=float, default=1e-4)
    ap.add_argument("--splits", default="train,dev,other")
    ap.add_argument("--base", choices=BASES, default="reazon-research/japanese-wav2vec2-base")
    args = ap.parse_args()
    from transformers import (Wav2Vec2CTCTokenizer, Wav2Vec2FeatureExtractor, Wav2Vec2ForCTC,
                              Wav2Vec2Processor)
    cv, work = Path(args.cv), Path(args.work)
    work.mkdir(parents=True, exist_ok=True)
    data = prepare(cv, work, args.splits.split(","))
    audio = np.load(work / "audio.npy", mmap_mode="r")
    items, offsets = data["items"], data["offsets"]
    vocab = {t: i for i, t in enumerate(SPECIAL + KANA)}
    (work / "vocab.json").write_text(json.dumps(vocab, ensure_ascii=False))
    tok = Wav2Vec2CTCTokenizer(str(work / "vocab.json"), pad_token="[PAD]", unk_token="[UNK]",
                               word_delimiter_token="|")
    fe = Wav2Vec2FeatureExtractor.from_pretrained(args.base, revision=BASES[args.base])
    proc = Wav2Vec2Processor(feature_extractor=fe, tokenizer=tok)
    model = Wav2Vec2ForCTC.from_pretrained(
        args.base, revision=BASES[args.base], vocab_size=len(vocab), pad_token_id=vocab["[PAD]"],
        ctc_loss_reduction="mean", ctc_zero_infinity=True, mask_time_prob=0.05, layerdrop=0.05,
        attention_dropout=0.0, hidden_dropout=0.0, feat_proj_dropout=0.0).cuda()
    model.freeze_feature_encoder()
    inv = {i: t for t, i in vocab.items()}
    blank = vocab["[PAD]"]

    train_idx = [i for i, it in enumerate(items) if not it["held_out"]]
    held_idx = [i for i, it in enumerate(items) if it["held_out"]][:600]
    lengths = [o[1] for o in offsets]
    hours = sum(lengths[i] for i in train_idx) / SR / 3600
    print(f"train {len(train_idx)} clips, {hours:.1f} h; held out {len(held_idx)}", flush=True)

    def batch(ix):
        waves = [np.asarray(audio[offsets[i][0]:offsets[i][0] + offsets[i][1]], np.float32) / 32767 for i in ix]
        x = fe(waves, sampling_rate=SR, padding=True, return_tensors="pt").input_values
        labs = [[vocab[c] for c in items[i]["label"]] for i in ix]
        n = max(map(len, labs))
        y = torch.full((len(ix), n), -100)
        for k, lab in enumerate(labs):
            y[k, :len(lab)] = torch.tensor(lab)
        return x.cuda(), y.cuda()

    def evaluate():
        model.eval()
        errs, total = 0, 0
        from measure import edit
        with torch.no_grad():
            for i in held_idx:
                x, _ = batch([i])
                with torch.autocast("cuda", dtype=torch.bfloat16):
                    lg = model(x).logits[0].float()
                hyp = greedy(lg, inv, blank)
                errs += edit(hyp, items[i]["label"])
                total += len(items[i]["label"])
        model.train()
        return errs / total

    opt = torch.optim.AdamW([p for p in model.parameters() if p.requires_grad], lr=args.lr, weight_decay=0.0)
    warm = 800

    def lr_at(s):
        return min(1.0, s / warm) * max(0.0, 0.5 * (1 + math.cos(math.pi * min(1.0, s / args.steps))))
    sched = torch.optim.lr_scheduler.LambdaLR(opt, lr_at)
    rng = random.Random(0)
    step, t0, log = 0, time.time(), []
    model.train()
    while step < args.steps:
        for ix in batches(lengths, train_idx, args.batch_seconds, rng):
            x, y = batch(ix)
            with torch.autocast("cuda", dtype=torch.bfloat16):
                loss = model(x, labels=y).loss
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            opt.step()
            sched.step()
            opt.zero_grad(set_to_none=True)
            step += 1
            if step % 100 == 0:
                print(f"step {step} loss {loss.item():.3f} {time.time() - t0:.0f}s", flush=True)
            if step % 2000 == 0 or step == args.steps:
                c = evaluate()
                log.append({"step": step, "loss": loss.item(), "held_out_cer": c})
                print(f"step {step} held-out CER {c:.3f}", flush=True)
                model.save_pretrained(work / "model")
                proc.save_pretrained(work / "model")
                (work / "log.json").write_text(json.dumps(log, indent=1))
            if step >= args.steps:
                break


if __name__ == "__main__":
    main()
