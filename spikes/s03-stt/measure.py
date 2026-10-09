#!/usr/bin/env python3
"""Spike 0.3 (ADR-002): local speech recognition for learner Japanese.

Input: the synthetic corpus of spike 0.4 (<id>_<richtig|fehler>_<voice>.wav, 24 kHz).
Every file is resampled to 16 kHz and also mixed with pink noise at 10 and 5 dB SNR.

Engines
  whisper   whisper.cpp (whisper-cli), temperature 0, no prompt, language ja; one run per
            model over all files, so the model loads once
  reazon    ReazonSpeech zipformer through sherpa-onnx (offline transducer)

Measures, after turning reference and hypothesis into katakana (pyopenjtalk), so spelling
(kanji or kana) does not count:
  CER          character error rate on the correct sentences
  fidelity     share of error forms whose transcript is closer to the error than to the
               correct form: the planted error survives recognition (ADR-002 asks ≥ 85 %)
  RTF          processing time / audio time (4 CPU threads)

  python measure.py --corpus-audio ../s04-tts/out/corpus --corpus ../../qa/corpus \
      --whisper-cli <whisper.cpp/build/bin/whisper-cli> --models <dir> --out out
"""
import argparse
import csv
import json
import re
import subprocess
import time
from pathlib import Path

import librosa
import numpy as np
import pyopenjtalk
import soundfile as sf

SNRS = [None, 10, 5]
VOICES = ["jf_alpha", "jm_kumo"]
PUNCT = re.compile(r"[\s、。，．,.!?！？「」『』（）()・…ー—\-]")


def kana(text):
    text = PUNCT.sub("", text)
    return PUNCT.sub("", pyopenjtalk.g2p(text, kana=True)) if text else ""


def edit(a, b):
    d = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        prev, d[0] = d[0], i
        for j, cb in enumerate(b, 1):
            prev, d[j] = d[j], min(d[j] + 1, d[j - 1] + 1, prev + (ca != cb))
    return d[len(b)]


def pink(n, rng):
    white = rng.standard_normal(n)
    f = np.fft.rfft(white)
    f /= np.sqrt(np.maximum(np.arange(len(f)), 1))
    p = np.fft.irfft(f, n)
    return p / np.std(p)


def prepare(src, dst):
    rng = np.random.default_rng(13)
    items = []
    for wav in sorted(src.glob("*.wav")):
        sid, kind, voice = wav.stem.split("_", 2)
        if voice not in VOICES:
            continue
        audio, _ = librosa.load(wav, sr=16000)
        for snr in SNRS:
            x = audio
            if snr is not None:
                noise = pink(len(audio), rng)
                x = audio + noise * np.sqrt(np.mean(audio**2) / (10 ** (snr / 10)))
                x = x / max(1.0, np.max(np.abs(x)))
            name = f"{sid}_{kind}_{voice}_{'clean' if snr is None else f'snr{snr}'}.wav"
            sf.write(dst / name, x, 16000, subtype="PCM_16")
            items.append({"file": dst / name, "id": sid, "kind": kind, "voice": voice, "snr": snr, "dur": len(x) / 16000})
    return items


def run_whisper(cli, model, items, threads=4):
    files = [str(i["file"]) for i in items]
    t0 = time.perf_counter()
    for chunk in range(0, len(files), 50):
        args = [cli, "-m", str(model), "-l", "ja", "-t", str(threads), "-tp", "0", "-nt", "-np", "-otxt", "-nf"]
        for f in files[chunk:chunk + 50]:
            args += ["-f", f]
        subprocess.run(args, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    dt = time.perf_counter() - t0
    out = {}
    for i in items:
        txt = Path(str(i["file"]) + ".txt")
        out[str(i["file"])] = txt.read_text(encoding="utf-8").strip() if txt.exists() else ""
        txt.unlink(missing_ok=True)
    return out, dt


def run_reazon(model_dir, items, threads=4):
    import sherpa_onnx
    d = Path(model_dir)
    rec = sherpa_onnx.OfflineRecognizer.from_transducer(
        encoder=str(next(d.glob("encoder*.int8.onnx"), next(d.glob("encoder*.onnx")))),
        decoder=str(next(d.glob("decoder*.onnx"))),
        joiner=str(next(d.glob("joiner*.int8.onnx"), next(d.glob("joiner*.onnx")))),
        tokens=str(d / "tokens.txt"), num_threads=threads, decoding_method="greedy_search")
    out = {}
    t0 = time.perf_counter()
    for i in items:
        audio, sr = sf.read(i["file"], dtype="float32")
        # The zipformer drops the first words without some silence around the utterance.
        pad = np.zeros(int(0.3 * sr), dtype=np.float32)
        audio = np.concatenate([pad, audio, pad])
        s = rec.create_stream()
        s.accept_waveform(sr, audio)
        rec.decode_stream(s)
        out[str(i["file"])] = s.result.text
    return out, time.perf_counter() - t0


def score(items, hyps, rows):
    by_id = {r["id"]: r for r in rows}
    res = {}
    for snr in SNRS:
        sel = [i for i in items if i["snr"] == snr]
        errs = chars = 0
        fid_hit = fid_n = 0
        for i in sel:
            row = by_id[i["id"]]
            hyp = kana(hyps[str(i["file"])])
            right = kana(row["richtig"])
            if i["kind"] == "richtig":
                errs += edit(hyp, right)
                chars += len(right)
            else:
                wrong = kana(row["fehlerform"])
                if wrong == right:
                    continue  # pitch-only or same text: nothing for STT to keep
                fid_n += 1
                fid_hit += edit(hyp, wrong) < edit(hyp, right)
        res["clean" if snr is None else f"snr{snr}"] = {
            "cer": errs / max(1, chars), "fidelity": fid_hit / max(1, fid_n), "fidelity_n": fid_n}
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus-audio", required=True)
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--whisper-cli", required=True)
    ap.add_argument("--models", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--engines", default="whisper-small,reazonspeech-zipformer,whisper-large-v3-turbo-q5",
                    help="comma-separated; each engine writes out/<engine>.json as soon as it is done")
    ap.add_argument("--subset", choices=["all", "small"], default="all",
                    help="small: one voice, clean and 5 dB only (for slow engines on a CPU)")
    args = ap.parse_args()
    out = Path(args.out)
    work = out / "audio16k"
    work.mkdir(parents=True, exist_ok=True)
    rows = list(csv.DictReader(open(Path(args.corpus) / "saetze.tsv", encoding="utf-8"), delimiter="\t"))
    items = prepare(Path(args.corpus_audio), work)
    if args.subset == "small":
        items = [i for i in items if i["voice"] == VOICES[0] and i["snr"] in (None, 5)]
    total = sum(i["dur"] for i in items)
    models = Path(args.models)
    report = {"files": len(items), "audio_s": total, "engines": {}}
    examples = {}
    engines = [
        ("whisper-small", lambda: run_whisper(args.whisper_cli, models / "ggml-small.bin", items)),
        ("whisper-large-v3-turbo-q5", lambda: run_whisper(args.whisper_cli, models / "ggml-large-v3-turbo-q5_0.bin", items)),
        ("reazonspeech-zipformer", lambda: run_reazon(next(models.glob("sherpa-onnx-zipformer-ja-reazonspeech*")), items)),
    ]
    wanted = args.engines.split(",")
    for name, run in engines:
        if name not in wanted:
            continue
        hyps, dt = run()
        result = {"files": len(items), "audio_s": total, "subset": args.subset, "rtf": dt / total, **score(items, hyps, rows)}
        report["engines"][name] = result
        examples[name] = {Path(k).stem: v for k, v in hyps.items() if "_clean" in k and "_jf_alpha" in k}
        (out / f"{name}.json").write_text(json.dumps({"result": result, "transcripts": examples[name]}, ensure_ascii=False, indent=1))
        print(name, json.dumps(result, ensure_ascii=False), flush=True)
    (out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))
    (out / "transcripts.json").write_text(json.dumps(examples, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
