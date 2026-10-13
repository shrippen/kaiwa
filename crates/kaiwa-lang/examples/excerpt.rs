//! Writes the dictionary excerpt for the demo and the tests (`data/dict/kaiwa-dict-excerpt.sqlite`):
//! every entry a word of the scenarios, the recorded answers and the QA scripts can look up,
//! copied unchanged from the full package (same schema, CC BY-SA 4.0 as the package).
//!
//!   cargo run -p kaiwa-lang --example excerpt -- <kaiwa-dict.sqlite> [out]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use kaiwa_lang::{Words, annotate};
use rusqlite::{Connection, params};

fn japanese(s: &str) -> bool {
    s.chars()
        .any(|c| matches!(c, '\u{3040}'..='\u{30ff}' | '\u{4e00}'..='\u{9fff}'))
}

fn strings(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) if japanese(s) => out.push(s.clone()),
        serde_json::Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let full = PathBuf::from(
        args.first()
            .ok_or("usage: excerpt <kaiwa-dict.sqlite> [out]")?,
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("data/dict/kaiwa-dict-excerpt.sqlite"));

    let mut texts = Vec::new();
    for dir in ["data/scenarios", "qa/fixtures", "qa/scripts", "qa/personas"] {
        for e in fs::read_dir(root.join(dir))? {
            let p = e?.path();
            if p.extension().is_some_and(|x| x == "json") {
                strings(&serde_json::from_str(&fs::read_to_string(&p)?)?, &mut texts);
            }
        }
    }
    let words = Words::new()?;
    let mut keys = BTreeSet::new();
    for t in &texts {
        let (_, ws) = annotate(&words, t);
        for w in ws {
            keys.insert(kaiwa_lang::dict::full_width(&w.key));
            keys.insert(w.key);
            keys.insert(w.text);
            keys.extend(w.fallback);
        }
    }

    if out.exists() {
        fs::remove_file(&out)?;
    }
    fs::create_dir_all(out.parent().ok_or("no parent")?)?;
    let db = Connection::open(&out)?;
    db.execute(
        "ATTACH DATABASE ?1 AS full",
        params![full.to_string_lossy()],
    )?;
    let schema: Vec<String> = db
        .prepare("SELECT sql FROM full.sqlite_master WHERE sql IS NOT NULL ORDER BY type = 'index', name")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    for sql in schema {
        db.execute_batch(&sql)?;
    }
    db.execute_batch("CREATE TEMP TABLE k (text TEXT PRIMARY KEY)")?;
    for k in &keys {
        db.execute("INSERT OR IGNORE INTO k VALUES (?1)", [k])?;
    }
    db.execute_batch(
        "CREATE TEMP TABLE ids AS SELECT DISTINCT entry AS id FROM full.form WHERE text IN (SELECT text FROM k);
         INSERT INTO entry SELECT * FROM full.entry WHERE id IN (SELECT id FROM ids);
         INSERT INTO form SELECT * FROM full.form WHERE entry IN (SELECT id FROM ids);
         INSERT INTO sense SELECT * FROM full.sense WHERE entry IN (SELECT id FROM ids);
         INSERT INTO accent SELECT * FROM full.accent
             WHERE word IN (SELECT text FROM form) OR reading IN (SELECT text FROM form WHERE kind = 'r');
         INSERT INTO entity SELECT * FROM full.entity;
         INSERT INTO source SELECT * FROM full.source;
         INSERT INTO meta SELECT * FROM full.meta;
         INSERT OR REPLACE INTO meta VALUES ('excerpt', 'demo and tests');",
    )?;
    let kanji: BTreeSet<String> = keys
        .iter()
        .flat_map(|k| k.chars())
        .filter(|c| matches!(c, '\u{4e00}'..='\u{9fff}'))
        .map(String::from)
        .collect();
    for k in &kanji {
        db.execute(
            "INSERT OR IGNORE INTO kanji SELECT * FROM full.kanji WHERE literal = ?1",
            [k],
        )?;
    }
    db.execute_batch("DETACH DATABASE full")?;
    let n: i64 = db.query_row("SELECT count(*) FROM entry", [], |r| r.get(0))?;
    db.execute_batch("VACUUM")?;
    println!(
        "{} words, {n} entries, {} kanji → {}",
        keys.len(),
        kanji.len(),
        out.display()
    );
    Ok(())
}
