//! The mora check on a recording: `cargo run -p kaiwa-speech --example mora_check -- <wav> <sentence>`.
//! The wav is 16 kHz mono; the model comes from `$KAIWA_MODELS/kaiwa-kana-ctc`. Prints one line per
//! finding: kind, mora, variant, margin.

use kaiwa_speech::g2p::G2p;
use kaiwa_speech::models::models_dir;
use kaiwa_speech::mora::{Checker, MODEL, SAMPLE_RATE};

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(wav), Some(text)) = (args.next(), args.next()) else {
        eprintln!("usage: mora_check <wav, 16 kHz mono> <sentence>");
        std::process::exit(2);
    };
    let mut reader = hound::WavReader::open(&wav).expect("wav");
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, SAMPLE_RATE, "16 kHz expected");
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(Result::unwrap).collect(),
        hound::SampleFormat::Int => {
            let scale = (1u32 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 / scale)
                .collect()
        }
    };
    let kana = G2p::new().expect("g2p").pronounced(&text);
    let mut checker = Checker::from_dir(&models_dir().join(MODEL), 4).expect("model");
    println!("kana {kana}");
    for f in checker.check(&samples, &kana).expect("check") {
        println!("{} {} {} {:.2}", f.kind.name(), f.mora, f.variant, f.margin);
    }
}
