//! The word lookup (/LF112/): the words of the character's lines and the learner's own, what a click shows, and
//! taking a word over into the review (/LF230/). The dictionary is the data package kaiwa-data
//! (L-06), downloaded only after the learner asks for it (M-03); without it the view says so
//! and offers the download. A word JMdict does not have may be a name (JMnedict, Q-13): then
//! the view shows its kind and reading, and it goes into no review.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use kaiwa_core::srs::{Item, ItemKind, Source};
use kaiwa_core::{Event, Lang, Moment};
use kaiwa_lang::{Dictionary, LineWord, Words, annotate, dictionary_path};
use serde_json::{Value, json};

use crate::Backend;

/// The segmenter, loaded once (IPADIC is in the binary).
pub fn words() -> &'static Words {
    static WORDS: OnceLock<Words> = OnceLock::new();
    WORDS.get_or_init(|| Words::new().expect("IPADIC is embedded"))
}

/// A character's line for KanteRubyText (Kante 1.33): `segments` with word numbers and the
/// `words` behind them.
///
/// Kanji without furigana in the markup get IPADIC's (a tutor reply comes without markup;
/// the reading aid over every kanji is the default, /LF110/).
pub fn line(markup: &str) -> Value {
    let markup = kaiwa_lang::complete(words(), markup);
    let (segments, words) = annotate(words(), &markup);
    json!({ "segments": segments, "words": words })
}

/// The learner's own line (plain text, recognised or typed) with furigana from IPADIC, as
/// [`line`] gives it, and the `markup` behind it.
pub fn own_line(text: &str) -> Value {
    let markup = kaiwa_lang::furigana(words(), text);
    let mut v = line(&markup);
    v["markup"] = markup.into();
    v
}

/// The dictionary at `path`, opened once and again after a download.
fn dictionary(
    path: &Path,
) -> Option<std::sync::MutexGuard<'static, Option<(std::path::PathBuf, Dictionary)>>> {
    static DICT: Mutex<Option<(std::path::PathBuf, Dictionary)>> = Mutex::new(None);
    let mut guard = DICT.lock().ok()?;
    if guard.as_ref().is_none_or(|(p, _)| p != path) {
        *guard = Dictionary::open(path).ok().map(|d| (path.to_path_buf(), d));
    }
    Some(guard)
}

/// Whether the dictionary is there (the package or `KAIWA_DICT`).
pub fn available() -> bool {
    let path = dictionary_path();
    path.exists() && dictionary(&path).is_some_and(|g| g.is_some())
}

/// The sources of the dictionary with licence and attribution, for "About" (/LD050/, Q-01);
/// empty without the dictionary.
pub fn sources() -> Vec<(String, String, String)> {
    let path = dictionary_path();
    if !path.exists() {
        return Vec::new();
    }
    dictionary(&path)
        .and_then(|g| g.as_ref().and_then(|(_, d)| d.sources().ok()))
        .unwrap_or_default()
}

/// What a click on `word` shows, in the UI language `lang`: reading, meanings, part of speech,
/// the target accent (M-04, without a judgement) and whether the word is in the review
/// already. `{"missing": true}` without the dictionary, `{"found": false}` for a word it
/// does not know.
pub fn lookup(backend: &Backend, word: &LineWord, lang: Lang) -> Value {
    #[cfg_attr(not(feature = "speech"), allow(unused_mut))]
    let mut v = lookup_in(&dictionary_path(), backend, word, lang);
    if v["missing"] != true {
        return v;
    }

    // What the download would bring (M-03): offered in the popup, only after a click.
    #[cfg(feature = "speech")]
    if let Some(offer) = crate::models::offer(kaiwa_lang::PACKAGE) {
        v["download"] = offer;
    }
    v
}

/// [`lookup`] in the dictionary at `path`.
pub fn lookup_in(path: &Path, backend: &Backend, word: &LineWord, lang: Lang) -> Value {
    enum Found {
        Word(kaiwa_lang::dict::Entry),
        Name(kaiwa_lang::dict::Name),
        Nothing,
    }
    let entry = if path.exists() {
        dictionary(path).and_then(|g| {
            g.as_ref().map(|(_, d)| {
                Ok(match d.lookup_word(word, lang.code())? {
                    Some(e) => Found::Word(e),
                    None => d.lookup_name(word)?.map_or(Found::Nothing, Found::Name),
                })
            })
        })
    } else {
        None
    };
    let Some(entry) = entry else {
        return json!({ "text": word.text, "missing": true });
    };
    match entry {
        Ok(Found::Word(e)) => {
            let known = backend
                .store
                .items()
                .map(|items| {
                    items
                        .iter()
                        .any(|i| kaiwa_core::norm(&i.text) == kaiwa_core::norm(&e.word))
                })
                .unwrap_or(false);
            let mut v = json!(e);
            v["text"] = json!(word.text);
            v["found"] = json!(true);
            v["inReview"] = json!(known);
            v
        }
        Ok(Found::Name(n)) => json!({
            "text": word.text,
            "found": true,
            "name": true,
            "word": n.word,
            "reading": n.reading,
            "pos": [kaiwa_lang::pos::name_label(&n.kinds, lang.code())],
            "senses": n.trans.map(|t| vec![vec![t]]).unwrap_or_default(),
            "lang": lang,
            "fallback": false,
            "accent": null,
            "inReview": false,
        }),
        Ok(Found::Nothing) => json!({ "text": word.text, "found": false }),
        Err::<_, kaiwa_lang::LangError>(e) => {
            json!({ "text": word.text, "found": false, "error": e.to_string() })
        }
    }
}

/// Takes the looked-up word into the review (/LF112/, /LF230/); `false` when it is there
/// already.
pub fn remember(
    backend: &Backend,
    text: &str,
    unit: Option<&str>,
    now: Moment,
) -> anyhow::Result<bool> {
    if backend.store.item(text)?.is_some() {
        return Ok(false);
    }
    let event = Event::WordAdded {
        text: text.into(),
        unit: unit.map(str::to_string),
    };
    crate::journal::record(backend, &event, now)?;
    Ok(true)
}

/// What a journaled [`remember`] does.
pub(crate) fn add_word(
    backend: &Backend,
    text: &str,
    unit: Option<&str>,
    now: Moment,
) -> anyhow::Result<()> {
    let mut item = Item::new(text, ItemKind::Word, Source::Lookup, now);
    item.unit = unit.map(str::to_string);
    backend.store.add_item(&item)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn excerpt() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/dict/kaiwa-dict-excerpt.sqlite")
    }

    #[test]
    fn a_line_gets_word_numbers() {
        let v = line("{駅|えき}までは{歩|ある}いて{五分|ごふん}です。");
        let words = v["words"].as_array().unwrap();
        assert!(words.iter().any(|w| w["key"] == "歩く"));
        assert_eq!(v["segments"][0]["word"], 0);
        assert!(
            v["segments"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()
                .get("word")
                .is_none()
        );
    }

    #[test]
    fn the_dictionary_is_opened_once_per_path() {
        let path = excerpt();
        assert!(dictionary(&path).is_some_and(|g| g.is_some()));
        let missing = Path::new("/nonexistent/kaiwa-dict.sqlite");
        assert!(dictionary(missing).is_some_and(|g| g.is_none()));
    }

    fn backend(name: &str) -> crate::TempBackend {
        crate::TempBackend::new(&format!("lookup-{name}"), false).unwrap()
    }

    fn word(markup: &str, i: usize) -> LineWord {
        annotate(words(), markup).1.remove(i)
    }

    #[test]
    fn a_click_shows_the_entry_and_the_word_goes_into_the_review_once() {
        let b = backend("click");
        let w = word("{駅|えき}までは", 0);
        let v = lookup_in(&excerpt(), &b, &w, Lang::De);
        assert_eq!(v["found"], true);
        assert_eq!(v["reading"], "えき");
        assert_eq!(v["senses"][0][0], "Bahnhof");
        assert_eq!(v["pos"][0], "Nomen");
        assert_eq!(v["inReview"], false);
        assert!(v["accent"]["target"].is_array());
        let now = crate::now();
        assert!(remember(&b, "駅", Some("n5-06"), now).unwrap());
        assert!(!remember(&b, "駅", None, now).unwrap());
        let item = b
            .store
            .items()
            .unwrap()
            .into_iter()
            .find(|i| i.text == "駅")
            .unwrap();
        assert_eq!(item.source, Source::Lookup);
        assert_eq!(item.unit.as_deref(), Some("n5-06"));
        assert_eq!(lookup_in(&excerpt(), &b, &w, Lang::De)["inReview"], true);
        // The next day it is due, marked as looked up.
        let due = crate::views::review(&b, now.plus_secs(86_400), 10).unwrap();
        let card = due
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["text"] == "駅")
            .unwrap();
        assert_eq!(card["fromLookup"], true);
        assert_eq!(card["fromMistake"], false);
    }

    #[test]
    fn a_name_shows_its_kind_and_reading() {
        let b = backend("name");
        let v = lookup_in(&excerpt(), &b, &word("{田中|たなか}です。", 0), Lang::De);
        assert_eq!(v["name"], true);
        assert_eq!(v["reading"], "たなか");
        assert_eq!(v["pos"][0], "Name · Familienname");
        assert_eq!(v["senses"][0][0], "Tanaka");
        let v = lookup_in(&excerpt(), &b, &word("山田さんは", 0), Lang::En);
        assert_eq!(v["word"], "山田");
        assert_eq!(v["reading"], "やまだ");
        // A name in katakana only.
        let v = lookup_in(&excerpt(), &b, &word("ミュラーと申します。", 0), Lang::De);
        assert_eq!(v["name"], true);
        assert_eq!(v["reading"], "ミュラー");
    }

    #[test]
    fn without_the_dictionary_the_view_says_so() {
        let b = backend("missing");
        let v = lookup_in(
            Path::new("/nonexistent/kaiwa-dict.sqlite"),
            &b,
            &word("駅", 0),
            Lang::De,
        );
        assert_eq!(v["missing"], true);
        assert_eq!(v["text"], "駅");
    }

    #[test]
    fn a_broken_dictionary_says_why_instead_of_not_found() {
        let b = backend("broken-dict");
        let file = std::env::temp_dir().join(format!("kaiwa-dict-{}.sqlite", std::process::id()));
        std::fs::copy(excerpt(), &file).unwrap();
        let db = rusqlite::Connection::open(&file).unwrap();
        db.execute_batch("DROP TABLE form").unwrap();
        drop(db);

        let v = lookup_in(&file, &b, &word("駅", 0), Lang::De);

        std::fs::remove_file(&file).unwrap();
        assert_eq!(v["found"], false);
        assert!(v["error"].as_str().is_some_and(|e| !e.is_empty()), "{v}");
    }
}
