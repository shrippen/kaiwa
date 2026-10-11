//! The app's mora hints on the real voices of spike 0.6 (`spikes/s06-moren/real.py`):
//! `cargo run --release -p kaiwa-speech --example mora_hints -- <clips.json> <edited dir> [48000]`.
//!
//! For each clip it reads the sentence as the app does (`G2p::pronounced_words`), checks the
//! original and the clips with a long vowel shortened (`long-`) or っ left out (`q-`), and counts
//! the hints (`mora::hints`: only shortenings, only sentences, at most one). With a rate the
//! 16 kHz clips are first brought to it, so the resampling of the microphone path runs too.
//! The model comes from `$KAIWA_MODELS/kaiwa-kana-ctc`.

use kaiwa_speech::g2p::G2p;
use kaiwa_speech::models::models_dir;
use kaiwa_speech::mora::{Checker, Kind, MODEL, SAMPLE_RATE};

fn read(path: &std::path::Path) -> Option<Vec<f32>> {
    let mut reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, SAMPLE_RATE, "16 kHz expected");
    let scale = (1u32 << (spec.bits_per_sample - 1)) as f32;
    Some(
        reader
            .samples::<i32>()
            .map(|s| s.unwrap() as f32 / scale)
            .collect(),
    )
}

/// Linear interpolation up to `rate`, like a microphone at that rate would give (roughly).
fn upsample(x: &[f32], rate: u32) -> Vec<f32> {
    let ratio = f64::from(rate) / f64::from(SAMPLE_RATE);
    let n = (x.len() as f64 * ratio) as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 / ratio;
            let j = t.floor() as usize;
            let f = (t - j as f64) as f32;
            let a = x[j.min(x.len() - 1)];
            let b = x[(j + 1).min(x.len() - 1)];
            a + (b - a) * f
        })
        .collect()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(clips), Some(dir)) = (args.next(), args.next()) else {
        eprintln!("usage: mora_hints <clips.json> <edited dir> [rate]");
        std::process::exit(2);
    };
    let rate: u32 = args
        .next()
        .and_then(|r| r.parse().ok())
        .unwrap_or(SAMPLE_RATE);
    let clips: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(clips).expect("clips.json")).expect("json");
    let g2p = G2p::new().expect("g2p");
    let mut checker = Checker::from_dir(&models_dir().join(MODEL), 4).expect("model");
    let dir = std::path::Path::new(&dir);
    // (clips, hints, hints of the right kind) for original, long-, q-
    let mut counts = [(0usize, 0usize, 0usize); 3];
    let mut unreadable = 0;
    for clip in &clips {
        let uid = clip["uid"].as_str().unwrap_or_default();
        // MORA_HINTS_PRON: OpenJTalk's reading from real.py instead of the app's, as one word.
        let words = if std::env::var_os("MORA_HINTS_PRON").is_some() {
            let pron = clip["pron"].as_str().unwrap_or_default().to_string();
            Some(vec![(pron.clone(), pron)])
        } else {
            g2p.pronounced_words(clip["sentence"].as_str().unwrap_or_default())
        };
        let Some(words) = words else {
            unreadable += 1;
            continue;
        };
        for (i, (tag, kind)) in [
            ("original", None),
            ("long-", Some(Kind::LongShort)),
            ("q-", Some(Kind::GeminateLeftOut)),
        ]
        .into_iter()
        .enumerate()
        {
            let Some(samples) = read(&dir.join(format!("{uid}_{tag}.wav"))) else {
                continue;
            };
            let samples = if rate == SAMPLE_RATE {
                samples
            } else {
                upsample(&samples, rate)
            };
            let hints = checker.hints(&samples, rate, &words).expect("check");
            if std::env::var_os("MORA_HINTS_LIST").is_some() {
                let (k, m) = hints
                    .first()
                    .map(|h| (h.kind.name(), h.margin))
                    .unwrap_or(("none", 0.0));
                println!(
                    "M {tag} {uid} {} {k} {m:.3}",
                    clip["speaker"].as_str().unwrap_or_default()
                );
            }
            let c = &mut counts[i];
            c.0 += 1;
            if let Some(h) = hints.first() {
                c.1 += 1;
                if Some(h.kind) == kind {
                    c.2 += 1;
                }
                if i > 0 && c.2 <= 3 && Some(h.kind) == kind {
                    println!("{tag:9} {uid}: {} → {} ({})", h.meant, h.heard, h.surface);
                }
            }
        }
    }
    let pct = |a: usize, b: usize| 100.0 * a as f64 / b.max(1) as f64;
    println!("rate {rate} Hz; {unreadable} sentences without a full reading skipped");
    let (n, h, _) = counts[0];
    println!(
        "original  {n:4} clips, hint on {h} ({:.1} %, false alarms)",
        pct(h, n)
    );
    for (i, tag) in [(1, "long-"), (2, "q-")] {
        let (n, h, right) = counts[i];
        println!(
            "{tag:9} {n:4} clips, hint on {h} ({:.1} %), of the right kind {right} ({:.1} %)",
            pct(h, n),
            pct(right, n)
        );
    }
}
