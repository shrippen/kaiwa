//! What the learner did, as plain data (ADR-010): the journal keeps these, and the profile's
//! state is what folding them through the backend's commands yields.

use serde::{Deserialize, Serialize};

use crate::srs::Grade;

/// One act of the learner that changes the profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// A review card answered: its text and the grade the answer earned.
    Review { text: String, grade: Grade },
    /// A setting changed, as `Backend::set_setting` takes it.
    Setting { key: String, value: String },
}
