//! Speak a line with the interim voice into a WAV file:
//! `cargo run -p kaiwa-speech --example say -- "はじめまして。" out.wav [voice] [speed]`
fn main() {
    let mut args = std::env::args().skip(1);
    let text = args.next().expect("text");
    let out = args.next().expect("out.wav");
    let name = args.next().unwrap_or_else(|| "jf_tebukuro".into());
    let speed: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let dir = kaiwa_speech::models::kokoro_dir();
    let mut voice = kaiwa_speech::tts::Voice::from_dir(&dir, &name, 2).expect("Kokoro");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: kaiwa_speech::tts::SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav = hound::WavWriter::create(&out, spec).expect("wav");
    for sentence in kaiwa_speech::tts::sentences(&text) {
        let t = std::time::Instant::now();
        let samples = voice.speak(&sentence, speed).expect("speak");
        println!(
            "{sentence}  {}  {:?}",
            voice.phonemes(&sentence),
            t.elapsed()
        );
        for s in samples {
            wav.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .expect("write");
        }
    }
    wav.finalize().expect("finalize");
}
