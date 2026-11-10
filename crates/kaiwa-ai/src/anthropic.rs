//! Anthropic Messages API (`POST /v1/messages`). The analysis is a structured output
//! (`output_config.format` with the analysis schema), so the answer's text is JSON that fits.
//!
//! Not tried against the live API in the session that wrote this (no key): the request shapes
//! follow the API reference, the tests here check them against a fake transport, and the
//! owner's local test (`kaiwa-sim run --live`) is the proof.

use serde_json::{Value, json};

use crate::transport::{Transport, UreqTransport};
use crate::{AiError, Analysis, ChatProvider, Role, TurnRequest, analysis_schema};

pub const DEFAULT_URL: &str = "https://api.anthropic.com/v1/messages";
pub const API_VERSION: &str = "2023-06-01";
/// The models think before they answer, and the thinking counts towards `max_tokens`: a
/// limit sized for the visible answer alone (a short line, a small JSON) cuts the answer off
/// or leaves none. Only tokens that are generated cost.
const MAX_TOKENS: u32 = 16_000;
/// Stands in for the learner entering the scene, before the character's opening line.
const SCENE_BEGINS: &str = "（会話が始まります。）";

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

    /// The conversation as the Messages API takes it: it must begin with the user, and a
    /// scene begins with the character's opening line, so a line stands in for the learner
    /// coming in.
    fn messages(r: &TurnRequest) -> Value {
        let mut messages = Vec::new();
        if r.messages
            .first()
            .is_some_and(|m| m.role == Role::Assistant)
        {
            messages.push(json!({ "role": "user", "content": SCENE_BEGINS }));
        }
        messages.extend(r.messages.iter().map(|m| json!(m)));
        Value::Array(messages)
    }

    pub fn reply_body(&self, r: &TurnRequest) -> Value {
        json!({
            "model": self.reply_model,
            "max_tokens": r.max_tokens.max(MAX_TOKENS),
            "system": r.system,
            "messages": Self::messages(r),
        })
    }

    /// The analysis as structured output (`output_config.format`): the answer's text is JSON
    /// that fits the schema. A forced tool call did the same once; the current models reject
    /// `tool_choice` of type `tool`.
    pub fn analysis_body(&self, r: &TurnRequest) -> Value {
        json!({
            "model": self.analysis_model,
            "max_tokens": r.max_tokens.max(MAX_TOKENS),
            "system": r.system,
            "messages": Self::messages(r),
            "output_config": {
                "format": { "type": "json_schema", "schema": analysis_schema() },
            },
        })
    }
}

/// Why an answer is no answer: it was cut off at the token limit, or declined.
fn stopped(answer: &Value) -> Result<(), AiError> {
    match answer["stop_reason"].as_str() {
        Some("max_tokens") => Err(AiError::Format(
            "the answer was cut off at the token limit".into(),
        )),
        Some("refusal") => Err(AiError::Format("the tutor declined to answer".into())),
        _ => Ok(()),
    }
}

/// The text blocks of a Messages API answer, joined.
pub fn parse_reply(answer: &Value) -> Result<String, AiError> {
    stopped(answer)?;
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

/// The analysis: the JSON in the answer's text.
pub fn parse_analysis(answer: &Value) -> Result<Analysis, AiError> {
    Analysis::from_text(&parse_reply(answer)?)
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
            body["messages"][2],
            json!({ "role": "user", "content": "コーヒーを好きです。" })
        );
    }

    #[test]
    fn analysis_is_a_structured_output() {
        let p = provider(json!({
            "content": [
                { "type": "thinking", "thinking": "" },
                { "type": "text", "text": r#"{"understood":true,"goals_met":[],"new_words":[],"reading":"",
                    "corrections":[{"severity":"wrong","kind":"particle","original":"コーヒーを好きです",
                    "better":"コーヒーが好きです","explanation":"好き verlangt が.","also_possible":false}]}"# }
            ],
            "stop_reason": "end_turn"
        }));
        let a = p.analyse(&request()).unwrap();
        assert_eq!(a.corrections[0].severity, Severity::Wrong);
        let sent = p.transport.requests.lock().unwrap();
        let body = &sent[0].2;
        assert_eq!(body["model"], "strong");
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(
            body["output_config"]["format"]["schema"]["required"][0],
            "understood"
        );
        // No forced tool call: the current models answer it with a 400.
        assert!(body.get("tool_choice").is_none() && body.get("tools").is_none());
    }

    #[test]
    fn the_conversation_begins_with_the_user_and_has_room_to_think() {
        let p = provider(json!({ "content": [{ "type": "text", "text": "はい。" }] }));
        p.reply(&request()).unwrap();
        let sent = p.transport.requests.lock().unwrap();
        let body = &sent[0].2;
        // The scene's opening is the character's: a line of the user's goes before it.
        let roles: Vec<&str> = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|m| m["role"].as_str())
            .collect();
        assert_eq!(roles, ["user", "assistant", "user"]);
        assert_eq!(body["messages"][2]["content"], "コーヒーを好きです。");
        // The request asked for 400 tokens; thinking needs more than the visible answer.
        assert_eq!(body["max_tokens"], MAX_TOKENS);
        // A conversation that begins with the user stays as it is.
        let mut r = request();
        r.messages.remove(0);
        assert_eq!(
            AnthropicProvider::<FakeTransport>::messages(&r)
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn an_answer_cut_off_or_declined_is_an_error() {
        for reason in ["max_tokens", "refusal"] {
            let answer = json!({
                "content": [{ "type": "text", "text": "コーヒーが{好|す" }],
                "stop_reason": reason
            });
            assert!(parse_reply(&answer).is_err(), "{reason}");
            assert!(parse_analysis(&answer).is_err(), "{reason}");
        }
    }

    #[test]
    fn answers_without_the_expected_parts_are_errors() {
        assert!(parse_reply(&json!({ "content": [] })).is_err());
        assert!(parse_analysis(&json!({ "content": [{ "type": "text", "text": "{}" }] })).is_err());
        assert!(parse_analysis(&json!({ "error": { "type": "overloaded_error" } })).is_err());
    }
}
