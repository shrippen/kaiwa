//! API keys in the system keyring (Lastenheft /LF713/): Secret Service (KWallet, GNOME
//! Keyring) on Linux, the Credential Manager on Windows, the Keychain on macOS; never in a
//! file or the profile. Service `org.shrippen.kaiwa`, one entry per provider (its preset id,
//! `crate::presets`) and profile: user `<provider>/<profile>` (decision V-01); deleting the
//! profile deletes its keys. An environment variable still wins, for the owner's local test.
//!
//! [`Keys`] is what the rest of Kaiwa uses: a [`KeyStore`] (the keyring, or [`MemoryKeys`] in
//! tests) and the profile's [`KeyHome`]. A profile outside the profiles directory has no
//! keyring access at all.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

const SERVICE: &str = "org.shrippen.kaiwa";

/// Where a provider's key comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyState {
    /// In the environment (local test).
    Environment,
    /// In the system keyring.
    Keyring,
    Missing,
    /// The profile has no keyring home ([`KeyHome::EnvOnly`]) and the environment has no key.
    EnvOnly,
    /// No keyring on this system (or it refused); the reason.
    Unavailable(String),
}

/// Why a key was not stored; the view layer words it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// Blank.
    Empty,
    /// The profile has no keyring home ([`KeyHome::EnvOnly`]).
    EnvOnly,
    /// The keyring refused; its reason.
    Keyring(String),
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("empty key"),
            Self::EnvOnly => f.write_str("the profile has no keyring entry"),
            Self::Keyring(e) => f.write_str(e),
        }
    }
}

/// Where keys are kept, by entry user name.
pub trait KeyStore: Send + Sync {
    fn get(&self, user: &str) -> Result<Option<String>, String>;
    fn set(&self, user: &str, key: &str) -> Result<(), String>;
    /// A missing entry is not an error.
    fn delete(&self, user: &str) -> Result<(), String>;
}

/// The system keyring.
pub struct Keyring;

impl KeyStore for Keyring {
    fn get(&self, user: &str) -> Result<Option<String>, String> {
        imp::get(user)
    }
    fn set(&self, user: &str, key: &str) -> Result<(), String> {
        imp::set(user, key)
    }
    fn delete(&self, user: &str) -> Result<(), String> {
        imp::delete(user)
    }
}

/// Keys in memory, owned by whoever made it (tests, `kaiwa-sim`).
#[derive(Default)]
pub struct MemoryKeys(Mutex<HashMap<String, String>>);

impl MemoryKeys {
    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl KeyStore for MemoryKeys {
    fn get(&self, user: &str) -> Result<Option<String>, String> {
        Ok(self.map().get(user).cloned())
    }
    fn set(&self, user: &str, key: &str) -> Result<(), String> {
        self.map().insert(user.to_string(), key.to_string());
        Ok(())
    }
    fn delete(&self, user: &str) -> Result<(), String> {
        self.map().remove(user);
        Ok(())
    }
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

/// Which keys a profile may use. The one place that decides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyHome {
    /// A profile in the profiles directory: its keys, named by its id (the file name without
    /// `.kaiwa`).
    Keyring(String),
    /// Any other profile file (`KAIWA_PROFILE`, the demo, a temporary directory): the
    /// environment only, the keyring is never read or written.
    EnvOnly,
}

impl KeyHome {
    /// The home of the profile file `path`, given the profiles directory `dir`. `None`: no
    /// directory has keyring homes (`KAIWA_DATA` moves the profiles elsewhere, and their ids
    /// would collide with the real ones).
    pub fn of(path: &Path, dir: Option<&Path>) -> Self {
        let in_dir = dir.is_some_and(|d| path.parent().is_some_and(|p| same_dir(p, d)));
        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".kaiwa"))
            .filter(|id| !id.is_empty());
        match id {
            Some(id) if in_dir => Self::Keyring(id.to_string()),
            _ => Self::EnvOnly,
        }
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

/// The keys of one profile.
#[derive(Clone)]
pub struct Keys {
    store: Arc<dyn KeyStore>,
    home: KeyHome,
}

impl Keys {
    pub fn new(store: Arc<dyn KeyStore>, home: KeyHome) -> Self {
        Self { store, home }
    }

    /// No keyring: the environment only.
    pub(crate) fn env_only() -> Self {
        Self::new(Arc::new(MemoryKeys::default()), KeyHome::EnvOnly)
    }

    /// Whether the profile has keyring entries, or the environment is its only source.
    pub fn keyring_home(&self) -> bool {
        self.home != KeyHome::EnvOnly
    }

    fn user(&self, provider: &str) -> Option<String> {
        match &self.home {
            KeyHome::Keyring(id) => Some(format!("{provider}/{id}")),
            KeyHome::EnvOnly => None,
        }
    }

    /// The key of a provider: the environment first, then the keyring.
    pub fn key(&self, provider: &str, env: impl Fn(&str) -> Option<String>) -> Option<String> {
        if !takes_key(provider) {
            return None;
        }
        env(env_var(provider))
            .filter(|k| !k.is_empty())
            .or_else(|| self.store.get(&self.user(provider)?).ok().flatten())
    }

    pub fn state(&self, provider: &str, env: impl Fn(&str) -> Option<String>) -> KeyState {
        if env(env_var(provider)).is_some_and(|k| !k.is_empty()) {
            return KeyState::Environment;
        }
        let Some(user) = self.user(provider) else {
            return KeyState::EnvOnly;
        };
        match self.store.get(&user) {
            Ok(Some(_)) => KeyState::Keyring,
            Ok(None) => KeyState::Missing,
            Err(e) => KeyState::Unavailable(e),
        }
    }

    /// Stores a key in the keyring; a blank key is an error.
    pub fn store(&self, provider: &str, key: &str) -> Result<(), KeyError> {
        let key = usable(key)?;
        let user = self.user(provider).ok_or(KeyError::EnvOnly)?;
        self.store.set(&user, key).map_err(KeyError::Keyring)
    }

    /// Removes the provider's key of the profile from the keyring.
    pub fn remove(&self, provider: &str) -> Result<(), String> {
        let Some(user) = self.user(provider) else {
            return Ok(());
        };
        self.store.delete(&user)
    }

    /// Removes the profile's keys of every provider (the profile is deleted). Every provider
    /// is tried; the errors come back together.
    pub fn remove_all(&self) -> Result<(), String> {
        let errors: Vec<String> = keyed()
            .filter_map(|p| self.remove(p).err().map(|e| format!("{p}: {e}")))
            .collect();
        if errors.is_empty() {
            return Ok(());
        }
        Err(errors.join("; "))
    }

    /// One-time move of the keys from before V-01, one entry per provider (user
    /// `<provider>`), to this profile: stored under the new name, then the old entry removed.
    /// Without an old entry nothing happens. A profile that has its own key keeps it; the old
    /// entry goes anyway. Nothing for a profile without a keyring home.
    pub fn migrate(&self) -> Result<(), String> {
        for provider in keyed() {
            let Some(new) = self.user(provider) else {
                return Ok(());
            };
            let Some(key) = self.store.get(provider)? else {
                continue;
            };
            if self.store.get(&new)?.is_none() {
                self.store.set(&new, &key)?;
            }
            self.store.delete(provider)?;
        }
        Ok(())
    }
}

/// The providers that take a key.
fn keyed() -> impl Iterator<Item = &'static str> {
    crate::presets::PRESETS
        .iter()
        .filter(|p| p.key_env.is_some())
        .map(|p| p.id)
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

/// The key as it is stored: trimmed, and not blank. Blank is no key: removing one is its own
/// step ([`Keys::remove`]).
fn usable(key: &str) -> Result<&str, KeyError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(KeyError::Empty);
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: fn(&str) -> Option<String> = |_| None;

    fn home(id: &str, store: &Arc<MemoryKeys>) -> Keys {
        Keys::new(store.clone(), KeyHome::Keyring(id.into()))
    }

    /// A keyring that refuses everything, like a system without Secret Service.
    struct Refusing;

    impl KeyStore for Refusing {
        fn get(&self, _: &str) -> Result<Option<String>, String> {
            Err("no Secret Service".into())
        }
        fn set(&self, _: &str, _: &str) -> Result<(), String> {
            Err("no Secret Service".into())
        }
        fn delete(&self, _: &str) -> Result<(), String> {
            Err("no Secret Service".into())
        }
    }

    #[test]
    fn a_blank_key_is_no_key() {
        // Spaces pasted by mistake must not remove the stored key (settings, review T-05).
        assert_eq!(usable("   "), Err(KeyError::Empty));
        assert_eq!(usable(" sk-1 "), Ok("sk-1"));
        let keys = home("a", &Arc::default());
        assert!(keys.store("anthropic", "   ").is_err());
    }

    #[test]
    fn the_environment_wins() {
        let keys = home("a", &Arc::default());
        let env = |k: &str| (k == "ANTHROPIC_API_KEY").then(|| "from-env".to_string());
        assert_eq!(keys.key("anthropic", env).as_deref(), Some("from-env"));
        assert_eq!(keys.state("anthropic", env), KeyState::Environment);
        assert_eq!(env_var("custom"), "KAIWA_API_KEY");
        assert_eq!(env_var("openrouter"), "OPENROUTER_API_KEY");
        let env = |k: &str| (k == "KAIWA_API_KEY").then(|| "x".to_string());
        assert_eq!(keys.key("ollama", env), None);
    }

    #[test]
    fn without_a_keyring_the_state_says_why() {
        // A keyring that refuses: the state names the reason, nothing panics, and storing
        // or removing is an error.
        let keys = Keys::new(Arc::new(Refusing), KeyHome::Keyring("a".into()));
        assert_eq!(
            keys.state("anthropic", NONE),
            KeyState::Unavailable("no Secret Service".into())
        );
        assert_eq!(keys.key("anthropic", NONE), None);
        assert!(keys.store("anthropic", "sk").is_err());
        assert!(keys.remove_all().is_err());
    }

    #[test]
    fn a_profile_does_not_see_the_key_of_another() {
        let store = Arc::new(MemoryKeys::default());
        let (a, b) = (home("a", &store), home("b", &store));
        a.store("anthropic", "sk-a").unwrap();
        assert_eq!(a.key("anthropic", NONE).as_deref(), Some("sk-a"));
        assert_eq!(b.key("anthropic", NONE), None);
        assert_eq!(b.state("anthropic", NONE), KeyState::Missing);

        // Removing B's key leaves A's.
        b.store("anthropic", "sk-b").unwrap();
        b.remove("anthropic").unwrap();
        assert_eq!(a.key("anthropic", NONE).as_deref(), Some("sk-a"));
        assert_eq!(b.key("anthropic", NONE), None);
    }

    #[test]
    fn remove_all_takes_every_provider_of_the_profile() {
        let store = Arc::new(MemoryKeys::default());
        let (a, b) = (home("a", &store), home("b", &store));
        a.store("anthropic", "sk-1").unwrap();
        a.store("openai", "sk-2").unwrap();
        b.store("openai", "sk-3").unwrap();
        a.remove_all().unwrap();
        assert_eq!(a.key("anthropic", NONE), None);
        assert_eq!(a.key("openai", NONE), None);
        assert_eq!(b.key("openai", NONE).as_deref(), Some("sk-3"));
    }

    #[test]
    fn migration_moves_the_old_entry_once() {
        let store = Arc::new(MemoryKeys::default());
        // Before V-01: one entry per provider.
        store.set("mistral", "sk-old").unwrap();
        let (first, second) = (home("first", &store), home("second", &store));

        first.migrate().unwrap();
        assert_eq!(first.key("mistral", NONE).as_deref(), Some("sk-old"));
        assert_eq!(store.get("mistral"), Ok(None), "the old entry is gone");

        // Again, and for another profile: nothing moves.
        first.migrate().unwrap();
        second.migrate().unwrap();
        assert_eq!(first.key("mistral", NONE).as_deref(), Some("sk-old"));
        assert_eq!(second.key("mistral", NONE), None);
    }

    #[test]
    fn only_a_profile_in_the_profiles_dir_has_a_keyring_home() {
        let dir = Some(Path::new("/p/profiles"));
        let home = |f: &str| KeyHome::of(Path::new(f), dir);
        assert_eq!(
            home("/p/profiles/default.kaiwa"),
            KeyHome::Keyring("default".into())
        );
        // The same file name elsewhere (KAIWA_PROFILE, demo, tmp) is not the same profile.
        assert_eq!(home("/tmp/kaiwa-demo-1/default.kaiwa"), KeyHome::EnvOnly);
        assert_eq!(home("/p/profiles/sub/default.kaiwa"), KeyHome::EnvOnly);
        assert_eq!(home("/p/profiles/.kaiwa"), KeyHome::EnvOnly);
        assert_eq!(home("/p/profiles/notes.txt"), KeyHome::EnvOnly);
        // KAIWA_DATA: no directory is the real one.
        assert_eq!(
            KeyHome::of(Path::new("/p/profiles/default.kaiwa"), None),
            KeyHome::EnvOnly
        );
    }

    #[test]
    fn an_env_only_profile_never_reaches_the_store() {
        let store = Arc::new(MemoryKeys::default());
        store.set("anthropic/default", "sk-real").unwrap();
        let keys = Keys::new(store.clone(), KeyHome::EnvOnly);

        assert_eq!(keys.key("anthropic", NONE), None);
        assert_eq!(keys.state("anthropic", NONE), KeyState::EnvOnly);
        assert_eq!(keys.store("anthropic", "sk-x"), Err(KeyError::EnvOnly));
        keys.remove("anthropic").unwrap();
        keys.remove_all().unwrap();
        keys.migrate().unwrap();
        assert_eq!(store.get("anthropic/default"), Ok(Some("sk-real".into())));

        // The environment still works.
        let env = |k: &str| (k == "ANTHROPIC_API_KEY").then(|| "from-env".to_string());
        assert_eq!(keys.key("anthropic", env).as_deref(), Some("from-env"));
        assert_eq!(keys.state("anthropic", env), KeyState::Environment);
    }

    #[test]
    fn without_a_keyring_home_the_missing_key_names_only_the_variable() {
        let env = |k: &str| (k == "KAIWA_PROVIDER").then(|| "anthropic".to_string());
        let Err(e) = crate::provider_from(&Keys::env_only(), env) else {
            panic!("no key, no tutor");
        };
        assert!(e.contains("ANTHROPIC_API_KEY"), "{e}");
        assert!(!e.contains("keyring") && !e.contains("settings"), "{e}");

        // A profile with a keyring home may use either.
        let keys = home("a", &Arc::default());
        let Err(e) = crate::provider_from(&keys, env) else {
            panic!("no key, no tutor");
        };
        assert!(e.contains("keyring"), "{e}");
    }
}
