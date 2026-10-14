//! Looks up every word of a sentence, as the app does after a click.
//!
//!   cargo run -p kaiwa-lang --example lookup -- <kaiwa-dict.sqlite> de "駅までは歩いて五分です。"

use std::path::PathBuf;

use kaiwa_lang::{Dictionary, Words, annotate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let [db, lang, text] = a.as_slice() else {
        return Err("usage: lookup <kaiwa-dict.sqlite> <de|en> <text>".into());
    };
    let dict = Dictionary::open(&PathBuf::from(db))?;
    let (_, words) = annotate(&Words::new()?, text);
    for w in words {
        let e = dict.lookup_word(&w, lang)?;
        match e {
            Some(e) => println!(
                "{} → {} 【{}】 {} | {:?}{} | accent {:?}",
                w.text,
                e.word,
                e.reading,
                e.pos.join(", "),
                e.senses.first(),
                if e.fallback { " (en)" } else { "" },
                e.accent.map(|a| a.pattern)
            ),
            None => match dict.lookup_name(&w)? {
                Some(n) => println!(
                    "{} → {} 【{}】 {} | {:?}",
                    w.text,
                    n.word,
                    n.reading,
                    kaiwa_lang::pos::name_label(&n.kinds, lang),
                    n.trans
                ),
                None => println!("{} → ({}) not found", w.text, w.key),
            },
        }
    }
    Ok(())
}
