//! What goes to a provider and what comes back.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// The character (the tutor's role in the scenario).
    Assistant,
    /// The learner.
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// One call: the instructions (role, level, scenario, what to analyse) and the conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRequest {
    pub system: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
}

/// Severity of a correction (Lastenheft /LF302/). Ordered from mild to severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Style,
    Unnatural,
    Wrong,
    BlocksUnderstanding,
}

/// What kind of mistake (same names as the personas' `language_errors[].type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionKind {
    Particle,
    Conjugation,
    WordChoice,
    Register,
    WordOrder,
    Counter,
    Omission,
    Loanword,
    Tense,
    Pronunciation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Correction {
    pub severity: Severity,
    pub kind: CorrectionKind,
    /// The learner's words, as heard.
    pub original: String,
    /// The better form.
    pub better: String,
    /// Short explanation in the UI language, with the Japanese example (/LL112/).
    pub explanation: String,
    /// Only another way to say it, not a mistake (/LF303/: shown as "Auch möglich").
    #[serde(default)]
    pub also_possible: bool,
}

/// The analysis of one learner turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Analysis {
    /// The character understood what the learner meant.
    pub understood: bool,
    /// Ids of scenario goals this turn reached.
    #[serde(default)]
    pub goals_met: Vec<String>,
    #[serde(default)]
    pub corrections: Vec<Correction>,
    /// Words the learner used or met for the first time (dictionary form).
    #[serde(default)]
    pub new_words: Vec<String>,
    /// The learner's turn with the readings of its kanji as furigana markup (`{漢字|かな}`),
    /// read in context; the dictionary's readings give way to it (ADR-004).
    #[serde(default)]
    pub reading: String,
}
