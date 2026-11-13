//! API keys in the system keyring (Lastenheft /LF713/): Secret Service (KWallet, GNOME
//! Keyring) on Linux, the Credential Manager on Windows, the Keychain on macOS; never in a
//! file or the profile. Service `org.shrippen.kaiwa`, one entry per provider (its preset id,
//! `crate::presets`) and profile: user `<provider>/<profile>` (decision V-01); deleting the
//! profile deletes its keys. An environment variable still wins, for the owner's local test.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

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

    fn entry(user: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, user).map_err(|e| e.to_string())
    }

    pub fn get(user: &str) -> Result<Option<String>, String> {
        match entry(user)?.get_password() {
            Ok(k) => Ok(Some(k)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn set(user: &str, key: &str) -> Result<(), String> {
        entry(user)?.set_password(key).map_err(|e| e.to_string())
    }

    pub fn delete(user: &str) -> Result<(), String> {
        match entry(user)?.delete_credential() {
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

/// Tests and `kaiwa-sim` never touch the real keyring: once switched, this process keeps its
/// keys in memory.
static IN_MEMORY: AtomicBool = AtomicBool::new(false);
static MEMORY: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Keeps the keys of this process in memory from now on (tests, `kaiwa-sim`).
pub fn use_memory() {
    IN_MEMORY.store(true, Ordering::SeqCst);
}

/// The keyring entry's user name: `<provider>/<profile>`.
fn user(provider: &str, profile: &str) -> String {
    format!("{provider}/{profile}")
}

fn memory<T>(f: impl FnOnce(&mut HashMap<String, String>) -> T) -> T {
    let mut m = MEMORY.lock().unwrap_or_else(|e| e.into_inner());
    f(m.get_or_insert_with(HashMap::new))
}

fn get(user: &str) -> Result<Option<String>, String> {
    if IN_MEMORY.load(Ordering::SeqCst) {
        return Ok(memory(|m| m.get(user).cloned()));
    }
    imp::get(user)
}

fn set(user: &str, key: &str) -> Result<(), String> {
    if IN_MEMORY.load(Ordering::SeqCst) {
        memory(|m| m.insert(user.to_string(), key.to_string()));
        return Ok(());
    }
    imp::set(user, key)
}

fn delete(user: &str) -> Result<(), String> {
    if IN_MEMORY.load(Ordering::SeqCst) {
        memory(|m| m.remove(user));
        return Ok(());
    }
    imp::delete(user)
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

/// The key of a provider for a profile: the environment first, then the keyring.
pub fn key(provider: &str, profile: &str, env: impl Fn(&str) -> Option<String>) -> Option<String> {
    if !takes_key(provider) {
        return None;
    }
    env(env_var(provider))
        .filter(|k| !k.is_empty())
        .or_else(|| get(&user(provider, profile)).ok().flatten())
}

pub fn state(provider: &str, profile: &str, env: impl Fn(&str) -> Option<String>) -> KeyState {
    if env(env_var(provider)).is_some_and(|k| !k.is_empty()) {
        return KeyState::Environment;
    }
    match get(&user(provider, profile)) {
        Ok(Some(_)) => KeyState::Keyring,
        Ok(None) => KeyState::Missing,
        Err(e) => KeyState::Unavailable(e),
    }
}

/// Stores a key in the keyring; a blank key is an error.
pub fn store(provider: &str, profile: &str, key: &str) -> Result<(), String> {
    set(&user(provider, profile), usable(key)?)
}

/// The key as it is stored: trimmed, and not blank. Blank is no key: removing one is its own
/// step ([`remove`]).
fn usable(key: &str) -> Result<&str, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("empty key".into());
    }
    Ok(key)
}

/// Removes the provider's key of the profile from the keyring.
pub fn remove(provider: &str, profile: &str) -> Result<(), String> {
    delete(&user(provider, profile))
}

/// The providers that take a key.
fn keyed() -> impl Iterator<Item = &'static str> {
    crate::presets::PRESETS
        .iter()
        .filter(|p| p.key_env.is_some())
        .map(|p| p.id)
}

/// Removes the profile's keys of every provider (the profile is deleted). Every provider is
/// tried; the errors come back together.
pub fn remove_all(profile: &str) -> Result<(), String> {
    let errors: Vec<String> = keyed()
        .filter_map(|p| remove(p, profile).err().map(|e| format!("{p}: {e}")))
        .collect();
    if errors.is_empty() {
        return Ok(());
    }
    Err(errors.join("; "))
}

/// One-time move of the keys from before V-01, one entry per provider (user `<provider>`), to
/// `profile`: stored under the new name, then the old entry removed. Without an old entry
/// nothing happens, so running it again changes nothing. A profile that has its own key keeps
/// it; the old entry goes anyway.
pub fn migrate(profile: &str) -> Result<(), String> {
    for provider in keyed() {
        let Some(key) = get(provider)? else {
            continue;
        };
        let new = user(provider, profile);
        if get(&new)?.is_none() {
            set(&new, &key)?;
        }
        delete(provider)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_key_is_no_key() {
        // Spaces pasted by mistake must not remove the stored key (settings, review T-05);
        // checked without the keyring, which the CI does not have.
        assert!(usable("   ").is_err());
        assert_eq!(usable(" sk-1 "), Ok("sk-1"));
        assert!(store("kaiwa-test-blank", "p", "   ").is_err());
    }

    #[test]
    fn the_environment_wins() {
        let env = |k: &str| (k == "ANTHROPIC_API_KEY").then(|| "from-env".to_string());
        assert_eq!(key("anthropic", "p", env).as_deref(), Some("from-env"));
        assert_eq!(state("anthropic", "p", env), KeyState::Environment);
        assert_eq!(env_var("custom"), "KAIWA_API_KEY");
        assert_eq!(env_var("openrouter"), "OPENROUTER_API_KEY");
        let env = |k: &str| (k == "KAIWA_API_KEY").then(|| "x".to_string());
        assert_eq!(key("ollama", "p", env), None);
    }

    #[test]
    fn without_a_keyring_the_state_says_why() {
        // In CI there is no Secret Service: the state is Missing or Unavailable, never a panic.
        use_memory();
        let s = state("kaiwa-test-provider", "p", |_| None);
        assert!(
            matches!(s, KeyState::Missing | KeyState::Unavailable(_)),
            "{s:?}"
        );
    }

    #[test]
    fn a_profile_does_not_see_the_key_of_another() {
        use_memory();
        store("anthropic", "kp-a", "sk-a").unwrap();
        let none = |_: &str| None;
        assert_eq!(key("anthropic", "kp-a", none).as_deref(), Some("sk-a"));
        assert_eq!(key("anthropic", "kp-b", none), None);
        assert_eq!(state("anthropic", "kp-b", none), KeyState::Missing);

        // Removing B's key (none) leaves A's.
        store("anthropic", "kp-b", "sk-b").unwrap();
        remove("anthropic", "kp-b").unwrap();
        assert_eq!(key("anthropic", "kp-a", none).as_deref(), Some("sk-a"));
        assert_eq!(key("anthropic", "kp-b", none), None);
        remove("anthropic", "kp-a").unwrap();
    }

    #[test]
    fn remove_all_takes_every_provider_of_the_profile() {
        use_memory();
        let none = |_: &str| None;
        store("anthropic", "kp-all", "sk-1").unwrap();
        store("openai", "kp-all", "sk-2").unwrap();
        store("openai", "kp-other", "sk-3").unwrap();
        remove_all("kp-all").unwrap();
        assert_eq!(key("anthropic", "kp-all", none), None);
        assert_eq!(key("openai", "kp-all", none), None);
        assert_eq!(key("openai", "kp-other", none).as_deref(), Some("sk-3"));
        remove_all("kp-other").unwrap();
    }

    #[test]
    fn migration_moves_the_old_entry_once() {
        use_memory();
        let none = |_: &str| None;
        // Before V-01: one entry per provider.
        set("mistral", "sk-old").unwrap();

        migrate("kp-first").unwrap();
        assert_eq!(key("mistral", "kp-first", none).as_deref(), Some("sk-old"));
        assert_eq!(get("mistral"), Ok(None), "the old entry is gone");

        // Again, and for another profile: nothing moves.
        migrate("kp-first").unwrap();
        migrate("kp-second").unwrap();
        assert_eq!(key("mistral", "kp-first", none).as_deref(), Some("sk-old"));
        assert_eq!(key("mistral", "kp-second", none), None);
        remove_all("kp-first").unwrap();
    }
}
