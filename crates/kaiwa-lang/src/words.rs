//! The words of a sentence, for the word lookup (/LF112/): lindera with IPADIC splits the text
//! into tokens, and these rules group them into the words a learner would click: a verb with
//! its endings (食べ・まし・た → 食べました, looked up as 食べる), a number with its counter
//! (五・分 → 五分). Particles stay words of their own; punctuation is no word.

use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use serde::Serialize;

use crate::LangError;

/// One word of a sentence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Word {
    /// As written in the sentence.
    pub surface: String,
    /// Byte range in the text.
    pub start: usize,
    pub end: usize,
    /// What to look up: the dictionary form of its first token (食べる for 食べました), or the
    /// surface for a noun group (五分).
    pub key: String,
    /// The reading of the surface in hiragana, from IPADIC ("" when unknown). A number group
    /// (二人, 一本, 三日) has none: its tokens' readings strung together miss the sound changes
    /// and special readings (ににん, いちほん, さんにち).
    pub reading: String,
    /// Whether the surface is the dictionary form (then the reading belongs to the key).
    pub plain: bool,
    /// IPADIC part of speech of the first token (名詞, 動詞 …).
    pub pos: String,
    /// For a group: what to look up when the whole is not in the dictionary (弁当 for お弁当,
    /// 五 for 五分).
    pub fallback: Option<String>,
    /// A proper noun (固有名詞: names, places), whose reading IPADIC often guesses.
    #[serde(skip)]
    pub proper: bool,
}

pub struct Words {
    segmenter: Segmenter,
}

struct Token {
    surface: String,
    start: usize,
    end: usize,
    pos: String,
    sub: String,
    base: String,
    reading: String,
}

impl Words {
    pub fn new() -> Result<Self, LangError> {
        let dictionary = load_dictionary("embedded://ipadic")
            .map_err(|e| LangError::Segmenter(e.to_string()))?;
        Ok(Self {
            segmenter: Segmenter::new(Mode::Normal, dictionary, None),
        })
    }

    /// The tokens of `text`; a つ right after digits (1つ, ２つ) is cut off first, as IPADIC
    /// otherwise reads it into the next word (1つください: a verb つください).
    fn tokens(&self, text: &str) -> Vec<Token> {
        let digit = |c: char| c.is_ascii_digit() || ('０'..='９').contains(&c);
        let mut cuts = Vec::new();
        let mut prev = None;
        for (i, c) in text.char_indices() {
            if c == 'つ' && prev.is_some_and(digit) {
                cuts.push(i);
                cuts.push(i + c.len_utf8());
            }
            prev = Some(c);
        }
        if cuts.is_empty() {
            return self.tokens_of(text, 0);
        }
        let mut out = Vec::new();
        let mut from = 0;
        for cut in cuts.into_iter().chain([text.len()]) {
            if cut > from {
                out.extend(self.tokens_of(&text[from..cut], from));
            }
            from = cut;
        }
        out
    }

    fn tokens_of(&self, text: &str, offset: usize) -> Vec<Token> {
        let Ok(tokens) = self.segmenter.segment(Cow::Borrowed(text)) else {
            return Vec::new();
        };
        tokens
            .into_iter()
            .map(|mut t| {
                let surface = t.surface.to_string();
                let (start, end) = (t.byte_start + offset, t.byte_end + offset);
                let details: Vec<String> = t.details().iter().map(|s| s.to_string()).collect();
                let field = |i: usize| {
                    details
                        .get(i)
                        .filter(|s| s.as_str() != "*")
                        .cloned()
                        .unwrap_or_default()
                };
                let base = match field(6) {
                    b if b.is_empty() => surface.clone(),
                    b => b,
                };
                Token {
                    start,
                    end,
                    pos: field(0),
                    sub: field(1),
                    base,
                    reading: hiragana(&field(7)),
                    surface,
                }
            })
            .collect()
    }

    /// The words of a text, in order. Punctuation, spaces and latin letters are left out.
    pub fn words(&self, text: &str) -> Vec<Word> {
        let mut out: Vec<Word> = Vec::new();
        let mut prev: Option<(String, String)> = None;
        // The last word is a number group: its reading stays unknown.
        let mut counted = false;
        // Where a number in digits ends (2人, ３本): the counter after it has no reading of
        // its own either (ふたり, さんぼん, not にん, ほん).
        let mut digits_end = None;
        for t in self.tokens(text) {
            if t.pos == "記号" || t.pos.is_empty() || !t.surface.chars().any(japanese) {
                prev = None;
                let digit = |c: char| c.is_ascii_digit() || ('０'..='９').contains(&c);
                digits_end = (t.surface.chars().all(|c| digit(c) || c == ',')
                    && t.surface.chars().any(digit))
                .then_some(t.end);
                continue;
            }
            // IPADIC cuts kana it does not know before a small kana (し・ゅみ for しゅみ); no
            // word starts with one, so the piece and the token before are one unknown word.
            if t.surface.starts_with(|c: char| SMALL.contains(c))
                && let (Some(_), Some(w)) = (&prev, out.last_mut())
                && w.end == t.start
            {
                w.surface.push_str(&t.surface);
                w.end = t.end;
                w.key = w.surface.clone();
                w.reading = hiragana(&w.surface);
                w.plain = true;
                w.pos = "名詞".into();
                w.fallback = None;
                prev = Some((w.pos.clone(), String::new()));
                continue;
            }
            let joins = match (&prev, out.last()) {
                (Some((pp, psub)), Some(_)) => joins(pp, psub, &t),
                _ => false,
            };
            if joins {
                let w = out.last_mut().expect("checked above");
                // A prefix makes a noun with the noun after it (お・弁当 → お弁当).
                if w.pos == "接頭詞" {
                    w.pos = t.pos.clone();
                    w.fallback = Some(t.base.clone());
                } else if w.pos == "名詞" && w.fallback.is_none() {
                    w.fallback = Some(w.key.clone());
                }
                w.surface.push_str(&t.surface);
                w.end = t.end;
                counted |= prev.as_ref().is_some_and(|(_, sub)| sub == "数") || t.sub == "数";
                if counted {
                    w.reading.clear();
                } else {
                    w.reading.push_str(&t.reading);
                }
                if w.pos == "名詞" {
                    // A noun group is looked up as written (五分), its first token as a fallback.
                    w.key = w.surface.clone();
                } else {
                    w.plain = false;
                }
            } else {
                counted = digits_end == Some(t.start) && t.pos == "名詞";
                out.push(Word {
                    plain: t.surface == t.base,
                    key: t.base.clone(),
                    surface: t.surface.clone(),
                    start: t.start,
                    end: t.end,
                    reading: if counted {
                        String::new()
                    } else {
                        t.reading.clone()
                    },
                    pos: t.pos.clone(),
                    fallback: None,
                    proper: t.sub == "固有名詞",
                });
            }
            digits_end = None;
            prev = Some((t.pos, t.sub));
        }
        out
    }
}

/// Whether a token belongs to the word before it.
fn joins(prev_pos: &str, prev_sub: &str, t: &Token) -> bool {
    let inflecting = matches!(prev_pos, "動詞" | "形容詞" | "助動詞");
    match t.pos.as_str() {
        // です and だ stand alone; other auxiliaries end the verb or adjective (まし・た, ない).
        "助動詞" => inflecting && !matches!(t.base.as_str(), "です" | "だ"),
        // て and で, ば after a verb: 歩い・て.
        "助詞" => {
            inflecting && t.sub == "接続助詞" && matches!(t.base.as_str(), "て" | "で" | "ば")
        }
        // いる, しまう … after て: 食べ・て・いる.
        "動詞" => {
            matches!(t.sub.as_str(), "非自立" | "接尾") && (inflecting || prev_pos == "助詞")
        }
        "形容詞" => t.sub == "非自立" && inflecting,
        // Counters and suffixes after a noun: 五・分, 田中・さん; a noun after a prefix: お・弁当.
        "名詞" => {
            prev_pos == "接頭詞"
                || prev_pos == "名詞" && (t.sub == "接尾" || prev_sub == "数" && t.sub == "数")
        }
        _ => false,
    }
}

/// Small kana, which never start a word.
const SMALL: &str = "ゃゅょぁぃぅぇぉゎっャュョァィゥェォヮッ";

/// Kana or kanji: a token without any is no word to look up (latin letters, digits).
fn japanese(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{30ff}' | '\u{3400}'..='\u{9fff}' | '々')
}

/// Katakana to hiragana; everything else unchanged.
pub fn hiragana(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ァ'..='ヶ' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surfaces(text: &str) -> Vec<(String, String)> {
        Words::new()
            .unwrap()
            .words(text)
            .into_iter()
            .map(|w| (w.surface, w.key))
            .collect()
    }

    fn pair(a: &str, b: &str) -> (String, String) {
        (a.into(), b.into())
    }

    #[test]
    fn a_verb_keeps_its_endings_and_is_looked_up_in_its_dictionary_form() {
        let w = surfaces("昨日、すしを食べました。");
        assert!(w.contains(&pair("食べました", "食べる")), "{w:?}");
        assert!(w.contains(&pair("を", "を")), "{w:?}");
        assert!(!w.iter().any(|(s, _)| s == "、" || s == "。"), "{w:?}");
    }

    #[test]
    fn te_forms_and_counters_stay_one_word() {
        let w = surfaces("駅までは歩いて五分です。");
        assert!(w.contains(&pair("歩いて", "歩く")), "{w:?}");
        assert!(w.contains(&pair("五分", "五分")), "{w:?}");
        assert!(w.contains(&pair("です", "です")), "{w:?}");
        let w = surfaces("お弁当、温めますか。");
        assert!(w.contains(&pair("お弁当", "お弁当")), "{w:?}");
    }

    #[test]
    fn kana_ipadic_does_not_know_stay_one_word() {
        let w = surfaces("しゅみ、なに？");
        assert_eq!(w[0], pair("しゅみ", "しゅみ"));
    }

    #[test]
    fn latin_letters_are_no_words() {
        let w = surfaces("Spricht man 「です」 in the café?");
        assert_eq!(w, [pair("です", "です")]);
    }

    #[test]
    fn ranges_point_into_the_text() {
        let text = "コーヒーが好きです。";
        for w in Words::new().unwrap().words(text) {
            assert_eq!(&text[w.start..w.end], w.surface);
        }
    }

    #[test]
    fn readings_come_in_hiragana() {
        let w = Words::new().unwrap().words("駅");
        assert_eq!(w[0].reading, "えき");
        assert!(w[0].plain);
        assert_eq!(hiragana("プロデューサー"), "ぷろでゅーさー");
    }
}
