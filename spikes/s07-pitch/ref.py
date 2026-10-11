"""Reference comparison: the tutor voice speaks the meant accent and its wrong variants.

In the app the tutor voice says the sentence first. Here OpenJTalk (HTS voice 'mei', the voice
whose accent is OpenJTalk's by construction) speaks every clip's sentence as meant, and once per
variant with one accent phrase changed to a wrong nucleus (the NJD accent of a word in that phrase
is changed until the full-context labels show the wanted phrase accent). Each reference is aligned
and measured like the learner (CTC moras, Praat, median per mora). A phrase scores against a
reference by the squared error of x = c + b * ref (c free; 'ref-fest': b = 1, 'ref-frei': b >= 0
fitted). Checks and folds as in pitch.py ('alle' is the same as 'nah' here: only the near variants
are spoken).

  python -I ref.py --model ../../target/kana-ctc-v2/model --work ../../target/pitch --out results \
      --template ../../target/pitch-consensus/clips.json
"""

import argparse
import json
import re
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))  # -I leaves the script's directory out
import pitch  # noqa: E402  (puts ../s06-moren on the path)

FITS = ("ref-fest", "ref-frei")


def phrase_accents(labels):
    """Accent type per accent phrase, in order, from full-context labels."""
    out, last = [], None
    for lab in labels:
        ph = lab.split("-")[1].split("+")[0]
        if ph in ("sil", "pau"):
            continue
        f = re.search(r"/F:(\d+)_(\d+)#\d+_\w+@(\d+)_", lab)
        bg = int(re.search(r"/I:\d+-\d+@(\d+)\+", lab).group(1))
        key = (bg, int(f.group(3)))
        if key != last:
            out.append((int(f.group(1)), int(f.group(2))))
            last = key
    return out


def variant_labels(text, i, target):
    """Labels with phrase i spoken with canonical nucleus `target`, others unchanged; or None."""
    import copy
    import pyopenjtalk
    feats = pyopenjtalk.run_frontend(text)
    base = phrase_accents(pyopenjtalk.make_label(feats))
    n = base[i][0]
    if pitch.canon(base[i][1], n) == target:
        return pyopenjtalk.make_label(feats)
    # nodes inside phrase i (by mora offset)
    m0 = sum(m for m, _ in base[:i])
    off, cand = 0, []
    for j, f in enumerate(feats):
        if m0 <= off < m0 + n and f["mora_size"] > 0:
            cand.append(j)
        off += max(f["mora_size"], 0)
    for j in cand:
        for a in range(0, feats[j]["mora_size"] + 1):
            fs = copy.deepcopy(feats)
            fs[j]["acc"] = a
            labs = pyopenjtalk.make_label(fs)
            acc = phrase_accents(labs)
            if len(acc) != len(base) or any(x != y for q, (x, y) in enumerate(zip(acc, base)) if q != i):
                continue
            if acc[i][0] == n and pitch.canon(acc[i][1], n) == target:
                return labs
    return None


def tts_worker(args):
    """(key, 16 kHz audio) for each wanted variant of one clip."""
    import librosa
    import pyopenjtalk
    uid, text, wants = args
    out = []
    for i, a in wants:
        labs = variant_labels(text, i, a) if i is not None else pyopenjtalk.extract_fullcontext(text)
        if labs is None:
            out.append((i, a, None))
            continue
        x, sr = pyopenjtalk.synthesize(labs)
        y = librosa.resample((x / 32768.0).astype(np.float32), orig_sr=sr, target_sr=pitch.SR)
        y, _ = librosa.effects.trim(y, top_db=35)
        out.append((i, a, y))
    return uid, out


def f0_worker(args):
    audio, times = args
    t, f = pitch.f0_track(audio, "praat")
    return pitch.mora_f0((t, f), times, "median")


def ref_sse(xs, r, fit):
    xs, r = np.array(xs, float), np.array(r, float)
    ok = ~np.isnan(xs) & ~np.isnan(r)
    if ok.sum() < 2:
        return None
    x, r = xs[ok], r[ok]
    if fit == "ref-frei":
        rc = r - r.mean()
        b = max(0.0, float((rc * (x - x.mean())).sum() / max((rc ** 2).sum(), 1e-9)))
    else:
        b = 1.0
    e = x - b * r
    return float(((e - e.mean()) ** 2).sum())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--work", required=True, help="work directory of pitch.py (measured.json)")
    ap.add_argument("--out", required=True)
    ap.add_argument("--template", help="clips.json of consensus.py (levels for the 'templ' fit)")
    args = ap.parse_args()
    import torch
    work = Path(args.work)
    recs = json.loads((work / "measured.json").read_text())
    origs = [r for r in recs if r["kind"] == "original"]
    refs_file = work / "refs.json"
    if refs_file.exists():
        refs = json.loads(refs_file.read_text())
    else:
        jobs = []
        for o in origs:
            wants = [(None, None)]
            for i, (n, k, _, _) in enumerate(o["phrases"]):
                if n < 2:
                    continue
                for a in set(pitch.near(k, n)) | set(pitch.error_kinds(k, n).values()):
                    wants.append((i, a))
            jobs.append((o["uid"], o["sentence"], wants))
        al = pitch.Aligner(args.model, "cuda" if torch.cuda.is_available() else "cpu")
        kana = {o["uid"]: o["kana"] for o in origs}
        refs, f0jobs, keys = {}, [], []
        with ProcessPoolExecutor(20) as ex:
            for uid, out in ex.map(tts_worker, jobs, chunksize=2):
                for i, a, y in out:
                    if y is None:
                        continue
                    times = al.mora_times(y, kana[uid])
                    if times is None:
                        continue
                    f0jobs.append((y, times))
                    keys.append((uid, i, a))
            for (uid, i, a), x in zip(keys, ex.map(f0_worker, f0jobs, chunksize=8)):
                refs.setdefault(uid, {})[f"{i}/{a}"] = x
        refs_file.write_text(json.dumps(refs))
    print(len(refs), "clips with references", flush=True)

    speakers = sorted({r["speaker"] for r in recs})
    half = set(speakers[::2])
    summary = {}
    by_uid = {}
    for r in recs:
        by_uid.setdefault(r["uid"], []).append(r)
    for seg in pitch.SEGS:
        for fit in FITS:
            tr = []
            for uid, rs in by_uid.items():
                ref = refs.get(uid)
                if not ref:
                    continue
                for r in rs:
                    x = pitch.values_of(r, "praat", seg, "median")
                    if x is None:
                        continue
                    for i, (n, k, bg, m0) in enumerate(r["phrases"]):
                        if n < 2 or (r["kind"] not in ("original", "sham") and i != r["phrase"]):
                            continue
                        k0 = pitch.canon(k, n)
                        base = ref.get("None/None")
                        if base is None:
                            continue
                        xs = x[m0:m0 + n]
                        cand = {k0: base[m0:m0 + n]}
                        for a in set(pitch.near(k, n)) | set(pitch.error_kinds(k, n).values()):
                            v = ref.get(f"{i}/{a}")
                            if v is not None:
                                cand[a] = v[m0:m0 + n]
                        sc = {a: ref_sse(xs, rv, fit) for a, rv in cand.items()}
                        sc = {a: v for a, v in sc.items() if v is not None}
                        if k0 not in sc or len(sc) < 2:
                            continue
                        near = [a for a in pitch.near(k, n) if a in sc]
                        j = min(near, key=lambda a: sc[a]) if near else None
                        d = (sc[k0] - sc[j], j) if j is not None else (None, None)
                        pairs = {kind: sc[k0] - sc[a] for kind, a in pitch.error_kinds(k, n).items() if a in sc}
                        tr.append({"uid": uid, "speaker": r["speaker"], "kind": r["kind"], "step": r["step"],
                                   "phrase": i, "n": n, "acc": k0, "spoken": r["spoken"], "bg": bg,
                                   "first": i == 0, "last": i == len(r["phrases"]) - 1,
                                   "d": {"alle": d, "nah": d}, "pairs": pairs,
                                   "rank1": all(sc[k0] < sc[a] for a in near),
                                   "best": min(sc, key=lambda a: sc[a])})
            for t in tr:
                t["fold"] = t["speaker"] in half
                t["error"] = t["kind"] not in ("original", "sham", "sham-lage")
            summary[f"praat/{seg}/median/{fit}"] = pitch.evaluate(tr)
    # Upper bound: the TTS voice as the speaker. Correct = the sentence as OpenJTalk accents it;
    # wrong = the same sentence spoken with one phrase's nucleus changed (real wrong-accent audio,
    # no resynthesis). Pattern fit as in pitch.py ('fest' with the voice's own step, 'templ' with
    # the levels learned from the consensus speakers when --template is given); folds by sentence.
    templ = None
    if args.template:
        tc = json.loads(Path(args.template).read_text())
        templ = pitch.learn_levels(tc, "praat", "median")
        templ["_w"] = pitch.learn_weights(tc, "praat", "median", templ)
    clips = sorted(refs)
    half = set(clips[::2])
    for fit in ("fest", "frei") + (("templ", "gelernt") if templ else ()):
        tr = []
        for o in origs:
            ref = refs.get(o["uid"], {})
            base = ref.get("None/None")
            if base is None:
                continue
            step = pitch.clip_step(o["phrases"], base)
            for i, (n, k, bg, m0) in enumerate(o["phrases"]):
                if n < 2:
                    continue
                k0 = pitch.canon(k, n)
                versions = [("original", k0, base)] + [
                    (kind, a, ref[f"{i}/{a}"]) for kind, a in pitch.error_kinds(k, n).items() if f"{i}/{a}" in ref]
                for kind, a, x in versions:
                    xs = x[m0:m0 + n]
                    if sum(not np.isnan(v) for v in xs) < 2:
                        continue
                    sc = pitch.phrase_scores(xs, n, step, templ, fit)[fit]
                    alts = {"alle": [j for j in range(n) if j != k0], "nah": pitch.near(k, n)}
                    d = {}
                    for c, al in alts.items():
                        j = min(al, key=lambda j: sc[j]) if al else None
                        d[c] = (sc[k0] - sc[j], j) if j is not None else (None, None)
                    tr.append({"uid": o["uid"], "speaker": o["uid"], "kind": kind, "step": "eigen",
                               "phrase": i, "n": n, "acc": k0, "spoken": a, "bg": bg,
                               "first": i == 0, "last": i == len(o["phrases"]) - 1, "d": d,
                               "pairs": {kk: sc[k0] - sc[aa] for kk, aa in pitch.error_kinds(k, n).items()},
                               "rank1": all(sc[k0] < sc[j] for j in range(n) if j != k0),
                               "best": min(range(n), key=lambda j: sc[j]),
                               "fold": o["uid"] in half, "error": kind != "original"})
        summary[f"tts-selbst/praat/ctc/median/{fit}"] = pitch.evaluate(tr)
    report = {"source": "Common Voice 19.0 ja test clips of pitch.py; references: OpenJTalk (mei) per variant",
              "fa_max": pitch.FA_MAX, "summary": summary}
    Path(args.out).mkdir(parents=True, exist_ok=True)
    (Path(args.out) / "pitch-ref.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))
    print(pitch.table(summary))


if __name__ == "__main__":
    main()
