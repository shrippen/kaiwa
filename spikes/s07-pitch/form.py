"""Wrong accents with a natural shape: the change between two accents as the tutor voice makes it.

pitch.py moves the F0 of whole moras up or down by a step: a wrong accent as a clean H/L pattern.
Real accents are not that clean (the fall spreads over two moras, the peak comes late). Here the
offset per mora is what changes in OpenJTalk's own contour (refs.json of ref.py) when the phrase is
spoken with the wrong nucleus instead of the meant one, scaled to the speaker:

  form:       scaled by the clip's step / the TTS voice's step (the speaker's own range)
  form-voll:  unscaled (the TTS voice's range, about twice the speakers')

Same clips, same phrases and kinds as pitch.py, WORLD on the MFA moras. The versions go to
measured-form.json, which `pitch.py --analyse-only --extra` adds to the trials.

  python -I form.py --model ../../target/kana-ctc-v2/model --work ../../target/pitch
"""

import argparse
import json
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))  # -I leaves the script's directory out
import pitch  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--clips-dir", default=None, help="the clips/ of spike 0.6 (default: next to --work)")
    args = ap.parse_args()
    import librosa
    import torch
    work = Path(args.work)
    clips_dir = Path(args.clips_dir) if args.clips_dir else work.parent / "s06-real" / "clips"
    recs = json.loads((work / "measured.json").read_text())
    refs = json.loads((work / "refs.json").read_text())
    origs = {r["uid"]: r for r in recs if r["kind"] == "original"}
    chosen = {}
    for r in recs:
        if r["kind"] not in ("original", "sham", "sham-lage"):
            chosen.setdefault(r["uid"], set()).add(r["phrase"])
    tasks, metas = [], []
    for uid, phrases in chosen.items():
        o, ref = origs[uid], refs.get(uid, {})
        base = ref.get("None/None")
        if base is None:
            continue
        scale = o["own_step"] / pitch.clip_step(o["phrases"], base)
        jobs = []
        for i in sorted(phrases):
            n, k, _, m0 = o["phrases"][i]
            for kind, a in pitch.error_kinds(k, n).items():
                alt = ref.get(f"{i}/{a}")
                if alt is None:
                    continue
                diff = np.nan_to_num(np.array(alt[m0:m0 + n], float) - np.array(base[m0:m0 + n], float))
                jobs.append((kind, "form", i, a, (scale * diff).tolist()))
                jobs.append((kind, "form-voll", i, a, diff.tolist()))
        if jobs:
            audio, _ = librosa.load(clips_dir / f"{uid}.wav", sr=pitch.SR)
            tasks.append((audio, o["times_mfa"], o["phrases"], jobs))
            metas.append(o)
    al = pitch.Aligner(args.model, "cuda" if torch.cuda.is_available() else "cpu")
    out, audios = [], []
    with ProcessPoolExecutor(20) as ex:
        for o, vs in zip(metas, ex.map(pitch.jobs_worker, tasks, chunksize=2)):
            meta = {k: o[k] for k in ("uid", "speaker", "sentence", "kana", "phrases", "times_mfa", "own_step")}
            for kind, st, i, a, y in vs:
                out.append(meta | {"kind": kind, "step": st, "phrase": i, "spoken": a,
                                   "times_ctc": al.mora_times(y, o["kana"])})
                audios.append(y)
        for r, tr in zip(out, ex.map(pitch.track_worker, audios, chunksize=8)):
            r["track"] = tr
    (work / "measured-form.json").write_text(json.dumps(out, ensure_ascii=False))
    print(len(out), "versions", flush=True)


if __name__ == "__main__":
    main()
