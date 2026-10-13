//! The lookup against the excerpt of the data package (data/dict, the same rows as kaiwa-data).

use std::path::Path;

use kaiwa_lang::{Dictionary, Words, annotate};

fn dict() -> Dictionary {
    Dictionary::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/dict/kaiwa-dict-excerpt.sqlite"),
    )
    .unwrap()
}

#[test]
fn a_verb_shows_reading_german_meaning_part_of_speech_and_accent() {
    let e = dict().lookup("食べる", None, "de").unwrap().unwrap();
    assert_eq!(e.reading, "たべる");
    assert_eq!(e.lang, "de");
    assert!(!e.fallback);
    assert!(e.senses[0].iter().any(|g| g == "essen"), "{:?}", e.senses);
    assert_eq!(e.pos, ["Verb (ichidan)", "transitiv"]);
    let a = e.accent.unwrap();
    assert_eq!(a.morae, ["た", "べ", "る", "(が)"]);
    assert_eq!(a.target, [0, 1, 0, 0]);
    assert!(e.common);
}

#[test]
fn english_ui_gets_english_meanings() {
    let e = dict().lookup("食べる", None, "en").unwrap().unwrap();
    assert_eq!(e.lang, "en");
    assert_eq!(e.senses[0], ["to eat"]);
    assert_eq!(e.pos[0], "verb (ichidan)");
}

#[test]
fn the_word_of_mara_s_journey() {
    // mara_word_lookup: プロデューサー shows reading, meaning and part of speech.
    let e = dict()
        .lookup("プロデューサー", None, "de")
        .unwrap()
        .unwrap();
    assert_eq!(e.reading, "プロデューサー");
    assert!(!e.senses.is_empty());
    assert!(e.pos.iter().any(|p| p == "Nomen"), "{:?}", e.pos);
}

#[test]
fn without_german_meanings_english_is_marked_as_fallback() {
    let d = dict();
    let words = Words::new().unwrap();
    let mut seen = 0;
    for key in ["プロデューサー", "コンビニ", "自己紹介", "駅", "温める"] {
        let (_, ws) = annotate(&words, key);
        if let Some(e) = d.lookup(&ws[0].key, None, "de").unwrap() {
            seen += 1;
            assert_eq!(e.fallback, e.lang == "en", "{key}");
        }
    }
    assert!(seen >= 4);
}

#[test]
fn the_reading_chooses_between_spellings_alike() {
    let d = dict();
    // 日本 is にほん or にっぽん in one entry; a given reading wins.
    if let Some(e) = d.lookup("日本", Some("にっぽん"), "de").unwrap() {
        assert_eq!(e.reading, "にっぽん");
    }
    assert!(d.lookup("ほげほげ", None, "de").unwrap().is_none());
}

#[test]
fn sources_carry_licence_and_attribution() {
    let s = dict().sources().unwrap();
    assert_eq!(s.len(), 3);
    assert!(
        s.iter()
            .all(|(_, l, a)| l == "CC BY-SA 4.0" && !a.is_empty())
    );
    assert_eq!(dict().version().unwrap(), "0.1.0");
}

fn first(text: &str) -> kaiwa_lang::Entry {
    let (_, ws) = annotate(&Words::new().unwrap(), text);
    dict().lookup_word(&ws[0], "de").unwrap().unwrap()
}

#[test]
fn a_particle_is_the_particle_without_accent() {
    let (_, ws) = annotate(&Words::new().unwrap(), "{駅|えき}は");
    let e = dict().lookup_word(&ws[1], "de").unwrap().unwrap();
    assert_eq!(e.pos, ["Partikel"]);
    assert_eq!(e.accent, None);
}

#[test]
fn an_expression_is_looked_up_as_written() {
    let e = first("いらっしゃいませ。");
    assert_eq!(e.word, "いらっしゃいませ");
    assert_eq!(e.pos, ["Ausdruck"]);
}

#[test]
fn an_inflected_verb_is_looked_up_in_its_dictionary_form() {
    let e = first("{温|あたた}めますか。");
    assert_eq!(e.word, "温める");
    let g = &e.senses[0];
    let mut uniq = g.clone();
    uniq.dedup();
    assert_eq!(&uniq, g, "glosses appear once");
}
