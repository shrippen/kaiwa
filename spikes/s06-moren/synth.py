"""Spike 0.6: synthetic corpus for the mora-length check (ADR-005, H-04).

Writes, with every Japanese Kokoro voice:
  - the minimal pairs m01-m13 (length, っ, ん), a and b, alone and in the frame これは＿です;
  - every sentence of saetze.tsv in its correct form, and the error form of the sentences
    whose error is a mora length (long vowel, っ, ん).

Besides Kokoro, OpenJTalk's HTS voice "Mei" (mei_normal, CC BY 3.0, via pyopenjtalk) speaks everything at
three speeds. Kokoro does not keep the mora lengths it is given (げんいん comes out as げにん,
ありがと as ありがとう); OpenJTalk sets every phone length from the kana, so its errors are
the ones written. Files already there are kept.

  python -I synth.py --models <dir with kokoro-v1.0.onnx, voices-v1.0.bin> --corpus ../../qa/corpus --out out
"""

import argparse
import csv
import json
from pathlib import Path

import pyopenjtalk
import soundfile as sf
from kokoro_onnx import Kokoro
from misaki import ja

KOKORO_VOICES = ["jf_alpha", "jf_gongitsune", "jf_nezumi", "jf_tebukuro", "jm_kumo"]
# OpenJTalk HTS at three speeds (Hinnerk speaks slowly).
OPENJTALK_VOICES = {"ojt": 1.0, "ojt_slow": 0.8, "ojt_fast": 1.25}
VOICES = KOKORO_VOICES + list(OPENJTALK_VOICES)
PAIR_KINDS = ("laenge", "sokuon", "hatsuon")
# Sentences whose error form differs from the correct one in mora length only.
MORA_SENTENCES = ("s04", "s05", "s23", "s24", "s25", "s26", "s27", "s28", "s44")
FRAME = "これは{}です。"


def word(cell):
    """'とり (鳥)' -> ('とり', '鳥'); 'げにん' -> ('げにん', 'げにん')."""
    kana, _, rest = cell.partition(" (")
    return kana, rest.split(",")[0].rstrip(")") if rest else kana


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--models", required=True)
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    out = Path(args.out) / "audio"
    out.mkdir(parents=True, exist_ok=True)
    models = Path(args.models)
    kokoro = Kokoro(str(models / "kokoro-v1.0.onnx"), str(models / "voices-v1.0.bin"))
    g2p = ja.JAG2P()
    corpus = Path(args.corpus)

    items = []  # what was said: id, text (as spoken), the forms it could be checked against
    for row in csv.DictReader(open(corpus / "minimalpaare.tsv", encoding="utf-8"), delimiter="\t"):
        if row["art"] not in PAIR_KINDS:
            continue
        a, b = word(row["a"]), word(row["b"])
        for side, (kana, _) in (("a", a), ("b", b)):
            items.append({"id": f"{row['id']}_{side}", "pair": row["id"], "kind": row["art"],
                          "spoken": kana, "forms": {"a": a[0], "b": b[0]}, "frame": False})
            items.append({"id": f"{row['id']}_{side}_frame", "pair": row["id"], "kind": row["art"],
                          "spoken": FRAME.format(kana), "forms": {"a": FRAME.format(a[0]), "b": FRAME.format(b[0])},
                          "frame": True})
    for row in csv.DictReader(open(corpus / "saetze.tsv", encoding="utf-8"), delimiter="\t"):
        items.append({"id": f"{row['id']}_richtig", "sentence": row["id"], "spoken": row["richtig"],
                      "intended": row["richtig"], "error": False})
        if row["id"] in MORA_SENTENCES:
            items.append({"id": f"{row['id']}_fehler", "sentence": row["id"], "spoken": row["fehlerform"],
                          "intended": row["richtig"], "error": True, "hint": row["hinweis"]})

    def phonemes(text):
        p = g2p(text)
        return p[0] if isinstance(p, tuple) else p

    for voice in VOICES:
        for it in items:
            path = out / f"{it['id']}_{voice}.wav"
            if path.exists():
                continue
            if voice in OPENJTALK_VOICES:
                audio, sr = pyopenjtalk.tts(it["spoken"], speed=OPENJTALK_VOICES[voice])
                audio = audio / 32768.0
            else:
                audio, sr = kokoro.create(phonemes(it["spoken"]), voice=voice, is_phonemes=True)
            sf.write(path, audio, sr)
    manifest = {"voices": VOICES, "items": items,
                "phonemes": {it["id"]: phonemes(it["spoken"]) for it in items}}
    (Path(args.out) / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=1))
    print(len(items) * len(VOICES), "files")


if __name__ == "__main__":
    main()
