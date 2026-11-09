//! API keys in the system keyring (Lastenheft /LF713/): Secret Service (KWallet, GNOME
//! Keyring) on Linux, the Credential Manager on Windows, the Keychain on macOS; never in a
//! file or the profile. Service `org.shrippen.kaiwa`, one entry per provider (its preset id,
//! `crate::presets`). An environment variable still wins, for the owner's local test.

const SERVICE: &str = "org.shrippen.kaiwa";

/// Where a provider's key comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyState {
    /// In the environment (local test).
    Environment,
    /// In the system keyring.
    Keyring,
    Missing,
    /// No keyring on this system (or it refused); the reason.
    Unavailable(String),
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
mod imp {
    use super::SERVICE;

    fn entry(provider: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, provider).map_err(|e| e.to_string())
    }

    pub fn get(provider: &str) -> Result<Option<String>, String> {
        match entry(provider)?.get_password() {
            Ok(k) => Ok(Some(k)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn set(provider: &str, key: &str) -> Result<(), String> {
        entry(provider)?
            .set_password(key)
            .map_err(|e| e.to_string())
    }

    pub fn delete(provider: &str) -> Result<(), String> {
        match entry(provider)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod imp {
    const NONE: &str = "no system keyring on this platform";
    pub fn get(_: &str) -> Result<Option<String>, String> {
        Err(NONE.into())
    }
    pub fn set(_: &str, _: &str) -> Result<(), String> {
        Err(NONE.into())
    }
    pub fn delete(_: &str) -> Result<(), String> {
        Err(NONE.into())
    }
}

/// The variable that overrides the keyring for a provider (its preset's); an unknown one
/// has `KAIWA_API_KEY`.
pub fn env_var(provider: &str) -> &'static str {
    crate::presets::preset(provider)
        .and_then(|p| p.key_env)
        .unwrap_or("KAIWA_API_KEY")
}

/// Whether the provider takes a key at all (a server on this computer does not; its
/// keyring entry is never asked).
fn takes_key(provider: &str) -> bool {
    crate::presets::preset(provider).is_none_or(|p| p.key_env.is_some())
}

/// The key of a provider: the environment first, then the keyring.
pub fn key(provider: &str, env: impl Fn(&str) -> Option<String>) -> Option<String> {
    if !takes_key(provider) {
        return None;
    }
    env(env_var(provider))
        .filter(|k| !k.is_empty())
        .or_else(|| imp::get(provider).ok().flatten())
}

pub fn state(provider: &str, env: impl Fn(&str) -> Option<String>) -> KeyState {
    if env(env_var(provider)).is_some_and(|k| !k.is_empty()) {
        return KeyState::Environment;
    }
    match imp::get(provider) {
        Ok(Some(_)) => KeyState::Keyring,
        Ok(None) => KeyState::Missing,
        Err(e) => KeyState::Unavailable(e),
    }
}

/// Stores a key in the keyring (an empty key removes it).
pub fn store(provider: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        imp::delete(provider)
    } else {
        imp::set(provider, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_wins() {
        let env = |k: &str| (k == "ANTHROPIC_API_KEY").then(|| "from-env".to_string());
        assert_eq!(key("anthropic", env).as_deref(), Some("from-env"));
        assert_eq!(state("anthropic", env), KeyState::Environment);
        assert_eq!(env_var("custom"), "KAIWA_API_KEY");
        assert_eq!(env_var("openrouter"), "OPENROUTER_API_KEY");
        let env = |k: &str| (k == "KAIWA_API_KEY").then(|| "x".to_string());
        assert_eq!(key("ollama", env), None);
    }

    #[test]
    fn without_a_keyring_the_state_says_why() {
        // In CI there is no Secret Service: the state is Missing or Unavailable, never a panic.
        let s = state("kaiwa-test-provider", |_| None);
        assert!(
            matches!(s, KeyState::Missing | KeyState::Unavailable(_)),
            "{s:?}"
        );
    }
}
