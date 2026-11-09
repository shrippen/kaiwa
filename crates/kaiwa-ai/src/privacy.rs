//! "What leaves the device?" (Lastenheft /LF711/, /LL400/): per part (speech recognition,
//! tutor, voice) the provider, whether it runs locally or in the cloud, and what is sent. The
//! tutor follows the same settings as [`crate::provider_from_env`]; recognition and voice are
//! local in Kaiwa (ReazonSpeech, Kokoro). The environment is passed in, so tests and
//! `kaiwa-sim` can ask for any setup.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Flow {
    pub part: String,
    pub provider: String,
    pub cloud: bool,
    /// "Lokal"/"Local" or "Cloud".
    #[serde(rename = "where")]
    pub place: String,
    /// What goes out, in words; "nichts"/"nothing" for local parts.
    pub sends: String,
}

/// The flows for the settings in `env` (`KAIWA_PROVIDER`, `KAIWA_OPENAI_BASE`), by preset.
pub fn flows(env: impl Fn(&str) -> Option<String>, lang: &str) -> Vec<Flow> {
    let de = lang == "de";
    let nothing = if de { "nichts" } else { "nothing" };
    let (llm, cloud) = match env("KAIWA_PROVIDER")
        .filter(|p| !p.is_empty())
        .and_then(|p| crate::presets::preset(&p))
    {
        Some(p) => {
            let cloud = p.cloud(&env);
            // A server other than the preset's is named, so the learner sees where it goes.
            let server = p.base_url(&env).filter(|b| b != p.base);
            let label = if p.id == "custom" && !de {
                "OpenAI-compatible"
            } else {
                p.label
            };
            (
                match server {
                    Some(b) => format!("{label} ({b})"),
                    None => label.to_string(),
                },
                cloud,
            )
        }
        None => (
            if de {
                "Keiner eingerichtet".into()
            } else {
                "None set up".into()
            },
            false,
        ),
    };
    let place = |cloud: bool| {
        if cloud {
            "Cloud".to_string()
        } else if de {
            "Lokal".to_string()
        } else {
            "Local".to_string()
        }
    };
    vec![
        Flow {
            part: if de {
                "Spracherkennung"
            } else {
                "Speech recognition"
            }
            .into(),
            provider: "ReazonSpeech (sherpa-onnx)".into(),
            cloud: false,
            place: place(false),
            sends: nothing.into(),
        },
        Flow {
            part: if de {
                "Tutor (Sprachmodell)"
            } else {
                "Tutor (language model)"
            }
            .into(),
            provider: llm,
            cloud,
            place: place(cloud),
            sends: if cloud {
                if de {
                    "Text: das Gespräch dieses Szenarios, ohne Audio, ohne Profil".into()
                } else {
                    "Text: this scenario's conversation, no audio, no profile".into()
                }
            } else {
                nothing.into()
            },
        },
        Flow {
            part: if de { "Stimme" } else { "Voice" }.into(),
            provider: "Kokoro-82M".into(),
            cloud: false,
            place: place(false),
            sends: nothing.into(),
        },
    ]
}

/// The flows for this process's environment.
pub fn flows_from_env(lang: &str) -> Vec<Flow> {
    flows(|k| std::env::var(k).ok(), lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| {
            pairs
                .iter()
                .find(|(a, _)| *a == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn local_only_sends_nothing() {
        for e in [
            env(&[]),
            env(&[("KAIWA_PROVIDER", "ollama"), ("KAIWA_MODEL", "qwen3:14b")]),
            env(&[
                ("KAIWA_PROVIDER", "custom"),
                ("KAIWA_OPENAI_BASE", "http://127.0.0.1:8080/v1"),
            ]),
        ] {
            let f = flows(e, "de");
            assert!(f.iter().all(|x| !x.cloud && x.sends == "nichts"), "{f:?}");
        }
    }

    #[test]
    fn a_cloud_tutor_sends_text_only() {
        let f = flows(env(&[("KAIWA_PROVIDER", "anthropic")]), "de");
        assert!(!f[0].cloud && f[1].cloud && !f[2].cloud);
        assert!(f[1].sends.starts_with("Text") && f[1].sends.contains("ohne Audio"));
        let f = flows(
            env(&[
                ("KAIWA_PROVIDER", "custom"),
                ("KAIWA_OPENAI_BASE", "https://api.example.com/v1"),
            ]),
            "en",
        );
        assert!(f[1].cloud && f[1].place == "Cloud");
        assert_eq!(
            f[1].provider,
            "OpenAI-compatible (https://api.example.com/v1)"
        );
        let f = flows(env(&[("KAIWA_PROVIDER", "openrouter")]), "de");
        assert!(f[1].cloud && f[1].provider == "OpenRouter");
    }
}
