//! Builds UniDic from a downloaded archive and prints the furigana of sentences (manual
//! check), or of each line of stdin with `-`.
//!   KAIWA_MODELS=… cargo run -p kaiwa-lang --example unidic_probe -- [archive] "文"… | -
use std::io::BufRead;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a.ends_with(".tar.gz")) {
        let archive = args.remove(0);
        let t = std::time::Instant::now();
        kaiwa_lang::unidic::build(std::path::Path::new(&archive), &kaiwa_lang::unidic::path())
            .unwrap();
        eprintln!("built in {:?}", t.elapsed());
    }
    let words = kaiwa_lang::Words::new().unwrap();
    if args.first().is_some_and(|a| a == "-") {
        for line in std::io::stdin().lock().lines() {
            println!("{}", kaiwa_lang::furigana(&words, &line.unwrap()));
        }
        return;
    }
    for s in args {
        println!("{s}\n  {}", kaiwa_lang::furigana(&words, &s));
    }
}
