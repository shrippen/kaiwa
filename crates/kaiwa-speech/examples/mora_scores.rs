//! Raw scores of the mora check for calibration and model comparison, on the real voices of
//! spike 0.6: `cargo run --release -p kaiwa-speech --example mora_scores -- <clips.json>
//! <edited dir> <model dir> [<second model dir>]`. Prints per clip and version (original,
//! long-, q-) the best score of each shortening kind for the first model, the second, and the
//! two averaged (log probabilities frame by frame; both need the same vocabulary).

use kaiwa_speech::g2p::G2p;
use kaiwa_speech::mora::{Checker, Kind};

fn read(path: &std::path::Path) -> Option<Vec<f32>> {
    let mut reader = hound::WavReader::open(path).ok()?;
    let scale = (1u32 << (reader.spec().bits_per_sample - 1)) as f32;
    Some(
        reader
            .samples::<i32>()
            .map(|s| s.unwrap() as f32 / scale)
            .collect(),
    )
}

fn score(kinds: &[(Kind, f32)], k: Kind) -> f32 {
    kinds
        .iter()
        .find(|(x, _)| *x == k)
        .map_or(f32::NEG_INFINITY, |(_, s)| *s)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let clips: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(&args[0]).expect("clips")).expect("json");
    let dir = std::path::Path::new(&args[1]);
    let mut a = Checker::from_dir(std::path::Path::new(&args[2]), 4).expect("model");
    let mut b = args
        .get(3)
        .map(|d| Checker::from_dir(std::path::Path::new(d), 4).expect("second model"));
    let g2p = G2p::new().expect("g2p");
    for clip in &clips {
        let uid = clip["uid"].as_str().unwrap_or_default();
        let spk = clip["speaker"].as_str().unwrap_or_default();
        let Some(words) = g2p.pronounced_words(clip["sentence"].as_str().unwrap_or_default())
        else {
            continue;
        };
        let kana: String = words.iter().map(|(_, k)| k.as_str()).collect();
        for tag in ["original", "long-", "q-"] {
            let Some(x) = read(&dir.join(format!("{uid}_{tag}.wav"))) else {
                continue;
            };
            let la = a.log_probs(&x).expect("a");
            let mut rows = vec![("a", a.raw_scores(&la, &kana))];
            if let Some(b) = b.as_mut() {
                let lb = b.log_probs(&x).expect("b");
                if lb.len() == la.len() {
                    let avg: Vec<f32> = la.iter().zip(&lb).map(|(p, q)| 0.5 * (p + q)).collect();
                    rows.push(("b", b.raw_scores(&lb, &kana)));
                    rows.push(("ab", a.raw_scores(&avg, &kana)));
                }
            }
            for (m, s) in rows {
                println!(
                    "S {tag} {uid} {spk} {m} {:.3} {:.3}",
                    score(&s, Kind::LongShort),
                    score(&s, Kind::GeminateLeftOut)
                );
            }
        }
    }
}
