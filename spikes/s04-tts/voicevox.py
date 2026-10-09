#!/usr/bin/env python3
"""Spike 0.4, part 2 (local test): VOICEVOX's accent, measured like Kokoro's in measure.py.

VOICEVOX takes its accent phrases from OpenJTalk's dictionary, so 箸, 橋 and 端 should differ.
Not run in CI or the cloud session: the downloader needs GitHub's API, and each voice has
its own terms, which the owner accepts when downloading.

  # once: https://github.com/VOICEVOX/voicevox_core/releases (0.16.x)
  ./download-linux-x64 -o voicevox_core          # accept the terms of the voices you want
  pip install voicevox_core-0.16.1-cp310-abi3-manylinux_2_34_x86_64.whl
  python -I voicevox.py --core voicevox_core --out out [--styles 13,21,...]

Without --styles it lists the styles and measures every one. Written against the 0.16 Python
API and not run yet: adjust names if the API differs.
"""
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np
import soundfile as sf
import io

sys.path.insert(0, str(Path(__file__).parent))
from measure import ACCENT_WORDS, dictionary_accent, expected_levels, mora_f0, observed_levels  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--core", required=True, help="directory the downloader wrote")
    ap.add_argument("--out", required=True)
    ap.add_argument("--styles", default="", help="comma-separated style ids; default all")
    a = ap.parse_args()
    from voicevox_core.blocking import Onnxruntime, OpenJtalk, Synthesizer, VoiceModelFile

    core = Path(a.core)
    ort_lib = next(core.glob("onnxruntime/lib/*onnxruntime*.so*"))
    ort = Onnxruntime.load_once(filename=str(ort_lib))
    ojt = OpenJtalk(str(next(core.glob("dict/open_jtalk_dic_utf_8-*"))))
    syn = Synthesizer(ort, ojt)
    for vvm in sorted(core.glob("models/vvms/*.vvm")):
        with VoiceModelFile.open(vvm) as model:
            syn.load_voice_model(model)
    styles = {}
    for meta in syn.metas():
        for st in meta.styles:
            styles[st.id] = f"{meta.name} ({st.name})"
    print(json.dumps(styles, ensure_ascii=False, indent=1))
    wanted = [int(x) for x in a.styles.split(",") if x] or list(styles)

    report = {}
    for sid in wanted:
        hits, rows, times = 0, [], []
        for word in ACCENT_WORDS:
            acc, morae = dictionary_accent(word)
            want = expected_levels(acc, morae)
            t0 = time.perf_counter()
            wav = syn.tts(word + "が", sid)
            times.append(time.perf_counter() - t0)
            audio, sr = sf.read(io.BytesIO(wav), dtype="float32")
            f0s = mora_f0(audio, sr, morae + 1)
            got = observed_levels(f0s) if f0s else None
            hits += got == want
            rows.append({"word": word, "want": want, "got": got})
        report[styles[sid]] = {"style": sid, "accent_hits": hits, "of": len(ACCENT_WORDS),
                               "synth_s_p50": float(np.percentile(times, 50)), "rows": rows}
        print(f"{styles[sid]}: accent {hits}/{len(ACCENT_WORDS)}", flush=True)
    Path(a.out).mkdir(parents=True, exist_ok=True)
    (Path(a.out) / "voicevox.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
