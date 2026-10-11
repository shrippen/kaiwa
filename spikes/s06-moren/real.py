"""Spike 0.6 with real voices (J-04): Common Voice Japanese, test split (CC0), one clip per speaker.

There is no free corpus of learners with mora errors, so the errors are cut into native speech.
MFA (japanese_mfa) gives the phone boundaries of each clip; then one site at a time is changed:

  long-  a long vowel (Vː) cut to the clip's median short vowel
  q-     a long consonant (Cː, っ) cut to the clip's median short consonant
  long+  a short vowel stretched to twice its length (whole pitch periods repeated)
  q+     a stop closure (k, t, p) lengthened by one mora of silence

Correct trials are each clip as it is and with a sham cut (one pitch period out of a short vowel),
so a check that reacts to the joins rather than to the lengths shows up as false alarms. ん before a
vowel (げんいん/げにん) changes a sound, not a length, and is left out.

The check is measure.py's variant comparison, thresholds per error kind, calibrated on half the
speakers and counted on the other half (and the other way round); once as it is, once with the
reference taken off: OpenJTalk speaks the meant sentence, as the tutor voice would in the app.

  python -I real.py --cv ../../target/cv19-ja --mfa <env>/bin/mfa --work ../../target/s06-real --out results
"""

import argparse
import csv
import json
import random
import re
import sys
from pathlib import Path

import librosa
import numpy as np
import pyopenjtalk
import soundfile as sf
import torch

sys.path.insert(0, str(Path(__file__).parent))  # -I leaves the script's directory out
import ctc  # noqa: E402
from measure import FA_MAX, MODELS, Model, best_by_kind, edit, model_tag, kind_alarm, kind_thresholds, load, variant_features  # noqa: E402
from mfa_check import VOWELS, phones, read_phones, run_mfa  # noqa: E402

SR = 16000
N_CLIPS = 500
XFADE = int(0.008 * SR)
STOPS = ("k", "c", "t", "p")
GEM_KINDS = ("long-", "q-", "long+", "q+")
# variants: the variant comparison as in measure.py; ref: with the reference taken off;
# supported: only variants the free transcription agrees with (closer to it than the meant kana)
CHECKS = ("variants", "ref", "supported")
# all: every kind is checked; shortened: only a long vowel or っ spoken short, the errors learners make
KIND_SETS = {"all": None, "shortened": ("long-", "q-")}


def cer(a, b):
    return edit(a, b) / max(len(b), 1)


def greedy(logp, token, model):
    ids = logp.argmax(-1)
    return "".join(token[i] for i, prev in zip(ids, np.concatenate(([-1], ids[:-1])))
                   if i != prev and i != model.blank and i != model.delim)


def select(cv, n, seed=0):
    rows = list(csv.DictReader(open(cv / "test.tsv", encoding="utf-8"), delimiter="\t", quoting=csv.QUOTE_NONE))
    rng = random.Random(seed)
    rng.shuffle(rows)
    seen, out = set(), []
    for r in rows:
        s = r["sentence"]
        if r["client_id"] in seen or r.get("down_votes", "0") not in ("0", ""):
            continue
        if not 8 <= len(s) <= 40 or re.search(r"[0-9０-９A-Za-zＡ-Ｚａ-ｚ]", s):
            continue
        seen.add(r["client_id"])
        out.append(r)
        if len(out) == n:
            break
    return out


def xfade_join(a, b):
    """Joins a and b with a short linear crossfade."""
    n = min(XFADE, len(a), len(b))
    if n == 0:
        return np.concatenate([a, b])
    w = np.linspace(0, 1, n)
    return np.concatenate([a[:-n], a[-n:] * (1 - w) + b[:n] * w, b[n:]])


def period(seg):
    """Pitch period in samples (YIN), or 8 ms where there is none."""
    if len(seg) < 2048:
        seg = np.pad(seg, (0, 2048 - len(seg)))
    f0 = librosa.yin(seg, fmin=70, fmax=400, sr=SR, frame_length=2048)
    f0 = np.median(f0[np.isfinite(f0)]) if np.isfinite(f0).any() else 125
    return max(int(SR / f0), 1)


def cut(audio, s, e, keep):
    """Removes the middle of [s, e) so that `keep` samples remain, in whole pitch periods."""
    length = e - s
    remove = length - keep
    if remove <= XFADE:
        return None
    p = period(audio[s:e])
    remove = max((remove // p) * p, p)
    a = s + (length - remove) // 2
    return xfade_join(audio[:a], audio[a + remove:])


def stretch(audio, s, e, factor):
    """Lengthens [s, e) by repeating whole pitch periods from its middle."""
    p = period(audio[s:e])
    add = int((e - s) * (factor - 1))
    mid = (s + e) // 2
    n = p * max(2, -(-int(0.04 * SR) // p))  # whole periods, at least 40 ms (longer than the crossfade)
    chunk = audio[mid - n // 2: mid - n // 2 + n]
    if len(chunk) < n or n > (e - s):
        return None
    out = audio[:mid]
    while add > 0:
        out = xfade_join(out, chunk)
        add -= len(chunk) - XFADE
    return xfade_join(out, audio[mid:])


def insert_silence(audio, at, n):
    return np.concatenate([audio[:at], np.zeros(n, audio.dtype), audio[at:]])


def edits(audio, ph, rng):
    """One trial per error kind present in the clip, plus the sham cut."""
    t = [(int(s * SR), int(e * SR), p) for s, e, p in ph]
    short_v = [e - s for s, e, p in t if p in VOWELS]
    short_c = [e - s for s, e, p in t if p in STOPS + ("s", "ɕ", "ts", "tɕ")]
    mora = np.median(short_v) if short_v else 0.1 * SR
    out = {}
    last = len(t) - 1
    inner = [i for i in range(1, last)]  # never the first or last phone
    vl = [i for i in inner if t[i][2].endswith("ː") and t[i][2].rstrip("ː") in VOWELS]
    cl = [i for i in inner if t[i][2].endswith("ː") and t[i][2].rstrip("ː") not in VOWELS]
    vs = [i for i in inner if t[i][2] in VOWELS and t[i][1] - t[i][0] >= 0.05 * SR]
    st = [i for i in inner if t[i][2] in STOPS and t[i - 1][2] in VOWELS]
    if vl and short_v:
        i = rng.choice(vl)
        out["long-"] = (i, cut(audio, t[i][0], t[i][1], int(np.median(short_v))))
    if cl and short_c:
        i = rng.choice(cl)
        out["q-"] = (i, cut(audio, t[i][0], t[i][1], int(np.median(short_c))))
    if vs:
        i = rng.choice(vs)
        out["long+"] = (i, stretch(audio, t[i][0], t[i][1], 2.0))
        j = rng.choice(vs)
        p = period(audio[t[j][0]:t[j][1]])
        sham = cut(audio, t[j][0], t[j][1], t[j][1] - t[j][0] - p) if p > XFADE else None
        out["sham"] = (j, sham)
    if st:
        i = rng.choice(st)
        out["q+"] = (i, insert_silence(audio, t[i][0] + int(0.01 * SR), int(0.8 * mora)))
    return {k: v for k, v in out.items() if v[1] is not None}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cv", required=True)
    ap.add_argument("--mfa", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--clips", type=int, default=N_CLIPS)
    ap.add_argument("--model", default="hiragana", help="as in measure.py")
    args = ap.parse_args()
    cv, work = Path(args.cv), Path(args.work)
    clips_dir = work / "clips"
    clips_dir.mkdir(parents=True, exist_ok=True)
    model = Model(args.model, "cuda" if torch.cuda.is_available() else "cpu")
    token = {v: k for k, v in model.vocab.items()}

    # 1. Clips whose reading the model hears (the meant text must match what was said). The first
    # run chooses them (with the hiragana model) and keeps the choice in clips.json, so every model
    # is measured on the same clips and the same cuts.
    chosen = work / "clips.json"
    clips = json.loads(chosen.read_text()) if chosen.exists() else []
    for c in clips:
        c["kana"] = model.kana(c["sentence"])
    for r in [] if clips else select(cv, args.clips * 2):
        uid = Path(r["path"]).stem
        wav = clips_dir / f"{uid}.wav"
        if not wav.exists():
            a, _ = librosa.load(cv / "audio" / "ja_test_0" / r["path"], sr=SR)
            a, _ = librosa.effects.trim(a, top_db=35)
            sf.write(wav, a, SR)
        kana = model.kana(r["sentence"])
        if not kana or any(c not in model.vocab for c in kana):
            continue
        heard = greedy(model.logp(load(wav, None, 0)), token, model)
        if cer(heard, kana) > 0.2:
            continue
        clips.append({"uid": uid, "speaker": r["client_id"][:12], "sentence": r["sentence"], "kana": kana,
                      "pron": ctc.pron(r["sentence"])})
        if len(clips) == args.clips:
            break
    print(len(clips), "clips kept")
    if not chosen.exists():
        chosen.write_text(json.dumps(clips, ensure_ascii=False))

    # 2. Phone boundaries with MFA (meant pronunciation); one directory per clip, so MFA adapts
    # to each speaker on its own
    corpus = work / "corpus"
    with open(work / "dict.txt", "w") as f:
        for c in clips:
            d = corpus / c["uid"]
            d.mkdir(parents=True, exist_ok=True)
            if not (d / f"{c['uid']}.wav").exists():
                (d / f"{c['uid']}.wav").symlink_to((clips_dir / f"{c['uid']}.wav").resolve())
            (d / f"{c['uid']}.lab").write_text(f"w{c['uid']}")
            f.write(f"w{c['uid']}\t1.0\t{' '.join(phones(c['pron']))}\n")
    out_dir = work / "aligned"
    if not out_dir.exists():
        run_mfa(args.mfa, corpus, work / "dict.txt", out_dir, 16)

    # 3. Edited audio, 4. scores. Reference: OpenJTalk speaks the meant sentence (it keeps the lengths
    # it is given), as the tutor voice would in the app; what the model hears into that is taken off.
    rng = random.Random(1)
    edited, refs = work / "edited", work / "reference"
    edited.mkdir(exist_ok=True)
    refs.mkdir(exist_ok=True)
    rows = []
    for c in clips:
        jf = out_dir / c["uid"] / f"{c['uid']}.json"
        if not jf.exists():
            continue
        ph = read_phones(jf)
        audio, _ = librosa.load(clips_dir / f"{c['uid']}.wav", sr=SR)
        ref_wav = refs / f"{c['uid']}.wav"
        if not ref_wav.exists():
            x, sr = pyopenjtalk.tts(c["sentence"])
            sf.write(ref_wav, librosa.resample(x / 32768.0, orig_sr=sr, target_sr=SR), SR)
        ref = variant_features(model.logp(load(ref_wav, None, 0)), c["kana"], model)["every"]
        versions = {"original": (None, audio)} | edits(audio, ph, rng)
        for kind, (i, a) in versions.items():
            path = edited / f"{c['uid']}_{kind}.wav"
            sf.write(path, a, SR)
            logp = model.logp(load(path, None, 0))
            feats = variant_features(logp, c["kana"], model)
            normed = best_by_kind({v: (k, j, d - ref[v][2]) for v, (k, j, d) in feats["every"].items() if v in ref})
            # Supported: the free transcription is closer to the variant than to the meant kana
            heard = greedy(logp, token, model)
            base = edit(heard, c["kana"])
            supported = best_by_kind({v: kjd for v, kjd in feats["every"].items() if edit(heard, v) < base})
            rows.append({"uid": c["uid"], "speaker": c["speaker"], "kana": c["kana"], "edit": kind,
                         "phone": ph[i][2] if i is not None else None,
                         "error": kind in GEM_KINDS, "variants": feats, "ref": normed,
                         "supported": supported})

    # Which kinds are checked at all: every kind, or only the shortenings learners make (H-04)
    for r in rows:
        for check in CHECKS:
            for name, kinds in KIND_SETS.items():
                r[f"{check}/{name}"] = {"by_kind": {k: v for k, v in r[check]["by_kind"].items()
                                                    if kinds is None or k in kinds}}
    keys = [f"{c}/{n}" for c in CHECKS for n in KIND_SETS]

    # Two folds by speaker
    speakers = sorted({r["speaker"] for r in rows})
    half = set(speakers[::2])
    for fold in (True, False):
        cal = [r for r in rows if (r["speaker"] in half) != fold and not r["error"]]
        for key in keys:
            th = kind_thresholds(cal, key)
            for r in rows:
                if (r["speaker"] in half) == fold:
                    r.setdefault("alarm", {})[key] = kind_alarm(r, th, key)

    def rate(sel, key):
        return (sum(r["alarm"][key] is not None for r in sel) / len(sel), len(sel)) if sel else (None, 0)

    summary = {}
    ok = [r for r in rows if not r["error"]]
    for key in keys:
        kinds = KIND_SETS[key.split("/")[1]] or GEM_KINDS
        err = [r for r in rows if r["error"] and r["edit"] in kinds]
        sm = {k: rate([r for r in rows if r["edit"] == k], key) for k in ("original", "sham") + GEM_KINDS}
        sm["false_alarms"] = rate(ok, key)
        sm["detection"] = rate(err, key)
        hits = [r for r in err if r["alarm"][key]]
        sm["kind_right"] = sum(r["alarm"][key] == r["edit"] for r in hits) / max(1, len(hits))
        summary[key] = sm
    # For the app (thresholds.json next to the model): calibrated on every speaker
    thresholds = {key: kind_thresholds(ok, key) for key in keys}
    report = {"source": "Common Voice 19.0 ja, test split (CC0)", "clips": len(clips), "fa_max": FA_MAX,
              "summary": summary, "thresholds": thresholds,
              "trials": [{k: r[k] for k in ("uid", "edit", "phone", "kana", "error", "alarm")}
                         | {f"{c}_by_kind": r[c]["by_kind"] for c in CHECKS}
                         for r in rows]}
    Path(args.out).mkdir(parents=True, exist_ok=True)
    tag = args.model if args.model in MODELS else model_tag(args.model)
    (Path(args.out) / f"real-cv-{tag}.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
