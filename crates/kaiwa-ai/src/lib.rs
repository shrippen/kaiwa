//! Chat providers for Kaiwa's tutor (ADR-004).
//!
//! A turn needs two calls: the character's **reply** (fast, in role, at the learner's level)
//! and the **analysis** of the learner's turn (structured: corrections with severity, goals
//! reached, new words). [`ChatProvider`] hides who answers: an OpenAI-compatible server
//! (llama.cpp, Ollama, LM Studio, cloud services), Anthropic's Messages API, or the
//! [`mock::MockProvider`] that replays recorded answers for tests and `kaiwa-sim`.
//!
//! HTTP goes through [`transport::Transport`], so the request building and the response
//! parsing of both real providers are tested against recorded payloads without a network.

pub mod anthropic;
pub mod mock;
pub mod openai;
pub mod transport;
pub mod types;

pub use types::{Analysis, ChatMessage, Correction, CorrectionKind, Role, Severity, TurnRequest};

/// Errors a provider can report. The engine shows them in words and never invents an answer.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("network: {0}")]
    Network(String),
    #[error("the provider answered with status {status}: {body}")]
    Status { status: u16, body: String },
    #[error("unexpected answer: {0}")]
    Format(String),
    #[error("no recorded answer for this turn")]
    NoFixture,
}

/// Who answers the tutor's two calls.
pub trait ChatProvider: Send + Sync {
    /// The character's next utterance, in Japanese, plain text.
    fn reply(&self, request: &TurnRequest) -> Result<String, AiError>;
    /// The analysis of the learner's last turn in `request.messages`.
    fn analyse(&self, request: &TurnRequest) -> Result<Analysis, AiError>;
}

/// The JSON schema an analysis must match (structured output for both providers).
pub fn analysis_schema() -> serde_json::Value {
    serde_json::from_str(include_str!("../schemas/analysis.json"))
        .expect("bundled schema is valid JSON")
}

/// The tutor named by the environment, for the owner's local test (ADR-004); `None` when
/// none is configured.
///
///   KAIWA_PROVIDER=anthropic  ANTHROPIC_API_KEY=…
///   KAIWA_PROVIDER=openai     KAIWA_MODEL=…  [KAIWA_OPENAI_BASE=http://localhost:11434/v1]  [KAIWA_API_KEY=…]
pub fn provider_from_env() -> Result<Option<std::sync::Arc<dyn ChatProvider>>, String> {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    match env("KAIWA_PROVIDER").as_deref() {
        None => Ok(None),
        Some("anthropic") => {
            let key = env("ANTHROPIC_API_KEY")
                .ok_or("KAIWA_PROVIDER=anthropic needs ANTHROPIC_API_KEY")?;
            Ok(Some(std::sync::Arc::new(
                anthropic::AnthropicProvider::preset(key),
            )))
        }
        Some("openai") => {
            let model = env("KAIWA_MODEL").ok_or("KAIWA_PROVIDER=openai needs KAIWA_MODEL")?;
            let base =
                env("KAIWA_OPENAI_BASE").unwrap_or_else(|| "http://localhost:11434/v1".into());
            let mut p = openai::OpenAiCompatible::new(base, model);
            p.api_key = env("KAIWA_API_KEY").unwrap_or_default();
            Ok(Some(std::sync::Arc::new(p)))
        }
        Some(other) => Err(format!(
            "KAIWA_PROVIDER={other}: expected anthropic or openai"
        )),
    }
}
