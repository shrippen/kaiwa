//! Kaiwa's conversation engine, without a GUI (Lastenheft /LL505/): scenarios with goals and
//! help, a session that sends each turn to a [`kaiwa_ai::ChatProvider`], the rules for which
//! corrections a learner sees, the diff for KanteDiffText and the summary at the end.

pub mod diff;
pub mod feedback;
pub mod prompts;
pub mod scenario;
pub mod session;

pub use feedback::Amount;
pub use scenario::Scenario;
pub use session::{Input, Session, Summary, TurnOutcome};

/// Text without furigana markup (`{漢字|かんじ}` → `漢字`, as in KanteRubyText).
pub fn plain(markup: &str) -> String {
    let mut out = String::with_capacity(markup.len());
    let mut rest = markup;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let tail = &rest[start + 1..];
        match (tail.find('|'), tail.find('}')) {
            (Some(bar), Some(end)) if bar < end => {
                out.push_str(&tail[..bar]);
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
    fn norm_ignores_punctuation() {
        assert_eq!(norm("コーヒーが 好きです。"), norm("コーヒーが好きです"));
    }
}
