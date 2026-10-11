"""Pre-study for /LF321/ (pitch accent, decision N-03: measurement only): can the F0 contour of real
native speech tell the accent OpenJTalk predicts from a wrong accent nucleus?

Material: the 500 Common Voice 19 ja test clips of spike 0.6 (real.py; one per speaker, CC0, the
kana model hears their text). Per clip:

  1. OpenJTalk (full-context labels) gives the accent phrases, their mora count and accent type.
  2. Mora times twice: the own kana CTC model (finetune.py, v2) aligns the pronounced kana, a mora
     runs from the first frame of its kana to the first frame of the next ('ctc'); and MFA's phones
     of spike 0.6, a mora from the end of the one before to the end of its vowel ('mfa').
  3. F0 per mora in semitones re the clip's median (Praat AC; pYIN as in ADR-005): the median of
     the mora's voiced frames ('median') or of their last 40 % ('spaet').
  4. Each accent phrase of n moras (n >= 2) is scored against every Tokyo pattern with nucleus
     k = 0 (heiban) .. n-1; k = n (odaka) looks like heiban inside the phrase and counts as such.
     Score of a pattern P (lower fits better): squared error of the fit F0 = c + b * P (c free per
     phrase); 'fest': b = the clip's own accent step, 'frei': b >= 0 fitted, 'trend': fixed step
     and a falling line. With --template (consensus.py's clips, other speakers): 'templ', the
     mean level of each mora role instead of H/L, and 'gelernt', a conditional logit over the
     candidates (fits and local falls as features, OpenJTalk's nucleus as the label).

Errors: there are no free learner recordings, so wrong accents are resynthesised (WORLD): in one
or two phrases per clip the F0 of the moras whose H/L would differ is moved by the step (the
clip's own, 2 or 4 semitones), with 25 ms ramps. Kinds: 'flach' (accented -> heiban), 'kern-1',
'kern+1' (nucleus one mora earlier/later), 'kopf' (heiban -> atamadaka). Controls: 'sham' (WORLD
resynthesis, F0 unchanged) and 'sham-lage' (the whole phrase moved up by the step: the pattern
stays, the joins are there). Every version is aligned and measured again, as the app would.

Checks (the alarm says 'not the intended pattern'): D = SSE(intended) - min SSE(alternatives) > t.
  alle: every other pattern of the phrase; nah: the near ones (k-1, k+1, heiban, atamadaka);
  paar: only the one alternative of a minimal pair (as /LF321/ wants hints only there).
Thresholds at 5 % false alarms on the correct phrases of half the speakers, counted on the other
half (and the other way round).

form.py adds wrong accents shaped like the tutor voice's (--extra measured-form.json).

  python -I pitch.py --clips ../../target/s06-real/clips.json --model ../../target/kana-ctc-v2/model \
      --work ../../target/pitch --out results
  python -I pitch.py ... --analyse-only --template ../../target/pitch-consensus/clips.json \
      --extra ../../target/pitch/measured-form.json
"""

import argparse
import json
import random
import re
import sys
import warnings
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np

warnings.filterwarnings("ignore")
sys.path.insert(0, str(Path(__file__).parent.parent / "s06-moren"))  # -I leaves the script's directory out
import ctc  # noqa: E402

SR = 16000
PAD = 0.5  # s of silence around the utterance for the CTC model (as in spike 0.6)
FRAME = 0.02  # s per wav2vec2 frame
TAIL = 0.12  # s a mora may last after its last kana frame (before a pause)
RAMP = 0.025  # s, half-width of the F0 ramp at a changed mora boundary
WORLD_FP = 5.0  # ms
FA_MAX = 0.05
STEPS = ("eigen", 2.0, 4.0)
F0_METHODS = ("praat", "pyin")
FITS = ("fest", "frei", "trend")
CHECKS = ("alle", "nah", "paar")


# ---------------------------------------------------------------- accent phrases (OpenJTalk)

def pron(text):
    import pyopenjtalk
    p = "".join(n["pron"] for n in pyopenjtalk.run_frontend(text))
    return "".join(c for c in ctc.kata_to_hira(p) if c in ctc.VOWEL or c in "んっー")


def accent_phrases(text):
    """[(n_moras, accent type, breath group, first mora index)] and the mora count, from
    OpenJTalk's full-context labels (A: mora position, F: phrase moras and accent, I: breath group)."""
    import pyopenjtalk
    phrases, moras, last = [], 0, None
    for lab in pyopenjtalk.extract_fullcontext(text):
        ph = lab.split("-")[1].split("+")[0]
        if ph in ("sil", "pau"):
            continue
        a2 = int(re.search(r"/A:-?\d+\+(\d+)\+", lab).group(1))
        f = re.search(r"/F:(\d+)_(\d+)#\d+_\w+@(\d+)_", lab)
        bg = int(re.search(r"/I:\d+-\d+@(\d+)\+", lab).group(1))
        n, acc, pos = int(f.group(1)), int(f.group(2)), int(f.group(3))
        key = (bg, pos, a2)
        if key != last:
            if a2 == 1:
                phrases.append([n, acc, bg, moras])
            moras += 1
            last = key
    return phrases, moras


def pattern(k, n):
    """Tokyo H/L per mora (1 = high) for nucleus k (0 = heiban; k = n looks like heiban here)."""
    if k == 1:
        return np.array([1] + [0] * (n - 1), float)
    if k == 0 or k >= n:
        return np.array([0] + [1] * (n - 1), float)
    return np.array([0] + [1] * (k - 1) + [0] * (n - k), float)


def canon(k, n):
    return 0 if k >= n else k


def near(k, n):
    """The nuclei a learner plausibly hits instead of k: one mora off, heiban, atamadaka."""
    k = canon(k, n)
    base = n if k == 0 else k
    return sorted({canon(j, n) for j in (base - 1, base + 1, 0, 1) if 0 <= j <= n} - {k})


def error_kinds(k, n):
    """Wrong nuclei to resynthesise: kind -> nucleus."""
    k = canon(k, n)
    out = {}
    if k == 0:
        out["kopf"] = 1
        if n >= 3:
            out["kern-1"] = n - 1
    else:
        out["flach"] = 0
        if k >= 2:
            out["kern-1"] = k - 1
        if k + 1 <= n - 1:
            out["kern+1"] = k + 1
    return out


# ---------------------------------------------------------------- alignment

def align(logp, ids, blank):
    """ctc.align of spike 0.6 (as merged), with the back-pointer step read as a Python int: there,
    `s -= back[t, s]` stays int8 and overflows for more than 63 kana."""
    T = logp.shape[0]
    ext = ctc._lattice(ids, blank)
    S = len(ext)
    neg = ctc.NEG
    delta = np.full(S, neg)
    delta[0] = logp[0, ext[0]]
    if S > 1:
        delta[1] = logp[0, ext[1]]
    skip = np.zeros(S, bool)
    skip[2:] = (ext[2:] != blank) & (ext[2:] != ext[:-2])
    back = np.zeros((T, S), np.int8)
    for t in range(1, T):
        cand = np.stack([delta, np.concatenate(([neg], delta[:-1])),
                         np.where(skip, np.concatenate(([neg, neg], delta[:-2])), neg)])
        k = cand.argmax(axis=0)
        back[t] = k
        delta = cand[k, np.arange(S)] + logp[t, ext]
    s = S - 1 if S == 1 or delta[-1] >= delta[-2] else S - 2
    starts, ends = [None] * len(ids), [None] * len(ids)
    for t in range(T - 1, -1, -1):
        if s % 2 == 1:
            j = s // 2
            if ends[j] is None:
                ends[j] = t
            starts[j] = t
        s -= int(back[t, s])
    return starts, ends


class Aligner:
    def __init__(self, path, device):
        import torch
        from transformers import AutoFeatureExtractor, AutoModelForCTC
        self.torch = torch
        self.fe = AutoFeatureExtractor.from_pretrained(path)
        self.model = AutoModelForCTC.from_pretrained(path, dtype=torch.float32).eval().to(device)
        self.vocab = json.loads((Path(path) / "vocab.json").read_text())
        self.blank = self.vocab["[PAD]"]
        self.delim = [self.vocab["|"]]
        self.device = device

    def logp(self, audio):
        x = np.pad(audio, int(PAD * SR))
        x = self.fe(x, sampling_rate=SR, return_tensors="pt").input_values.to(self.device)
        with self.torch.no_grad():
            lg = self.model(x).logits[0].float().cpu().numpy()
        return ctc.blank_merged(ctc.log_softmax(lg), self.blank, self.delim[0])

    def ids(self, kana):
        return [self.vocab[c] for c in kana] if all(c in self.vocab for c in kana) else None

    def heard(self, logp):
        ids = logp.argmax(-1)
        return [int(i) for i, p in zip(ids, np.concatenate(([-1], ids[:-1]))) if i != p and i != self.blank and i not in self.delim]

    def mora_times(self, audio, kana):
        """(start, end) in s per mora of `kana`, or None."""
        ids = self.ids(kana)
        if ids is None:
            return None
        lp = self.logp(audio)
        starts, ends = align(lp, ids, self.blank)
        if any(s is None for s in starts):
            return None
        out, i = [], 0
        for m in ctc.moras(kana):
            out.append((starts[i], ends[i + len(m) - 1]))
            i += len(m)
        times = []
        dur = len(audio) / SR
        for j, (s, e) in enumerate(out):
            t0 = s * FRAME - PAD
            nxt = out[j + 1][0] * FRAME - PAD if j + 1 < len(out) else dur
            t1 = min(nxt, (e + 1) * FRAME - PAD + TAIL)
            times.append((max(t0, 0.0), max(min(t1, dur), max(t0, 0.0) + FRAME)))
        return times


MFA_VOWELS = ("a", "i", "ɯ", "ɨ", "e", "o")


def mfa_mora_times(path, kana):
    """(start, end) per mora from MFA's phones (spike 0.6, real.py): a mora runs from the end of
    the mora before to the end of its vowel; a long vowel is split in half between its two moras,
    ん ends with its nasal, っ halfway through the long consonant. None when the phones do not fit."""
    ph = [(s, e, p) for s, e, p in json.loads(Path(path).read_text())["tiers"]["phones"]["entries"]
          if p not in ("", "sil", "sp", "spn")]
    if not ph:
        return None
    m = ctc.moras(kana)
    vow = [i for i, (_, _, p) in enumerate(ph) if p.rstrip("ː") in MFA_VOWELS]
    out, vi, pi, prev_end, shared = [], 0, 0, ph[0][0], None
    for j, mo in enumerate(m):
        ext = mo == "ー" or (j and m[j - 1] not in "んっー" and ctc.is_extension(m[j - 1], mo))
        if ext:
            if shared is None:
                return None
            end, shared = shared, None
            pi = vow[vi - 1] + 1
        elif mo in "んっ":
            if pi >= len(ph) or ph[pi][2].rstrip("ː") in MFA_VOWELS:
                return None
            s, e, p = ph[pi]
            if p.endswith("ː"):
                end = (s + e) / 2
            else:
                end = e
                pi += 1
        else:
            if vi >= len(vow):
                return None
            s, e, p = ph[vow[vi]]
            vi += 1
            pi = vow[vi - 1] + 1
            nxt = m[j + 1] if j + 1 < len(m) else None
            if p.endswith("ː") and nxt is not None and (nxt == "ー" or ctc.is_extension(mo, nxt)):
                end = (s + e) / 2
                shared = e
            else:
                end = e
        out.append((prev_end, end))
        prev_end = end
    if vi != len(vow):
        return None
    return out


# ---------------------------------------------------------------- F0

def f0_track(audio, method):
    """(times, f0 in Hz with 0 for unvoiced), 10 ms hop. Praat in two passes (range per voice)."""
    if method == "praat":
        import parselmouth
        snd = parselmouth.Sound(audio.astype(np.float64), SR)
        p = snd.to_pitch_ac(time_step=0.01, pitch_floor=60, pitch_ceiling=600)
        f = p.selected_array["frequency"]
        v = f[f > 0]
        if len(v) > 10:
            lo, hi = np.percentile(v, [25, 75])
            p = snd.to_pitch_ac(time_step=0.01, pitch_floor=max(50, 0.75 * lo), pitch_ceiling=min(800, 1.5 * hi))
            f = p.selected_array["frequency"]
        return np.array(p.xs()), np.nan_to_num(f)
    import librosa
    f, vflag, _ = librosa.pyin(audio, fmin=60, fmax=600, sr=SR, frame_length=1024, hop_length=160)
    f = np.where(vflag, np.nan_to_num(f), 0.0)
    return np.arange(len(f)) * 0.01, f


def mora_f0(track, times, value="median"):
    """F0 per mora in semitones re the clip median; NaN with fewer than 2 voiced frames.
    value 'median': median of the mora's voiced frames; 'spaet': median of their last 40 % (the
    target the mora reaches; the fall after a nucleus starts late in the accented mora)."""
    t, f = track
    v = f[f > 0]
    if len(v) < 5 or not times:
        return None
    ref = np.median(v)
    out = []
    for s, e in times:
        sel = f[(t >= s) & (t < e) & (f > 0)]
        if value == "spaet":
            sel = sel[-max(2, int(np.ceil(0.4 * len(sel)))):]
        out.append(float(np.median(12 * np.log2(sel / ref))) if len(sel) >= 2 else float("nan"))
    return out


def clip_step(phr, x):
    """The clip's own accent step: median over phrases of mean(H) - mean(L) under OpenJTalk's
    pattern, limited to 1.5 .. 6 semitones."""
    d = []
    for n, k, _, m0 in phr:
        if n < 2:
            continue
        p = pattern(k, n)
        xs = np.array(x[m0:m0 + n])
        ok = ~np.isnan(xs)
        if (p[ok] == 1).any() and (p[ok] == 0).any():
            d.append(xs[ok][p[ok] == 1].mean() - xs[ok][p[ok] == 0].mean())
    return float(np.clip(np.median(d), 1.5, 6.0)) if d else 3.0


def sse(xs, p, fit, step):
    ok = ~np.isnan(xs)
    x, p = xs[ok], p[ok]
    if fit == "fest":
        r = x - step * p
        return float(((r - r.mean()) ** 2).sum())
    if fit == "trend":  # fixed step plus a falling line over the phrase (declination)
        r = x - step * p
        j = np.flatnonzero(ok).astype(float)
        jc = j - j.mean()
        d = min(0.0, float((jc * (r - r.mean())).sum() / (jc ** 2).sum()))
        r = r - d * jc
        return float(((r - r.mean()) ** 2).sum())
    if p.min() == p.max():
        return float(((x - x.mean()) ** 2).sum())
    pc = p - p.mean()
    b = max(0.0, float((pc * (x - x.mean())).sum() / (pc ** 2).sum()))
    r = x - b * p
    return float(((r - r.mean()) ** 2).sum())


def phrase_scores(xs, n, step, levels=None, only=None):
    """Score (lower = fits better) per canonical nucleus 0..n-1, per fit: the SSE of the pattern fits,
    'templ' with learned levels (fixed shape), 'gelernt' minus the learned logit. `only`: one fit."""
    xs = np.array(xs, float)
    fits = [only] if only else list(FITS) + (["templ"] if levels else []) + (["gelernt"] if levels and "_w" in levels else [])
    out = {}
    for fit in fits:
        if fit == "templ":
            out[fit] = {k: sse(xs, shape(k, n, levels), "fest", 1.0) for k in range(n)}
        elif fit == "gelernt":
            w = np.array(levels["_w"])
            out[fit] = {k: -float(w @ cand_features(xs, n, k, step, levels)) for k in range(n)}
        else:
            out[fit] = {k: sse(xs, pattern(k, n), fit, step) for k in range(n)}
    return out


def cand_features(xs, n, k, step, levels):
    """Features of candidate nucleus k for the learned scorer: the three fits (per mora) and the
    local cues (fall right after the nucleus, fall one mora later, rise into mora 2, the largest
    fall in a heiban candidate), each in a slot of its accent class."""
    ok = ~np.isnan(xs)
    nv = max(ok.sum(), 1)

    def val(j):
        return xs[j] if 0 <= j < n and ok[j] else np.nan

    fits = [sse(xs, pattern(k, n), "fest", step) / nv, sse(xs, pattern(k, n), "frei", step) / nv,
            sse(xs, shape(k, n, levels), "fest", 1.0) / nv]
    rise = val(1) - val(0)
    if k == 0:
        drops = [val(j) - val(j + 1) for j in range(n - 1)]
        drops = [d for d in drops if not np.isnan(d)]
        cues = [0.0, 0.0, rise, max(drops) if drops else 0.0]
        cls = 0
    else:
        cues = [val(k - 1) - val(k), val(k) - val(k + 1), -rise if k == 1 else rise, 0.0]
        cls = 1 if k == 1 else 2
    cues = [0.0 if np.isnan(c) else float(c) for c in cues]
    slots = np.zeros(3 * len(cues))
    slots[cls * len(cues):(cls + 1) * len(cues)] = cues
    return np.concatenate([fits, slots])


def learn_weights(clips, f0m, value, levels, l2=1.0):
    """Conditional logit over the candidate nuclei of each phrase, OpenJTalk's nucleus as the label
    (consensus speakers, not the test split)."""
    from scipy.optimize import minimize
    data = []
    for c in clips:
        t, f = c["track"][f0m]
        x = mora_f0((np.array(t), np.array(f)), c["times_ctc"], value)
        if x is None:
            continue
        step = clip_step(c["phrases"], x)
        for n, k, _, m0 in c["phrases"]:
            xs = np.array(x[m0:m0 + n], float)
            if n < 2 or (~np.isnan(xs)).sum() < 2:
                continue
            data.append((np.array([cand_features(xs, n, j, step, levels) for j in range(n)]), canon(k, n)))

    def loss(w):
        tot, g = l2 * (w @ w), 2 * l2 * w
        for F, y in data:
            z = F @ w
            z = z - z.max()
            p = np.exp(z) / np.exp(z).sum()
            tot -= np.log(p[y] + 1e-12)
            g += F.T @ p - F[y]
        return tot / len(data), g / len(data)

    d = data[0][0].shape[1]
    w = minimize(loss, np.zeros(d), jac=True, method="L-BFGS-B").x
    return [round(float(v), 4) for v in w]


# ---------------------------------------------------------------- learned template

CATS = ("flach-1", "flach-H", "kopf-1", "anfang-L", "vor-H", "kern-H", "nach-1", "nach-2+")


def category(j, k, n):
    """Role of mora j (0-based) in a phrase of n moras with canonical nucleus k."""
    k = canon(k, n)
    if k == 0:
        return "flach-1" if j == 0 else "flach-H"
    if j == 0:
        return "kopf-1" if k == 1 else "anfang-L"
    if j < k - 1:
        return "vor-H"
    if j == k - 1:
        return "kern-H"
    return "nach-1" if j == k else "nach-2+"


def shape(k, n, levels):
    return np.array([levels[category(j, k, n)] for j in range(n)], float)


def learn_levels(clips, f0m, value):
    """Mean level of each mora role (semitones, re the phrase mean) by least squares over every
    phrase of every clip, with OpenJTalk's accent as the truth (speakers of the consensus set,
    none of them in the test split)."""
    rows, ys = [], []
    for c in clips:
        t, f = c["track"][f0m]
        x = mora_f0((np.array(t), np.array(f)), c["times_ctc"], value)
        if x is None:
            continue
        for n, k, _, m0 in c["phrases"]:
            if n < 2:
                continue
            xs = np.array(x[m0:m0 + n])
            ok = ~np.isnan(xs)
            if ok.sum() < 2:
                continue
            a = np.zeros((n, len(CATS)))
            for j in range(n):
                a[j, CATS.index(category(j, k, n))] = 1
            a, y = a[ok], xs[ok]
            rows.append(a - a.mean(axis=0))
            ys.append(y - y.mean())
    sol = np.linalg.lstsq(np.vstack(rows), np.concatenate(ys), rcond=None)[0]
    return {c: round(float(v), 3) for c, v in zip(CATS, sol)}


# ---------------------------------------------------------------- resynthesis (WORLD)

def world_versions(audio, times, phr, step_own, rng, jobs=None):
    """Resynthesised versions: (kind, step, phrase index, nucleus spoken, audio). `jobs`: instead
    of the default versions, these (kind, step, phrase index, nucleus, offset per mora)."""
    import pyworld
    x = audio.astype(np.float64)
    f0, tt = pyworld.harvest(x, SR, frame_period=WORLD_FP, f0_floor=60, f0_ceil=600)
    sp = pyworld.cheaptrick(x, f0, tt, SR)
    ap = pyworld.d4c(x, f0, tt, SR)

    def synth(offset_st):
        y = pyworld.synthesize(f0 * 2 ** (offset_st / 12), sp, ap, SR, WORLD_FP)[:len(audio)]
        y = np.pad(y, (0, len(audio) - len(y)))
        return np.clip(y, -1, 1).astype(np.float32)

    def curve(i, per_mora):
        """Offset in semitones per WORLD frame: per_mora[j] for mora j of phrase i, ramped."""
        n, _, _, m0 = phr[i]
        pts_t, pts_v = [tt[0]], [0.0]
        vals = list(per_mora)
        seg = [times[m0 + j] for j in range(n)]
        prev = 0.0
        for j, ((s, e), v) in enumerate(zip(seg, vals)):
            if v != prev:
                pts_t += [s - RAMP, s + RAMP]
                pts_v += [prev, v]
            prev = v
        if prev != 0.0:
            e = seg[-1][1]
            pts_t += [e - RAMP, e + RAMP]
            pts_v += [prev, 0.0]
        pts_t.append(tt[-1] + 1)
        pts_v.append(0.0)
        order = np.argsort(pts_t, kind="stable")
        return np.interp(tt, np.array(pts_t)[order], np.array(pts_v)[order])

    if jobs is not None:
        return [(kind, st, i, a, synth(curve(i, off))) for kind, st, i, a, off in jobs]
    out = [("sham", None, None, None, synth(np.zeros_like(f0)))]
    elig = [i for i, (n, k, _, m0) in enumerate(phr) if n >= 2 and error_kinds(k, n)]
    for i in rng.sample(elig, min(2, len(elig))):
        n, k, _, m0 = phr[i]
        p = pattern(k, n)
        out.append(("sham-lage", step_own, i, canon(k, n), synth(curve(i, [step_own] * n))))
        for kind, a in error_kinds(k, n).items():
            for st in STEPS:
                d = step_own if st == "eigen" else st
                out.append((kind, st, i, a, synth(curve(i, d * (pattern(a, n) - p)))))
    return out


# ---------------------------------------------------------------- workers

def track_worker(audio):
    """F0 tracks of one version: method -> (times, f0 Hz)."""
    out = {}
    for m in F0_METHODS:
        t, f = f0_track(audio, m)
        out[m] = (np.round(t, 4).tolist(), np.round(f, 1).tolist())
    return out


def resynth_worker(args):
    audio, times, phr, step, seed = args
    return world_versions(audio, times, phr, step, random.Random(seed))


def jobs_worker(args):
    audio, times, phr, jobs = args
    return world_versions(audio, times, phr, None, None, jobs)


# ---------------------------------------------------------------- analysis

SEGS = ("ctc", "mfa")
VALUES = ("median", "spaet")


def configs():
    return [(m, s, v, f) for m in F0_METHODS for s in SEGS for v in VALUES for f in FITS]


def values_of(rec, f0m, seg, value):
    times = rec["times_ctc"] if seg == "ctc" else rec["times_mfa"]
    t, f = rec["track"][f0m]
    return mora_f0((np.array(t), np.array(f)), times, value)


def trials_of(rec, x, step, fit, levels=None):
    """Phrase-level trials of one measured version: intended nucleus, D per check, best alternative."""
    if x is None:
        return []
    out = []
    for i, (n, k, bg, m0) in enumerate(rec["phrases"]):
        if n < 2:
            continue
        if rec["kind"] not in ("original", "sham") and i != rec["phrase"]:
            continue
        xs = x[m0:m0 + n]
        if sum(not np.isnan(v) for v in xs) < 2:
            continue
        sc = phrase_scores(xs, n, step, levels, fit)[fit]
        k0 = canon(k, n)
        alts = {"alle": [j for j in range(n) if j != k0], "nah": near(k, n)}
        d = {}
        for c, al in alts.items():
            j = min(al, key=lambda j: sc[j]) if al else None
            d[c] = (sc[k0] - sc[j], j) if j is not None else (None, None)
        # paar: intended against each minimal-pair partner (the wrong nuclei we resynthesise)
        pairs = {kind: sc[k0] - sc[a] for kind, a in error_kinds(k, n).items()}
        out.append({"uid": rec["uid"], "speaker": rec["speaker"], "kind": rec["kind"], "step": rec["step"],
                    "phrase": i, "n": n, "acc": k0, "spoken": rec["spoken"], "bg": bg,
                    "first": i == 0, "last": i == len(rec["phrases"]) - 1,
                    "d": d, "pairs": pairs, "rank1": all(sc[k0] < sc[j] for j in range(n) if j != k0),
                    "best": min(range(n), key=lambda j: sc[j])})
    return out


def analyse(recs, templates=None):
    speakers = sorted({r["speaker"] for r in recs})
    half = set(speakers[::2])
    summary, phrase_rows = {}, {}
    by_uid = {}
    for r in recs:
        by_uid.setdefault(r["uid"], []).append(r)
    cfgs = configs() + ([(m, sg, v, fit) for fit in ("templ", "gelernt") for m in F0_METHODS for sg in SEGS
                         for v in VALUES] if templates else [])
    for f0m, seg, value, fit in cfgs:
        levels = templates.get(f"{f0m}/{value}") if templates else None
        tr = []
        for uid, rs in by_uid.items():
            orig = next(r for r in rs if r["kind"] == "original")
            xo = values_of(orig, f0m, seg, value)
            if xo is None:
                continue
            step = clip_step(orig["phrases"], xo)
            for r in rs:
                tr += trials_of(r, xo if r is orig else values_of(r, f0m, seg, value), step, fit, levels)
        for t in tr:
            t["fold"] = t["speaker"] in half
            t["error"] = t["kind"] not in ("original", "sham", "sham-lage")
        key = f"{f0m}/{seg}/{value}/{fit}"
        summary[key] = evaluate(tr)
        phrase_rows[key] = [{k: t[k] for k in ("uid", "phrase", "n", "acc", "best", "rank1")}
                            for t in tr if t["kind"] == "original"]
    return summary, phrase_rows


def items(t, check):
    """(threshold group, statistic) of a trial. alle/nah: one, a common threshold. paar: one per
    minimal-pair partner, each kind with its own threshold (a correct phrase is tested against every
    partner it has, a wrong one against the pattern it was given)."""
    if check != "paar":
        return [(None, t["d"][check][0])] if t["d"][check][0] is not None else []
    if t["error"]:
        return [(t["kind"], t["pairs"][t["kind"]])] if t["kind"] in t["pairs"] else []
    return list(t["pairs"].items())


def evaluate(tr):
    orig = [t for t in tr if t["kind"] == "original"]
    res = {"phrases": len(orig), "agreement_rank1": _rate([t["rank1"] for t in orig])}
    by = {}
    for name, f in {"heiban": lambda t: t["acc"] == 0, "atamadaka": lambda t: t["acc"] == 1,
                    "nakadaka": lambda t: t["acc"] >= 2,
                    "n=2": lambda t: t["n"] == 2, "n=3-4": lambda t: 3 <= t["n"] <= 4,
                    "n=5-7": lambda t: 5 <= t["n"] <= 7, "n>=8": lambda t: t["n"] >= 8,
                    "erste Phrase": lambda t: t["first"], "letzte Phrase": lambda t: t["last"],
                    "mitte": lambda t: not t["first"] and not t["last"]}.items():
        by[name] = _rate([t["rank1"] for t in orig if f(t)])
    res["agreement_by"] = by
    # How the best pattern differs from OpenJTalk's where they disagree (nucleus later/earlier, flat)
    diff = {"kern spaeter": 0, "kern frueher": 0, "flach statt Kern": 0, "Kern statt flach": 0}
    for t in orig:
        if t["rank1"]:
            continue
        a, b = t["acc"], t["best"]
        if a == 0:
            diff["Kern statt flach"] += 1
        elif b == 0:
            diff["flach statt Kern"] += 1
        elif b > a:
            diff["kern spaeter"] += 1
        else:
            diff["kern frueher"] += 1
    res["disagreement"] = diff
    res["checks"] = {}
    for check in CHECKS:
        out = {"thresholds": {}, "false_alarms": {}, "detection": {}, "detection_by_kind": {}, "place_right": None}
        its = [(t, g, v) for t in tr for g, v in items(t, check)]
        alarm = {}
        for fold in (True, False):
            groups = {g for _, g, _ in its}
            for g in groups:
                cal = [v for t, gg, v in its if gg == g and t["fold"] != fold and not t["error"]]
                th = float(np.quantile(cal, 1 - FA_MAX)) if cal else float("inf")
                out["thresholds"].setdefault(str(g), []).append(th)
                for t, gg, v in its:
                    if gg == g and t["fold"] == fold:
                        alarm[(id(t), g)] = v > th

        def rate(f):
            return _rate([alarm[(id(t), g)] for t, g, v in its if f(t, g)])

        out["false_alarms"] = {k: rate(lambda t, g, k=k: t["kind"] == k) for k in ("original", "sham", "sham-lage")}
        out["false_alarms"]["alle richtigen"] = rate(lambda t, g: not t["error"])
        if check == "paar":
            out["false_alarms_by_kind"] = {k: rate(lambda t, g, k=k: not t["error"] and g == k)
                                           for k in ("flach", "kern-1", "kern+1", "kopf")}
        steps = [st for st in STEPS if any(t["step"] == st for t in tr if t["error"])] + sorted(
            {t["step"] for t in tr if t["error"] and t["step"] not in STEPS}, key=str)
        # Area under the ROC curve per step (statistic of wrong vs correct items, pooled over kinds)
        # and detection at looser false-alarm rates (thresholds per group, same folds)
        corr = np.array([v for t, g, v in its if not t["error"]])
        out["auc"] = {str(st): _auc(corr, np.array([v for t, g, v in its if t["error"] and t["step"] == st]))
                      for st in steps}
        out["detection_at_fa"] = {}
        for fa in (0.10, 0.20):
            hit = []
            for fold in (True, False):
                for g in {gg for _, gg, _ in its}:
                    cal = [v for t, gg, v in its if gg == g and t["fold"] != fold and not t["error"]]
                    th = float(np.quantile(cal, 1 - fa)) if cal else float("inf")
                    hit += [(t["step"], v > th) for t, gg, v in its if gg == g and t["fold"] == fold and t["error"]]
            out["detection_at_fa"][str(fa)] = {str(st): _rate([h for s_, h in hit if s_ == st]) for st in steps}
        for st in steps:
            out["detection"][str(st)] = rate(lambda t, g, st=st: t["error"] and t["step"] == st)
            out["detection_by_kind"][str(st)] = {
                k: rate(lambda t, g, k=k, st=st: t["kind"] == k and t["step"] == st)
                for k in ("flach", "kern-1", "kern+1", "kopf")}
        if check != "paar":
            out["place_right"] = _rate([t["d"][check][1] == t["spoken"] for t, g, v in its
                                        if t["error"] and t["step"] == "eigen" and alarm[(id(t), g)]])
        out["detection_by_n"] = {name: rate(lambda t, g, f=f: t["error"] and t["step"] == "eigen" and f(t))
                                 for name, f in {"n=2": lambda t: t["n"] == 2, "n=3-4": lambda t: 3 <= t["n"] <= 4,
                                                 "n>=5": lambda t: t["n"] >= 5}.items()}
        res["checks"][check] = out
    return res


def _auc(neg, pos):
    """P(a wrong item scores above a correct one), ties half."""
    if len(neg) == 0 or len(pos) == 0:
        return None
    allv = np.concatenate([neg, pos])
    order = allv.argsort(kind="stable")
    ranks = np.empty(len(allv))
    ranks[order] = np.arange(1, len(allv) + 1)
    for v in np.unique(allv):  # average ranks of ties
        m = allv == v
        if m.sum() > 1:
            ranks[m] = ranks[m].mean()
    return float((ranks[len(neg):].sum() - len(pos) * (len(pos) + 1) / 2) / (len(neg) * len(pos)))


def _rate(xs):
    return [float(np.mean(xs)) if xs else None, len(xs)]


def table(summary):
    """Short text table: config, agreement, per check FA (originals) and detection (own/2/4 st)."""
    lines = []
    for key, s in summary.items():
        cells = [f"{key:28s} agr {s['agreement_rank1'][0]:.2f} (n={s['phrases']})"]
        for c in CHECKS:
            ch = s["checks"][c]
            fa = ch["false_alarms"]["alle richtigen"][0]
            det = "/".join(f"{v[0] or 0:.2f}" for v in ch["detection"].values())
            cells.append(f"{c}: FA {fa:.3f} det {det}")
        lines.append(" | ".join(cells))
    return "\n".join(lines)


# ---------------------------------------------------------------- main

def jsonable(r):
    return {k: v for k, v in r.items() if k != "audio"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--clips", required=True, help="clips.json of spike 0.6 (real.py); clips/ and aligned/ next to it")
    ap.add_argument("--model", required=True, help="own kana CTC model (finetune.py)")
    ap.add_argument("--work", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--keep-audio", type=int, default=30, help="clips whose versions are written as wav")
    ap.add_argument("--analyse-only", action="store_true", help="re-read measured.json")
    ap.add_argument("--extra", nargs="*", default=[], help="more measured versions (form.py)")
    ap.add_argument("--template", help="clips.json of consensus.py: learn the mora-role levels there")
    args = ap.parse_args()
    work = Path(args.work)
    measured = work / "measured.json"
    if args.analyse_only:
        recs = json.loads(measured.read_text())
        for e in args.extra:
            recs += json.loads(Path(e).read_text())
        skipped = json.loads((work / "skipped.json").read_text())
    else:
        recs, skipped = measure_all(args, work)
        measured.write_text(json.dumps(recs, ensure_ascii=False))
        (work / "skipped.json").write_text(json.dumps(skipped))
    templates = None
    if args.template:
        tc = json.loads(Path(args.template).read_text())
        templates = {f"{m}/{v}": learn_levels(tc, m, v) for m in F0_METHODS for v in VALUES}
        for key, lv in templates.items():
            m, v = key.split("/")
            lv["_w"] = learn_weights(tc, m, v, lv)
        print(json.dumps(templates, indent=1))
    summary, phrase_rows = analyse(recs, templates)
    n_clips = len({r["uid"] for r in recs})
    report = {"source": "Common Voice 19.0 ja, test split (CC0), clips of spike 0.6 (one per speaker)",
              "clips": n_clips, "skipped": skipped, "fa_max": FA_MAX,
              "versions": {k: sum(r["kind"] == k for r in recs) for k in sorted({r["kind"] for r in recs})},
              "templates": templates, "summary": summary}
    Path(args.out).mkdir(parents=True, exist_ok=True)
    (Path(args.out) / "pitch-cv.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))
    (work / "phrases.json").write_text(json.dumps(phrase_rows, ensure_ascii=False))
    print(table(summary))


def measure_all(args, work):
    import librosa
    import soundfile as sf
    import torch
    (work / "audio").mkdir(parents=True, exist_ok=True)
    clips = json.loads(Path(args.clips).read_text())
    if args.limit:
        clips = clips[:args.limit]
    clip_dir = Path(args.clips).parent / "clips"
    mfa_dir = Path(args.clips).parent / "aligned"
    al = Aligner(args.model, "cuda" if torch.cuda.is_available() else "cpu")

    # 1. accent phrases, alignment of the originals (CTC, and MFA from spike 0.6)
    base, skipped = [], {"moras": 0, "align": 0, "heard": 0, "mfa": 0}
    for c in clips:
        kana = pron(c["sentence"])
        phr, nm = accent_phrases(c["sentence"])
        if nm != len(ctc.moras(kana)):
            skipped["moras"] += 1
            continue
        audio, _ = librosa.load(clip_dir / f"{c['uid']}.wav", sr=SR)
        ids = al.ids(kana)
        if ids is None:
            skipped["align"] += 1
            continue
        if ctc_cer(al.heard(al.logp(audio)), ids) > 0.2:
            skipped["heard"] += 1
            continue
        times = al.mora_times(audio, kana)
        if times is None:
            skipped["align"] += 1
            continue
        mfa = mfa_dir / c["uid"] / f"{c['uid']}.json"
        tm = mfa_mora_times(mfa, kana) if mfa.exists() else None
        if tm is None:
            skipped["mfa"] += 1
            continue
        base.append({"uid": c["uid"], "speaker": c["speaker"], "sentence": c["sentence"], "kana": kana,
                     "phrases": phr, "times_ctc": times, "times_mfa": tm, "audio": audio})
    print(len(base), "clips", skipped, flush=True)

    # 2. F0 of the originals, the clip's step (Praat, MFA moras), resynthesis on the MFA moras
    with ProcessPoolExecutor(20) as ex:
        tracks = list(ex.map(track_worker, [b["audio"] for b in base], chunksize=4))
        for b, tr in zip(base, tracks):
            b["track"] = tr
            x = mora_f0((np.array(tr["praat"][0]), np.array(tr["praat"][1])), b["times_mfa"])
            b["step"] = clip_step(b["phrases"], x) if x else 3.0
        vers = list(ex.map(resynth_worker, [(b["audio"], b["times_mfa"], b["phrases"], b["step"], j)
                                            for j, b in enumerate(base)], chunksize=2))
    print("resynthesised", sum(len(v) for v in vers), flush=True)

    # 3. align and measure every version again (WORLD keeps the timing, so the MFA moras still hold)
    recs, jobs = [], []
    for j, (b, vs) in enumerate(zip(base, vers)):
        meta = {k: b[k] for k in ("uid", "speaker", "sentence", "kana", "phrases", "times_mfa")}
        recs.append(meta | {"kind": "original", "step": None, "phrase": None, "spoken": None,
                            "own_step": b["step"], "times_ctc": b["times_ctc"], "track": b["track"]})
        for kind, st, i, a, y in vs:
            if j < args.keep_audio:
                sf.write(work / "audio" / f"{b['uid']}_{kind}_{st}_{i}.wav", y, SR)
            recs.append(meta | {"kind": kind, "step": st, "phrase": i, "spoken": a, "own_step": b["step"],
                                "times_ctc": al.mora_times(y, b["kana"])})
            jobs.append(y)
    vrecs = [r for r in recs if r["kind"] != "original"]
    with ProcessPoolExecutor(20) as ex:
        for r, tr in zip(vrecs, ex.map(track_worker, jobs, chunksize=8)):
            r["track"] = tr
    return recs, skipped


def ctc_cer(a, b):
    import difflib
    sm = difflib.SequenceMatcher(None, a, b, autojunk=False)
    return 1 - sum(m.size for m in sm.get_matching_blocks()) / max(len(b), 1)


if __name__ == "__main__":
    main()
