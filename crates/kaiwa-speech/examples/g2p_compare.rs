//! Compare the G2P with misaki's output: `cargo run -p kaiwa-speech --example g2p_compare -- golden.json`
//! where golden.json is `[[text, phonemes], …]`.
fn main() {
    let path = std::env::args().nth(1).expect("golden.json");
    let text = std::fs::read_to_string(path).expect("read");
    let g = kaiwa_speech::g2p::G2p::new().expect("dictionary");
    let mut same = 0;
    let mut total = 0;
    for line in text
        .trim()
        .trim_start_matches("[[")
        .trim_end_matches("]]")
        .split("], [")
    {
        let parts: Vec<&str> = line.split("\", \"").map(|s| s.trim_matches('"')).collect();
        let (t, want) = (parts[0], parts[1]);
        let got = g.phonemes(t);
        total += 1;
        if got == want {
            same += 1;
        } else {
            println!("{t}\n  misaki {want}\n  kaiwa  {got}");
        }
    }
    println!("{same}/{total} identical");
}
