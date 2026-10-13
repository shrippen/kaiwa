//! The tutor the learner set up in the settings (Q-15): which provider, which model, which
//! server. The key stays in the system keyring ([`crate::secrets`]), never here. The
//! environment (`KAIWA_PROVIDER`, `KAIWA_MODEL`, `KAIWA_OPENAI_BASE`) still wins, for the
//! owner's local test.

use serde::{Deserialize, Serialize};

use crate::{AiError, Analysis, ChatProvider, TurnRequest};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tutor {
    /// "anthropic", "openai" (any OpenAI-compatible server) or empty for none.
    #[serde(default)]
    pub provider: String,
    /// The model name for an OpenAI-compatible server (`qwen3:14b`); Anthropic uses its presets.
    #[serde(default)]
    pub model: String,
    /// The server of an OpenAI-compatible provider; empty is Ollama on this machine.
    #[serde(default)]
    pub base_url: String,
}

impl Tutor {
    /// The settings as the variables the providers read, the environment on top.
    pub fn lookup<'a>(
        &'a self,
        env: impl Fn(&str) -> Option<String> + 'a,
    ) -> impl Fn(&str) -> Option<String> + 'a {
        move |k: &str| {
            env(k).filter(|v| !v.is_empty()).or_else(|| {
                let v = match k {
                    "KAIWA_PROVIDER" => &self.provider,
                    "KAIWA_MODEL" => &self.model,
                    "KAIWA_OPENAI_BASE" => &self.base_url,
                    _ => return None,
                };
                (!v.is_empty()).then(|| v.clone())
            })
        }
    }
}

/// Stands in when no tutor is set up: every call says so, nothing is invented.
pub struct NoTutor;

impl ChatProvider for NoTutor {
    fn reply(&self, _: &TurnRequest) -> Result<String, AiError> {
        Err(AiError::NoTutor)
    }
    fn analyse(&self, _: &TurnRequest) -> Result<Analysis, AiError> {
        Err(AiError::NoTutor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_wins_over_the_settings() {
        let t = Tutor {
            provider: "openai".into(),
            model: "qwen3:14b".into(),
            base_url: String::new(),
        };
        let none = |_: &str| None;
        let l = t.lookup(none);
        assert_eq!(l("KAIWA_PROVIDER").as_deref(), Some("openai"));
        assert_eq!(l("KAIWA_OPENAI_BASE"), None);
        assert_eq!(l("ANTHROPIC_API_KEY"), None);
        let env = |k: &str| (k == "KAIWA_PROVIDER").then(|| "anthropic".to_string());
        let l = t.lookup(env);
        assert_eq!(l("KAIWA_PROVIDER").as_deref(), Some("anthropic"));
        assert_eq!(l("KAIWA_MODEL").as_deref(), Some("qwen3:14b"));
    }

    #[test]
    fn without_a_tutor_every_call_says_so() {
        let r = TurnRequest {
            system: String::new(),
            messages: vec![],
            max_tokens: 10,
        };
        assert_eq!(NoTutor.reply(&r).unwrap_err(), AiError::NoTutor);
        assert_eq!(NoTutor.translate("駅", "de").unwrap_err(), AiError::NoTutor);
    }
}
