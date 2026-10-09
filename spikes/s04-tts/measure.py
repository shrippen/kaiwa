#!/usr/bin/env python3
"""Spike 0.4 (ADR-003): Kokoro (ONNX) as the local Japanese voice.

Measures per voice
  - accent: does the voice put the fall where the dictionary does? Words of the accent
    minimal pairs plus the particle が, F0 per mora (pYIN, morae split evenly over the voiced
    part), compared with the accent kernel from OpenJTalk's dictionary (pyopenjtalk);
  - speed: synthesis time per sentence and the real-time factor over the 50 corpus sentences;
and writes the synthetic corpus (correct and error forms, every voice) for spike 0.3.

  python measure.py --models <dir with kokoro-v1.0.onnx, voices-v1.0.bin> --corpus ../../qa/corpus --out out
"""
import argparse
import csv
import json
import time
from pathlib import Path

import librosa
import numpy as np
import pyopenjtalk
import soundfile as sf
from kokoro_onnx import Kokoro
from misaki import ja

VOICES = ["jf_alpha", "jf_gongitsune", "jf_nezumi", "jf_tebukuro", "jm_kumo"]
# Words of the accent pairs (qa/corpus/minimalpaare.tsv m14-m20), each with が so a final
# fall (odaka) can be told from none (heiban).
ACCENT_WORDS = ["箸", "橋", "端", "雨", "飴", "神", "紙", "牡蠣", "柿", "鮭", "酒", "日本", "二本"]


def dictionary_accent(word):
    """Accent kernel (0 = flat) and mora count of the word, from OpenJTalk's dictionary."""
    nodes = [n for n in pyopenjtalk.run_frontend(word) if n["pos"] != "記号"]
    node = nodes[0]
    return node["acc"], node["mora_size"]


def expected_levels(acc, morae):
    """Tokyo pattern: high (1) or low (0) per mora of the word, then the particle."""
    if acc == 1:
        word = [1] + [0] * (morae - 1)
    else:
        kernel = acc if acc > 0 else morae + 1
        word = [0] + [1 if i < kernel else 0 for i in range(1, morae)]
    particle = 1 if acc == 0 else 0
    return word + [particle]


def mora_f0(audio, sr, n):
    f0, voiced, _ = librosa.pyin(audio, fmin=70, fmax=500, sr=sr, frame_length=1024)
    idx = np.where(voiced & ~np.isnan(f0))[0]
    if len(idx) < n * 3:
        return None
    span = idx[0], idx[-1] + 1
    seg = np.array_split(np.arange(*span), n)
    out = []
    for s in seg:
        vals = f0[s][~np.isnan(f0[s])]
        out.append(float(np.median(vals)) if len(vals) else float("nan"))
    return out


def observed_levels(f0s):
    """High/low per mora: above or below the midpoint of the word's range."""
    arr = np.array(f0s)
    if np.isnan(arr).any():
        return None
    mid = (arr.max() + arr.min()) / 2
    return [1 if v >= mid else 0 for v in arr]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--models", required=True)
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    out = Path(args.out)
    (out / "corpus").mkdir(parents=True, exist_ok=True)
    models = Path(args.models)
    kokoro = Kokoro(str(models / "kokoro-v1.0.onnx"), str(models / "voices-v1.0.bin"))
    g2p = ja.JAG2P()

    def say(text, voice, speed=1.0):
        phonemes = g2p(text)
        if isinstance(phonemes, tuple):  # newer misaki returns (phonemes, tokens)
            phonemes = phonemes[0]
        t0 = time.perf_counter()
        audio, sr = kokoro.create(phonemes, voice=voice, speed=speed, is_phonemes=True)
        return audio, sr, time.perf_counter() - t0

    report = {"voices": {}, "accent_words": ACCENT_WORDS,
              # misaki's Japanese G2P (0.7.x) is romaji-based: 箸が, 橋が and 端が all become "haɕi ɡa",
              # so the model has to guess the accent; homophones cannot differ.
              "g2p_sample": {w: g2p(w + "が") for w in ("箸", "橋", "端")}}
    rows = list(csv.DictReader(open(Path(args.corpus) / "saetze.tsv", encoding="utf-8"), delimiter="\t"))

    for voice in VOICES:
        # Accent
        hits, checked, details = 0, 0, []
        for word in ACCENT_WORDS:
            acc, morae = dictionary_accent(word)
            want = expected_levels(acc, morae)
            audio, sr, _ = say(word + "が", voice)
            f0s = mora_f0(audio, sr, morae + 1)
            got = observed_levels(f0s) if f0s else None
            ok = got == want
            checked += 1
            hits += ok
            details.append({"word": word, "acc": acc, "want": want, "got": got, "f0": f0s, "ok": ok})
        # Speed and the synthetic corpus
        times, durations = [], []
        for row in rows:
            for kind, text in (("richtig", row["richtig"]), ("fehler", row["fehlerform"])):
                if text == "—" or (kind == "fehler" and text == row["richtig"]):
                    continue
                audio, sr, dt = say(text, voice)
                sf.write(out / "corpus" / f"{row['id']}_{kind}_{voice}.wav", audio, sr)
                if kind == "richtig":
                    times.append(dt)
                    durations.append(len(audio) / sr)
        report["voices"][voice] = {
            "accent_hits": hits,
            "accent_checked": checked,
            "accent": details,
            "synth_s_p50": float(np.percentile(times, 50)),
            "synth_s_p95": float(np.percentile(times, 95)),
            "rtf": float(sum(times) / sum(durations)),
        }
        print(f"{voice}: accent {hits}/{checked}, synth p50 {np.percentile(times, 50):.2f} s, RTF {sum(times) / sum(durations):.2f}", flush=True)

    (out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
