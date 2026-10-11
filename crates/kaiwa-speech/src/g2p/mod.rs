//! Japanese text → the phonemes Kokoro was trained on (H-02). A port of misaki's Japanese
//! G2P (0.7.4) onto lindera with IPADIC instead of MeCab with UniDic. Readings come from the
//! dictionary's pronunciation field, so the particle は reads わ and 東京 とーきょー.
//!
//! There is no pitch accent: misaki has none either (spike 0.4), which is why Kokoro is only
//! the interim voice for the character and never a model to repeat after.

mod numbers;
mod table;

use std::borrow::Cow;
use std::collections::HashMap;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use unicode_normalization::UnicodeNormalization;

use crate::SpeechError;

/// Small kana that only modify the kana before them.
const SUTEGANA: &str = "ゃゅょぁぃぅぇぉ";
/// Phonemes that close a phrase: no space before them.
const CLOSE: &str = "]).,?!:";

pub struct G2p {
    segmenter: Segmenter,
    table: HashMap<&'static str, &'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A word with a reading.
    Kana,
    /// Punctuation.
    Symbol,
    /// Latin letters go through as they are.
    Ascii,
    /// No reading (kanji the dictionary does not know): silent, as in misaki.
    Unknown,
}

struct Word {
    surface: String,
    kana: String,
    kind: Kind,
    /// Part of speech and its two subdivisions of the last token in the word.
    pos: String,
    sub: [String; 2],
    /// The final kana of a verb keeps its sound (思う stays omoɯ, not omoː).
    verb_end: Option<usize>,
}

/// Whether a token joins the word before it. misaki groups by a list of 147,000 words; these
/// rules get the same grouping for the common cases of a conversation.
fn joins(prev_pos: &str, prev_sub: &[String; 2], pos: &str, sub: &[String; 2], base: &str) -> bool {
    match pos {
        // Auxiliaries chain (まし・た → maɕita), but です and だ start a word (ikʲi tai desɨ).
        "助動詞" if base == "です" || base == "だ" => false,
        "助動詞" => prev_pos == "助動詞" || prev_pos == "動詞" && prev_sub[0] == "非自立",
        // The connecting て/で belongs to the verb (待っ・て → maʔte).
        "助詞" => sub[0] == "接続助詞" && ["動詞", "形容詞", "助動詞"].contains(&prev_pos),
        // Suffixes of compounds (郵便・局), but not さん or counters (tanaka saɴ, ɲi neɴ).
        "名詞" => sub[0] == "接尾" && sub[1] == "一般" && prev_pos == "名詞",
        _ => false,
    }
}

/// Long vowels as UniDic writes them and Tokyo speech has them: えい → えー, おう → おー,
/// いい → いー, うう → うー, within a word.
fn lengthen(kana: &str, keep: Option<usize>) -> String {
    let chars: Vec<char> = kana.chars().collect();
    let mut out = String::with_capacity(kana.len());
    for (i, &c) in chars.iter().enumerate() {
        let vowel = i.checked_sub(1).and_then(|j| vowel_of(chars[j]));
        let long = Some(i) != keep
            && match c {
                'い' => matches!(vowel, Some('e' | 'i')),
                'う' => matches!(vowel, Some('o' | 'u')),
                _ => false,
            };
        out.push(if long { 'ー' } else { c });
    }
    out
}

/// The vowel a hiragana ends in (a i u e o), for the small kana too.
fn vowel_of(c: char) -> Option<char> {
    const ROWS: [(&str, char); 5] = [
        ("あかがさざただなはばぱまやらわぁゃゎ", 'a'),
        ("いきぎしじちぢにひびぴみりぃ", 'i'),
        ("うくぐすずつづぬふぶぷむゆるぅゅゔ", 'u'),
        ("えけげせぜてでねへべぺめれぇ", 'e'),
        ("おこごそぞとどのほぼぽもよろをぉょ", 'o'),
    ];
    ROWS.iter()
        .find(|(row, _)| row.contains(c))
        .map(|(_, v)| *v)
}

impl G2p {
    pub fn new() -> Result<Self, SpeechError> {
        let dict = load_dictionary("embedded://ipadic")
            .map_err(|e| SpeechError::ModelLoad(format!("IPADIC: {e}")))?;
        Ok(Self {
            segmenter: Segmenter::new(Mode::Normal, dict, None),
            table: table::KANA.iter().copied().collect(),
        })
    }

    /// The phoneme string for Kokoro, e.g. 「コーヒーが好きです。」 → `koːçiː ɡa sɨkʲi desɨ.`
    pub fn phonemes(&self, text: &str) -> String {
        let words = self.words(&normalize(text));
        let mut out: Vec<(String, bool)> = Vec::with_capacity(words.len());
        for word in &words {
            let roma = self.word(word);
            let mut space = true;
            if "「『«".contains(word.surface.as_str()) || "([".contains(roma.as_str()) {
                if let Some(prev) = out.last_mut() {
                    prev.1 = true;
                }
                space = false;
            } else if "」』»".contains(word.surface.as_str()) || CLOSE.contains(roma.as_str()) {
                if let Some(prev) = out.last_mut() {
                    prev.1 = false;
                }
            } else if roma == " " {
                space = false;
            }
            out.push((roma, space));
        }
        let joined: String = out
            .iter()
            .map(|(r, space)| if *space { format!("{r} ") } else { r.clone() })
            .collect();
        tidy(&joined)
    }

    /// The pronounced hiragana of a text, as the mora check's model writes them: 「東京から来ました。」
    /// → とーきょーからきました, は as わ. Punctuation, latin letters and unknown kanji drop out.
    pub fn pronounced(&self, text: &str) -> String {
        self.words(&normalize(text))
            .into_iter()
            .filter(|w| w.kind == Kind::Kana)
            .map(|w| w.kana)
            .collect()
    }

    fn words(&self, text: &str) -> Vec<Word> {
        let Ok(tokens) = self.segmenter.segment(Cow::Borrowed(text)) else {
            return Vec::new();
        };
        let mut words: Vec<Word> = Vec::new();
        for mut tok in tokens {
            let surface = tok.surface.to_string();
            let details: Vec<String> = tok.details().iter().map(|s| s.to_string()).collect();
            let field = |i: usize| details.get(i).filter(|s| s.as_str() != "*").cloned();
            let pos = field(0).unwrap_or_default();
            let sub = [field(1).unwrap_or_default(), field(2).unwrap_or_default()];
            let base = field(6).unwrap_or_default();
            let reading = field(8).or_else(|| field(7));
            // A number reads as one word (さんびゃく, not さん・びゃく).
            let reading = if surface.bytes().all(|b| b.is_ascii_digit()) {
                Some(numbers::kana(&surface))
            } else {
                reading
            };
            let kind = if pos == "記号" || surface.chars().all(|c| self.is_symbol(c)) {
                Kind::Symbol
            } else if reading.is_some() || surface.chars().all(is_kana) {
                Kind::Kana
            } else if surface.is_ascii() {
                Kind::Ascii
            } else {
                Kind::Unknown
            };
            let kana = hiragana(&reading.unwrap_or_else(|| surface.clone()));
            let joins = words.last().is_some_and(|prev| {
                kind == Kind::Kana
                    && prev.kind == Kind::Kana
                    && joins(&prev.pos, &prev.sub, &pos, &sub, &base)
            });
            match words.last_mut() {
                Some(prev) if joins => {
                    prev.surface.push_str(&surface);
                    if pos == "動詞" {
                        prev.verb_end = Some(prev.kana.chars().count() + kana.chars().count() - 1);
                    }
                    prev.kana.push_str(&kana);
                    prev.pos = pos;
                    prev.sub = sub;
                }
                _ => {
                    let verb_end = (pos == "動詞").then(|| kana.chars().count().saturating_sub(1));
                    words.push(Word {
                        surface,
                        kana,
                        kind,
                        pos,
                        sub,
                        verb_end,
                    });
                }
            }
        }
        for w in words.iter_mut().filter(|w| w.kind == Kind::Kana) {
            w.kana = lengthen(&w.kana, w.verb_end);
        }
        words
    }

    fn is_symbol(&self, c: char) -> bool {
        let mut buf = [0u8; 4];
        let s: &str = c.encode_utf8(&mut buf);
        !is_kana(c) && self.table.contains_key(s)
    }

    fn word(&self, word: &Word) -> String {
        match word.kind {
            Kind::Ascii => word.surface.clone(),
            Kind::Unknown => String::new(),
            Kind::Symbol => word
                .surface
                .chars()
                .map(|c| {
                    let mut buf = [0u8; 4];
                    let s: &str = c.encode_utf8(&mut buf);
                    self.table.get(s).map_or(c.to_string(), |p| p.to_string())
                })
                .collect(),
            Kind::Kana => {
                let kana: Vec<char> = word.kana.chars().collect();
                (0..kana.len())
                    .map(|i| {
                        let prev = i.checked_sub(1).map(|j| kana[j]);
                        self.kana(prev, kana[i], kana.get(i + 1).copied())
                    })
                    .collect()
            }
        }
    }

    fn get(&self, k: &str) -> Option<&'static str> {
        self.table.get(k).copied()
    }

    /// One kana with its neighbours (misaki `_get_single_mapping`).
    fn kana(&self, prev: Option<char>, k: char, next: Option<char>) -> String {
        let pair = |a: char, b: char| format!("{a}{b}");
        match k {
            'ゝ' | 'ヽ' => return prev.map(|p| self.kana(None, p, None)).unwrap_or_default(),
            'ゞ' | 'ヾ' | '々' | '〃' => return String::new(),
            _ => {}
        }
        if let Some(p) = prev
            && let Some(ph) = self.get(&pair(p, k))
        {
            return ph.into();
        }
        if let Some(n) = next
            && self.get(&pair(k, n)).is_some()
        {
            return String::new();
        }
        if let Some(n) = next
            && SUTEGANA.contains(n)
        {
            if k == 'っ' {
                return String::new();
            }
            let base = self.get(&k.to_string()).unwrap_or("");
            let mut base: Vec<char> = base.chars().collect();
            base.pop();
            let small = self.get(&n.to_string()).unwrap_or("");
            return base.into_iter().collect::<String>() + small;
        }
        if SUTEGANA.contains(k) {
            return String::new();
        }
        match k {
            'ー' => "ː".into(),
            'っ' => "ʔ".into(),
            'ん' => {
                // m before m, p, b; ŋ before k, ɡ; ɲ before ɲ, ʨ, ʥ; n before n, t, d, ɾ, z; else ɴ.
                let after = next.and_then(|n| self.get(&n.to_string())).unwrap_or("");
                match after.chars().next() {
                    Some('m' | 'p' | 'b') => "m",
                    Some('k' | 'ɡ') => "ŋ",
                    Some('ɲ' | 'ʨ' | 'ʥ') => "ɲ",
                    Some('n' | 't' | 'd' | 'ɾ' | 'z') => "n",
                    _ => "ɴ",
                }
                .into()
            }
            _ => self.get(&k.to_string()).unwrap_or("").into(),
        }
    }
}

fn is_kana(c: char) -> bool {
    ('\u{3041}'..='\u{30ff}').contains(&c)
}

/// Katakana to hiragana; ー and the rare ヷ…ヺ stay.
fn hiragana(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{30a1}'..='\u{30f6}' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// NFKC (full-width Latin and digits to half-width, half-width katakana to full-width), a wave
/// dash before a number reads から, and numbers stand apart as digits.
fn normalize(text: &str) -> String {
    let mut wave = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let before_digit = chars
            .get(i + 1)
            .is_some_and(|n| n.is_ascii_digit() || ('０'..='９').contains(n));
        if (c == '〜' || c == '～') && before_digit {
            wave.push_str("から");
        } else {
            wave.push(c);
        }
    }
    let nfkc: String = kanji_numbers(&wave.nfkc().collect::<String>());
    let mut out = String::with_capacity(nfkc.len());
    let mut digits = String::new();
    for c in nfkc.chars().chain(std::iter::once('\0')) {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        if !digits.is_empty() {
            out.push(' ');
            out.push_str(&digits);
            digits.clear();
        }
        if c != '\0' {
            out.push(c);
        }
    }
    out
}

/// Kanji numbers the dictionary would split (三・百 → san çakɯ) become digits first.
fn kanji_numbers(text: &str) -> String {
    const NUMERALS: &str = "〇一二三四五六七八九十百千万億";
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    for c in text.chars().chain(std::iter::once('\0')) {
        if NUMERALS.contains(c) {
            run.push(c);
            continue;
        }
        if !run.is_empty() {
            out.push_str(&numbers::from_kanji(&run).unwrap_or_else(|| run.clone()));
            run.clear();
        }
        if c != '\0' {
            out.push(c);
        }
    }
    out
}

/// Collapse spaces, brackets as Kokoro knows them, and no space around a glottal stop
/// unless punctuation or a quote sits there.
fn tidy(s: &str) -> String {
    let collapsed: Vec<char> = s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('(', "«")
        .replace(')', "»")
        .chars()
        .collect();
    let mut out = String::with_capacity(collapsed.len());
    for (i, &c) in collapsed.iter().enumerate() {
        if c == ' ' {
            let prev = i.checked_sub(1).map(|j| collapsed[j]);
            let next = collapsed.get(i + 1).copied();
            let before_stop =
                next == Some('ʔ') && !prev.is_some_and(|p| "!\",.:;?»—…”".contains(p));
            let after_stop = prev == Some('ʔ') && !next.is_some_and(|n| "\"«“".contains(n));
            if before_stop || after_stop {
                continue;
            }
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g2p() -> G2p {
        G2p::new().unwrap()
    }

    #[test]
    fn sentences_read_as_misaki_reads_them() {
        let g = g2p();
        for (text, want) in [
            ("コーヒーが好きです。", "koːçiː ɡa sɨkʲi desɨ."),
            ("ビールを二つください。", "bʲiːɾɯ o ɸɯtaʦɨ kɯdasai."),
            ("きのう東京に行きました。", "kʲinoː toːkʲoː ɲi ikʲi maɕita."),
            ("ちょっと待ってください。", "ʨoʔto maʔte kɯdasai."),
            ("これはいくらですか。", "koɾe βa ikɯɾa desɨ ka."),
            ("おいしいです。", "oiɕiː desɨ."),
            ("映画を見ました。", "eːɡa o mʲi maɕita."),
            ("日本に行きたいです。", "ɲiʔpoɴ ɲi ikʲi tai desɨ."),
            ("ドアが開きました。", "doa ɡa çiɾakʲi maɕita."),
            ("三百円です。", "sambʲakɯ eɴ desɨ."),
        ] {
            assert_eq!(g.phonemes(text), want, "{text}");
        }
    }

    #[test]
    fn particles_read_by_sound_and_n_assimilates() {
        let g = g2p();
        assert_eq!(g.phonemes("東京へ"), "toːkʲoː e");
        assert_eq!(g.phonemes("こんばんは"), "kombaɴβa");
        assert_eq!(g.phonemes("てんぷら"), "tempɯɾa");
        assert_eq!(g.phonemes("思う"), "omoɯ", "a verb keeps its final う");
        assert_eq!(g.phonemes("行きましょう"), "ikʲi maɕoː");
    }

    #[test]
    fn numbers_and_full_width_digits() {
        let g = g2p();
        assert_eq!(g.phonemes("８時"), g.phonemes("8時"));
        assert!(
            g.phonemes("1万円").starts_with("iʨi"),
            "{}",
            g.phonemes("1万円")
        );
    }

    #[test]
    fn pronounced_kana_as_openjtalk_gives_them() {
        // Expected: pyopenjtalk 'pron' (spikes/s06-moren, ctc.pron)
        let g = g2p();
        for (text, want) in [
            ("東京から来ました。", "とーきょーからきました"),
            ("原因はわかりません。", "げんいんわわかりません"),
            ("ちょっと待ってください。", "ちょっとまってください"),
            ("おばあさんは元気です。", "おばーさんわげんきです"),
            ("切手を三枚ください。", "きってをさんまいください"),
        ] {
            assert_eq!(g.pronounced(text), want, "{text}");
        }
    }

    #[test]
    fn small_helpers() {
        assert_eq!(hiragana("コーヒー"), "こーひー");
        assert_eq!(tidy("ʨo ʔ to  (a)"), "ʨoʔto «a»");
        assert!(normalize("3〜5").contains("から"));
    }
}
