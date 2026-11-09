//! Language data for Kaiwa (/LD050/): the words of a sentence and the dictionary package
//! kaiwa-data (L-06, ADR-007), for the word lookup (/LF112/).

pub mod accent;
pub mod dict;
pub mod pos;
pub mod unidic;
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

/// Furigana markup for plain text (the learner's own line, recognised or typed; kanji the
/// tutor left without a reading): the readings IPADIC gives, kana around the kanji kept apart
/// (`{歩|ある}いて`). Cautious (owner, 9.11.2026): IPADIC reads without the sentence, so a
/// word it often reads wrong stays without ([`unsure`]); a number with its counter has no
/// reading anyway. A word whose reading does not fit its kana gets the reading over the
/// whole word. Braces and bars in the text become full-width, so they stay text.
pub fn furigana(words: &Words, text: &str) -> String {
    let safe = |t: &str| t.replace('{', "｛").replace('}', "｝").replace('|', "｜");
    // The words with a fixed reading first; the dictionaries read the parts between them.
    let mut out = String::new();
    let mut rest = text;
    while let Some((at, word, markup)) = FIXED
        .iter()
        .filter_map(|(w, m)| rest.find(w).map(|at| (at, *w, *m)))
        .min_by_key(|(at, w, _)| (*at, std::cmp::Reverse(w.len())))
    {
        out.push_str(&read(words, &rest[..at], &safe));
        out.push_str(markup);
        rest = &rest[at + word.len()..];
    }
    out.push_str(&read(words, rest, &safe));
    out
}

/// Words both dictionaries read wrong, with the reading people say (お父さん: UniDic ちち,
/// 富士山: やま).
const FIXED: &[(&str, &str)] = &[
    ("お父さん", "お{父|とう}さん"),
    ("お母さん", "お{母|かあ}さん"),
    ("お兄さん", "お{兄|にい}さん"),
    ("お姉さん", "お{姉|ねえ}さん"),
    ("お祖父さん", "お{祖父|じい}さん"),
    ("お祖母さん", "お{祖母|ばあ}さん"),
    ("富士山", "{富士山|ふじさん}"),
];

fn read(words: &Words, text: &str, safe: &impl Fn(&str) -> String) -> String {
    if text.is_empty() {
        return String::new();
    }
    match unidic::tokens(text) {
        Some(tokens) => with_unidic(text, &tokens, safe),
        None => with_ipadic(words, text, safe),
    }
}

/// [`furigana`] with IPADIC's readings, cautiously ([`unsure`]).
fn with_ipadic(words: &Words, text: &str, safe: &impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut pos = 0;
    for w in words.words(text) {
        if w.start < pos {
            continue;
        }
        out.push_str(&safe(&text[pos..w.start]));
        let reading = if unsure(&w) { "" } else { w.reading.as_str() };
        out.push_str(&ruby(&safe(&w.surface), reading));
        pos = w.end;
    }
    out.push_str(&safe(&text[pos..]));
    out
}

/// Markup with furigana over every kanji: what the markup reads stays (the scenario's, the
/// tutor's, `new` included), the plain parts get IPADIC's as in [`furigana`]. A tutor reply
/// usually comes without markup; the reading aid over every kanji is the default (/LF110/).
pub fn complete(words: &Words, markup: &str) -> String {
    parse(markup)
        .iter()
        .map(|m| match (m.reading.is_empty(), m.is_new) {
            (true, _) => furigana(words, &m.text),
            (false, true) => format!("{{{}|{}|new}}", m.text, m.reading),
            (false, false) => format!("{{{}|{}}}", m.text, m.reading),
        })
        .collect()
}

/// [`furigana`] with UniDic's readings: a number or counter next to another one, or right
/// after digits, has none (三本 さんぼん, 三千 さんぜん change together).
fn with_unidic(text: &str, tokens: &[unidic::Token], safe: &impl Fn(&str) -> String) -> String {
    let digits = |s: &str| {
        !s.is_empty()
            && s.chars().all(|c| {
                c.is_ascii_digit() || ('０'..='９').contains(&c) || matches!(c, ',' | '，')
            })
            && s.chars()
                .any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c))
    };
    let joined = |i: usize, j: usize| tokens[i].end == tokens[j].start;
    let mut out = String::new();
    let mut pos = 0;
    for (i, t) in tokens.iter().enumerate() {
        if t.start < pos {
            continue;
        }
        out.push_str(&safe(&text[pos..t.start]));
        let next_to = |j: usize| {
            (tokens[j].counted || digits(&tokens[j].surface))
                && if j < i { joined(j, i) } else { joined(i, j) }
        };
        // A number before a counter UniDic files as a suffix (人, 冊: 四人 よにん, 一冊
        // いっさつ) changes its reading with it, too.
        let suffix_after = i + 1 < tokens.len() && tokens[i + 1].suffix && joined(i, i + 1);
        let group = t.counted
            && ((i > 0 && next_to(i - 1))
                || (i + 1 < tokens.len() && next_to(i + 1))
                || suffix_after);
        let after_digits = i > 0 && digits(&tokens[i - 1].surface) && joined(i - 1, i);
        // A suffix is read with the word before, which UniDic does not always see
        // (中国人, 日本人: にん for じん).
        let orphan = t.suffix;
        let reading = match &t.reading {
            _ if t.surface == "何" => nani(&text[t.end..]),
            Some(r) if !group && !after_digits && !orphan => r.as_str(),
            _ => "",
        };
        out.push_str(&ruby(&safe(&t.surface), reading));
        pos = t.end;
    }
    out.push_str(&safe(&text[pos..]));
    out
}

/// 何 on its own by what follows, as UniDic reads it なん everywhere: なん before です, だ, の
/// and という (何ですか, 何の, 何という), なに before a particle (何を, 何が, 何か, 何も, 何と何);
/// otherwise none (何で is either; 何か月 is a counter).
fn nani(after: &str) -> &'static str {
    // か before a kanji is a counter (何か月 なんかげつ, 何か国 なんかこく), not 何か.
    let counter = after
        .strip_prefix('か')
        .and_then(|r| r.chars().next())
        .is_some_and(is_kanji);
    // と is なん only in 何という/何と言う; 何と何 is なに.
    let to = after
        .strip_prefix('と')
        .is_some_and(|r| r.starts_with('い') || r.starts_with('言'));
    if counter {
        ""
    } else if to || ["です", "だ", "の"].iter().any(|p| after.starts_with(p)) {
        "なん"
    } else if ["を", "が", "か", "も", "に", "は", "へ", "や", "と"]
        .iter()
        .any(|p| after.starts_with(p))
    {
        "なに"
    } else {
        ""
    }
}

/// Single kanji with several common readings on their own, as a noun (角 かど/かく, the
/// weekdays 月火水木金土日, 上 うえ/じょう, 方 ほう/かた …). Until UniDic is there, they
/// stay without a reading rather than with a guessed one.
pub(crate) const AMBIGUOUS: &str = "角月火水木金土日上下方間今後中前先生人分外内家年時物気";

/// A word whose reading depends on the sentence more than IPADIC sees: 何 (なに or なん),
/// names and places (固有名詞), and an [`AMBIGUOUS`] kanji standing alone as a noun.
fn unsure(w: &words::Word) -> bool {
    let mut chars = w.surface.chars();
    let single = matches!((chars.next(), chars.next()), (Some(c), None) if AMBIGUOUS.contains(c));
    w.surface.contains('何') || w.proper || single && w.pos == "名詞"
}

pub(crate) fn is_kanji(c: char) -> bool {
    kanji(c)
}

fn kanji(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{9fff}' | '々' | '〆')
}

/// One word with its reading as markup.
fn ruby(surface: &str, reading: &str) -> String {
    if reading.is_empty() || !surface.chars().any(kanji) {
        return surface.to_string();
    }
    // Runs of kanji and of everything else.
    let mut runs: Vec<(String, bool)> = Vec::new();
    for c in surface.chars() {
        match runs.last_mut() {
            Some((r, k)) if *k == kanji(c) => r.push(c),
            _ => runs.push((c.to_string(), kanji(c))),
        }
    }
    let reading: Vec<char> = words::hiragana(reading).chars().collect();
    match align(&runs, &reading) {
        Some(parts) => runs
            .iter()
            .zip(parts)
            .map(|((r, k), p)| {
                if *k {
                    format!("{{{r}|{p}}}")
                } else {
                    r.clone()
                }
            })
            .collect(),
        None => format!("{{{surface}|{}}}", reading.iter().collect::<String>()),
    }
}

/// The reading of each run: kana runs must match literally, a kanji run takes at least one
/// kana. `None` when the reading does not fit.
pub(crate) fn align(runs: &[(String, bool)], reading: &[char]) -> Option<Vec<String>> {
    let Some(((run, is_kanji), rest)) = runs.split_first() else {
        return reading.is_empty().then(Vec::new);
    };
    if !is_kanji {
        let kana: Vec<char> = words::hiragana(run).chars().collect();
        if !reading.starts_with(&kana) {
            return None;
        }
        let mut tail = align(rest, &reading[kana.len()..])?;
        tail.insert(0, run.clone());
        return Some(tail);
    }
    // The shortest reading first that lets the rest fit; the last run takes all that is left.
    let max = if rest.is_empty() {
        reading.len()
    } else {
        reading.len().saturating_sub(1)
    };
    let min = if rest.is_empty() {
        reading.len().max(1)
    } else {
        1
    };
    for n in min..=max {
        if let Some(mut tail) = align(rest, &reading[n..]) {
            tail.insert(0, reading[..n].iter().collect());
            return Some(tail);
        }
    }
    None
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

    /// The IPADIC path, whether UniDic is installed on this machine or not.
    fn ipadic(words: &Words, text: &str) -> String {
        with_ipadic(words, text, &|t: &str| {
            t.replace('{', "｛").replace('}', "｝").replace('|', "｜")
        })
    }

    #[test]
    fn plain_text_gets_furigana_from_ipadic() {
        let words = Words::new().unwrap();
        assert_eq!(
            ipadic(&words, "駅まで歩いて五分です。"),
            "{駅|えき}まで{歩|ある}いて五分です。"
        );
        assert_eq!(
            ipadic(&words, "コーヒーが好きです。"),
            "コーヒーが{好|す}きです。"
        );
        // A number with its counter has no reliable reading from IPADIC (ににん, いちほん):
        // no furigana rather than wrong ones.
        for (text, group) in [
            ("二人です。", "二人"),
            ("一本ください。", "一本"),
            ("三日に来ました。", "三日"),
            ("三千円です。", "三千円"),
            ("二十歳です。", "二十歳"),
            ("一人で来ました。", "一人"),
        ] {
            let m = ipadic(&words, text);
            assert!(m.starts_with(group), "{m}");
        }
        // The same with the number in digits, as speech recognition often writes it.
        for (text, head) in [
            ("2人です。", "2人です"),
            ("３本ください。", "３本ください"),
            ("3日に来ました。", "3日に"),
            ("5分です。", "5分です"),
            ("1,000円です。", "1,000円です"),
        ] {
            let m = ipadic(&words, text);
            assert!(m.starts_with(head), "{m}");
        }
        // A noun not right after the digits keeps its reading.
        assert_eq!(ipadic(&words, "2時に駅です。"), "2時に{駅|えき}です。");
        // Kana only, latin letters and marks stay as they are; braces stay text.
        assert_eq!(ipadic(&words, "はい、OK {x}"), "はい、OK ｛x｝");
        // The markup reads back as the same text.
        let (segs, _) = annotate(&words, &ipadic(&words, "取り扱いに注意してください。"));
        let text: String = segs.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "取り扱いに注意してください。");
    }

    #[test]
    fn a_reply_without_markup_gets_furigana_and_marked_ones_stay() {
        let words = Words::new().unwrap();
        assert_eq!(
            complete(&words, "何が好きですか。"),
            furigana(&words, "何が好きですか。")
        );
        assert!(complete(&words, "何が好きですか。").contains("{好|す}"));
        // The scenario's (or tutor's) reading wins, `new` stays.
        assert_eq!(
            complete(&words, "{田中|たなか}です。{駅|えき|new}まで歩いて"),
            "{田中|たなか}です。{駅|えき|new}まで{歩|ある}いて"
        );
        // Kana only: unchanged.
        assert_eq!(complete(&words, "はい、そうです。"), "はい、そうです。");
    }

    /// Tokens as UniDic gives them, for the rules of `with_unidic` without the dictionary.
    fn unidic_tokens(parts: &[(&str, Option<&str>, bool, bool)]) -> (String, Vec<unidic::Token>) {
        let mut text = String::new();
        let tokens = parts
            .iter()
            .map(|&(surface, reading, counted, suffix)| {
                let start = text.len();
                text.push_str(surface);
                unidic::Token {
                    surface: surface.into(),
                    start,
                    end: text.len(),
                    reading: reading.map(Into::into),
                    counted,
                    suffix,
                }
            })
            .collect();
        (text, tokens)
    }

    #[test]
    fn nani_reads_by_what_follows() {
        let read = |parts: &[(&str, Option<&str>, bool, bool)]| {
            let (text, tokens) = unidic_tokens(parts);
            with_unidic(&text, &tokens, &|s: &str| s.to_string())
        };
        let nan = Some("なん");
        assert_eq!(
            read(&[("何", nan, false, false), ("を", None, false, false)]),
            "{何|なに}を"
        );
        assert_eq!(
            read(&[("何", nan, false, false), ("か", None, false, false)]),
            "{何|なに}か"
        );
        assert_eq!(
            read(&[("何", nan, false, false), ("です", None, false, false)]),
            "{何|なん}です"
        );
        assert_eq!(
            read(&[("何", nan, false, false), ("の", None, false, false)]),
            "{何|なん}の"
        );
        assert_eq!(
            read(&[("何", nan, false, false), ("？", None, false, false)]),
            "何？"
        );
        assert_eq!(
            read(&[("何", nan, false, false), ("か月", None, false, false)]),
            "何か月"
        );
        assert_eq!(
            read(&[("何", nan, false, false), ("という", None, false, false)]),
            "{何|なん}という"
        );
        assert_eq!(
            read(&[
                ("何", nan, false, false),
                ("と", None, false, false),
                ("何", nan, false, false),
                ("を", None, false, false)
            ]),
            "{何|なに}と{何|なに}を"
        );
    }

    #[test]
    fn a_number_before_a_suffix_counter_goes_without() {
        let read = |parts: &[(&str, Option<&str>, bool, bool)]| {
            let (text, tokens) = unidic_tokens(parts);
            with_unidic(&text, &tokens, &|s: &str| s.to_string())
        };
        // 人 and 冊 come as 接尾辞, not 助数詞 (probe, 9.11.2026).
        assert_eq!(
            read(&[
                ("四", Some("よん"), true, false),
                ("人", Some("にん"), false, true)
            ]),
            "四人"
        );
        assert_eq!(
            read(&[
                ("一", Some("いち"), true, false),
                ("冊", Some("さつ"), false, true)
            ]),
            "一冊"
        );
        // A number alone keeps its reading.
        assert_eq!(
            read(&[
                ("三", Some("さん"), true, false),
                ("、", None, false, false)
            ]),
            "{三|さん}、"
        );
    }

    #[test]
    fn a_comma_is_no_number_and_1tsu_is_a_counter() {
        let words = Words::new().unwrap();
        assert_eq!(ipadic(&words, "はい,駅です。"), "はい,{駅|えき}です。");
        let w = words.words("1つください");
        assert!(w.iter().any(|x| x.key == "くださる"), "{w:?}");
        assert!(!w.iter().any(|x| x.key == "つる"), "{w:?}");
        // The cut keeps the byte offsets of the whole text.
        let text = "りんごを2つください。";
        for x in words.words(text) {
            assert_eq!(&text[x.start..x.end], x.surface);
        }
    }

    #[test]
    fn family_words_read_as_people_say_them() {
        let words = Words::new().unwrap();
        assert_eq!(
            furigana(&words, "お父さんとお兄さんは？"),
            "お{父|とう}さんとお{兄|にい}さんは？"
        );
        assert!(furigana(&words, "富士山に登りました。").starts_with("{富士山|ふじさん}に"));
    }

    #[test]
    fn readings_that_depend_on_the_sentence_are_left_out() {
        let words = Words::new().unwrap();
        for (text, head) in [
            ("お仕事は何ですか。", "お{仕事|しごと}は何ですか"),
            (
                "二つ目の角を右です。",
                "{二|ふた}つ{目|め}の角を{右|みぎ}です",
            ),
            ("月、水、金はどうですか。", "月、水、金はどうですか"),
            ("何曜日ですか。", "何"),
        ] {
            let m = ipadic(&words, text);
            assert!(m.starts_with(head), "{m}");
        }
        // A word of two kanji or a verb keeps its reading.
        assert!(ipadic(&words, "駅で食べました。").contains("{食|た}べ"));
        assert!(ipadic(&words, "仕事です。").contains("{仕事|しごと}"));
    }

    #[test]
    fn a_reading_splits_around_the_kana() {
        assert_eq!(ruby("取り扱い", "とりあつかい"), "{取|と}り{扱|あつか}い");
        assert_eq!(ruby("お弁当", "おべんとう"), "お{弁当|べんとう}");
        assert_eq!(ruby("食べました", "タベマシタ"), "{食|た}べました");
        // A reading that does not fit the kana goes over the whole word.
        assert_eq!(ruby("行く", "おこなう"), "{行く|おこなう}");
        assert_eq!(ruby("はい", "はい"), "はい");
        assert_eq!(ruby("駅", ""), "駅");
    }

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
