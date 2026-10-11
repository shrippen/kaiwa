"""CTC pieces of the mora check, in plain numpy so the Rust port (kaiwa-speech) can mirror them
line by line: the log-likelihood of a kana sequence (forward algorithm), the best alignment
(Viterbi) and the variants a learner's mora error would produce.

Tokens are hiragana as the model writes them (orthographic: とうきょう, おばあさん, コーヒー as こーひー).
"""

import numpy as np

NEG = -1e30
SMALL_Y = "ゃゅょ"
SMALL = SMALL_Y + "ぁぃぅぇぉゎ"  # join the kana before them into one mora (てぃ, ふぁ)
VOWEL_KANA = "あいうえお"
# Vowel of each hiragana mora (the last letter of its rōmaji).
_ROWS = {
    "a": "あかがさざただなはばぱまやらわぁゃ",
    "i": "いきぎしじちぢにひびぴみりぃ",
    "u": "うくぐすずつづぬふぶぷむゆるぅゅゔ",
    "e": "えけげせぜてでねへべぺめれぇ",
    "o": "おこごそぞとどのほぼぽもよろをぉょ",
}
VOWEL = {k: v for v, ks in _ROWS.items() for k in ks}
N_ROW = {"あ": "な", "い": "に", "う": "ぬ", "え": "ね", "お": "の",
         "や": "にゃ", "ゆ": "にゅ", "よ": "にょ"}
N_ROW_BACK = {v: k for k, v in N_ROW.items()}
# Moras a っ can stand before (voiceless obstruents).
GEMINABLE = set("かきくけこさしすせそたちつてとぱぴぷぺぽ")


def kata_to_hira(s):
    return "".join(chr(ord(c) - 0x60) if "ァ" <= c <= "ヶ" else c for c in s)


def reading(text):
    """Orthographic hiragana reading of a Japanese text (pyopenjtalk 'read', punctuation dropped)."""
    import pyopenjtalk
    read = "".join(n["read"] for n in pyopenjtalk.run_frontend(text))
    return "".join(c for c in kata_to_hira(read) if c in VOWEL or c in "んっー")


def moras(kana):
    """Splits into moras: a small kana (ゃゅょ, ぁぃぅぇぉ) joins the kana before it."""
    out = []
    for c in kana:
        if c in SMALL and out:
            out[-1] += c
        else:
            out.append(c)
    return out


def vowel_of(mora):
    return VOWEL.get(mora[-1])


def is_extension(prev, mora):
    """Is `mora` the second half of a long vowel after `prev`?"""
    v = vowel_of(prev) if prev else None
    if v is None:
        return False
    same = VOWEL_KANA["aiueo".index(v)]
    return mora in ("ー", same) or (v, mora) in (("o", "う"), ("e", "い"))


def variants(kana):
    """Mora-length errors a learner can make in `kana`: list of (kind, mora index, variant kana).

    kind: 'long-' (long vowel spoken short), 'long+' (short vowel spoken long), 'q-' (っ left out),
    'q+' (っ added), 'n-' (ん before a vowel merged: げんいん -> げにん), 'n+' (the reverse),
    'y' (small and large ゃゅょ swapped: びょういん / びよういん).
    """
    m = moras(kana)
    out = []

    def join(parts):
        return "".join(parts)

    for i, mo in enumerate(m):
        prev = m[i - 1] if i else None
        if prev and prev not in "んっー" and is_extension(prev, mo):
            out.append(("long-", i, join(m[:i] + m[i + 1:])))
        elif vowel_of(mo) and not (i + 1 < len(m) and is_extension(mo, m[i + 1])):
            v = vowel_of(mo)
            for ext in {"a": "あ", "i": "い", "u": "う", "e": "え", "o": "う"}[v] + ("お" if v == "o" else ""):
                out.append(("long+", i, join(m[:i + 1] + [ext] + m[i + 1:])))
        if mo == "っ":
            out.append(("q-", i, join(m[:i] + m[i + 1:])))
        elif mo[0] in GEMINABLE and i and m[i - 1] not in "っん":
            out.append(("q+", i, join(m[:i] + ["っ"] + m[i:])))
        if mo == "ん" and i + 1 < len(m) and m[i + 1] in N_ROW:
            out.append(("n-", i, join(m[:i] + [N_ROW[m[i + 1]]] + m[i + 2:])))
        if mo in N_ROW_BACK and i:
            out.append(("n+", i, join(m[:i] + ["ん", N_ROW_BACK[mo]] + m[i + 1:])))
        if len(mo) == 2 and mo[1] in SMALL_Y and vowel_of(mo[0]) == "i":
            big = {"ゃ": "や", "ゅ": "ゆ", "ょ": "よ"}[mo[1]]
            out.append(("y", i, join(m[:i] + [mo[0], big] + m[i + 1:])))
        if mo in "やゆよ" and i and vowel_of(m[i - 1]) == "i" and len(m[i - 1]) == 1:
            small = {"や": "ゃ", "ゆ": "ゅ", "よ": "ょ"}[mo]
            out.append(("y", i, join(m[:i - 1] + [m[i - 1] + small] + m[i + 1:])))
    return out


def log_softmax(x):
    x = x - x.max(axis=-1, keepdims=True)
    return x - np.log(np.exp(x).sum(axis=-1, keepdims=True))


def blank_merged(logp, blank, delim):
    """The model writes '|' between words; for alignment it counts as silence like the blank."""
    out = logp.copy()
    out[:, blank] = np.logaddexp(logp[:, blank], logp[:, delim])
    return out


def _lattice(ids, blank):
    ext = [blank]
    for t in ids:
        ext += [t, blank]
    return np.array(ext)


def log_likelihood(logp, ids, blank):
    """log P(ids | audio) under CTC (forward algorithm)."""
    T = logp.shape[0]
    ext = _lattice(ids, blank)
    S = len(ext)
    if S > 2 * T + 1:
        return NEG
    alpha = np.full(S, NEG)
    alpha[0] = logp[0, ext[0]]
    if S > 1:
        alpha[1] = logp[0, ext[1]]
    skip = np.zeros(S, bool)
    skip[2:] = (ext[2:] != blank) & (ext[2:] != ext[:-2])
    for t in range(1, T):
        a1 = np.concatenate(([NEG], alpha[:-1]))
        a2 = np.where(skip, np.concatenate(([NEG, NEG], alpha[:-2])), NEG)
        alpha = np.logaddexp(np.logaddexp(alpha, a1), a2) + logp[t, ext]
    return float(np.logaddexp(alpha[-1], alpha[-2]) if S > 1 else alpha[-1])


def align(logp, ids, blank):
    """Viterbi alignment: for each token of `ids` the first frame it is emitted on."""
    T = logp.shape[0]
    ext = _lattice(ids, blank)
    S = len(ext)
    delta = np.full(S, NEG)
    delta[0] = logp[0, ext[0]]
    if S > 1:
        delta[1] = logp[0, ext[1]]
    skip = np.zeros(S, bool)
    skip[2:] = (ext[2:] != blank) & (ext[2:] != ext[:-2])
    back = np.zeros((T, S), np.int8)
    for t in range(1, T):
        cand = np.stack([delta,
                         np.concatenate(([NEG], delta[:-1])),
                         np.where(skip, np.concatenate(([NEG, NEG], delta[:-2])), NEG)])
        k = cand.argmax(axis=0)
        back[t] = k
        delta = cand[k, np.arange(S)] + logp[t, ext]
    s = S - 1 if S == 1 or delta[-1] >= delta[-2] else S - 2
    path = np.zeros(T, int)
    for t in range(T - 1, -1, -1):
        path[t] = s
        s -= back[t, s]
    starts = [None] * len(ids)
    ends = [None] * len(ids)
    for t, s in enumerate(path):
        if s % 2 == 1:
            j = s // 2
            if starts[j] is None:
                starts[j] = t
            ends[j] = t
    return starts, ends
