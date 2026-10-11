"""Spike 0.6, cross-check (I-01): the Montreal Forced Aligner (HMM, japanese_mfa, CC BY 4.0) on the
same trials as measure.py.

Two checks:
  - mfa_duration: the meant pronunciation is aligned; long vowels (Vː), long consonants (Cː, っ) and
    ん before a vowel (ɰ̃) get their length relative to the speaker's mora length.
  - mfa_choice: the dictionary holds the meant pronunciation and every mora-error variant (ctc.variants),
    the variants with a lower prior. MFA picks the pronunciation that fits; picking a variant is an
    alarm. The prior is the knob for the false alarms; it is chosen leaving one voice out, as in
    measure.py.

Needs `mfa` (conda-forge montreal-forced-aligner 3.x) with the models japanese_mfa (acoustic, dictionary):

  python -I mfa_check.py --corpus-audio ../../target/s06 --mfa <mfa binary> --work ../../target/s06/mfa --out results
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

import librosa
import numpy as np
import pyopenjtalk
import soundfile as sf

sys.path.insert(0, str(Path(__file__).parent))  # -I leaves the script's directory out
import ctc  # noqa: E402
from measure import FA_MAX, PAD, SR, edit, family, fold_of, kind_alarm, kind_thresholds, threshold, trials  # noqa: E402,E501

PRIORS = (1.0, 0.3, 0.1, 0.03, 0.01, 0.001)

# Hiragana -> (consonant, vowel) in the phone set of japanese_mfa.
_BASE = {
    "あ": ("", "a"), "い": ("", "i"), "う": ("", "ɯ"), "え": ("", "e"), "お": ("", "o"),
    "か": ("k", "a"), "き": ("c", "i"), "く": ("k", "ɯ"), "け": ("k", "e"), "こ": ("k", "o"),
    "が": ("ɡ", "a"), "ぎ": ("ɟ", "i"), "ぐ": ("ɡ", "ɯ"), "げ": ("ɡ", "e"), "ご": ("ɡ", "o"),
    "さ": ("s", "a"), "し": ("ɕ", "i"), "す": ("s", "ɨ"), "せ": ("s", "e"), "そ": ("s", "o"),
    "ざ": ("z", "a"), "じ": ("ʑ", "i"), "ず": ("z", "ɨ"), "ぜ": ("z", "e"), "ぞ": ("z", "o"),
    "た": ("t", "a"), "ち": ("tɕ", "i"), "つ": ("ts", "ɨ"), "て": ("t", "e"), "と": ("t", "o"),
    "だ": ("d", "a"), "ぢ": ("ʑ", "i"), "づ": ("z", "ɨ"), "で": ("d", "e"), "ど": ("d", "o"),
    "な": ("n", "a"), "に": ("ɲ", "i"), "ぬ": ("n", "ɯ"), "ね": ("n", "e"), "の": ("n", "o"),
    "は": ("h", "a"), "ひ": ("ç", "i"), "ふ": ("ɸ", "ɯ"), "へ": ("h", "e"), "ほ": ("h", "o"),
    "ば": ("b", "a"), "び": ("bʲ", "i"), "ぶ": ("b", "ɯ"), "べ": ("b", "e"), "ぼ": ("b", "o"),
    "ぱ": ("p", "a"), "ぴ": ("pʲ", "i"), "ぷ": ("p", "ɯ"), "ぺ": ("p", "e"), "ぽ": ("p", "o"),
    "ま": ("m", "a"), "み": ("mʲ", "i"), "む": ("m", "ɯ"), "め": ("m", "e"), "も": ("m", "o"),
    "や": ("j", "a"), "ゆ": ("j", "ɨ"), "よ": ("j", "o"),
    "ら": ("ɾ", "a"), "り": ("ɾʲ", "i"), "る": ("ɾ", "ɯ"), "れ": ("ɾ", "e"), "ろ": ("ɾ", "o"),
    "わ": ("w", "a"), "を": ("", "o"), "ゔ": ("v", "ɯ"),
}
_SMALL = {"ゃ": "a", "ゅ": "ɨ", "ょ": "o", "ぁ": "a", "ぃ": "i", "ぅ": "ɯ", "ぇ": "e", "ぉ": "o"}
_AFTER_N = {"z": "dz", "ʑ": "dʑ"}  # affricate after ん and at the start


def pron(text):
    """Pronounced hiragana (pyopenjtalk 'pron': は as わ, long vowels as ー)."""
    p = "".join(n["pron"] for n in pyopenjtalk.run_frontend(text))
    return "".join(c for c in ctc.kata_to_hira(p) if c in ctc.VOWEL or c in "んっー")


def phones(kana):
    """Hiragana -> japanese_mfa phones; one entry per phone."""
    m = ctc.moras(kana)
    segs = []  # per mora: [consonant or '', vowel] or a special
    for j, mo in enumerate(m):
        if mo in "んっ":
            segs.append([mo, ""])
        elif mo == "ー" or (j and m[j - 1] not in "んっー" and ctc.is_extension(m[j - 1], mo)):
            segs.append(["ー", ""])
        else:
            c, v = _BASE[mo[0]]
            if len(mo) == 2:
                v = _SMALL[mo[1]]
                if c == "" and mo[0] == "い":
                    c = "j"
            segs.append([c, v])
    out = []
    for j, (c, v) in enumerate(segs):
        nxt = segs[j + 1][0] if j + 1 < len(segs) else None
        if c == "ー":
            for k in range(len(out) - 1, -1, -1):
                if out[k] in ("a", "i", "ɯ", "ɨ", "e", "o"):
                    out[k] += "ː"
                    break
            continue
        if c == "っ":
            continue  # lengthens the next consonant
        if c == "ん":
            if nxt is None:
                out.append("ɴ")
            elif nxt in ("m", "mʲ", "n", "ɲ"):
                pass  # ん + m -> mː (三枚 s a mː a i)
            elif nxt in ("p", "b", "pʲ", "bʲ"):
                out.append("m")
            elif nxt in ("t", "d", "ts", "z", "tɕ", "ʑ", "ɾ", "ɾʲ"):
                out.append("n")
            elif nxt in ("k", "ɡ"):
                out.append("ŋ")
            elif nxt in ("c", "ɟ"):
                out.append("ɲ")
            else:
                out.append("ɰ̃")
            continue
        prev = segs[j - 1][0] if j else None
        if c in _AFTER_N and (prev is None or prev == "ん"):
            c = _AFTER_N[c]
        if c:
            long_c = prev == "っ" or (prev == "ん" and c in ("m", "mʲ", "n", "ɲ"))
            out.append(c + "ː" if long_c else c)
        elif prev == "っ":
            out.append("ʔ")
        if v == "ɯ" and c in ("s", "ɕ", "ts", "z", "dz", "ʑ", "dʑ", "tɕ", "j"):
            v = "ɨ"
        out.append(v)
    return out


VOWELS = ("a", "i", "ɯ", "ɨ", "e", "o")


def run_mfa(mfa, corpus, dictionary, out, jobs):
    cmd = [mfa, "align", str(corpus), str(dictionary), "japanese_mfa", str(out), "--clean", "--quiet",
           "-j", str(jobs), "--output_format", "json", "--beam", "100", "--retry_beam", "400", "--no_tokenization"]
    # MFA calls its tools (OpenFst, Kaldi, PostgreSQL) from its own environment
    env = dict(os.environ, PATH=f"{Path(mfa).resolve().parent}:{os.environ['PATH']}")
    subprocess.run(cmd, check=True, env=env)


def read_phones(path):
    d = json.loads(path.read_text())
    tiers = d["tiers"]
    return [(s, e, p) for s, e, p in tiers["phones"]["entries"] if p not in ("", "sil", "sp", "spn")]


def duration_features(ph):
    """Lengths relative to the mean mora; long elements and short vowels (last vowel left out)."""
    if not ph:
        return None
    speech = sum(e - s for s, e, _ in ph)
    n_moras = sum(1 for _, _, p in ph if p.rstrip("ː") in VOWELS) + sum(
        1 for _, _, p in ph if p.endswith("ː") or p in ("ɰ̃", "ɴ", "ʔ"))
    rate = speech / max(n_moras, 1)
    long_el, short_el = [], []
    last_vowel = max((i for i, (_, _, p) in enumerate(ph) if p.rstrip("ː") in VOWELS), default=-1)
    for i, (s, e, p) in enumerate(ph):
        r = (e - s) / rate
        if p.endswith("ː"):
            # a long vowel counts its own two moras; a long consonant one mora
            long_el.append(r / (2 if p.rstrip("ː") in VOWELS else 1))
        elif p == "ɰ̃":
            long_el.append(r)
        elif p in VOWELS and i != last_vowel:
            short_el.append(r)
    return {"rate_s": rate, "min_long": min(long_el) if long_el else None,
            "max_short": max(short_el) if short_el else None}


GEMINABLE_C = ("k", "c", "t", "p", "s", "ɕ", "ts", "tɕ")


def class_features(ph):
    """Per class of sound the most suspicious length in the utterance, relative to the mean mora,
    oriented so that larger means more likely an error:
      Vː, Cː, ɰ̃ too short (spoken short), V and a voiceless C too long (spoken long, っ added)."""
    if not ph:
        return {"by_kind": {}}
    speech = sum(e - s for s, e, _ in ph)
    n_moras = sum(1 for _, _, p in ph if p.rstrip("ː") in VOWELS) + sum(
        1 for _, _, p in ph if p.endswith("ː") or p in ("ɰ̃", "ɴ", "ʔ"))
    rate = speech / max(n_moras, 1)
    last_vowel = max((i for i, (_, _, p) in enumerate(ph) if p.rstrip("ː") in VOWELS), default=-1)
    worst = {}

    def put(cls, v):
        worst[cls] = max(worst.get(cls, -np.inf), v)

    for i, (s, e, p) in enumerate(ph):
        r = (e - s) / rate
        if i == last_vowel:
            continue  # the last vowel takes the trailing breath
        if p.endswith("ː"):
            put("Vː short" if p.rstrip("ː") in VOWELS else "Cː short", -r)
        elif p == "ɰ̃":
            put("ɰ̃ short", -r)
        elif p in VOWELS:
            put("V long", r)
        elif p in GEMINABLE_C and 0 < i:
            put("C long", r)
    return {"by_kind": worst}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus-audio", required=True)
    ap.add_argument("--mfa", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--jobs", type=int, default=16)
    args = ap.parse_args()
    root = Path(args.corpus_audio)
    work = Path(args.work)
    manifest = json.loads((root / "manifest.json").read_text())

    # Corpus: one utterance per trial (an audio can be two trials: meant a, meant b), speaker = voice.
    corpus = work / "corpus"
    rows = []
    prons = {}
    for voice in manifest["voices"]:
        (corpus / voice).mkdir(parents=True, exist_ok=True)
        for n, t in enumerate(trials(manifest)):
            uid = f"{voice}-{n:04d}"
            wav = corpus / voice / f"{uid}.wav"
            if not wav.exists():
                a, _ = librosa.load(root / "audio" / f"{t['item']}_{voice}.wav", sr=SR)
                sf.write(wav, np.pad(a, int(PAD * SR)), SR)
            for text in (t["intended"], t["spoken"]):
                if text not in prons:
                    prons[text] = pron(text)
            k = prons[t["intended"]]
            word = f"w{n:04d}"
            (corpus / voice / f"{uid}.lab").write_text(word)
            meant = " ".join(phones(k))
            var = {}
            for kind, i, v in ctc.variants(k):
                p = " ".join(phones(v))
                if p != meant and p not in var:
                    var[p] = (kind, v)
            rows.append(dict(t, voice=voice, uid=uid, word=word, kana=k, spoken_kana=prons[t["spoken"]],
                             meant=meant, variants=var))

    words = {}
    for r in rows:
        words[r["word"]] = (r["meant"], r["variants"])  # same per trial index across voices

    # Run 1: meant pronunciation only -> durations
    def write_dict(path, prior):
        with open(path, "w") as f:
            for w, (meant, var) in sorted(words.items()):
                f.write(f"{w}\t1.0\t{meant}\n")
                if prior is not None:
                    for p in var:
                        f.write(f"{w}\t{prior}\t{p}\n")

    def align(tag, prior):
        out = work / f"out-{tag}"
        if not out.exists():
            d = work / f"dict-{tag}.txt"
            write_dict(d, prior)
            run_mfa(args.mfa, corpus, d, out, args.jobs)
        return out

    out = align("meant", None)
    for r in rows:
        f = out / r["voice"] / f"{r['uid']}.json"
        ph = read_phones(f) if f.exists() else []
        r["mfa_duration"] = duration_features(ph)
        r["mfa_cls"] = class_features(ph)

    # Runs 2..: with the variants at several priors -> which pronunciation MFA picks
    for prior in PRIORS:
        out = align(f"var{prior:g}", prior)
        for r in rows:
            f = out / r["voice"] / f"{r['uid']}.json"
            got = " ".join(p for _, _, p in read_phones(f)) if f.exists() else None
            r.setdefault("choice", {})[prior] = got

    def norm(p):  # MFA writes one phone per entry; compare without optional devoicing marks
        return p.replace("̥", "") if p else p

    for r in rows:
        r["picked"] = {pr: (None if g is None else "meant" if norm(g) == norm(r["meant"])
                            else r["variants"].get(g, ("other", g))[0]) for pr, g in r["choice"].items()}

    # Thresholds leaving one voice out
    for fold in sorted({fold_of(v) for v in manifest["voices"]}):
        cal_ok = [r for r in rows if fold_of(r["voice"]) != fold and not r["error"] and r["mfa_duration"]]
        th = {"min_long": threshold([r["mfa_duration"]["min_long"] for r in cal_ok], False, FA_MAX / 2),
              "max_short": threshold([r["mfa_duration"]["max_short"] for r in cal_ok], True, FA_MAX / 2)}
        # the largest prior whose alarms on correct speech stay within FA_MAX
        prior = min(PRIORS)
        for pr in PRIORS:
            fa = np.mean([r["picked"][pr] not in ("meant", None) for r in cal_ok])
            if fa <= FA_MAX:
                prior = pr
                break
        th["prior"] = prior
        th["cls"] = kind_thresholds(cal_ok, "mfa_cls")
        for r in rows:
            if fold_of(r["voice"]) == fold:
                d = r["mfa_duration"]
                dur = bool(d) and ((d["min_long"] is not None and th["min_long"] is not None
                                    and d["min_long"] < th["min_long"])
                                   or (d["max_short"] is not None and th["max_short"] is not None
                                       and d["max_short"] > th["max_short"]))
                pick = r["picked"][prior]
                r["thresholds"] = th
                r["flags"] = {"mfa_duration": dur, "mfa_choice": pick not in ("meant", None),
                              "mfa_class": kind_alarm(r, th["cls"], "mfa_cls") is not None}
                r["class_alarm"] = kind_alarm(r, th["cls"], "mfa_cls")
                chosen = r["choice"][prior]
                v = r["variants"].get(chosen, (None, None))[1] if chosen else None
                r["choice_closer"] = v is not None and edit(v, r["spoken_kana"]) < edit(r["kana"], r["spoken_kana"])

    def rate(sel, check):
        return (sum(r["flags"][check] for r in sel) / len(sel), len(sel)) if sel else (None, 0)

    summary = {}
    for fam in ("kokoro", "openjtalk", "all"):
        f_rows = [r for r in rows if fam == "all" or family(r["voice"]) == fam]
        summary[fam] = {}
        for group in ("word", "frame", "sentence", "all"):
            g = [r for r in f_rows if group == "all" or r["group"] == group]
            summary[fam][group] = {}
            for check in ("mfa_duration", "mfa_class", "mfa_choice"):
                fa = rate([r for r in g if not r["error"]], check)
                det = rate([r for r in g if r["error"]], check)
                summary[fam][group][check] = {"false_alarms": fa[0], "n_ok": fa[1], "detection": det[0],
                                              "n_err": det[1]}
            hits = [r for r in g if r["error"] and r["flags"]["mfa_choice"]]
            summary[fam][group]["choice_place_right"] = (
                sum(r["choice_closer"] for r in hits) / len(hits)) if hits else None
    by_kind = {fam: {kind: {c: rate([r for r in rows if r["kind"] == kind and r["error"]
                                     and (fam == "all" or family(r["voice"]) == fam)], c)[0]
                            for c in ("mfa_duration", "mfa_class", "mfa_choice")}
                     for kind in ("laenge", "sokuon", "hatsuon", "sentence")}
               for fam in ("kokoro", "openjtalk", "all")}
    report = {"aligner": "montreal-forced-aligner, japanese_mfa", "fa_max": FA_MAX, "priors": PRIORS,
              "summary": summary, "by_kind": by_kind,
              "thresholds": {fold_of(v): next(r["thresholds"] for r in rows if r["voice"] == v)
                             for v in manifest["voices"]},
              "trials": [{k: r[k] for k in ("item", "voice", "group", "kind", "intended", "spoken", "kana",
                                             "spoken_kana", "error", "meant", "mfa_duration", "mfa_cls", "class_alarm", "flags",
                                             "picked", "choice_closer")}
                         | {"choice": {str(p): c for p, c in r["choice"].items()}} for r in rows]}
    Path(args.out).mkdir(parents=True, exist_ok=True)
    (Path(args.out) / "mfa.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))
    print(json.dumps({"summary": summary, "by_kind": by_kind}, indent=1))


if __name__ == "__main__":
    main()
