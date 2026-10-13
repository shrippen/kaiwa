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
fn levels_come_from_the_tanos_lists() {
    assert_eq!(dict().jlpt("駅", "えき").unwrap(), Some(5));
    assert_eq!(dict().jlpt("駅", "").unwrap(), Some(5));
    assert_eq!(dict().jlpt("ほげほげ", "").unwrap(), None);
}

#[test]
fn sources_carry_licence_and_attribution() {
    let s = dict().sources().unwrap();
    assert_eq!(s.len(), 5);
    // The tanos lists are CC BY (L-09), everything else CC BY-SA.
    assert!(s.iter().all(|(n, l, a)| {
        !a.is_empty() && (l == "CC BY-SA 4.0" || (n.contains("tanos") && l == "CC BY"))
    }));
    assert!(dict().version().unwrap().starts_with("0.2."));
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

/// /LF221/ (automatic part): every word of the scenarios and the recorded answers is in the
/// dictionary, except the known gaps in data/dict/known-gaps.txt (names …).
#[test]
fn content_words_are_in_the_dictionary() {
    fn strings(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
            serde_json::Value::Object(o) => o.values().for_each(|x| strings(x, out)),
            _ => {}
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let gaps = std::fs::read_to_string(root.join("data/dict/known-gaps.txt")).unwrap();
    let gaps: Vec<&str> = gaps
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .collect();
    let (d, words) = (dict(), Words::new().unwrap());
    let mut unknown = std::collections::BTreeSet::new();
    for dir in ["data/scenarios", "qa/fixtures"] {
        for e in std::fs::read_dir(root.join(dir)).unwrap() {
            let p = e.unwrap().path();
            let mut texts = Vec::new();
            strings(
                &serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap(),
                &mut texts,
            );
            for t in texts {
                for w in annotate(&words, &t).1 {
                    if !gaps.contains(&w.text.as_str())
                        && d.lookup_word(&w, "de").unwrap().is_none()
                        && d.lookup_name(&w).unwrap().is_none()
                    {
                        unknown.insert(format!(
                            "{} ({})",
                            w.text,
                            p.file_name().unwrap().to_string_lossy()
                        ));
                    }
                }
            }
        }
    }
    assert!(unknown.is_empty(), "not in the dictionary: {unknown:?}");
}
