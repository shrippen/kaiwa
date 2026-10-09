//! Anthropic Messages API (`POST /v1/messages`). The analysis uses a forced tool call whose
//! input schema is the analysis schema, so the answer is structured.

use serde_json::{Value, json};

use crate::transport::{Transport, UreqTransport};
use crate::{AiError, Analysis, ChatProvider, TurnRequest, analysis_schema};

pub const DEFAULT_URL: &str = "https://api.anthropic.com/v1/messages";
pub const API_VERSION: &str = "2023-06-01";
const TOOL: &str = "report_analysis";

pub struct AnthropicProvider<T: Transport = UreqTransport> {
    pub url: String,
    pub api_key: String,
    /// Fast model for the character's reply.
    pub reply_model: String,
    /// Stronger model for the analysis.
    pub analysis_model: String,
    pub transport: T,
}

impl AnthropicProvider<UreqTransport> {
    /// The preset of ADR-004: a fast model answers, a stronger one analyses.
    pub fn preset(api_key: impl Into<String>) -> Self {
        Self {
            url: DEFAULT_URL.into(),
            api_key: api_key.into(),
            reply_model: "claude-haiku-5-5".into(),
            analysis_model: "claude-sonnet-5-5".into(),
            transport: UreqTransport::default(),
        }
    }
}

impl<T: Transport> AnthropicProvider<T> {
    fn headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("x-api-key", self.api_key.clone()),
            ("anthropic-version", API_VERSION.into()),
        ]
    }

    pub fn reply_body(&self, r: &TurnRequest) -> Value {
        json!({
            "model": self.reply_model,
            "max_tokens": r.max_tokens,
            "system": r.system,
            "messages": r.messages,
        })
    }

    pub fn analysis_body(&self, r: &TurnRequest) -> Value {
        json!({
            "model": self.analysis_model,
            "max_tokens": r.max_tokens,
            "system": r.system,
            "messages": r.messages,
            "tools": [{
                "name": TOOL,
                "description": "Report the analysis of the learner's last turn.",
                "input_schema": analysis_schema(),
            }],
            "tool_choice": { "type": "tool", "name": TOOL },
        })
    }
}

/// The text blocks of a Messages API answer, joined.
pub fn parse_reply(answer: &Value) -> Result<String, AiError> {
    let blocks = answer["content"]
        .as_array()
        .ok_or_else(|| AiError::Format("no content".into()))?;
    let text: String = blocks
        .iter()
        .filter(|b| b["type"] == "text")
        .filter_map(|b| b["text"].as_str())
        .collect();
    if text.trim().is_empty() {
        return Err(AiError::Format("no text in the answer".into()));
    }
    Ok(text.trim().to_string())
}

/// The input of the forced tool call.
pub fn parse_analysis(answer: &Value) -> Result<Analysis, AiError> {
    let blocks = answer["content"]
        .as_array()
        .ok_or_else(|| AiError::Format("no content".into()))?;
    let input = blocks
        .iter()
        .find(|b| b["type"] == "tool_use" && b["name"] == TOOL)
        .map(|b| b["input"].clone())
        .ok_or_else(|| AiError::Format("no analysis tool call".into()))?;
    serde_json::from_value(input).map_err(|e| AiError::Format(e.to_string()))
}

impl<T: Transport> ChatProvider for AnthropicProvider<T> {
    fn reply(&self, request: &TurnRequest) -> Result<String, AiError> {
        parse_reply(&self.transport.post_json(
            &self.url,
            &self.headers(),
            &self.reply_body(request),
        )?)
    }
    fn analyse(&self, request: &TurnRequest) -> Result<Analysis, AiError> {
        parse_analysis(&self.transport.post_json(
            &self.url,
            &self.headers(),
            &self.analysis_body(request),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::FakeTransport;
    use crate::{ChatMessage, Severity};

    fn provider(answer: Value) -> AnthropicProvider<FakeTransport> {
        AnthropicProvider {
            url: DEFAULT_URL.into(),
            api_key: "test-key".into(),
            reply_model: "fast".into(),
            analysis_model: "strong".into(),
            transport: FakeTransport::new(answer),
        }
    }

    fn request() -> TurnRequest {
        TurnRequest {
            system: "Du bist Kassiererin in einer Konbini.".into(),
            messages: vec![
                ChatMessage::assistant("いらっしゃいませ。"),
                ChatMessage::user("コーヒーを好きです。"),
            ],
            max_tokens: 400,
        }
    }

    #[test]
    fn reply_request_and_answer() {
        let p = provider(json!({
            "id": "msg_01", "type": "message", "role": "assistant", "model": "fast",
            "content": [{ "type": "text", "text": "コーヒーが好きなんですね。" }],
            "stop_reason": "end_turn"
        }));
        assert_eq!(p.reply(&request()).unwrap(), "コーヒーが好きなんですね。");
        let sent = p.transport.requests.lock().unwrap();
        let (url, headers, body) = &sent[0];
        assert_eq!(url, DEFAULT_URL);
        assert!(headers.contains(&("x-api-key".into(), "test-key".into())));
        assert!(headers.contains(&("anthropic-version".into(), API_VERSION.into())));
        assert_eq!(body["model"], "fast");
        assert_eq!(body["system"], "Du bist Kassiererin in einer Konbini.");
        assert_eq!(
            body["messages"][1],
            json!({ "role": "user", "content": "コーヒーを好きです。" })
        );
    }

    #[test]
    fn analysis_is_a_forced_tool_call() {
        let p = provider(json!({
            "content": [
                { "type": "text", "text": "…" },
                { "type": "tool_use", "id": "toolu_01", "name": TOOL, "input": {
                    "understood": true, "goals_met": [], "new_words": [],
                    "corrections": [{ "severity": "wrong", "kind": "particle", "original": "コーヒーを好きです",
                        "better": "コーヒーが好きです", "explanation": "好き verlangt が.", "also_possible": false }]
                } }
            ],
            "stop_reason": "tool_use"
        }));
        let a = p.analyse(&request()).unwrap();
        assert_eq!(a.corrections[0].severity, Severity::Wrong);
        let sent = p.transport.requests.lock().unwrap();
        let body = &sent[0].2;
        assert_eq!(body["model"], "strong");
        assert_eq!(body["tool_choice"], json!({ "type": "tool", "name": TOOL }));
        assert_eq!(
            body["tools"][0]["input_schema"]["required"][0],
            "understood"
        );
    }

    #[test]
    fn answers_without_the_expected_parts_are_errors() {
        assert!(parse_reply(&json!({ "content": [] })).is_err());
        assert!(parse_analysis(&json!({ "content": [{ "type": "text", "text": "{}" }] })).is_err());
        assert!(parse_analysis(&json!({ "error": { "type": "overloaded_error" } })).is_err());
    }
}
