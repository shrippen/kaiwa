//! Language data for Kaiwa (/LD050/): the words of a sentence and the dictionary package
//! kaiwa-data (L-06, ADR-007), for the word lookup (/LF112/).

pub mod accent;
pub mod dict;
pub mod pos;
pub mod words;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use accent::Accent;
pub use dict::{Dictionary, Entry};
pub use words::{Word, Words};

/// The directory of the data package release this Kaiwa reads (kaiwa-data, L-06): the
/// archive's top directory, kept as it is under [`models_dir`].
pub const PACKAGE: &str = "kaiwa-data-0.2.0";

/// Where downloaded models and data live: `$KAIWA_MODELS`, else `$XDG_DATA_HOME/kaiwa/models`,
/// else `~/.local/share/kaiwa/models`. Nothing of it goes into the repository or the binary.
pub fn models_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("KAIWA_MODELS") {
        return PathBuf::from(dir);
    }
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    data.unwrap_or_else(|| PathBuf::from("."))
        .join("kaiwa/models")
}

/// The dictionary: `$KAIWA_DICT` (the demo's excerpt, tests), else the downloaded package.
pub fn dictionary_path() -> PathBuf {
    std::env::var_os("KAIWA_DICT")
        .map(PathBuf::from)
        .unwrap_or_else(|| models_dir().join(PACKAGE).join("kaiwa-dict.sqlite"))
}

#[derive(Debug, Error)]
pub enum LangError {
    #[error("segmenter: {0}")]
    Segmenter(String),
    #[error("dictionary: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("the dictionary has schema {0:?}, this Kaiwa reads schema {schema}", schema = dict::SCHEMA)]
    Schema(Option<u32>),
}

/// A piece of a line for KanteRubyText (Kante 1.33): text, its reading, whether the word is
/// new, and the number of its word (`None` for punctuation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Segment {
    pub text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reading: String,
    #[serde(rename = "isNew", skip_serializing_if = "std::ops::Not::not")]
    pub is_new: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word: Option<usize>,
}

/// A word of a line, to look up when it is clicked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineWord {
    /// As written.
    pub text: String,
    /// What to look up (dictionary form).
    pub key: String,
    /// The reading of the key, if known: the furigana of the line for a word in its
    /// dictionary form, else IPADIC's.
    pub reading: Option<String>,
    /// IPADIC part of speech (助詞, 動詞 …), to choose between entries alike.
    pub pos: String,
    /// What to look up when the group is not in the dictionary (弁当 for お弁当).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
}

/// Furigana markup (`{駅|えき}までは{歩|ある|new}いて`, as in KanteRubyText) to segments with
/// word numbers and the words behind them. A reading never splits: a word boundary inside a
/// word with furigana goes to the end of that word.
pub fn annotate(words: &Words, markup: &str) -> (Vec<Segment>, Vec<LineWord>) {
    let marked = parse(markup);
    let plain: String = marked.iter().map(|m| m.text.as_str()).collect();
    let found = words.words(&plain);

    // The word at each byte of the plain text.
    let mut at: Vec<Option<usize>> = vec![None; plain.len()];
    for (i, w) in found.iter().enumerate() {
        for slot in &mut at[w.start..w.end] {
            *slot = Some(i);
        }
    }

    let mut segments: Vec<Segment> = Vec::new();
    let mut readings: Vec<String> = vec![String::new(); found.len()];
    let mut pos = 0;
    for m in &marked {
        let start = pos;
        pos += m.text.len();
        if !m.reading.is_empty() {
            let word = at[start..pos].iter().flatten().next().copied();
            if let Some(w) = word {
                readings[w].push_str(&m.reading);
            }
            segments.push(Segment {
                text: m.text.clone(),
                reading: m.reading.clone(),
                is_new: m.is_new,
                word,
            });
            continue;
        }
        // Plain text: split where the word changes.
        let mut run = String::new();
        let mut run_word = None;
        for (off, ch) in m.text.char_indices() {
            let w = at[start + off];
            if !run.is_empty() && w != run_word {
                segments.push(Segment {
                    text: std::mem::take(&mut run),
                    reading: String::new(),
                    is_new: false,
                    word: run_word,
                });
            }
            run_word = w;
            run.push(ch);
            if let Some(w) = w {
                readings[w].push(ch);
            }
        }
        if !run.is_empty() {
            segments.push(Segment {
                text: run,
                reading: String::new(),
                is_new: false,
                word: run_word,
            });
        }
    }

    let line_words = found
        .iter()
        .zip(readings)
        .map(|(w, marked)| LineWord {
            text: w.surface.clone(),
            key: w.key.clone(),
            reading: if w.plain {
                // Only when the furigana cover every kanji of the word; else IPADIC's.
                Some(words::hiragana(&marked)).filter(|r| {
                    !r.is_empty() && !r.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c))
                })
            } else {
                None
            }
            .or_else(|| (w.plain && !w.reading.is_empty()).then(|| w.reading.clone())),
            pos: w.pos.clone(),
            fallback: w.fallback.clone(),
        })
        .collect();
    (segments, line_words)
}

struct Marked {
    text: String,
    reading: String,
    is_new: bool,
}

fn parse(markup: &str) -> Vec<Marked> {
    let mut out = Vec::new();
    let mut rest = markup;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|c| open + c) else {
            break;
        };
        let inner = &rest[open + 1..close];
        let parts: Vec<&str> = inner.split('|').collect();
        if parts.len() < 2 {
            break;
        }
        if open > 0 {
            out.push(Marked {
                text: rest[..open].to_string(),
                reading: String::new(),
                is_new: false,
            });
        }
        out.push(Marked {
            text: parts[0].to_string(),
            reading: parts[1].to_string(),
            is_new: parts.get(2) == Some(&"new"),
        });
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        out.push(Marked {
            text: rest.to_string(),
            reading: String::new(),
            is_new: false,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_becomes_segments_with_word_numbers() {
        let words = Words::new().unwrap();
        let (segs, ws) = annotate(
            &words,
            "{駅|えき}までは{歩|ある|new}いて{五分|ごふん}です。",
        );
        let text: String = segs.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "駅までは歩いて五分です。");
        // 歩 (with its reading) and いて are one word.
        let aru = segs.iter().find(|s| s.text == "歩").unwrap();
        assert!(aru.is_new);
        let ite = segs.iter().find(|s| s.text == "いて").unwrap();
        assert_eq!(aru.word, ite.word);
        assert_eq!(ws[aru.word.unwrap()].key, "歩く");
        // The period is no word.
        assert_eq!(segs.last().unwrap().word, None);
        // A plain word takes its reading from the furigana.
        let eki = &ws[segs[0].word.unwrap()];
        assert_eq!(
            (eki.key.as_str(), eki.reading.as_deref()),
            ("駅", Some("えき"))
        );
        let gofun = ws.iter().find(|w| w.text == "五分").unwrap();
        assert_eq!(gofun.reading.as_deref(), Some("ごふん"));
    }

    #[test]
    fn plain_text_without_markup() {
        let words = Words::new().unwrap();
        let (segs, ws) = annotate(&words, "コーヒーが好きです。駅");
        assert_eq!(ws[0].key, "コーヒー");
        // Without furigana the reading comes from IPADIC, never the kanji themselves.
        assert_eq!(ws.last().unwrap().reading.as_deref(), Some("えき"));
        assert!(segs.iter().all(|s| s.reading.is_empty()));
        assert_eq!(ws[2].text, "好き");
    }

    #[test]
    fn broken_markup_stays_text() {
        let m = parse("あ{い");
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].text, "あ{い");
    }
}
