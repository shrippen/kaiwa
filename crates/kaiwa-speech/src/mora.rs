//! The mora check (/LF320/, ADR-005, J-01, J-02): does the learner keep long vowels, っ and ん?
//! A kana CTC model scores the meant kana against the variants a mora error would give
//! (おばあさん against おばさん, きって against きて, げんいん against げにん); a variant that
//! wins by more than the threshold of its kind is an error, and names the place. Port of `spikes/s06-moren/ctc.py`.
//!
//! Kana here are pronounced hiragana as the own model writes them: ー for long vowels (とーきょー),
//! わ for the particle は.

use std::collections::HashMap;
use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;

use crate::SpeechError;

/// The vowel of each hiragana that carries one (the last letter of its rōmaji).
fn vowel(c: char) -> Option<char> {
    const ROWS: [(char, &str); 5] = [
        ('a', "あかがさざただなはばぱまやらわぁゃ"),
        ('i', "いきぎしじちぢにひびぴみりぃ"),
        ('u', "うくぐすずつづぬふぶぷむゆるぅゅゔ"),
        ('e', "えけげせぜてでねへべぺめれぇ"),
        ('o', "おこごそぞとどのほぼぽもよろをぉょ"),
    ];
    ROWS.iter().find(|(_, ks)| ks.contains(c)).map(|(v, _)| *v)
}

const SMALL_Y: &str = "ゃゅょ";
/// Small kana join the kana before them into one mora (しゅ, てぃ).
const SMALL: &str = "ゃゅょぁぃぅぇぉゎ";
/// Moras a っ can stand before (voiceless obstruents).
const GEMINABLE: &str = "かきくけこさしすせそたちつてとぱぴぷぺぽ";
/// ん before a vowel or y-kana, and the n-row mora they merge into (げんいん → げにん).
const N_ROW: [(&str, &str); 8] = [
    ("あ", "な"),
    ("い", "に"),
    ("う", "ぬ"),
    ("え", "ね"),
    ("お", "の"),
    ("や", "にゃ"),
    ("ゆ", "にゅ"),
    ("よ", "にょ"),
];

fn vowel_kana(v: char) -> char {
    match v {
        'a' => 'あ',
        'i' => 'い',
        'u' => 'う',
        'e' => 'え',
        _ => 'お',
    }
}

/// Splits kana into moras: a small kana joins the kana before it.
pub fn moras(kana: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in kana.chars() {
        match out.last_mut() {
            Some(last) if SMALL.contains(c) => last.push(c),
            _ => out.push(c.to_string()),
        }
    }
    out
}

fn vowel_of(mora: &str) -> Option<char> {
    mora.chars().last().and_then(vowel)
}

/// Is `mora` the second half of a long vowel after `prev`?
pub fn is_extension(prev: &str, mora: &str) -> bool {
    let Some(v) = vowel_of(prev) else {
        return false;
    };
    let same = vowel_kana(v).to_string();
    mora == "ー" || mora == same || (v == 'o' && mora == "う") || (v == 'e' && mora == "い")
}

/// The kinds of mora error, each with a threshold of its own (ん before a vowel needs a much
/// higher one than っ: the models hear げにん into げんいん).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// A long vowel spoken short (おばあさん → おばさん).
    LongShort,
    /// A short vowel spoken long (おばさん → おばあさん).
    ShortLong,
    /// っ left out (きって → きて).
    GeminateLeftOut,
    /// っ added (きて → きって).
    GeminateAdded,
    /// ん before a vowel merged with it (げんいん → げにん).
    NMerged,
    /// The reverse (げにん → げんいん).
    NSplit,
    /// Small and large ゃゅょ swapped (びょういん ↔ びよういん).
    SmallY,
}

impl Kind {
    /// The name in the spike's results and in `thresholds.json`.
    pub fn name(self) -> &'static str {
        match self {
            Kind::LongShort => "long-",
            Kind::ShortLong => "long+",
            Kind::GeminateLeftOut => "q-",
            Kind::GeminateAdded => "q+",
            Kind::NMerged => "n-",
            Kind::NSplit => "n+",
            Kind::SmallY => "y",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [
            Kind::LongShort,
            Kind::ShortLong,
            Kind::GeminateLeftOut,
            Kind::GeminateAdded,
            Kind::NMerged,
            Kind::NSplit,
            Kind::SmallY,
        ]
        .into_iter()
        .find(|k| k.name() == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub kind: Kind,
    /// Index of the mora the error is at.
    pub mora: usize,
    pub kana: String,
}

/// The mora errors a learner can make in `kana`. `long_mark` lengthens a vowel: ー for pronounced
/// kana; `None` uses the vowel kana (お → おう and おお).
pub fn variants(kana: &str, long_mark: Option<char>) -> Vec<Variant> {
    let m = moras(kana);
    let mut out = Vec::new();
    let mut push = |kind, mora, parts: Vec<&str>| {
        out.push(Variant {
            kind,
            mora,
            kana: parts.concat(),
        })
    };
    let all: Vec<&str> = m.iter().map(String::as_str).collect();
    for (i, mo) in all.iter().copied().enumerate() {
        let prev = i.checked_sub(1).map(|j| all[j]);
        let next = all.get(i + 1).copied();
        let before = &all[..i];
        let after = &all[i + 1..];
        if prev.is_some_and(|p| !matches!(p, "ん" | "っ" | "ー") && is_extension(p, mo)) {
            push(Kind::LongShort, i, [before, after].concat());
        } else if let Some(v) = vowel_of(mo).filter(|_| !next.is_some_and(|n| is_extension(mo, n)))
        {
            let exts: Vec<String> = match long_mark {
                Some(c) => vec![c.to_string()],
                None if v == 'o' => vec!["う".into(), "お".into()],
                None => vec![vowel_kana(v).to_string()],
            };
            for ext in &exts {
                push(
                    Kind::ShortLong,
                    i,
                    [before, &[mo, ext.as_str()], after].concat(),
                );
            }
        }
        if mo == "っ" {
            push(Kind::GeminateLeftOut, i, [before, after].concat());
        } else if mo.chars().next().is_some_and(|c| GEMINABLE.contains(c))
            && prev.is_some_and(|p| p != "っ" && p != "ん")
        {
            push(
                Kind::GeminateAdded,
                i,
                [before, &["っ", mo], after].concat(),
            );
        }
        if mo == "ん" {
            if let Some((_, merged)) = next.and_then(|n| N_ROW.iter().find(|(v, _)| *v == n)) {
                push(
                    Kind::NMerged,
                    i,
                    [before, &[*merged], &all[(i + 2).min(all.len())..]].concat(),
                );
            }
        }
        if i > 0 {
            if let Some((v, _)) = N_ROW.iter().find(|(_, n)| *n == mo) {
                push(Kind::NSplit, i, [before, &["ん", v], after].concat());
            }
        }
        let mut cs = mo.chars();
        if let (Some(first), Some(small), None) = (cs.next(), cs.next(), cs.next()) {
            if SMALL_Y.contains(small) && vowel(first) == Some('i') {
                let big = match small {
                    'ゃ' => "や",
                    'ゅ' => "ゆ",
                    _ => "よ",
                };
                let first = first.to_string();
                push(
                    Kind::SmallY,
                    i,
                    [before, &[first.as_str(), big], after].concat(),
                );
            }
        }
        if let Some(p) = prev.filter(|p| p.chars().count() == 1 && vowel_of(p) == Some('i')) {
            let small = match mo {
                "や" => Some("ゃ"),
                "ゆ" => Some("ゅ"),
                "よ" => Some("ょ"),
                _ => None,
            };
            if let Some(small) = small {
                let joined = format!("{p}{small}");
                push(
                    Kind::SmallY,
                    i,
                    [&all[..i - 1], &[joined.as_str()], after].concat(),
                );
            }
        }
    }
    out
}

/// Frame-wise log probabilities (`frames × vocab`, row-major) from the model's logits, with the
/// word delimiter counted as silence like the blank (the model writes it between words).
pub fn log_probs(logits: &[f32], vocab: usize, blank: usize, delim: Option<usize>) -> Vec<f32> {
    let mut out = Vec::with_capacity(logits.len());
    for row in logits.chunks_exact(vocab) {
        let max = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let lse = max + row.iter().map(|x| (x - max).exp()).sum::<f32>().ln();
        let start = out.len();
        out.extend(row.iter().map(|x| x - lse));
        if let Some(d) = delim {
            out[start + blank] = log_add(out[start + blank], out[start + d]);
        }
    }
    out
}

fn log_add(a: f32, b: f32) -> f32 {
    if a == f32::NEG_INFINITY {
        return b;
    }
    if b == f32::NEG_INFINITY {
        return a;
    }
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    hi + (lo - hi).exp().ln_1p()
}

/// The CTC lattice: blank, token, blank, token, …, blank; and where a state may skip one.
fn lattice(ids: &[usize], blank: usize) -> (Vec<usize>, Vec<bool>) {
    let mut ext = vec![blank];
    for &t in ids {
        ext.push(t);
        ext.push(blank);
    }
    let skip = (0..ext.len())
        .map(|s| s >= 2 && ext[s] != blank && ext[s] != ext[s - 2])
        .collect();
    (ext, skip)
}

/// log P(ids | audio) under CTC (forward algorithm). `logp` from [`log_probs`].
pub fn log_likelihood(logp: &[f32], vocab: usize, ids: &[usize], blank: usize) -> f32 {
    let frames = logp.len() / vocab;
    let (ext, skip) = lattice(ids, blank);
    let n = ext.len();
    if frames == 0 || n > 2 * frames + 1 {
        return f32::NEG_INFINITY;
    }
    let mut alpha = vec![f32::NEG_INFINITY; n];
    alpha[0] = logp[ext[0]];
    if n > 1 {
        alpha[1] = logp[ext[1]];
    }
    let mut next = vec![f32::NEG_INFINITY; n];
    for t in 1..frames {
        let row = &logp[t * vocab..(t + 1) * vocab];
        for s in 0..n {
            let mut a = alpha[s];
            if s >= 1 {
                a = log_add(a, alpha[s - 1]);
            }
            if skip[s] {
                a = log_add(a, alpha[s - 2]);
            }
            next[s] = a + row[ext[s]];
        }
        std::mem::swap(&mut alpha, &mut next);
    }
    if n > 1 {
        log_add(alpha[n - 1], alpha[n - 2])
    } else {
        alpha[0]
    }
}

/// Viterbi alignment: for each token of `ids` the first and last frame it is emitted on.
pub fn align(logp: &[f32], vocab: usize, ids: &[usize], blank: usize) -> Vec<(usize, usize)> {
    let frames = logp.len() / vocab;
    let (ext, skip) = lattice(ids, blank);
    let n = ext.len();
    if frames == 0 || n > 2 * frames + 1 {
        return Vec::new();
    }
    let mut delta = vec![f32::NEG_INFINITY; n];
    delta[0] = logp[ext[0]];
    if n > 1 {
        delta[1] = logp[ext[1]];
    }
    let mut back = vec![0u8; frames * n];
    let mut next = vec![f32::NEG_INFINITY; n];
    for t in 1..frames {
        let row = &logp[t * vocab..(t + 1) * vocab];
        for s in 0..n {
            let mut best = (delta[s], 0u8);
            if s >= 1 && delta[s - 1] > best.0 {
                best = (delta[s - 1], 1);
            }
            if skip[s] && delta[s - 2] > best.0 {
                best = (delta[s - 2], 2);
            }
            back[t * n + s] = best.1;
            next[s] = best.0 + row[ext[s]];
        }
        std::mem::swap(&mut delta, &mut next);
    }
    let mut s = if n == 1 || delta[n - 1] >= delta[n - 2] {
        n - 1
    } else {
        n - 2
    };
    let mut path = vec![0; frames];
    for t in (0..frames).rev() {
        path[t] = s;
        s -= back[t * n + s] as usize;
    }
    let mut spans = vec![(usize::MAX, 0); ids.len()];
    for (t, &s) in path.iter().enumerate() {
        if s % 2 == 1 {
            let span = &mut spans[s / 2];
            span.0 = span.0.min(t);
            span.1 = t;
        }
    }
    spans
}

/// The model's directory under [`crate::models::models_dir`].
pub const MODEL: &str = "kaiwa-kana-ctc";
/// Silence around the utterance, as in training and in the spike.
const PAD_SECONDS: f32 = 0.5;
pub const SAMPLE_RATE: u32 = 16_000;

/// A mora error the check reports: the variant beat the meant kana by `margin` over the
/// threshold of its kind.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub kind: Kind,
    pub mora: usize,
    pub variant: String,
    pub margin: f32,
}

/// The own kana CTC model (J-02) with its thresholds. `thresholds.json` names the kinds that are
/// checked at all (J-05: only where the false-alarm limit holds).
pub struct Checker {
    session: Session,
    vocab: HashMap<char, usize>,
    vocab_size: usize,
    blank: usize,
    delim: Option<usize>,
    thresholds: HashMap<Kind, f32>,
}

impl Checker {
    /// Loads `model.int8.onnx` (or `model.onnx`), `vocab.json` and `thresholds.json` from `dir`.
    pub fn from_dir(dir: &Path, threads: usize) -> Result<Self, SpeechError> {
        let missing = || SpeechError::ModelMissing(dir.display().to_string());
        let model = ["model.int8.onnx", "model.onnx"]
            .iter()
            .map(|f| dir.join(f))
            .find(|p| p.exists())
            .ok_or_else(missing)?;
        let read = |name: &str| -> Result<serde_json::Map<String, serde_json::Value>, SpeechError> {
            let text = std::fs::read_to_string(dir.join(name)).map_err(|_| missing())?;
            serde_json::from_str(&text).map_err(|e| SpeechError::ModelLoad(format!("{name}: {e}")))
        };
        let tokens = read("vocab.json")?;
        let mut vocab = HashMap::new();
        let (mut blank, mut delim, mut vocab_size) = (None, None, 0);
        for (token, id) in &tokens {
            let Some(id) = id.as_u64().map(|i| i as usize) else {
                continue;
            };
            vocab_size = vocab_size.max(id + 1);
            match token.as_str() {
                "[PAD]" => blank = Some(id),
                "|" => delim = Some(id),
                t => {
                    let mut cs = t.chars();
                    if let (Some(c), None) = (cs.next(), cs.next()) {
                        vocab.insert(c, id);
                    }
                }
            }
        }
        let blank = blank.ok_or_else(|| SpeechError::ModelLoad("vocab.json: no [PAD]".into()))?;
        let thresholds = read("thresholds.json")?
            .iter()
            .filter_map(|(k, v)| Some((Kind::from_name(k)?, v.as_f64()? as f32)))
            .collect();
        let load = |e: ort::Error| SpeechError::ModelLoad(format!("{} ({e})", model.display()));
        let onnx = crate::onnx();
        let session = Session::builder()
            .map_err(load)?
            .with_intra_threads(threads)
            .map_err(|e| load(e.into()))?
            .with_memory_pattern(false)
            .map_err(|e| load(e.into()))?
            .commit_from_file(&model)
            .map_err(load)?;
        drop(onnx);
        Ok(Self {
            session,
            vocab,
            vocab_size,
            blank,
            delim,
            thresholds,
        })
    }

    /// Frame-wise log probabilities of 16 kHz mono `samples` (frames × vocab, see [`log_probs`]).
    pub fn log_probs(&mut self, samples: &[f32]) -> Result<Vec<f32>, SpeechError> {
        let pad = (PAD_SECONDS * SAMPLE_RATE as f32) as usize;
        let mut x = vec![0.0f32; pad];
        x.extend_from_slice(samples);
        x.resize(x.len() + pad, 0.0);
        // As the feature extractor does: zero mean, unit variance
        let n = x.len() as f32;
        let mean = x.iter().sum::<f32>() / n;
        let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n;
        let scale = 1.0 / (var + 1e-7).sqrt();
        x.iter_mut().for_each(|v| *v = (*v - mean) * scale);
        let fail = |e: ort::Error| SpeechError::Voice(format!("mora check: {e}"));
        let len = x.len() as i64;
        let _onnx = crate::onnx();
        let outputs = self
            .session
            .run(ort::inputs!["samples" => Tensor::from_array((vec![1i64, len], x)).map_err(fail)?])
            .map_err(fail)?;
        let (_, logits) = outputs["logits"]
            .try_extract_tensor::<f32>()
            .map_err(fail)?;
        Ok(log_probs(logits, self.vocab_size, self.blank, self.delim))
    }

    fn ids(&self, kana: &str) -> Option<Vec<usize>> {
        kana.chars().map(|c| self.vocab.get(&c).copied()).collect()
    }

    /// The mora errors in `samples` when the learner meant `kana` (pronounced hiragana, see
    /// [`crate::g2p::G2p::pronounced`]): per checked kind the strongest variant over its
    /// threshold, strongest first.
    pub fn check(&mut self, samples: &[f32], kana: &str) -> Result<Vec<Finding>, SpeechError> {
        let logp = self.log_probs(samples)?;
        Ok(self.findings(&logp, kana))
    }

    fn findings(&self, logp: &[f32], kana: &str) -> Vec<Finding> {
        let Some(ids) = self.ids(kana) else {
            return Vec::new();
        };
        let base = log_likelihood(logp, self.vocab_size, &ids, self.blank);
        let mut best: HashMap<Kind, Finding> = HashMap::new();
        for v in variants(kana, Some('ー')) {
            let Some(&threshold) = self.thresholds.get(&v.kind) else {
                continue;
            };
            let Some(ids) = self.ids(&v.kana) else {
                continue;
            };
            let margin = log_likelihood(logp, self.vocab_size, &ids, self.blank) - base - threshold;
            if margin > 0.0 && best.get(&v.kind).is_none_or(|f| margin > f.margin) {
                best.insert(
                    v.kind,
                    Finding {
                        kind: v.kind,
                        mora: v.mora,
                        variant: v.kana,
                        margin,
                    },
                );
            }
        }
        let mut out: Vec<Finding> = best.into_values().collect();
        out.sort_by(|a, b| b.margin.total_cmp(&a.margin));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kanas(v: &[Variant]) -> Vec<(Kind, usize, &str)> {
        v.iter()
            .map(|v| (v.kind, v.mora, v.kana.as_str()))
            .collect()
    }

    #[test]
    fn moras_join_small_kana() {
        assert_eq!(moras("びょういん"), ["びょ", "う", "い", "ん"]);
        assert_eq!(moras("てぃっしゅ"), ["てぃ", "っ", "しゅ"]);
    }

    #[test]
    fn variants_match_the_spike() {
        // Expected lists from spikes/s06-moren/ctc.py
        use Kind::*;
        assert_eq!(
            kanas(&variants("おばあさん", None)),
            [
                (ShortLong, 0, "おうばあさん"),
                (ShortLong, 0, "おおばあさん"),
                (LongShort, 2, "おばさん"),
                (ShortLong, 3, "おばあさあん"),
                (GeminateAdded, 3, "おばあっさん"),
            ]
        );
        assert_eq!(
            kanas(&variants("きって", None)),
            [
                (ShortLong, 0, "きいって"),
                (GeminateLeftOut, 1, "きて"),
                (ShortLong, 2, "きってえ")
            ]
        );
        assert_eq!(
            kanas(&variants("げんいん", None)),
            [
                (ShortLong, 0, "げえんいん"),
                (NMerged, 1, "げにん"),
                (ShortLong, 2, "げんいいん")
            ]
        );
        assert_eq!(
            kanas(&variants("びょういん", None)),
            [
                (SmallY, 0, "びよういん"),
                (LongShort, 1, "びょいん"),
                (ShortLong, 2, "びょういいん")
            ]
        );
        assert_eq!(
            kanas(&variants("しゅじん", None)),
            [
                (ShortLong, 0, "しゅうじん"),
                (SmallY, 0, "しゆじん"),
                (ShortLong, 1, "しゅじいん")
            ]
        );
        assert_eq!(
            kanas(&variants("とーきょー", Some('ー'))),
            [
                (LongShort, 1, "ときょー"),
                (GeminateAdded, 2, "とーっきょー"),
                (SmallY, 2, "とーきよー"),
                (LongShort, 3, "とーきょ"),
            ]
        );
        assert!(kanas(&variants("げにん", None)).contains(&(NSplit, 1, "げんいん")));
    }

    // Logits (6 frames × 5 tokens: blank 0, 1–3, delimiter 4) and values from ctc.py.
    const LOGITS: [f32; 30] = [
        0.0012, 0.2987, -0.2741, -0.8906, -0.4547, -0.9916, 0.0601, 1.3402, -0.4922, -0.6205,
        0.4898, 0.3569, 0.1054, -0.9305, -0.0293, 0.6953, -1.3442, -0.4576, -1.9012, -1.2895,
        -1.8417, -0.2351, -1.2674, 0.2713, 0.1568, -0.1869, -2.5168, -0.5387, -0.0485, 0.1133,
    ];

    /// Needs the model (`KAIWA_MODELS`/kaiwa-kana-ctc).
    #[test]
    fn the_model_runs_on_silence() {
        let dir = crate::models::models_dir().join(MODEL);
        let Ok(mut checker) = Checker::from_dir(&dir, 2) else {
            eprintln!("skipped: no mora model in {}", dir.display());
            return;
        };
        let logp = checker.log_probs(&vec![0.0; SAMPLE_RATE as usize]).unwrap();
        assert_eq!(logp.len() % checker.vocab_size, 0);
        assert!(
            logp.len() / checker.vocab_size > 50,
            "about 50 frames per second"
        );
        checker
            .check(&vec![0.0; SAMPLE_RATE as usize], "きって")
            .unwrap();
    }

    #[test]
    fn kind_names_round_trip() {
        for k in ["long-", "long+", "q-", "q+", "n-", "n+", "y"] {
            assert_eq!(Kind::from_name(k).map(Kind::name), Some(k));
        }
        assert_eq!(Kind::from_name("x"), None);
    }

    #[test]
    fn forward_and_viterbi_match_the_spike() {
        let lp = log_probs(&LOGITS, 5, 0, Some(4));
        for (ids, want) in [
            (&[1, 2][..], -3.168_594_6),
            (&[1, 1][..], -4.215_431_6),
            (&[3][..], -4.384_317_6),
        ] {
            let got = log_likelihood(&lp, 5, ids, 0);
            assert!((got - want).abs() < 1e-3, "{ids:?}: {got} vs {want}");
        }
        let firsts: Vec<usize> = align(&lp, 5, &[1, 2, 3], 0).iter().map(|s| s.0).collect();
        assert_eq!(firsts, [0, 1, 4]);
        assert!(log_likelihood(&lp, 5, &[1, 2, 3, 1], 0).is_finite());
        assert_eq!(log_likelihood(&lp, 5, &[1; 7], 0), f32::NEG_INFINITY);
    }
}
