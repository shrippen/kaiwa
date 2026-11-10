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
}
