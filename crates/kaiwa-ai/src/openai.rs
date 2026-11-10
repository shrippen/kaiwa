//! OpenAI-compatible chat completions (`POST {base}/chat/completions`): llama.cpp server,
//! Ollama, LM Studio and cloud services with that interface. The analysis asks for
//! `response_format: json_schema`; servers that ignore it still have to answer JSON,
//! which is then checked against the types.

use serde_json::{Value, json};

use crate::transport::{Transport, UreqTransport};
use crate::{AiError, Analysis, ChatProvider, TurnRequest, analysis_schema};

/// Models that reason before they answer (qwen3, Gemini) do it inside `max_tokens`: a limit
/// sized for the visible answer alone cuts the answer off. Only generated tokens cost.
const MAX_TOKENS: u32 = 4_096;

pub struct OpenAiCompatible<T: Transport = UreqTransport> {
    /// e.g. `http://localhost:11434/v1` (Ollama) or `http://localhost:8080/v1` (llama.cpp).
    pub base_url: String,
    /// Empty for local servers without a key.
    pub api_key: String,
    pub reply_model: String,
    pub analysis_model: String,
    pub transport: T,
}

impl OpenAiCompatible<UreqTransport> {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        let model = model.into();
        Self {
            base_url: base_url.into(),
            api_key: String::new(),
            reply_model: model.clone(),
            analysis_model: model,
            transport: UreqTransport::default(),
        }
    }
}

impl<T: Transport> OpenAiCompatible<T> {
    fn url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    fn headers(&self) -> Vec<(&'static str, String)> {
        if self.api_key.is_empty() {
            vec![]
        } else {
            vec![("authorization", format!("Bearer {}", self.api_key))]
        }
    }

    fn messages(r: &TurnRequest) -> Value {
        let mut m = vec![json!({ "role": "system", "content": r.system })];
        m.extend(
            r.messages
                .iter()
                .map(|x| json!({ "role": x.role, "content": x.content })),
        );
        Value::Array(m)
    }

    pub fn reply_body(&self, r: &TurnRequest) -> Value {
        json!({ "model": self.reply_model, "max_tokens": r.max_tokens.max(MAX_TOKENS), "messages": Self::messages(r) })
    }

    pub fn analysis_body(&self, r: &TurnRequest) -> Value {
        json!({
            "model": self.analysis_model,
            "max_tokens": r.max_tokens.max(MAX_TOKENS),
            "messages": Self::messages(r),
            "response_format": {
                "type": "json_schema",
                "json_schema": { "name": "analysis", "strict": true, "schema": analysis_schema() },
            },
        })
    }
}

fn content(answer: &Value) -> Result<&str, AiError> {
    // Cut off at the token limit: half a line, or half a JSON, is no answer.
    if answer["choices"][0]["finish_reason"] == "length" {
        return Err(AiError::Format(
            "the answer was cut off at the token limit".into(),
        ));
    }
    answer["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| AiError::Format("no choices[0].message.content".into()))
}

pub fn parse_reply(answer: &Value) -> Result<String, AiError> {
    let text = content(answer)?.trim();
    if text.is_empty() {
        return Err(AiError::Format("empty reply".into()));
    }
    Ok(text.to_string())
}

/// The JSON in the message content, read tolerantly (`Analysis::from_text`): servers that
/// ignore `response_format` and small local models wrap it or stray from the schema.
pub fn parse_analysis(answer: &Value) -> Result<Analysis, AiError> {
    Analysis::from_text(content(answer)?)
}

impl<T: Transport> ChatProvider for OpenAiCompatible<T> {
    fn reply(&self, request: &TurnRequest) -> Result<String, AiError> {
        parse_reply(&self.transport.post_json(
            &self.url(),
            &self.headers(),
            &self.reply_body(request),
        )?)
    }
    fn analyse(&self, request: &TurnRequest) -> Result<Analysis, AiError> {
        parse_analysis(&self.transport.post_json(
            &self.url(),
            &self.headers(),
            &self.analysis_body(request),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChatMessage;
    use crate::transport::FakeTransport;

    fn provider(answer: Value, key: &str) -> OpenAiCompatible<FakeTransport> {
        OpenAiCompatible {
            base_url: "http://localhost:11434/v1/".into(),
            api_key: key.into(),
            reply_model: "qwen".into(),
            analysis_model: "qwen".into(),
            transport: FakeTransport::new(answer),
        }
    }

    fn request() -> TurnRequest {
        TurnRequest {
            system: "Rolle".into(),
            messages: vec![ChatMessage::user("こんにちは")],
            max_tokens: 200,
        }
    }

    fn answer(content: &str) -> Value {
        json!({ "id": "chatcmpl-1", "object": "chat.completion",
                "choices": [{ "index": 0, "finish_reason": "stop", "message": { "role": "assistant", "content": content } }] })
    }

    #[test]
    fn reply_puts_the_system_prompt_first() {
        let p = provider(answer("こんにちは！"), "");
        assert_eq!(p.reply(&request()).unwrap(), "こんにちは！");
        let sent = p.transport.requests.lock().unwrap();
        let (url, headers, body) = &sent[0];
        assert_eq!(url, "http://localhost:11434/v1/chat/completions");
        assert!(headers.is_empty(), "local servers get no key");
        assert_eq!(
            body["messages"][0],
            json!({ "role": "system", "content": "Rolle" })
        );
        assert_eq!(
            body["messages"][1],
            json!({ "role": "user", "content": "こんにちは" })
        );
    }

    #[test]
    fn analysis_asks_for_the_schema_and_tolerates_fences() {
        let json_text = r#"```json
{"understood": true, "goals_met": ["greet"], "corrections": [], "new_words": []}
```"#;
        let p = provider(answer(json_text), "sk-test");
        let a = p.analyse(&request()).unwrap();
        assert_eq!(a.goals_met, vec!["greet".to_string()]);
        let sent = p.transport.requests.lock().unwrap();
        assert!(
            sent[0]
                .1
                .contains(&("authorization".into(), "Bearer sk-test".into()))
        );
        assert_eq!(sent[0].2["response_format"]["json_schema"]["strict"], true);
    }

    #[test]
    fn invalid_analysis_is_an_error() {
        assert!(parse_analysis(&answer("Gut gemacht!")).is_err());
        assert!(parse_analysis(&answer(r#"{"understood": "ja"}"#)).is_err());
        assert!(parse_reply(&json!({ "choices": [] })).is_err());
    }
}
