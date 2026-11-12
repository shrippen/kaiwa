//! The tutors Kaiwa knows by name (Q-15): Anthropic with its own interface, every other one
//! an OpenAI-compatible server at a known address. A preset says where the server is,
//! whether it runs on this computer, which variable overrides its key and which model to
//! try; the learner only picks it and, for a cloud, pastes the key.

/// How Kaiwa talks to the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    /// Anthropic's Messages API, models from ADR-004.
    Anthropic,
    /// `POST {base}/chat/completions`.
    OpenAi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    /// Stored in the profile and in `KAIWA_PROVIDER`; also the keyring entry of its key.
    pub id: &'static str,
    pub label: &'static str,
    pub api: Api,
    /// The server; empty for "custom", where the learner gives it.
    pub base: &'static str,
    /// Runs on this computer: nothing leaves the device.
    pub local: bool,
    /// The variable that overrides the keyring; `None` when the server takes no key.
    pub key_env: Option<&'static str>,
    /// A model to try, shown as the example in the settings (the learner names the model;
    /// Anthropic uses its presets and needs none).
    pub example_model: &'static str,
    /// Where the learner gets a key, empty without one.
    pub keys_at: &'static str,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "ollama",
        label: "Ollama",
        api: Api::OpenAi,
        base: "http://localhost:11434/v1",
        local: true,
        key_env: None,
        example_model: "qwen3:14b",
        keys_at: "",
    },
    Preset {
        id: "lmstudio",
        label: "LM Studio",
        api: Api::OpenAi,
        base: "http://localhost:1234/v1",
        local: true,
        key_env: None,
        example_model: "qwen3-14b",
        keys_at: "",
    },
    Preset {
        id: "anthropic",
        label: "Anthropic Claude",
        api: Api::Anthropic,
        base: "https://api.anthropic.com",
        local: false,
        key_env: Some("ANTHROPIC_API_KEY"),
        example_model: "",
        keys_at: "console.anthropic.com",
    },
    Preset {
        id: "openrouter",
        label: "OpenRouter",
        api: Api::OpenAi,
        base: "https://openrouter.ai/api/v1",
        local: false,
        key_env: Some("OPENROUTER_API_KEY"),
        example_model: "anthropic/claude-sonnet-4.5",
        keys_at: "openrouter.ai/keys",
    },
    Preset {
        id: "openai",
        label: "OpenAI",
        api: Api::OpenAi,
        base: "https://api.openai.com/v1",
        local: false,
        key_env: Some("OPENAI_API_KEY"),
        example_model: "gpt-4.1",
        keys_at: "platform.openai.com/api-keys",
    },
    Preset {
        id: "gemini",
        label: "Google Gemini",
        api: Api::OpenAi,
        base: "https://generativelanguage.googleapis.com/v1beta/openai",
        local: false,
        key_env: Some("GEMINI_API_KEY"),
        example_model: "gemini-2.5-flash",
        keys_at: "aistudio.google.com/apikey",
    },
    Preset {
        id: "mistral",
        label: "Mistral",
        api: Api::OpenAi,
        base: "https://api.mistral.ai/v1",
        local: false,
        key_env: Some("MISTRAL_API_KEY"),
        example_model: "mistral-large-latest",
        keys_at: "console.mistral.ai/api-keys",
    },
    Preset {
        id: "custom",
        label: "OpenAI-kompatibel",
        api: Api::OpenAi,
        base: "",
        local: false,
        key_env: Some("KAIWA_API_KEY"),
        example_model: "qwen3:14b",
        keys_at: "",
    },
];

pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

const LOCAL_HOSTS: [&str; 4] = ["localhost", "127.0.0.1", "[::1]", "0.0.0.0"];

/// The lowercase host of `url`, without scheme, userinfo and port.
///
/// `http://user@Host:80/p?q` gives `host`; `http://[::1]:80/` gives `[::1]`.
fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);

    // An IPv6 literal keeps its brackets; its colons are not a port.
    let host = match host_port.strip_prefix('[') {
        Some(_) => host_port.split_inclusive(']').next().unwrap_or_default(),
        None => host_port.split(':').next().unwrap_or_default(),
    };
    host.to_ascii_lowercase()
}

/// A server on this computer. The host must match exactly: `abc.localhost.run` is remote.
pub fn local_host(url: &str) -> bool {
    LOCAL_HOSTS.contains(&host_of(url).as_str())
}

impl Preset {
    /// The server in use: `KAIWA_OPENAI_BASE` (or the settings) over the preset's.
    pub fn base_url(&self, env: impl Fn(&str) -> Option<String>) -> Option<String> {
        env("KAIWA_OPENAI_BASE")
            .filter(|b| !b.is_empty() && self.api == Api::OpenAi)
            .or_else(|| (!self.base.is_empty()).then(|| self.base.to_string()))
    }

    /// Whether the conversations leave the device with this setup.
    pub fn cloud(&self, env: impl Fn(&str) -> Option<String>) -> bool {
        match self.base_url(env) {
            Some(b) => !local_host(&b),
            None => !self.local,
        }
    }

    /// Needs the learner to name a model.
    pub fn needs_model(&self) -> bool {
        self.api == Api::OpenAi
    }

    /// Needs a key before it can run: a cloud with a key variable; a local or custom server
    /// may take one but runs without.
    pub fn needs_key(&self) -> bool {
        self.key_env.is_some() && self.id != "custom"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_is_complete() {
        let mut ids: Vec<_> = PRESETS.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), PRESETS.len());
        for p in PRESETS {
            assert_eq!(p.local, local_host(p.base), "{}", p.id);
            assert_eq!(p.key_env.is_some(), !p.local, "{}", p.id);
            assert_eq!(p.keys_at.is_empty(), !p.needs_key(), "{}", p.id);
            assert_eq!(p.example_model.is_empty(), !p.needs_model(), "{}", p.id);
            assert!(p.id == "custom" || p.base.starts_with("http"), "{}", p.id);
        }
    }

    #[test]
    fn the_server_follows_the_override() {
        let none = |_: &str| None;
        let ollama = preset("ollama").unwrap();
        assert!(!ollama.cloud(none));
        let remote = |k: &str| (k == "KAIWA_OPENAI_BASE").then(|| "https://gpu.example/v1".into());
        assert!(ollama.cloud(remote));
        let custom = preset("custom").unwrap();
        assert_eq!(custom.base_url(none), None);
        assert!(custom.cloud(none));
        let anthropic = preset("anthropic").unwrap();
        assert_eq!(
            anthropic.base_url(remote).as_deref(),
            Some("https://api.anthropic.com")
        );
    }

    #[test]
    fn local_host_matches_the_host_only() {
        for url in [
            "http://localhost:11434/v1",
            "http://127.0.0.1:8080",
            "http://[::1]:8080/v1",
            "http://0.0.0.0:8080",
            "HTTP://LocalHost/v1",
            "localhost:11434",
        ] {
            assert!(local_host(url), "{url}");
        }
        for url in [
            "https://abc.localhost.run/v1",
            "http://localhost:1@gpu.example.com/v1",
            "https://gpu.example.com/localhost",
            "https://gpu.example.com/?h=127.0.0.1",
            "https://127.0.0.1.example.com/v1",
            "",
        ] {
            assert!(!local_host(url), "{url}");
        }
    }
}
