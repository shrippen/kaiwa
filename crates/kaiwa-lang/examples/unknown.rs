//! Lists the words of the shipped content (scenarios, recorded answers) the dictionary does not
//! know: typos, odd spellings or segmentation the lookup cannot follow (/LF221/). With
//! `--readings`, the words whose entry reads differently from the reading of the line (a
//! homograph chosen wrong, e.g. 何 as なに where the line says なん).
//!
//!   cargo run -p kaiwa-lang --example unknown -- <kaiwa-dict.sqlite> [--readings]

use std::fs;
use std::path::{Path, PathBuf};

use kaiwa_lang::{Dictionary, Words, annotate};

fn strings(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s)
            if s.chars()
                .any(|c| matches!(c, '\u{3040}'..='\u{30ff}' | '\u{4e00}'..='\u{9fff}')) =>
        {
            out.push(s.clone())
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: unknown <kaiwa-dict.sqlite>")?,
    );
    let readings = std::env::args().any(|a| a == "--readings");
    let dict = Dictionary::open(&db)?;
    let words = Words::new()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for dir in ["data/scenarios", "qa/fixtures"] {
        for e in fs::read_dir(root.join(dir))? {
            let p = e?.path();
            let file = p.file_name().unwrap().to_string_lossy().to_string();
            let mut texts = Vec::new();
            strings(&serde_json::from_str(&fs::read_to_string(&p)?)?, &mut texts);
            for t in texts {
                for w in annotate(&words, &t).1 {
                    match dict.lookup_word(&w, "de")? {
                        None if !readings => println!("{file}\t{}\t({})\t{t}", w.text, w.key),
                        Some(e) if readings => {
                            let line = w.reading.clone().unwrap_or_default();
                            let entry = kaiwa_lang::words::hiragana(&e.reading);
                            if !line.is_empty() && line != entry && e.word == w.text {
                                println!(
                                    "{file}\t{} line {line} entry {entry}\t{:?}",
                                    w.text,
                                    e.senses.first()
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}
