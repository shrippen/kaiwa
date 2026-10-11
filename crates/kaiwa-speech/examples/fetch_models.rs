//! Downloads the speech models as the app's settings do (F-08):
//! `cargo run -p kaiwa-speech --example fetch_models -- [name …]`, into `$KAIWA_MODELS`.
//! Without names it fetches every model that is missing.

use std::sync::atomic::AtomicBool;

use kaiwa_speech::download::{MODELS, download};

fn main() {
    let names: Vec<String> = std::env::args().skip(1).collect();
    let stop = AtomicBool::new(false);
    for m in MODELS {
        if !names.is_empty() && !names.iter().any(|n| n == m.name) {
            continue;
        }
        if m.present() {
            println!("{}: present in {}", m.name, m.path().display());
            continue;
        }
        let total = m.download_bytes();
        println!(
            "{} ({}, {}): {} MB from {}",
            m.name,
            m.licence,
            m.origin,
            total / 1_000_000,
            m.path().display()
        );
        let progress = |done: u64| {
            if done == total || done % 50_000_000 < 65_536 {
                eprint!("\r  {} / {} MB", done / 1_000_000, total / 1_000_000);
            }
        };
        match download(&m, &progress, &stop) {
            Ok(()) => println!("\n  ok, present: {}", m.present()),
            Err(e) => println!("\n  failed: {e}"),
        }
    }
}
