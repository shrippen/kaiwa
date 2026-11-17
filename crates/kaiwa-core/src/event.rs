//! What the learner did, as plain data (ADR-010): the journal keeps these, and the profile's
//! state is what folding them through the backend's commands yields.

use serde::{Deserialize, Serialize};

use crate::clock::{Day, Moment};
use crate::difficulty::Felt;
use crate::learner::{LearnerModel, Skill};
use crate::progress::Pause;
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
    /// A placement measured these levels (/LF200/).
    Placed { model: LearnerModel },
    /// "I know this already" or "too hard" for a skill (/LF202/).
    Nudged { skill: Skill, felt: Felt },
    /// Units count as done without their talks (/LF215/).
    Skipped {
        units: Vec<String>,
        offer: SkipOffer,
    },
    /// A looked-up word taken into the review (/LF112/).
    WordAdded { text: String, unit: Option<String> },
    /// Pause mode for these days (/LF602/).
    Paused { pause: Pause },
    /// The pause running on `day` ended; the day counts again.
    PauseEnded { day: Day },
    /// The learner disputed a correction (/LF307/); it goes to the QA list.
    Disputed {
        scenario: String,
        learner: String,
        original: String,
        better: String,
        explanation: String,
    },
    /// Tutor costs booked; `talk_ended` counts a paid talk (/LF507/).
    CostsBooked {
        cents: f64,
        calls: u32,
        unknown_calls: u32,
        talk_ended: bool,
    },
    /// Back after a break: what is due is spread over the next days (/LF231/).
    BreakSpread,
}

/// Which offer a skip took.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipOffer {
    /// The unit the path offers after three good talks; it uses the offer up.
    Unit,
    /// The units the placement lets the learner leave out.
    Placement,
}
