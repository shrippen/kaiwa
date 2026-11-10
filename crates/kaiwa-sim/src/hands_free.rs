//! Hinnerk speaking hands-free, through the app's own end of turn (`kaiwa_core::turn_end`,
//! /LF101/, /LF102/): the persona check `hinnerk_no_cutoff`, over many seeds.
//!
//! Its rule since T-01 (owner, 10.11.2026): thinking and length end no turn; the first turn
//! the app can tell was cut off brings the offer of a longer pause, two steps on
//! (1.2 → 3 → 5 s); once the pause fits him, no turn is cut off any more. Until then he is
//! cut off: the old rule ("no turn is closed before its end") was not what the app does with
//! a fixed pause, and the check did not run under it (review #65).

use anyhow::Result;
use serde_json::Value;

/// Hinnerk speaks hands-free the way his persona says (`speech`): he thinks 3–7 s before he
/// answers, pauses 1.5–3.2 s within a sentence, says a filler now and then and speaks 2.5–3.8
/// morae a second. His level runs through the end of turn, starting at the default pause of
/// 1.2 s, over `CUTOFF_SEEDS` seeds of `CUTOFF_TURNS` turns each. Thinking never ends a turn and nor does its length; a pause
/// longer than the setting cuts it off, and what was heard so far is what the recogniser would
/// give. A cut counts when the app can tell it (`sounds_unfinished`: the part ends on a
/// particle or a connecting form; after a bare まいあさ it cannot). Every such turn offers
/// a pause two steps longer, which he takes; in the end the pause fits him, and from then on
/// no turn is cut off. The recogniser and the voice are not played (audio mode); the text of
/// a cut is the part before the pause.
pub(crate) fn slow_speaker(p: &Value) -> Result<Option<String>> {
    use kaiwa_core::turn_end::{EndOfTurn, PAUSE_STEPS};
    let range = |key: &str, scale: f64| -> Result<(f64, f64)> {
        let r = &p["speech"][key];
        match (r[0].as_f64(), r[1].as_f64()) {
            (Some(a), Some(b)) => Ok((a * scale, b * scale)),
            _ => anyhow::bail!("speech.{key} missing"),
        }
    };
    let speech = Speech {
        think: range("pause_before_answer_ms", 0.001)?,
        within: range("pause_within_turn_ms", 0.001)?,
        rate: range("rate_mora_per_s", 1.0)?,
        fillers: p["speech"]["fillers"]
            .as_array()
            .is_some_and(|f| !f.is_empty()),
    };
    let mut found = Vec::new();
    let mut longest = 0.0f64;
    for seed in 0..CUTOFF_SEEDS {
        let (why, turn) = hands_free(&speech, 0x4869_6E6E + seed);
        longest = longest.max(turn);
        if let Some(why) = why {
            found.push(format!("seed {seed}: {why}"));
        }
    }
    if longest < 20.0 {
        found.push(format!(
            "the longest turn took {longest:.1} s: no long turn shown"
        ));
    }
    // /LF102/: his shortest pause within a sentence ends a turn at the default pause, but not
    // with a filler in the middle of it: the filler is voice.
    let ends_at_default = |filler: bool| {
        let gap = speech.within.0;
        let mut e = EndOfTurn::new(PAUSE_STEPS[0]);
        let mut level = vec![(0.2f32, 3.0), (0.65, 1.2)];
        if filler {
            level.extend([(0.2, gap / 2.0), (0.5, 0.4), (0.2, gap / 2.0)]);
        } else {
            level.push((0.2, gap));
        }
        level.push((0.65, 1.2));
        level
            .iter()
            .flat_map(|(l, secs)| std::iter::repeat_n(*l, (secs / CUTOFF_STEP).round() as usize))
            .any(|l| e.update(l, CUTOFF_STEP))
    };
    if !ends_at_default(false) {
        found.push(format!(
            "a pause of {} s does not end a turn at 1.2 s",
            speech.within.0
        ));
    } else if ends_at_default(true) {
        found.push("a filler in the pause did not keep the turn going".into());
    }
    found.truncate(4);
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

const CUTOFF_SEEDS: u64 = 40;
const CUTOFF_TURNS: usize = 200;
const CUTOFF_STEP: f64 = 0.08;
/// The most turns of his the app cuts off before the pause fits him, on any seed: two, one
/// at 1.2 s and one at 3 s (measured on all seeds; a cut after a bare adverb, which the app
/// cannot tell, would add one and show here).
const MAX_CUT: u32 = 2;

/// How a persona speaks (`speech` in its file), in seconds and morae per second.
struct Speech {
    think: (f64, f64),
    within: (f64, f64),
    rate: (f64, f64),
    fillers: bool,
}

/// One seed of `slow_speaker`: what went wrong, if anything, and the longest turn in seconds.
fn hands_free(speech: &Speech, seed: u64) -> (Option<String>, f64) {
    use kaiwa_core::turn_end::{CutOffs, EndOfTurn, PAUSE_STEPS, sounds_unfinished};
    const LINES: [&[&str]; 5] = [
        &["わたしは", "アールダースです。"],
        &["まえは", "とうだいで", "はたらいていました。"],
        &["まごが", "にほんに", "すんでいます。"],
        &["まいあさ", "うみを", "みて", "さんぽします。"],
        &["にほんごは", "むずかしいけど", "たのしいです。"],
    ];
    let (think, within, rate) = (speech.think, speech.within, speech.rate);
    let mut rng = crate::days::Rng(seed);
    let mut pause = PAUSE_STEPS[0];
    let mut cuts = CutOffs::default();
    // Cut-off turns the app can tell, and the offers: (turn, such turns so far, pause offered).
    let (mut told, mut offers) = (0u32, Vec::new());
    // Every cut-off turn, told or not.
    let mut cut = 0u32;
    let mut longest = 0.0f64;
    for turn in 0..CUTOFF_TURNS {
        let line = LINES[turn % LINES.len()];
        // The level of the turn: thinking, then the parts with the pauses between them.
        let mut level: Vec<(f32, f64)> = vec![(0.2, rng.between(think.0, think.1))];
        let mut ends = Vec::new();
        for (i, part) in line.iter().enumerate() {
            if i > 0 {
                let gap = rng.between(within.0, within.1);
                if speech.fillers && rng.chance(0.3) {
                    // A filler in the middle of the pause: quieter, still voice (/LF102/).
                    level.push((0.2, gap / 2.0));
                    level.push((0.5, 0.4));
                    level.push((0.2, gap / 2.0));
                } else {
                    level.push((0.2, gap));
                }
            }
            let morae = part.chars().filter(|c| !"。、ゃゅょ".contains(*c)).count() as f64;
            level.push((0.65, morae / rng.between(rate.0, rate.1)));
            ends.push(level.len() - 1);
        }
        level.push((0.2, 6.0));
        let mut e = EndOfTurn::new(pause);
        let mut ended = None;
        'feed: for (n, (l, secs)) in level.iter().enumerate() {
            for _ in 0..(secs / CUTOFF_STEP).round() as usize {
                if e.update(*l, CUTOFF_STEP) {
                    ended = Some(n);
                    break 'feed;
                }
            }
        }
        let Some(at) = ended else {
            return (Some(format!("turn {turn} never ended")), longest);
        };
        longest = longest.max(e.listened_secs());
        // The parts spoken before the turn ended.
        let said = ends.iter().filter(|i| **i < at).count();
        if said == 0 {
            return (
                Some(format!("turn {turn} ended while he was thinking")),
                longest,
            );
        }
        let text: String = line[..said].concat();
        if said < line.len() {
            if f64::from(pause) >= within.1 {
                return (
                    Some(format!("cut off at a pause of {pause} s: {text}")),
                    longest,
                );
            }
            told += u32::from(sounds_unfinished(&text));
            cut += 1;
        }
        if let Some(next) = cuts.after_turn(true, &text, pause) {
            // Two steps on, never beyond the longest.
            let step = PAUSE_STEPS.iter().position(|s| *s == pause);
            let want = step.map(|i| PAUSE_STEPS[(i + 2).min(PAUSE_STEPS.len() - 1)]);
            if want != Some(next) {
                return (
                    Some(format!("offered {next} s at a pause of {pause} s")),
                    longest,
                );
            }
            offers.push((turn, told, next));
            pause = next;
            cuts.reset();
        }
    }
    let why = match offers.first() {
        None => Some("no longer pause offered".to_string()),
        Some((_, told, _)) if *told != 1 => Some(format!(
            "the first offer came after {told} cut-off turns the app can tell, not the first"
        )),
        _ if offers.iter().zip(1..).any(|((_, told, _), n)| *told != n) => Some(format!(
            "an offer not at every cut-off turn the app can tell: {offers:?}"
        )),
        _ if cut > MAX_CUT => Some(format!(
            "{cut} turns cut off before the pause fit him (at most {MAX_CUT} so far)"
        )),
        _ if f64::from(pause) < within.1 => Some(format!(
            "after {CUTOFF_TURNS} turns the pause is {pause} s, his pauses go up to {} s",
            within.1
        )),
        _ => None,
    };
    if std::env::var_os("KAIWA_SIM_VERBOSE").is_some() {
        eprintln!("hands_free, seed {seed}: {cut} cut, offers {offers:?}, longest {longest:.1} s");
    }
    (why, longest)
}

#[test]
fn hinnerk_is_cut_off_only_until_the_pause_fits() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let p: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("qa/personas/hinnerk.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(slow_speaker(&p).unwrap(), None);
}
