//! A provider that replays recorded answers: tests, `kaiwa-sim` in CI, and the app's demo
//! mode. Until the real providers are tried locally (ADR-004, spike 0.5 moved to the owner's
//! local test), every engine test runs on these fixtures.
//!
//! Fixture file (JSON): a list of turns, each the learner's words and what to answer.
//!
//! ```json
//! [{ "learner": "コーヒーを好きです。", "reply": "コーヒーが好きなんですね。",
//!    "analysis": { "understood": true, "goals_met": [], "corrections": [ … ], "new_words": [] } }]
//! ```
//!
//! The learner's words are matched without punctuation and spaces. A turn without a fixture
//! answers [`AiError::NoFixture`], unless a fallback is set.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{AiError, Analysis, ChatProvider, Role, TurnRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub learner: String,
    pub reply: String,
    pub analysis: Analysis,
}

#[derive(Debug, Default)]
pub struct MockProvider {
    turns: HashMap<String, Fixture>,
    /// Answer for turns without a fixture (the character does not understand).
    pub fallback: Option<Fixture>,
    /// The character's first line, when the conversation has no learner turn yet.
    pub opening: Option<String>,
}

/// Matching key: without punctuation and white space.
pub fn key(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && !"、。，．,.!?！？「」『』…・".contains(*c))
        .collect()
}

impl MockProvider {
    pub fn new(fixtures: impl IntoIterator<Item = Fixture>) -> Self {
        Self {
            turns: fixtures.into_iter().map(|f| (key(&f.learner), f)).collect(),
            ..Default::default()
        }
    }

    pub fn from_json(json: &str) -> Result<Self, AiError> {
        let fixtures: Vec<Fixture> =
            serde_json::from_str(json).map_err(|e| AiError::Format(e.to_string()))?;
        Ok(Self::new(fixtures))
    }

    pub fn with_fallback(mut self, reply: impl Into<String>) -> Self {
        self.fallback = Some(Fixture {
            learner: String::new(),
            reply: reply.into(),
            analysis: Analysis {
                understood: false,
                goals_met: vec![],
                corrections: vec![],
                new_words: vec![],
            },
        });
        self
    }

    pub fn with_opening(mut self, line: impl Into<String>) -> Self {
        self.opening = Some(line.into());
        self
    }

    fn lookup(&self, request: &TurnRequest) -> Result<&Fixture, AiError> {
        let last = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .ok_or(AiError::NoFixture)?;
        self.turns
            .get(&key(&last.content))
            .or(self.fallback.as_ref())
            .ok_or(AiError::NoFixture)
    }
}

impl ChatProvider for MockProvider {
    fn reply(&self, request: &TurnRequest) -> Result<String, AiError> {
        if !request.messages.iter().any(|m| m.role == Role::User) {
            return self.opening.clone().ok_or(AiError::NoFixture);
        }
        Ok(self.lookup(request)?.reply.clone())
    }
    fn analyse(&self, request: &TurnRequest) -> Result<Analysis, AiError> {
        Ok(self.lookup(request)?.analysis.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChatMessage;

    const FIXTURES: &str = r#"[
      { "learner": "コーヒーを好きです。", "reply": "コーヒーが好きなんですね。",
        "analysis": { "understood": true, "goals_met": ["likes"], "new_words": [],
          "corrections": [{ "severity": "wrong", "kind": "particle", "original": "コーヒーを好きです",
            "better": "コーヒーが好きです", "explanation": "好き verlangt が.", "also_possible": false }] } }
    ]"#;

    fn turn(text: &str) -> TurnRequest {
        TurnRequest {
            system: String::new(),
            messages: vec![ChatMessage::user(text)],
            max_tokens: 100,
        }
    }

    #[test]
    fn replays_by_the_learners_words_without_punctuation() {
        let m = MockProvider::from_json(FIXTURES).unwrap();
        assert_eq!(
            m.reply(&turn("コーヒー を 好きです")).unwrap(),
            "コーヒーが好きなんですね。"
        );
        assert_eq!(
            m.analyse(&turn("コーヒーを好きです！")).unwrap().goals_met,
            vec!["likes".to_string()]
        );
    }

    #[test]
    fn unknown_turns_need_a_fallback() {
        let m = MockProvider::from_json(FIXTURES).unwrap();
        assert!(matches!(m.reply(&turn("はい")), Err(AiError::NoFixture)));
        let m = m.with_fallback("すみません、もう一度お願いします。");
        assert!(!m.analyse(&turn("はい")).unwrap().understood);
    }

    #[test]
    fn opening_line_before_the_first_learner_turn() {
        let m = MockProvider::default().with_opening("いらっしゃいませ。");
        let empty = TurnRequest {
            system: String::new(),
            messages: vec![],
            max_tokens: 100,
        };
        assert_eq!(m.reply(&empty).unwrap(), "いらっしゃいませ。");
    }
}
