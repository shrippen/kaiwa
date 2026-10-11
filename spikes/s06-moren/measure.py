"""Spike 0.6: does a kana CTC model find mora-length errors (long vowel, っ, ん)? (ADR-005, I-01)

Each trial is an audio file and the form the learner meant. Two checks are measured:

  - duration: the meant kana are aligned to the audio (Viterbi); each mora gets a length relative
    to the speaker's median mora. A long vowel, っ or ん that is too short is an error, a short
    mora that is too long as well (ADR-005, item 3).
  - variants: the CTC log-likelihood of the meant kana against the variants a mora error would
    give (おばあさん -> おばさん, きって -> きて, げんいん -> げにん ...). The best variant beating
    the meant form by more than a margin is an error, and names the place.

The thresholds are calibrated per check so that correct speech gives at most 5 % alarms, leaving
one voice out each time (calibrate on the others, count on it). OpenJTalk's three speeds are one
voice here. Results come per family: Kokoro (does not keep the lengths it is given, see synth.py)
and OpenJTalk (does).

  python -I measure.py --corpus-audio ../../target/s06 --out results [--model reazon-large] [--noise 10]
"""

import argparse
import json
import sys
import zlib
from collections import defaultdict
from pathlib import Path

import librosa
import numpy as np
import torch
from transformers import Wav2Vec2ForCTC, Wav2Vec2Processor

sys.path.insert(0, str(Path(__file__).parent))  # -I leaves the script's directory out
import ctc  # noqa: E402

# Kana CTC models (Apache-2.0 each): name, revision, the token the model writes between words, and
# which kana it writes: 'read' (orthographic, とう) or 'pron' (pronounced, とー). Any other value of
# --model is a local directory of an own model (finetune.py), which writes pronounced kana.
MODELS = {
    # Hiragana per character; Common Voice and JSUT.
    "hiragana": ("vumichien/wav2vec2-large-xlsr-japanese-hiragana", "017225bb128a6b1c6de9d58391f891908d69bc7b",
                 "|", "read"),
    # ReazonSpeech v2.0 (35,000 h); subwords with kanji, every hiragana also as a token of its own.
    "reazon-large": ("reazon-research/japanese-wav2vec2-large-rs35kh", "2162c9edb97f362f7623d6b3a2e417458e879bca",
                     "▁", "read"),
    "reazon-base": ("reazon-research/japanese-wav2vec2-base-rs35kh", None, "▁", "read"),
}
SR = 16000
PAD = 0.5  # s of silence around the utterance (spike 0.3: without it the start gets lost)
FA_MAX = 0.05
FRAME_S = 0.02


def pink_noise(n, rng):
    spec = np.fft.rfft(rng.standard_normal(n))
    f = np.arange(len(spec))
    spec[1:] /= np.sqrt(f[1:])
    spec[0] = 0
    x = np.fft.irfft(spec, n)
    return x / np.sqrt(np.mean(x ** 2))


def load(path, snr, seed):
    a, _ = librosa.load(path, sr=SR)
    if snr is not None:
        p = np.mean(a[np.abs(a) > 1e-4] ** 2)
        a = a + pink_noise(len(a), np.random.default_rng(seed)) * np.sqrt(p / 10 ** (snr / 10))
    return np.pad(a, int(PAD * SR))


class Model:
    def __init__(self, key, device):
        name, revision, delim, writes = MODELS.get(key, (key, None, "|", "pron"))
        self.name, self.revision = name, revision
        self.pron = writes == "pron"
        self.proc = Wav2Vec2Processor.from_pretrained(name, revision=revision)
        self.model = Wav2Vec2ForCTC.from_pretrained(name, revision=revision).eval().to(device)
        self.device = device
        vocab = self.proc.tokenizer.get_vocab()
        self.vocab = vocab
        self.blank = self.proc.tokenizer.pad_token_id
        self.delim = vocab[delim]

    def logp(self, audio):
        x = self.proc(audio, sampling_rate=SR, return_tensors="pt").input_values.to(self.device)
        with torch.no_grad():
            lg = self.model(x).logits[0].float().cpu().numpy()
        return ctc.blank_merged(ctc.log_softmax(lg), self.blank, self.delim)

    def kana(self, text):
        return ctc.pron(text) if self.pron else ctc.reading(text)

    def ids(self, kana):
        return [self.vocab[c] for c in kana]


def duration_features(logp, kana, model):
    """Mora lengths relative to the median mora; the shortest long element and the longest short mora."""
    ids = model.ids(kana)
    starts, ends = ctc.align(logp, ids, model.blank)
    m = ctc.moras(kana)
    # first token index of each mora
    first, k = [], 0
    for mo in m:
        first.append(k)
        k += len(mo)
    spans = []
    for j in range(len(m)):
        s = starts[first[j]]
        e = starts[first[j + 1]] if j + 1 < len(m) else ends[-1] + 1
        spans.append(max(e - s, 1))
    plain = [spans[j] for j in range(len(m) - 1)
             if m[j] not in "んっー" and not (j and ctc.is_extension(m[j - 1], m[j]))]
    rate = float(np.median(plain)) if plain else float(np.median(spans))
    long_el, short_el = [], []
    for j, mo in enumerate(m):
        r = spans[j] / rate
        if mo == "っ" or (mo == "ん" and j + 1 < len(m) and m[j + 1] in ctc.N_ROW) or (
                j and m[j - 1] not in "んっー" and ctc.is_extension(m[j - 1], mo)):
            long_el.append((r, j, mo))
        elif j + 1 < len(m) and not ctc.is_extension(mo, m[j + 1]):
            short_el.append((r, j, mo))  # last mora left out: it carries the trailing silence
    return {
        "rate_s": rate * FRAME_S,
        "spans": spans,
        "min_long": min(long_el)[0] if long_el else None,
        "max_short": max(short_el)[0] if short_el else None,
    }


def variant_features(logp, kana, model):
    base = ctc.log_likelihood(logp, model.ids(kana), model.blank)
    every = {}
    for kind, i, var in ctc.variants(kana, "ー" if model.pron else None):
        if any(c not in model.vocab for c in var):
            continue
        every[var] = (kind, i, ctc.log_likelihood(logp, model.ids(var), model.blank) - base)
    out = {"base": base, "every": every}
    out.update(best_by_kind({v: (k, i, d) for v, (k, i, d) in every.items()}))
    return out


def best_by_kind(every):
    """every: variant -> (kind, mora index, delta). The strongest variant per kind and overall."""
    best = defaultdict(lambda: (-np.inf, None, None))
    for var, (kind, i, d) in every.items():
        if d > best[kind][0]:
            best[kind] = (d, i, var)
    top = max(best.values(), key=lambda b: b[0]) if best else (-np.inf, None, None)
    return {"max_delta": top[0], "flag_variant": top[2],
            "by_kind": {k: v[0] for k, v in best.items()}, "best": dict(best)}


def reference_normalise(rows):
    """Takes off each variant's delta the median delta of the same variant on correct speech of the
    same text by the other voices: what the model hears into the text anyway (げんいん -> げにん).
    In the app the reference is the tutor voice speaking the meant sentence."""
    refs = defaultdict(list)
    for r in rows:
        if not r["error"]:
            refs[(r["group"], r["intended"])].append(r)
    for r in rows:
        others = [o for o in refs[(r["group"], r["intended"])] if fold_of(o["voice"]) != fold_of(r["voice"])]
        every = {}
        for var, (kind, i, d) in r["variants"]["every"].items():
            ds = [o["variants"]["every"][var][2] for o in others if var in o["variants"]["every"]]
            if ds:
                every[var] = (kind, i, d - float(np.median(ds)))
        r["ref"] = best_by_kind(every)


def edit(a, b):
    d = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        prev, d[0] = d[0], i
        for j, cb in enumerate(b, 1):
            prev, d[j] = d[j], min(d[j] + 1, d[j - 1] + 1, prev + (ca != cb))
    return d[-1]


def trials(manifest):
    out = []
    for it in manifest["items"]:
        if "pair" in it:
            side = it["id"].split("_")[1]
            other = "b" if side == "a" else "a"
            group = "frame" if it["frame"] else "word"
            out.append({"item": it["id"], "group": group, "kind": it["kind"], "intended": it["forms"][side],
                        "spoken": it["spoken"], "error": False})
            out.append({"item": it["id"], "group": group, "kind": it["kind"], "intended": it["forms"][other],
                        "spoken": it["spoken"], "error": True})
        else:
            out.append({"item": it["id"], "group": "sentence", "kind": "sentence", "intended": it["intended"],
                        "spoken": it["spoken"], "error": it["error"]})
    return out


def threshold(values, higher_is_error, fa):
    """Threshold that keeps alarms on correct trials at or below `fa`."""
    v = np.sort(np.array([x for x in values if x is not None]))
    if not len(v):
        return None
    q = np.quantile(v, 1 - fa if higher_is_error else fa, method="higher" if higher_is_error else "lower")
    return float(q)


def flags(r, th):
    """Alarm of each check for one trial, given thresholds."""
    d = r["duration"]
    dur = (d["min_long"] is not None and d["min_long"] < th["min_long"]) or (
        d["max_short"] is not None and d["max_short"] > th["max_short"])
    var = r["variants"]["max_delta"] > th["max_delta"]
    kind = kind_alarm(r, th["by_kind"]) is not None
    ref = kind_alarm(r, th["ref_by_kind"], "ref") is not None
    return {"duration": bool(dur), "variants": bool(var), "variants_kind": kind, "variants_ref": ref,
            "both": bool(dur and var), "either": bool(dur or var)}


def kind_alarm(r, th_kind, feature="variants"):
    """The error kind with the largest margin over its own threshold, or None."""
    over = [(d - th_kind[k], k) for k, d in r[feature]["by_kind"].items() if k in th_kind and d > th_kind[k]]
    return max(over)[1] if over else None


def kind_thresholds(ok, feature="variants"):
    """One threshold per error kind (ん before a vowel needs a much higher one than っ, see README).
    All kinds share one quantile level, the largest whose combined alarms stay at or below FA_MAX."""
    kinds = sorted({k for r in ok for k in r[feature]["by_kind"]})
    values = {k: np.array([r[feature]["by_kind"].get(k, -np.inf) for r in ok]) for k in kinds}
    lo, hi = 0.0, FA_MAX
    for _ in range(30):
        p = (lo + hi) / 2
        th = {k: float(np.quantile(v[np.isfinite(v)], 1 - p)) if np.isfinite(v).any() else np.inf
              for k, v in values.items()}
        fa = np.mean([kind_alarm(r, th, feature) is not None for r in ok])
        lo, hi = (p, hi) if fa <= FA_MAX else (lo, p)
    return {k: float(np.quantile(v[np.isfinite(v)], 1 - lo)) if np.isfinite(v).any() else np.inf
            for k, v in values.items()}


def calibrate(rows):
    ok = [r for r in rows if not r["error"]]
    # The duration check has two features; each gets half of the 5 %.
    th = {"min_long": threshold([r["duration"]["min_long"] for r in ok], False, FA_MAX / 2),
          "max_short": threshold([r["duration"]["max_short"] for r in ok], True, FA_MAX / 2)}
    th["max_delta"] = threshold([r["variants"]["max_delta"] for r in ok], True, FA_MAX)
    th["by_kind"] = kind_thresholds(ok)
    th["ref_by_kind"] = kind_thresholds(ok, "ref")
    return th


def model_tag(path):
    """Name of an own model for result files: its directory, or the one above a plain 'model'."""
    p = Path(path)
    return p.parent.name if p.name == "model" else p.name


def fold_of(voice):
    return "ojt" if voice.startswith("ojt") else voice


def family(voice):
    return "openjtalk" if voice.startswith("ojt") else "kokoro"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus-audio", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--noise", type=float, default=None, help="pink noise at this SNR in dB")
    ap.add_argument("--model", default="hiragana", help=f"one of {', '.join(MODELS)} or a model directory")
    args = ap.parse_args()
    root = Path(args.corpus_audio)
    manifest = json.loads((root / "manifest.json").read_text())
    model = Model(args.model, "cuda" if torch.cuda.is_available() else "cpu")
    readings = {}

    def kana(text):
        if text not in readings:
            readings[text] = model.kana(text)
        return readings[text]

    rows = []
    cache = {}
    for voice in manifest["voices"]:
        for n, t in enumerate(trials(manifest)):
            key = (t["item"], voice)
            if key not in cache:
                cache[key] = model.logp(load(root / "audio" / f"{t['item']}_{voice}.wav", args.noise,
                                         zlib.crc32(f"{t['item']}_{voice}".encode())))
            logp = cache[key]
            k = kana(t["intended"])
            r = dict(t, voice=voice, kana=k, spoken_kana=kana(t["spoken"]),
                     duration=duration_features(logp, k, model), variants=variant_features(logp, k, model))
            v = r["variants"]["flag_variant"]
            # Does the best variant come closer to what was actually said?
            r["variant_closer"] = v is not None and edit(v, r["spoken_kana"]) < edit(k, r["spoken_kana"])
            rows.append(r)
        cache.clear()

    reference_normalise(rows)
    # Leave one voice out
    for fold in sorted({fold_of(v) for v in manifest["voices"]}):
        th = calibrate([r for r in rows if fold_of(r["voice"]) != fold])
        for r in rows:
            if fold_of(r["voice"]) == fold:
                r["thresholds"] = th
                r["flags"] = flags(r, th)
                k = kind_alarm(r, th["by_kind"])
                v = r["variants"]["best"].get(k, (None, None, None))[2] if k else None
                r["kind_variant"] = v
                r["kind_closer"] = v is not None and edit(v, r["spoken_kana"]) < edit(r["kana"], r["spoken_kana"])
                k = kind_alarm(r, th["ref_by_kind"], "ref")
                v = r["ref"]["best"].get(k, (None, None, None))[2] if k else None
                r["ref_variant"] = v
                r["ref_closer"] = v is not None and edit(v, r["spoken_kana"]) < edit(r["kana"], r["spoken_kana"])

    def rate(sel, check):
        return (sum(r["flags"][check] for r in sel) / len(sel), len(sel)) if sel else (None, 0)

    summary, by_kind = {}, {}
    for fam in ("kokoro", "openjtalk", "all"):
        f_rows = [r for r in rows if fam == "all" or family(r["voice"]) == fam]
        summary[fam], by_kind[fam] = {}, {}
        for group in ("word", "frame", "sentence", "all"):
            g = [r for r in f_rows if group == "all" or r["group"] == group]
            summary[fam][group] = {}
            for check in ("duration", "variants", "variants_kind", "variants_ref", "both", "either"):
                fa = rate([r for r in g if not r["error"]], check)
                det = rate([r for r in g if r["error"]], check)
                summary[fam][group][check] = {"false_alarms": fa[0], "n_ok": fa[1], "detection": det[0],
                                              "n_err": det[1]}
            hits = [r for r in g if r["error"] and r["flags"]["variants"]]
            kind_hits = [r for r in g if r["error"] and r["flags"]["variants_kind"]]
            summary[fam][group]["kind_place_right"] = (
                sum(r["kind_closer"] for r in kind_hits) / len(kind_hits)) if kind_hits else None
            ref_hits = [r for r in g if r["error"] and r["flags"]["variants_ref"]]
            summary[fam][group]["ref_place_right"] = (
                sum(r["ref_closer"] for r in ref_hits) / len(ref_hits)) if ref_hits else None
            summary[fam][group]["variants_place_right"] = (
                sum(r["variant_closer"] for r in hits) / len(hits)) if hits else None
        for kind in ("laenge", "sokuon", "hatsuon", "sentence"):
            g = [r for r in f_rows if r["kind"] == kind and r["error"]]
            by_kind[fam][kind] = {c: rate(g, c)[0] for c in ("duration", "variants", "variants_kind", "variants_ref")}
    by_voice = {v: {c: {"fa": rate([r for r in rows if r["voice"] == v and not r["error"]], c)[0],
                        "det": rate([r for r in rows if r["voice"] == v and r["error"]], c)[0]}
                    for c in ("duration", "variants")} for v in manifest["voices"]}
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    name = (args.model if args.model in MODELS else model_tag(args.model)) + ("" if args.noise is None else f"-snr{args.noise:g}")
    report = {"model": model.name, "revision": model.revision, "noise_snr_db": args.noise, "fa_max": FA_MAX,
              "summary": summary, "by_kind": by_kind, "by_voice": by_voice,
              "thresholds": {fold_of(v): next(r["thresholds"] for r in rows if r["voice"] == v)
                             for v in manifest["voices"]},
              "trials": [{k: r[k] for k in ("item", "voice", "group", "kind", "intended", "spoken", "kana",
                                             "spoken_kana", "error", "flags", "variant_closer", "kind_variant", "ref_variant")}
                         | {"min_long": r["duration"]["min_long"], "max_short": r["duration"]["max_short"],
                            "rate_s": r["duration"]["rate_s"], "max_delta": r["variants"]["max_delta"],
                            "flag_variant": r["variants"]["flag_variant"],
                            "delta_by_kind": r["variants"]["by_kind"], "ref_by_kind": r["ref"]["by_kind"]}
                         for r in rows]}
    (out / f"{name}.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))
    print(json.dumps({"summary": summary, "by_kind": by_kind}, indent=1))


if __name__ == "__main__":
    main()
