//! What the learner did, as plain data (ADR-010): the journal keeps these, and the profile's
//! state is what folding them through the backend's commands yields.

use serde::{Deserialize, Serialize};

use crate::clock::{Day, Moment};
use crate::session::SessionState;
use crate::srs::Grade;

/// One act of the learner that changes the profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// A review card answered: its text and the grade the answer earned.
    Review { text: String, grade: Grade },
    /// A setting changed, as `Backend::set_setting` takes it.
    Setting { key: String, value: String },
    /// A talk begun: its state at the start. The event's id is the talk's id.
    TalkBegun { state: SessionState },
    /// The state of talk `talk` kept after a turn, an aid or a mark.
    TalkSaved { talk: String, state: SessionState },
    /// Talk `talk` ended: its state, the time it began (the end minus all time spent in it)
    /// and the day that gets its practice; the end is the event's moment.
    TalkFinished {
        talk: String,
        state: SessionState,
        started: Moment,
        ended: Moment,
        day: Day,
    },
    /// The learner marked turns of the finished talk `talk` (V-05): its state with the marks.
    TalkCorrected { talk: String, state: SessionState },
}
