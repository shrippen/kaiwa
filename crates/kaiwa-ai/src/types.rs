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
    /// Not everything the tutor wrote could be read ([`Analysis::from_text`]): what fits is
    /// here, the rest is gone. The turn says so and does not count for the level (T-11).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub incomplete: bool,
}

impl Analysis {
    /// Reads an analysis from the model's text. What stands around the JSON object goes: a
    /// code fence, what the model thought aloud, a sentence before or after it (small local
    /// models and servers that do not enforce the schema write those). The object that says
    /// whether the learner was `understood` is the analysis; without that there is none. What
    /// else of it does not fit the schema is left out and the analysis marked [`incomplete`]
    /// (ADR-004, T-11): never quietly, a turn whose only correction was unreadable must not
    /// read as flawless.
    ///
    /// [`incomplete`]: Analysis::incomplete
    pub fn from_text(text: &str) -> Result<Self, crate::AiError> {
        const OPEN: &str = "<think>";
        const CLOSE: &str = "</think>";
        let format = |why: String| crate::AiError::Format(format!("{why}: {text}"));
        // The whole JSON objects outside of thinking, in order. Thinking may quote braces
        // ({漢字|かな}) or hold a draft of the analysis; neither is the answer. Inside an object
        // the tags are text (an explanation may name them).
        let mut objects = Vec::new();
        let mut at = 0;
        loop {
            let rest = &text[at..];
            let Some((i, mark)) = ["{", OPEN, CLOSE]
                .iter()
                .filter_map(|m| rest.find(m).map(|i| (i, *m)))
                .min()
            else {
                break;
            };
            match mark {
                OPEN => match rest[i..].find(CLOSE) {
                    Some(end) => at += i + end + CLOSE.len(),
                    // Never closed: the thinking was cut off, there is no answer behind it.
                    None => break,
                },
                // Closed without an opening (some chat templates put the opening into the
                // prompt): all before it was thinking.
                CLOSE => {
                    objects.clear();
                    at += i + CLOSE.len();
                }
                _ => {
                    let mut values = serde_json::Deserializer::from_str(&rest[i..])
                        .into_iter::<serde_json::Value>();
                    match values.next() {
                        Some(Ok(v)) if v.is_object() => {
                            objects.push(v);
                            at += i + values.byte_offset();
                        }
                        _ => at += i + 1,
                    }
                }
            }
        }
        // The first object that says `understood` is the analysis; a sentence before it may
        // hold an object of its own ("The format is {} as asked.").
        match objects.into_iter().find(|v| v.get("understood").is_some()) {
            Some(v) => Self::repaired(v).map_err(format),
            None => Err(format("no JSON object that says `understood`".into())),
        }
    }

    /// The analysis in `object`, field by field: what fits the schema is taken, what does not
    /// (a field of the wrong type, a correction of an unknown kind, a field the schema does not
    /// know, no `corrections` at all) is left out and makes the analysis incomplete.
    fn repaired(object: serde_json::Value) -> Result<Self, String> {
        use serde_json::Value;
        let Value::Object(mut fields) = object else {
            return Err("no JSON object".into());
        };
        let Some(Value::Bool(understood)) = fields.remove("understood") else {
            return Err("`understood` is not true or false".into());
        };
        let mut incomplete = false;
        // The strings of a list; anything else in it, or no list, is a gap.
        let mut strings = |field: Option<Value>| -> Vec<String> {
            let items = match field {
                None => return Vec::new(),
                Some(Value::Array(items)) => items,
                Some(_) => {
                    incomplete = true;
                    return Vec::new();
                }
            };
            let n = items.len();
            let read: Vec<String> = items
                .into_iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            incomplete |= read.len() < n;
            read
        };
        let goals_met = strings(fields.remove("goals_met"));
        let new_words = strings(fields.remove("new_words"));
        let corrections = match fields.remove("corrections") {
            Some(Value::Array(items)) => {
                let n = items.len();
                let read: Vec<Correction> = items
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                incomplete |= read.len() < n;
                read
            }
            // No corrections named is not "no mistakes" (a server without the schema that
            // wrote `correction`).
            _ => {
                incomplete = true;
                Vec::new()
            }
        };
        let reading = match fields.remove("reading") {
            None => String::new(),
            Some(Value::String(s)) => s,
            Some(_) => {
                incomplete = true;
                String::new()
            }
        };
        // What the schema does not know may be a field under another name.
        incomplete |= !fields.is_empty();
        Ok(Self {
            understood,
            goals_met,
            corrections,
            new_words,
            reading,
            incomplete,
        })
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
    fn a_draft_in_thinking_is_not_the_analysis() {
        let wrong = r#"{"understood":true,"corrections":[{"severity":"wrong","kind":"particle","original":"a","better":"b","explanation":"c"}]}"#;
        let draft = r#"Draft: {"understood":true,"corrections":[]} wait, particle error."#;
        for text in [
            format!("<think>{draft}</think>\n{wrong}"),
            // Some chat templates put the opening into the prompt: only the closing arrives.
            format!("{draft}</think>\n{wrong}"),
            format!("The learner said it.</think>\n{wrong}"),
            format!("<think>one</think> then <think>{draft}</think>{wrong}"),
        ] {
            let a = Analysis::from_text(&text).unwrap();
            assert_eq!(a.corrections.len(), 1, "{text}");
        }
        // The tags inside the analysis are text.
        let named = r#"{"understood":true,"corrections":[{"severity":"wrong","kind":"particle","original":"a","better":"b","explanation":"</think> und <think> sind keine Wörter"}]}"#;
        let a = Analysis::from_text(named).unwrap();
        assert_eq!(
            a.corrections[0].explanation,
            "</think> und <think> sind keine Wörter"
        );
    }

    #[test]
    fn what_does_not_fit_the_schema_is_left_out_and_said() {
        // T-11: repaired, but never quietly. The word <think> in an explanation is no block.
        let explained = r#"{"understood":true,"corrections":[{"severity":"wrong","kind":"particle","original":"a","better":"b","explanation":"Schreibe <think> nie"}]}"#;
        let a = Analysis::from_text(explained).unwrap();
        assert!(a.corrections.len() == 1 && !a.incomplete);
        let good = r#"{"severity":"wrong","kind":"particle","original":"a","better":"b","explanation":"c"}"#;
        let bad = r#"{"severity":"wrong","kind":"grammar","original":"a","better":"b","explanation":"c"}"#;
        for (text, corrections, goals) in [
            // The only correction is unreadable: no correction, and not flawless.
            (
                format!(r#"{{"understood":true,"corrections":[{bad}]}}"#),
                0,
                0,
            ),
            // One of two: the readable one stays.
            (
                format!(r#"{{"understood":true,"corrections":[{bad},{good}]}}"#),
                1,
                0,
            ),
            (
                r#"{"understood":true,"corrections":{"severity":"wrong"}}"#.into(),
                0,
                0,
            ),
            (r#"{"understood":true,"corrections":null}"#.into(), 0, 0),
            // No corrections named, or under another name, is not "no mistakes".
            (r#"{"understood":true}"#.into(), 0, 0),
            (
                format!(r#"{{"understood":true,"correction":[{good}]}}"#),
                0,
                0,
            ),
            (
                r#"{"understood":true,"goals_met":"name","corrections":[]}"#.into(),
                0,
                0,
            ),
            (
                r#"{"understood":true,"goals_met":["name",3],"corrections":[]}"#.into(),
                0,
                1,
            ),
            (
                r#"{"understood":true,"corrections":[],"reading":null}"#.into(),
                0,
                0,
            ),
            (
                r#"{"understood":true,"corrections":[],"note":"all fine"}"#.into(),
                0,
                0,
            ),
        ] {
            let a = Analysis::from_text(&text).unwrap();
            assert!(a.incomplete && a.understood, "{text}");
            assert_eq!(
                (a.corrections.len(), a.goals_met.len()),
                (corrections, goals),
                "{text}"
            );
        }
        // Goals, new words and the reading may be left out, as before: no gap.
        let a = Analysis::from_text(r#"{"understood":false,"corrections":[]}"#).unwrap();
        assert!(!a.incomplete && !a.understood);
    }

    #[test]
    fn without_understood_there_is_no_analysis() {
        for bad in [
            r#"{"understood":"ja"}"#,
            r#"{"understood":null,"corrections":[]}"#,
            "{}",
            "I cannot analyse this.",
            // A draft in thinking that was cut off, also behind a sentence with braces.
            r#"The format is {} as asked. <think>Draft: {"understood":true,"corrections":[]} wait, there is a particle error, let me"#,
            // Thinking that never closes has no answer behind it.
            r#"<think>The learner said the name. {"understood":true,"corrections":[]}"#,
            r#"<think>Draft: {"understood":true,"corrections":[]} wait, there is a particle error, let me"#,
            r#"{"understood":true,"corrections":["#,
        ] {
            assert!(Analysis::from_text(bad).is_err(), "{bad}");
        }
        // The answer is the first object that says `understood`; a second one behind it does
        // not stand in for it.
        let two = r#"{"understood":true,"corrections":[{"severity":"wrong","kind":"grammar","original":"a","better":"b","explanation":"c"}]} Note {"understood":true,"corrections":[]}"#;
        assert!(Analysis::from_text(two).unwrap().incomplete);
    }
}
