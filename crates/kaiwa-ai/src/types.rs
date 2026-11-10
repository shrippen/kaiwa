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

impl Analysis {
    /// Reads an analysis from the model's text. A server that enforces the schema gives exactly
    /// the schema; others (small local models, servers that ignore `response_format`) wrap the
    /// JSON in a code fence, a `<think>` block or a sentence, write `null` for an empty list,
    /// or name a kind of mistake the schema does not have. None of that should cost the learner
    /// the whole turn: the JSON is cut out, `null` counts as empty, and a correction that does
    /// not read is left out while the others stay.
    pub fn from_text(text: &str) -> Result<Self, crate::AiError> {
        let format = |why: String| crate::AiError::Format(format!("{why}: {text}"));
        // From the first brace to the last: whatever stands around the object goes.
        let json = match (text.find('{'), text.rfind('}')) {
            (Some(a), Some(b)) if a < b => &text[a..=b],
            _ => return Err(format("no JSON object".into())),
        };
        let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format(e.to_string()))?;
        let understood = v["understood"]
            .as_bool()
            .ok_or_else(|| format("no `understood`".into()))?;
        let strings = |key: &str| -> Vec<String> {
            v[key]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        };
        let corrections = v["corrections"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| serde_json::from_value::<Correction>(c.clone()).ok())
            .collect();
        Ok(Self {
            understood,
            goals_met: strings("goals_met"),
            corrections,
            new_words: strings("new_words"),
            reading: v["reading"].as_str().unwrap_or_default().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_analysis_reads_through_what_models_put_around_it() {
        let plain = r#"{"understood":true,"goals_met":["name"],"corrections":[],"new_words":[],"reading":""}"#;
        let a = Analysis::from_text(plain).unwrap();
        assert!(a.understood && a.goals_met == ["name"]);
        for wrapped in [
            format!("```json\n{plain}\n```"),
            format!("<think>The learner said their name.</think>\n{plain}"),
            format!("Here is the analysis:\n```\n{plain}\n```\nLet me know if you need more."),
        ] {
            assert_eq!(Analysis::from_text(&wrapped).unwrap(), a, "{wrapped}");
        }
        // `null` for an empty list or reading.
        let nulls = r#"{"understood":false,"goals_met":null,"corrections":null,"new_words":null,"reading":null}"#;
        let n = Analysis::from_text(nulls).unwrap();
        assert!(!n.understood && n.corrections.is_empty() && n.reading.is_empty());
    }

    #[test]
    fn one_correction_that_does_not_read_does_not_cost_the_others() {
        let text = r#"{"understood":true,"corrections":[
            {"severity":"wrong","kind":"particle","original":"コーヒーを好きです","better":"コーヒーが好きです","explanation":"好き verlangt が.","also_possible":false},
            {"severity":"minor","kind":"spelling","original":"a","better":"b","explanation":"c"},
            {"severity":"wrong","correction":"ビールを二つください"}]}"#;
        let a = Analysis::from_text(text).unwrap();
        assert_eq!(a.corrections.len(), 1);
        assert_eq!(a.corrections[0].kind, CorrectionKind::Particle);
        // No object, or one without the one field every analysis has, is an error.
        assert!(Analysis::from_text("I cannot analyse this.").is_err());
        assert!(Analysis::from_text("{}").is_err());
    }
}
