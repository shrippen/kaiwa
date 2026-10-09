//! Scenarios (/LF115/, /LF117/): a role, a register, an opening line, goals the learner has
//! to reach, and help in three steps per goal (/LF114/). Stored as JSON in `data/scenarios/`.

use serde::{Deserialize, Serialize};

/// Text in the UI languages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Text {
    pub de: String,
    pub en: String,
}

impl Text {
    pub fn get(&self, lang: &str) -> &str {
        if lang == "de" { &self.de } else { &self.en }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Register {
    Casual,
    Polite,
    Keigo,
}

/// Help for one goal, from a nudge to a full sentence to say after the character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Help {
    /// Step 1: a keyword (Japanese, with reading).
    pub keyword: String,
    /// Step 2: the start of a sentence.
    pub start: String,
    /// Step 3: a full example sentence.
    pub example: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub text: Text,
    pub help: Help,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub unit: String,
    pub title: Text,
    /// The character: who they are and how they speak (goes into the tutor's instructions).
    pub character: Text,
    pub register: Register,
    /// The character's first line (furigana markup as in KanteRubyText).
    pub opening: String,
    pub goals: Vec<Goal>,
}

impl Scenario {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn goal(&self, id: &str) -> Option<&Goal> {
        self.goals.iter().find(|g| g.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_scenarios_load() {
        let s = Scenario::from_json(include_str!(
            "../../../data/scenarios/n5-02-jikoshoukai.json"
        ))
        .unwrap();
        assert_eq!(s.unit, "N5-02");
        assert_eq!(s.register, Register::Polite);
        assert!(s.goals.len() >= 3);
        for g in &s.goals {
            assert!(
                !g.help.example.is_empty(),
                "goal {} has a full example",
                g.id
            );
        }
    }
}
