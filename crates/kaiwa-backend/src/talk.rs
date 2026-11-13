//! The conversation's log as the page shows it (one entry per line, the learner's and the
//! character's) against the session's turns. They are not the same list: a line the tutor
//! never answered (the network, the cost limit) stays in the log, marked `failed`, but is no
//! turn of the session. Everything the page does to "this line" (misrecognised /LF113/,
//! dispute /LF307/, the recording's number /LF712/) goes through here.

use serde_json::Value;

/// Whether a log entry is a line of the learner's that became a turn of the session.
fn is_turn(entry: &Value) -> bool {
    entry["who"] == "learner" && entry["failed"] != true
}

/// The session's turn index of the log entry at `index`; `None` for the character's lines,
/// for a line the tutor never answered and beyond the log.
pub fn turn_of(log: &[Value], index: usize) -> Option<usize> {
    is_turn(log.get(index)?).then(|| turns(&log[..index]))
}

/// How many turns of the session the log holds: the index the next one gets.
pub fn turns(log: &[Value]) -> usize {
    log.iter().filter(|e| is_turn(e)).count()
}

/// What a voice that just loaded still has to say.
#[derive(Debug, PartialEq)]
pub enum Owed {
    /// The tutor's reply came while the voice was still loading.
    Reply(String),
    /// The opening line, once per talk.
    Opening,
    Nothing,
}

/// What the character could not say yet because the voice was not there: a reply that came
/// early, and whether the opening was said (a reload after the idle time must not say it
/// again).
#[derive(Debug, Default)]
pub struct Debt {
    reply: Option<String>,
    opening_said: bool,
}

impl Debt {
    /// A new talk: nothing is owed, the opening is still to say.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The opening was said (or is being said).
    pub fn opening_said(&mut self) {
        self.opening_said = true;
    }

    /// A reply came and there was no voice to say it; the latest one is kept.
    pub fn reply_early(&mut self, markup: &str) {
        self.reply = Some(markup.to_string());
    }

    /// No voice will come (model missing): nothing is owed any more.
    pub fn forgive(&mut self) {
        self.reply = None;
    }

    /// The voice is ready; what does the talk want now? `lines` is the log's length.
    pub fn voice_ready(&mut self, lines: usize, busy: bool) -> Owed {
        if let Some(markup) = self.reply.take() {
            return Owed::Reply(markup);
        }
        if lines != 1 || busy || self.opening_said {
            return Owed::Nothing;
        }

        self.opening_said = true;
        Owed::Opening
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_line_the_tutor_never_answered_is_no_turn() {
        // A fails (network), B is the same line again, C follows.
        let log = vec![
            json!({ "who": "character" }),
            json!({ "who": "learner", "text": "A", "failed": true }),
            json!({ "who": "learner", "text": "B" }),
            json!({ "who": "character" }),
            json!({ "who": "learner", "text": "C" }),
            json!({ "who": "character" }),
        ];
        assert_eq!(turn_of(&log, 1), None);
        assert_eq!(turn_of(&log, 2), Some(0), "B is the session's first turn");
        assert_eq!(turn_of(&log, 4), Some(1));
        assert_eq!(turn_of(&log, 0), None);
        assert_eq!(turn_of(&log, 9), None);
        assert_eq!(turns(&log), 2);
    }

    #[test]
    fn a_reply_before_the_voice_is_said_once_the_voice_is_ready() {
        let mut d = Debt::default();
        d.opening_said();
        d.reply_early("もしもし");
        d.reply_early("こんにちは");
        assert_eq!(d.voice_ready(3, false), Owed::Reply("こんにちは".into()));
        assert_eq!(d.voice_ready(3, false), Owed::Nothing, "only once");
    }

    #[test]
    fn a_missing_voice_forgives_the_reply() {
        let mut d = Debt::default();
        d.reply_early("こんにちは");
        d.forgive();
        assert_eq!(d.voice_ready(3, false), Owed::Nothing);
    }

    #[test]
    fn the_opening_is_said_the_first_time_only() {
        let mut d = Debt::default();
        assert_eq!(d.voice_ready(1, false), Owed::Opening);
        // Idle unload, a reload while the talk is still at the opening.
        assert_eq!(d.voice_ready(1, false), Owed::Nothing);
        d.reset();
        assert_eq!(d.voice_ready(1, false), Owed::Opening, "a new talk");
    }

    #[test]
    fn an_opening_said_elsewhere_is_not_said_again() {
        let mut d = Debt::default();
        d.opening_said();
        assert_eq!(d.voice_ready(1, false), Owed::Nothing);
    }

    #[test]
    fn a_turn_in_flight_has_no_opening() {
        let mut d = Debt::default();
        assert_eq!(d.voice_ready(1, true), Owed::Nothing);
    }
}
