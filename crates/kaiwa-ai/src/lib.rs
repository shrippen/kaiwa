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
pub mod cost;
pub mod mock;
pub mod openai;
pub mod presets;
pub mod privacy;
pub mod secrets;
pub mod transport;
pub mod tutor;
pub mod types;

pub use tutor::{NoTutor, Tutor};
pub use types::{Analysis, ChatMessage, Correction, CorrectionKind, Role, Severity, TurnRequest};

/// Errors a provider can report. The engine shows them in words and never invents an answer.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum AiError {
    #[error("network: {0}")]
    Network(String),
    #[error("the provider answered with status {status}: {body}")]
    Status { status: u16, body: String },
    #[error("unexpected answer: {0}")]
    Format(String),
    #[error("no recorded answer for this turn")]
    NoFixture,
    /// The month's cost limit is reached (/LF714/).
    #[error("the cost limit for this month is reached")]
    Budget,
    /// No tutor is set up (Q-15): the settings have none and the environment names none.
    #[error("no tutor is set up")]
    NoTutor,
}

/// Who answers the tutor's two calls.
pub trait ChatProvider: Send + Sync {
    /// The character's next utterance, in Japanese; kanji may carry furigana markup
    /// (`{漢字|かんじ}`, see `prompts`), which the caller cleans with `kaiwa_core::clean_markup`.
    fn reply(&self, request: &TurnRequest) -> Result<String, AiError>;
    /// The analysis of the learner's last turn in `request.messages`.
    fn analyse(&self, request: &TurnRequest) -> Result<Analysis, AiError>;
    /// The character's `line` in the UI language `lang` ("de", "en") for the listening aid
    /// (/LF111/): a reply call with a translation prompt.
    fn translate(&self, line: &str, lang: &str) -> Result<String, AiError> {
        let language = if lang == "en" { "English" } else { "German" };
        let text = self.reply(&TurnRequest {
            system: format!(
                "Translate the Japanese line the user sends into {language}. Answer with the \
                 translation only: no Japanese, no romaji, no notes."
            ),
            messages: vec![ChatMessage::user(line)],
            max_tokens: 200,
        })?;
        Ok(text.trim().to_string())
    }
}

/// The JSON schema an analysis must match (structured output for both providers).
pub fn analysis_schema() -> serde_json::Value {
    serde_json::from_str(include_str!("../schemas/analysis.json"))
        .expect("bundled schema is valid JSON")
}

/// The tutor named by the environment, for the owner's local test (ADR-004); `None` when
/// none is configured. See [`provider_from`].
pub fn provider_from_env(
    profile: &str,
) -> Result<Option<std::sync::Arc<dyn ChatProvider>>, String> {
    provider_from(profile, |k| std::env::var(k).ok())
}

/// The tutor named by `env` (the settings with the environment on top, [`Tutor::lookup`]);
/// `None` when none is configured. `KAIWA_PROVIDER` is a preset id ([`presets::PRESETS`]):
///
///   KAIWA_PROVIDER=anthropic   [ANTHROPIC_API_KEY=…, else the system keyring]
///   KAIWA_PROVIDER=openrouter  KAIWA_MODEL=…  [OPENROUTER_API_KEY=…, else the keyring]
///   KAIWA_PROVIDER=ollama      KAIWA_MODEL=…  [KAIWA_OPENAI_BASE=… over the preset's server]
///   KAIWA_PROVIDER=custom      KAIWA_MODEL=…  KAIWA_OPENAI_BASE=…  [KAIWA_API_KEY=…]
pub fn provider_from(
    profile: &str,
    env: impl Fn(&str) -> Option<String>,
) -> Result<Option<std::sync::Arc<dyn ChatProvider>>, String> {
    let env = |k: &str| env(k).filter(|v| !v.is_empty());
    let Some(id) = env("KAIWA_PROVIDER") else {
        return Ok(None);
    };
    let Some(p) = presets::preset(&id) else {
        let known: Vec<_> = presets::PRESETS.iter().map(|p| p.id).collect();
        return Err(format!(
            "KAIWA_PROVIDER={id}: expected one of {}",
            known.join(", ")
        ));
    };
    let key = secrets::key(p.id, profile, env);
    if p.needs_key() && key.is_none() {
        return Err(format!(
            "KAIWA_PROVIDER={id} needs a key: in the settings (system keyring) or {}",
            p.key_env.unwrap_or_default()
        ));
    }
    match p.api {
        presets::Api::Anthropic => {
            let a = anthropic::AnthropicProvider::preset(key.unwrap_or_default());
            Ok(Some(std::sync::Arc::new(anthropic::AnthropicProvider {
                url: a.url,
                api_key: a.api_key,
                reply_model: a.reply_model,
                analysis_model: a.analysis_model,
                transport: cost::Metered::new(a.transport, true),
            })))
        }
        presets::Api::OpenAi => {
            let model =
                env("KAIWA_MODEL").ok_or(format!("KAIWA_PROVIDER={id} needs KAIWA_MODEL"))?;
            let base = p
                .base_url(env)
                .ok_or(format!("KAIWA_PROVIDER={id} needs KAIWA_OPENAI_BASE"))?;
            let cloud = !presets::local_host(&base);
            let o = openai::OpenAiCompatible::new(base, model);
            Ok(Some(std::sync::Arc::new(openai::OpenAiCompatible {
                base_url: o.base_url,
                api_key: key.unwrap_or_default(),
                reply_model: o.reply_model,
                analysis_model: o.analysis_model,
                transport: cost::Metered::new(o.transport, cloud),
            })))
        }
    }
}
