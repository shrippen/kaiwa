//! Looks up every word of every Japanese text of the scenarios, fixtures, scripts and personas,
//! as a click does (word, else name), one TSV line per word. Run it on two revisions and diff
//! the output to see what a change to the ranking did.
//!
//!   cargo run -p kaiwa-lang --example lookup_corpus -- <kaiwa-dict.sqlite> > out.tsv
//!
//! Columns: text, surface, shown word, reading, first sense (English).

use std::fs;
use std::path::{Path, PathBuf};

use kaiwa_lang::{Dictionary, Words, annotate};

const DIRS: [&str; 4] = ["data/scenarios", "qa/fixtures", "qa/scripts", "qa/personas"];

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
    let db = std::env::args()
        .nth(1)
        .ok_or("usage: lookup_corpus <kaiwa-dict.sqlite>")?;
    let dict = Dictionary::open(&PathBuf::from(db))?;
    let words = Words::new()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let mut texts = Vec::new();
    for dir in DIRS {
        let mut files: Vec<PathBuf> = fs::read_dir(root.join(dir))?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<_, _>>()?;
        files.sort();
        for p in files
            .iter()
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
        {
            strings(&serde_json::from_str(&fs::read_to_string(p)?)?, &mut texts);
        }
    }

    for text in &texts {
        // As the app does for a line (backend `line`): IPADIC's furigana for bare kanji.
        let markup = kaiwa_lang::complete(&words, text);
        for w in annotate(&words, &markup).1 {
            let (word, reading, sense) = match dict.lookup_word(&w, "en")? {
                Some(e) => (
                    e.word,
                    e.reading,
                    e.senses.first().map(|s| s.join("; ")).unwrap_or_default(),
                ),
                None => match dict.lookup_name(&w)? {
                    Some(n) => (
                        n.word,
                        n.reading,
                        format!("name: {}", n.trans.unwrap_or_default()),
                    ),
                    None => ("-".into(), String::new(), String::new()),
                },
            };
            println!("{text}\t{}\t{word}\t{reading}\t{sense}", w.text);
        }
    }
    eprintln!("{} texts", texts.len());
    Ok(())
}
