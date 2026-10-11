"""Who is wrong where OpenJTalk's accent and the measured contour disagree: OpenJTalk or the speaker?

Common Voice `other` has sentences read by many people. For sentences with at least --min-speakers
different speakers (clips without down votes, none from the test split), every clip is aligned and
measured as in pitch.py (CTC moras, Praat, median per mora, fixed step). Then per accent phrase:

  single:    is OpenJTalk's pattern the best fit for one speaker (as in pitch.py)?
  pooled:    is it the best fit for the mean contour of all speakers of the sentence? Speaker noise
             averages out, a reading that everyone shares (or a bias of the measurement) does not.
  pairs:     do two speakers of the same sentence get the same best pattern?
  majority:  the pattern most speakers fit best, and whether it is OpenJTalk's.

  python -I consensus.py --cv ../../target/cv19-ja --model ../../target/kana-ctc-v2/model \
      --work ../../target/pitch-consensus --out results
"""

import argparse
import collections
import csv
import json
import random
import sys
from concurrent.futures import ProcessPoolExecutor
from itertools import combinations
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))  # -I leaves the script's directory out
import pitch  # noqa: E402  (puts ../s06-moren on the path)
import ctc  # noqa: E402

F0M, VALUE, FIT = "praat", "median", "fest"


def rows_of(path):
    return list(csv.DictReader(open(path, encoding="utf-8"), delimiter="\t", quoting=csv.QUOTE_NONE))


def load(path):
    import librosa
    a, _ = librosa.load(path, sr=pitch.SR)
    a, _ = librosa.effects.trim(a, top_db=35)
    return a


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cv", required=True)
    ap.add_argument("--model", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--sentences", type=int, default=60)
    ap.add_argument("--min-speakers", type=int, default=6)
    ap.add_argument("--max-speakers", type=int, default=10)
    args = ap.parse_args()
    import torch
    cv, work = Path(args.cv), Path(args.work)
    work.mkdir(parents=True, exist_ok=True)
    test_speakers = {r["client_id"] for r in rows_of(cv / "test.tsv")}
    by_sent = collections.defaultdict(dict)
    for r in rows_of(cv / "other.tsv"):
        if r["client_id"] in test_speakers or (r["down_votes"] or "0") != "0":
            continue
        by_sent[r["sentence"]].setdefault(r["client_id"], r)
    cands = [s for s, sp in by_sent.items() if len(sp) >= args.min_speakers]
    rng = random.Random(0)
    rng.shuffle(cands)
    where = {f.name: f for d in (cv / "audio").iterdir() if d.name.startswith("ja_other") for f in d.iterdir()}
    al = pitch.Aligner(args.model, "cuda" if torch.cuda.is_available() else "cpu")

    sentences, n_clips = [], 0
    for s in cands:
        kana = pitch.pron(s)
        phr, nm = pitch.accent_phrases(s)
        if nm != len(ctc.moras(kana)) or al.ids(kana) is None or not any(n >= 2 for n, *_ in phr):
            continue
        rows = sorted(by_sent[s].values(), key=lambda r: r["path"])
        rng.shuffle(rows)
        clips = []
        for r in rows:
            if r["path"] not in where:
                continue
            a = load(where[r["path"]])
            if len(a) < 0.5 * pitch.SR or pitch.ctc_cer(al.heard(al.logp(a)), al.ids(kana)) > 0.2:
                continue
            times = al.mora_times(a, kana)
            if times is None:
                continue
            clips.append({"uid": Path(r["path"]).stem, "speaker": r["client_id"][:12], "audio": a, "times": times})
            if len(clips) == args.max_speakers:
                break
        if len(clips) >= args.min_speakers:
            sentences.append({"sentence": s, "kana": kana, "phrases": phr, "clips": clips})
            n_clips += len(clips)
            print(len(sentences), s, len(clips), flush=True)
        if len(sentences) == args.sentences:
            break

    flat = [c for s in sentences for c in s["clips"]]
    with ProcessPoolExecutor(20) as ex:
        for c, tr in zip(flat, ex.map(pitch.track_worker, [c["audio"] for c in flat], chunksize=4)):
            c["track"] = tr
            t, f = tr[F0M]
            c["x"] = pitch.mora_f0((np.array(t), np.array(f)), c["times"], VALUE)
    # every clip as measured, for the template that pitch.py learns from these speakers (--template)
    (work / "clips.json").write_text(json.dumps(
        [{"sentence": s["sentence"], "kana": s["kana"], "phrases": s["phrases"], "uid": c["uid"],
          "speaker": c["speaker"], "times_ctc": c["times"], "track": c["track"]}
         for s in sentences for c in s["clips"]], ensure_ascii=False))

    rows = []
    for s in sentences:
        clips = [c for c in s["clips"] if c["x"] is not None]
        steps = [pitch.clip_step(s["phrases"], c["x"]) for c in clips]
        X = np.array([c["x"] for c in clips], float)
        for i, (n, k, bg, m0) in enumerate(s["phrases"]):
            if n < 2:
                continue
            k0 = pitch.canon(k, n)
            best = []
            for c, st in zip(clips, steps):
                xs = c["x"][m0:m0 + n]
                if sum(not np.isnan(v) for v in xs) < 2:
                    continue
                sc = pitch.phrase_scores(xs, n, st)[FIT]
                best.append(min(range(n), key=lambda j: sc[j]))
            if len(best) < 3:
                continue
            # pooled contour: mean over speakers (each already re its own median)
            xm = np.nanmean(X[:, m0:m0 + n], axis=0)
            sc = pitch.phrase_scores(xm, n, float(np.median(steps)))[FIT]
            pooled = min(range(n), key=lambda j: sc[j])
            cnt = collections.Counter(best)
            maj, maj_n = cnt.most_common(1)[0]
            m = ctc.moras(s["kana"])
            rows.append({"sentence": s["sentence"], "phrase": "".join(m[m0:m0 + n]), "n": n, "openjtalk": k0,
                         "speakers": len(best), "best": best, "pooled": pooled,
                         "pooled_contour": [round(float(v), 2) for v in xm],
                         "majority": maj, "majority_share": maj_n / len(best),
                         "single_agree": float(np.mean([b == k0 for b in best])),
                         "pair_agree": float(np.mean([a == b for a, b in combinations(best, 2)]))})

    def mean(key, sel=None):
        sel = rows if sel is None else sel
        return [float(np.mean([r[key] for r in sel])), len(sel)] if sel else [None, 0]

    clear = [r for r in rows if r["majority_share"] >= 0.6]

    def kind(r):
        """How the majority differs from OpenJTalk: the two shapes a measurement bias would give
        (a late peak, a falling heiban phrase), or something else."""
        a, b = r["openjtalk"], r["majority"]
        if a == b:
            return "gleich"
        if a >= 1 and b == a + 1:
            return "Kern eine More spaeter"
        if a == 0 and b == 1:
            return "flach als Kopf"
        return "anders"
    summary = {
        "sentences": len(sentences), "clips": n_clips, "phrases": len(rows),
        "single_agree": mean("single_agree"),
        "pooled_agree": [float(np.mean([r["pooled"] == r["openjtalk"] for r in rows])), len(rows)],
        "speaker_pair_agree": mean("pair_agree"),
        "majority_is_openjtalk": [float(np.mean([r["majority"] == r["openjtalk"] for r in rows])), len(rows)],
        "clear_majority (>=60 %)": {
            "share": len(clear) / max(1, len(rows)),
            "is_openjtalk": float(np.mean([r["majority"] == r["openjtalk"] for r in clear])) if clear else None,
            "kinds": dict(collections.Counter(kind(r) for r in clear)),
            "not_openjtalk": [r["phrase"] + f" OJT {r['openjtalk']} / Mehrheit {r['majority']}"
                              for r in clear if r["majority"] != r["openjtalk"]]},
        # Nucleus on the mora before a final の (男の, 村の): Tokyo speech drops an odaka noun's accent
        # before の, so the phrase should be heiban; what do the speakers fit best?
        "nucleus_before_no": dict(collections.Counter(
            "flach" if r["majority"] == 0 else "wie OpenJTalk" if r["majority"] == r["openjtalk"] else "anders"
            for r in rows if r["phrase"].endswith("の") and r["n"] >= 3 and r["openjtalk"] == r["n"] - 1)),
        "by_n": {name: mean("single_agree", [r for r in rows if f(r["n"])])
                 for name, f in {"n=2": lambda n: n == 2, "n=3-4": lambda n: 3 <= n <= 4,
                                 "n>=5": lambda n: n >= 5}.items()} if rows else {},
    }
    Path(args.out).mkdir(parents=True, exist_ok=True)
    (Path(args.out) / "consensus-cv.json").write_text(
        json.dumps({"source": "Common Voice 19.0 ja, other split (CC0), sentences read by several speakers",
                    "config": f"{F0M}/ctc/{VALUE}/{FIT}", "summary": summary, "phrases": rows},
                   ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
