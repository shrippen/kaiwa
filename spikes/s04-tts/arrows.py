#!/usr/bin/env python3
"""Spike 0.4, experiment: does Kokoro follow a fall marked with ↘ after the accent kernel?
Kokoro's vocabulary has tone arrows (from its Mandarin training); misaki's Japanese G2P sets
none. The accent words get their kernel from OpenJTalk's dictionary and an arrow after that
mora; the measurement is the same as in measure.py.

  python arrows.py --models <dir> --out out
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np
from kokoro_onnx import Kokoro

sys.path.insert(0, str(Path(__file__).parent))
from measure import ACCENT_WORDS, VOICES, dictionary_accent, expected_levels, mora_f0, observed_levels  # noqa: E402

# The words in misaki's phonemes, split into morae by hand (two or three morae each).
MORAE = {
    "箸": ["ha", "ɕi"], "橋": ["ha", "ɕi"], "端": ["ha", "ɕi"], "雨": ["a", "me"], "飴": ["a", "me"],
    "神": ["ka", "mi"], "紙": ["ka", "mi"], "牡蠣": ["ka", "ki"], "柿": ["ka", "ki"],
    "鮭": ["sa", "ke"], "酒": ["sa", "ke"], "日本": ["ni", "ho", "ɴ"], "二本": ["ni", "ho", "ɴ"],
}


def marked(word):
    acc, _ = dictionary_accent(word)
    morae = list(MORAE[word])
    if acc > 0:
        morae[acc - 1] += "↘"
    return "".join(morae) + " ɡa"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--models", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    m = Path(a.models)
    kokoro = Kokoro(str(m / "kokoro-v1.0.onnx"), str(m / "voices-v1.0.bin"))
    report = {}
    for voice in VOICES:
        hits = 0
        rows = []
        for word in ACCENT_WORDS:
            acc, morae = dictionary_accent(word)
            want = expected_levels(acc, morae)
            ph = marked(word)
            audio, sr = kokoro.create(ph, voice=voice, is_phonemes=True)
            f0s = mora_f0(audio, sr, morae + 1)
            got = observed_levels(f0s) if f0s else None
            hits += got == want
            rows.append({"word": word, "phonemes": ph, "want": want, "got": got})
        report[voice] = {"hits": hits, "of": len(ACCENT_WORDS), "rows": rows}
        print(f"{voice}: accent with arrows {hits}/{len(ACCENT_WORDS)}", flush=True)
    Path(a.out).mkdir(parents=True, exist_ok=True)
    (Path(a.out) / "arrows.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
