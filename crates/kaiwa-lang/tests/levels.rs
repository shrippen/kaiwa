//! Level check (INHALT-ENTWURF 3.1, /LF221/): every word the learner is helped to say (the
//! start and example of each goal) is at the level of its unit or below, or a core word of the
//! unit or of a unit it requires, or a reviewed exception in `data/dict/level-exceptions.txt`.
//! A word's level is that of the first unit listing it as a core word, else its tanos level
//! (kaiwa-data `jlpt`); names, particles and words without a level are not judged.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kaiwa_lang::{Dictionary, Words, annotate};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(path)).unwrap()).unwrap()
}

fn list(v: &Value, key: &str) -> Vec<String> {
    v[key]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// n5 → 5 … n1 → 1.
fn level_of(unit: &str) -> u8 {
    unit.as_bytes().get(1).map_or(5, |b| b - b'0')
}

#[test]
fn help_sentences_stay_at_their_units_level() {
    let dict = Dictionary::open(&root().join("data/dict/kaiwa-dict-excerpt.sqlite")).unwrap();
    let words = Words::new().unwrap();
    let exceptions: BTreeSet<String> =
        std::fs::read_to_string(root().join("data/dict/level-exceptions.txt"))
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .collect();
    let mut units = Vec::new();
    for level in ["n5", "n4", "n3", "n2", "n1"] {
        let p = root().join(format!("data/curriculum/{level}.json"));
        if p.exists() {
            units.extend(
                read(&format!("data/curriculum/{level}.json"))["units"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
            );
        }
    }
    let by_id: BTreeMap<String, &Value> = units
        .iter()
        .map(|u| (u["id"].as_str().unwrap().to_string(), u))
        .collect();
    let mut core_level: BTreeMap<String, u8> = BTreeMap::new();
    for u in &units {
        for w in list(u, "words") {
            core_level
                .entry(w)
                .or_insert(level_of(u["id"].as_str().unwrap()));
        }
    }
    let known = |start: &str| {
        let (mut seen, mut todo, mut out) =
            (BTreeSet::new(), vec![start.to_string()], BTreeSet::new());
        while let Some(u) = todo.pop() {
            if seen.insert(u.clone())
                && let Some(v) = by_id.get(&u)
            {
                out.extend(list(v, "words"));
                todo.extend(list(v, "requires"));
            }
        }
        out
    };
    let mut above = Vec::new();
    let mut used_exceptions = BTreeSet::new();
    for u in &units {
        let id = u["id"].as_str().unwrap();
        let level = level_of(id);
        let allowed = known(id);
        for sc in list(u, "scenarios") {
            let s = read(&format!("data/scenarios/{sc}.json"));
            for g in s["goals"].as_array().into_iter().flatten() {
                for field in ["start", "example"] {
                    let text = g["help"][field].as_str().unwrap_or_default();
                    for w in annotate(&words, text).1 {
                        if matches!(w.pos.as_str(), "助詞" | "助動詞" | "記号")
                            || allowed.contains(&w.key)
                            || allowed.contains(&w.text)
                        {
                            continue;
                        }
                        let lv = core_level.get(&w.key).copied().or_else(|| {
                            let r = w.reading.as_deref().unwrap_or_default();
                            dict.jlpt(&w.key, r)
                                .ok()
                                .flatten()
                                .or_else(|| dict.jlpt(&w.key, "").ok().flatten())
                        });
                        if lv.is_some_and(|l| l < level) {
                            if exceptions.contains(&w.key) {
                                used_exceptions.insert(w.key.clone());
                            } else {
                                above.push(format!("{sc}: {} (N{})", w.key, lv.unwrap()));
                            }
                        }
                    }
                }
            }
        }
    }
    above.sort();
    above.dedup();
    assert!(
        above.is_empty(),
        "words above their unit's level: {above:#?}"
    );
    let stale: Vec<&String> = exceptions.difference(&used_exceptions).collect();
    assert!(
        stale.is_empty(),
        "exceptions no help sentence needs any more: {stale:?}"
    );
}
