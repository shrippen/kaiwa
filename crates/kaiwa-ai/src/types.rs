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
    /// Reads an analysis from the model's text. What stands around the JSON object goes: a
    /// code fence, a `<think>` block, a sentence before or after it (small local models and
    /// servers that do not enforce the schema write those). The object itself must fit the
    /// schema; an answer that does not is discarded, not repaired (ADR-004).
    pub fn from_text(text: &str) -> Result<Self, crate::AiError> {
        let format = |why: String| crate::AiError::Format(format!("{why}: {text}"));
        // Without what the model thought aloud: it may quote braces ({漢字|かな}).
        let mut rest = text;
        let mut body = String::new();
        while let Some(a) = rest.find("<think>") {
            body.push_str(&rest[..a]);
            // A block that never closes is no block (the word may stand in an explanation).
            let Some(b) = rest[a..].find("</think>") else {
                break;
            };
            rest = &rest[a + b + "</think>".len()..];
        }
        body.push_str(rest);
        // The first whole JSON object that is an analysis; a sentence before it may hold
        // braces or an object of its own ("The format is {} as asked.").
        let mut why = None;
        let mut from = 0;
        while let Some(at) = body[from..].find('{').map(|i| from + i) {
            let mut values =
                serde_json::Deserializer::from_str(&body[at..]).into_iter::<serde_json::Value>();
            from = at + 1;
            if let Some(Ok(v)) = values.next()
                && v.is_object()
            {
                match serde_json::from_value(v) {
                    Ok(a) => return Ok(a),
                    Err(e) => why = why.or(Some(e.to_string())),
                }
                // What stands inside an object that is no analysis is not one either.
                from = at + values.byte_offset();
            }
        }
        Err(format(
            why.unwrap_or_else(|| "no complete JSON object".into()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: &str =
        r#"{"understood":true,"goals_met":["name"],"corrections":[],"new_words":[],"reading":""}"#;

    #[test]
    fn an_analysis_reads_through_what_models_put_around_it() {
        let a = Analysis::from_text(PLAIN).unwrap();
        assert!(a.understood && a.goals_met == ["name"]);
        for wrapped in [
            format!("```json\n{PLAIN}\n```"),
            format!("<think>The learner said their name.</think>\n{PLAIN}"),
            format!("<think>The reading is written as {{漢字|かな}}.</think>\n{PLAIN}"),
            format!("Here is the analysis:\n```\n{PLAIN}\n```\nLet me know if you need more."),
            format!("The format is {{}} as asked.\n{PLAIN}"),
            format!("Example: {{\"a\":1}}\n{PLAIN}"),
        ] {
            assert_eq!(Analysis::from_text(&wrapped).unwrap(), a, "{wrapped}");
        }
    }

    #[test]
    fn an_analysis_that_does_not_fit_the_schema_is_discarded() {
        // ADR-004: discarded, not repaired. A correction of an unknown kind is not dropped
        // quietly (the turn would read as flawless); the whole answer is an error.
        // The word <think> in an explanation is no block.
        let explained = r#"{"understood":true,"corrections":[{"severity":"wrong","kind":"particle","original":"a","better":"b","explanation":"Schreibe <think> nie"}]}"#;
        assert_eq!(Analysis::from_text(explained).unwrap().corrections.len(), 1);
        for bad in [
            r#"{"understood":true,"corrections":[{"severity":"wrong","kind":"grammar","original":"a","better":"b","explanation":"c"}]}"#,
            r#"{"understood":true,"corrections":{"severity":"wrong"}}"#,
            r#"{"understood":"ja"}"#,
            r#"{"understood":true,"goals_met":"name"}"#,
            "{}",
            "I cannot analyse this.",
            r#"{"understood":true,"corrections":["#,
        ] {
            assert!(Analysis::from_text(bad).is_err(), "{bad}");
        }
    }
}
