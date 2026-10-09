//! Kaiwa's conversation engine and learning model, without a GUI (Lastenheft /LL505/):
//! scenarios with goals and help, a session that sends each turn to a [`kaiwa_ai::ChatProvider`],
//! the rules for which corrections a learner sees, the summary at the end, and around it the
//! learner model, the difficulty regulator, the curriculum, spaced repetition and the daily goal
//! with streak and XP.

pub mod clock;
pub mod curriculum;
pub mod diff;
pub mod difficulty;
pub mod feedback;
pub mod learner;
pub mod progress;
pub mod prompts;
pub mod scenario;
pub mod session;
pub mod srs;

pub use clock::{Clock, Day, Moment};
pub use curriculum::{CourseProgress, Curriculum};
pub use difficulty::{Difficulty, Regulator};
pub use feedback::Amount;
pub use learner::LearnerModel;
pub use scenario::Scenario;
pub use session::{Input, Session, SessionState, Stats, Summary, TurnOutcome};

/// Text without furigana markup (`{漢字|かんじ}` → `漢字`, as in KanteRubyText).
pub fn plain(markup: &str) -> String {
    unmark(markup, false)
}

/// Text as it is spoken: the readings in place of the kanji, in katakana (`{田中|たなか}` →
/// `タナカ`), so the voice says names and rare words as written and a tokenizer keeps each
/// reading as one word.
pub fn spoken(markup: &str) -> String {
    unmark(markup, true)
}

fn unmark(markup: &str, reading: bool) -> String {
    let mut out = String::with_capacity(markup.len());
    let mut rest = markup;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let tail = &rest[start + 1..];
        match (tail.find('|'), tail.find('}')) {
            (Some(bar), Some(end)) if bar < end => {
                let ruby = &tail[bar + 1..end];
                if reading {
                    out.extend(ruby.split('|').next().unwrap_or(ruby).chars().map(katakana));
                } else {
                    out.push_str(&tail[..bar]);
                }
                rest = &tail[end + 1..];
            }
            _ => {
                out.push('{');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

fn katakana(c: char) -> char {
    match c {
        'ぁ'..='ゖ' => char::from_u32(c as u32 + 0x60).unwrap_or(c),
        _ => c,
    }
}

/// Comparison key for Japanese text: without punctuation and white space.
pub fn norm(text: &str) -> String {
    kaiwa_ai::mock::key(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_strips_readings() {
        assert_eq!(plain("{駅|えき}までは{歩|ある|new}いて"), "駅までは歩いて");
        assert_eq!(plain("はい{"), "はい{");
        assert_eq!(plain("a {b} c"), "a {b} c");
    }

    #[test]
    fn spoken_uses_the_readings() {
        assert_eq!(
            spoken("はじめまして。{田中|たなか}です。"),
            "はじめまして。タナカです。"
        );
        assert_eq!(
            spoken("{駅|えき}までは{歩|ある|new}いて"),
            "エキまではアルいて"
        );
        assert_eq!(spoken("はい{"), "はい{");
    }

    #[test]
    fn norm_ignores_punctuation() {
        assert_eq!(norm("コーヒーが 好きです。"), norm("コーヒーが好きです"));
    }
}
