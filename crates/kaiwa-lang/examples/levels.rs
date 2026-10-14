//! Level check (INHALT-ENTWURF 3.1, report only): the words of each unit's scenarios (opening,
//! help sentences, recorded replies) whose Kaiwa level lies above the unit's level and that the unit or
//! the units it requires (transitively) do not list as core words. The level of a word is the
//! level of the first unit that lists it, else its tanos level (kaiwa-data `jlpt`, Q-01); a word
//! with neither is counted apart. Names and particles are left out.
//!
//!   cargo run -p kaiwa-lang --example levels -- <kaiwa-dict.sqlite> [unit-prefix] [--learner]
//!
//! `--learner`: only the help sentences, what the learner is asked to say.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use kaiwa_lang::{Dictionary, Words, annotate};
use serde_json::Value;

fn level_of(id: &str) -> u8 {
    // n5 → 5 … n1 → 1
    id.as_bytes().get(1).map(|b| b - b'0').unwrap_or(5)
}

fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = PathBuf::from(
        args.first()
            .ok_or("usage: levels <kaiwa-dict.sqlite> [prefix]")?,
    );
    let learner_only = args.iter().any(|a| a == "--learner");
    let only = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_default();
    let dict = Dictionary::open(&db)?;
    let words = Words::new()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let mut units: Vec<Value> = Vec::new();
    for level in ["n5", "n4", "n3", "n2", "n1"] {
        let p = root.join(format!("data/curriculum/{level}.json"));
        if p.exists() {
            let c: Value = serde_json::from_str(&fs::read_to_string(p)?)?;
            units.extend(c["units"].as_array().cloned().unwrap_or_default());
        }
    }
    let id = |u: &Value| u["id"].as_str().unwrap_or_default().to_string();
    let list = |u: &Value, k: &str| -> Vec<String> {
        u[k].as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    // The level a core word gets from the first unit that lists it.
    let mut core_level: BTreeMap<String, u8> = BTreeMap::new();
    for u in &units {
        for w in list(u, "words") {
            core_level.entry(w).or_insert(level_of(&id(u)));
        }
    }
    let by_id: BTreeMap<String, &Value> = units.iter().map(|u| (id(u), u)).collect();
    let known = |start: &str| -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut todo = vec![start.to_string()];
        let mut words = BTreeSet::new();
        while let Some(u) = todo.pop() {
            if !seen.insert(u.clone()) {
                continue;
            }
            if let Some(v) = by_id.get(&u) {
                words.extend(list(v, "words"));
                todo.extend(list(v, "requires"));
            }
        }
        words
    };

    let (mut total, mut above_all, mut unknown_all) = (0usize, 0usize, 0usize);
    for u in &units {
        let uid = id(u);
        if !uid.starts_with(&only) {
            continue;
        }
        let level = level_of(&uid);
        let allowed = known(&uid);
        let mut texts = Vec::new();
        for sc in list(u, "scenarios") {
            let sp = root.join(format!("data/scenarios/{sc}.json"));
            let s: Value = serde_json::from_str(&fs::read_to_string(sp)?)?;
            if !learner_only {
                strings(&s["opening"], &mut texts);
            }
            // The keyword carries its reading in brackets, which falls apart into kana pieces.
            for g in s["goals"].as_array().into_iter().flatten() {
                strings(&g["help"]["start"], &mut texts);
                strings(&g["help"]["example"], &mut texts);
            }
            let fp = root.join(format!("qa/fixtures/{sc}.json"));
            if fp.exists() && !learner_only {
                let f: Value = serde_json::from_str(&fs::read_to_string(fp)?)?;
                for t in f.as_array().into_iter().flatten() {
                    strings(&t["reply"], &mut texts);
                }
            }
        }
        let (mut n, mut above, mut unknown) = (0usize, BTreeSet::new(), BTreeSet::new());
        for t in &texts {
            for w in annotate(&words, t).1 {
                if w.pos == "助詞" || w.pos == "助動詞" || w.pos == "記号" {
                    continue;
                }
                n += 1;
                let key = w.key.clone();
                if allowed.contains(&key) || allowed.contains(&w.text) {
                    continue;
                }
                let lv = core_level.get(&key).copied().or_else(|| {
                    dict.jlpt(&key, w.reading.as_deref().unwrap_or_default())
                        .ok()
                        .flatten()
                        .or_else(|| dict.jlpt(&key, "").ok().flatten())
                });
                match lv {
                    Some(l) if l < level => {
                        above.insert(format!("{key} (N{l})"));
                    }
                    Some(_) => {}
                    None if dict.lookup_name(&w).ok().flatten().is_some() => {}
                    None => {
                        unknown.insert(key);
                    }
                }
            }
        }
        total += n;
        above_all += above.len();
        unknown_all += unknown.len();
        println!(
            "{uid} (N{level}): {n} words, {} above: {}",
            above.len(),
            above.iter().cloned().collect::<Vec<_>>().join(", ")
        );
        if !unknown.is_empty() {
            println!(
                "    no level: {}",
                unknown.iter().cloned().collect::<Vec<_>>().join(", ")
            );
        }
    }
    println!(
        "{total} words; {above_all} distinct above their unit's level; {unknown_all} without a level"
    );
    Ok(())
}
