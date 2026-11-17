//! What goes wrong in the background (opening, purging, wiping, moving keys): the backend
//! collects it and the front end tells the learner, whichever front end it is.

use std::path::PathBuf;

use kaiwa_store::Store;

use crate::{Backend, Start, key_store};

/// A failure no command was waiting for.
#[derive(Debug)]
pub enum Trouble {
    /// The profile could not be opened; a spare in memory stands in and saves nothing.
    Unopened(anyhow::Error),
    /// Expired recordings stayed when the profile opened (/LF712/).
    Unpurged(anyhow::Error),
    /// The recordings stayed when all data of the profile was wiped (/LF710/).
    RecordingsKept(anyhow::Error),
    /// The keyring kept keys of the profile when it was wiped.
    KeysKept(String),
    /// The keys of before V-01 stayed where they were.
    KeysUnmoved(String),
    /// The keys moved, but the note that says so could not be written: the next start tries
    /// again.
    KeysMarkerLost(String),
    /// The tutor the settings or the environment name cannot be built (no key, model or base).
    TutorUnready(String),
}

impl Backend {
    /// The profile the process starts on; if it cannot be opened, a spare on an in-memory
    /// store, which leaves the file alone, with [`Trouble::Unopened`] queued.
    pub fn open_or_spare() -> Self {
        let start = Start::read();
        Self::or_spare(start.path.clone(), Self::open_start(&start))
    }

    /// `opened`, or the spare for the profile at `path`.
    pub(crate) fn or_spare(path: PathBuf, opened: anyhow::Result<Self>) -> Self {
        let e = match opened {
            Ok(b) => return b,
            Err(e) => e,
        };
        let keys = kaiwa_ai::secrets::Keys::new(
            key_store(),
            kaiwa_ai::secrets::KeyHome::of(&path, kaiwa_store::keyring_dir().as_deref()),
        );
        let db = Store::in_memory().expect("an empty store in memory opens");
        let mut spare = Self::assemble(db, keys, path, false).expect("the built-in data loads");
        spare.spare = true;
        spare.trouble(Trouble::Unopened(e));
        spare
    }

    /// Whether this is the stand-in for a profile that could not be opened: it saves nothing,
    /// so what would reach outside (an import, keys, deleting recordings) is refused.
    pub fn is_spare(&self) -> bool {
        self.spare
    }

    pub(crate) fn trouble(&self, t: Trouble) {
        self.troubles
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(t);
    }

    /// What went wrong in the background since the last call; once, for the front end to tell
    /// the learner.
    pub fn take_troubles(&self) -> Vec<Trouble> {
        std::mem::take(&mut *self.troubles.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corrupt_profile_opens_a_spare_and_says_so() {
        let dir = std::env::temp_dir().join(format!("kaiwa-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("broken.kaiwa");
        let garbage = b"this is not a database, not even close".repeat(40);
        std::fs::write(&file, &garbage).unwrap();

        let spare = Backend::or_spare(file.clone(), Backend::open_at(file.clone(), false));

        let troubles = spare.take_troubles();
        assert!(matches!(troubles.as_slice(), [Trouble::Unopened(_)]));
        assert!(spare.take_troubles().is_empty(), "said once");
        // The spare works, and the broken file stays as it was.
        assert!(spare.profile().is_ok());
        assert_eq!(std::fs::read(&file).unwrap(), garbage);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn spare_at(name: &str) -> Backend {
        let dir = std::env::temp_dir().join(format!("kaiwa-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Backend::or_spare(dir.join("x.kaiwa"), Err(anyhow::anyhow!("broken")))
    }

    #[test]
    fn the_spare_says_so_in_the_profile_view() {
        let spare = spare_at("spare-view");
        assert_eq!(crate::views::profile(&spare).unwrap()["spare"], true);
        let ok = crate::TempBackend::new("spare-view-ok", false).unwrap();
        assert_eq!(crate::views::profile(&ok).unwrap()["spare"], false);
    }

    #[test]
    fn the_spare_refuses_what_would_not_be_kept() {
        let spare = spare_at("spare-refuses");
        let now = crate::now();

        let import = spare.import_from("/nonexistent/backup.json");
        assert!(matches!(import, Err(crate::transfer::ImportError::Spare)));

        let rec = spare.set_setting("keepRecordings", "1", now);
        assert!(matches!(rec, Err(crate::settings::SettingError::Spare)));
        assert!(
            spare.store.events().unwrap().is_empty(),
            "a refused change is no event"
        );
        let event = kaiwa_core::Event::Setting {
            key: "keepRecordings".into(),
            value: "1".into(),
        };
        let folded = crate::journal::apply(&spare, "x-1", &event, now);
        assert!(matches!(folded, Err(crate::journal::JournalError::Spare)));

        spare
            .store
            .update_profile(|p| p.tutor.provider = "anthropic".into())
            .unwrap();
        let store = spare.store_key("sk-x");
        assert_eq!(store, Some(Err(kaiwa_ai::secrets::KeyError::Spare)));
        let remove = spare.remove_key();
        assert_eq!(remove, Some(Err(kaiwa_ai::secrets::KeyError::Spare)));
    }

    #[test]
    fn wiping_the_spare_is_the_way_out() {
        let mut spare = spare_at("spare-wipe");
        spare.wipe().unwrap();
        assert!(!spare.is_spare());
    }

    #[test]
    fn a_tutor_that_cannot_be_set_up_reaches_the_learner() {
        // The local test's environment wins over the settings.
        if std::env::var_os("KAIWA_PROVIDER").is_some()
            || std::env::var_os("ANTHROPIC_API_KEY").is_some()
        {
            return;
        }
        let b = crate::TempBackend::new("tutor-no-key", false).unwrap();
        b.store
            .update_profile(|p| p.tutor.provider = "anthropic".into())
            .unwrap();

        // Chosen, but no key anywhere.
        b.provider("n5-01-aisatsu");

        let troubles = b.take_troubles();
        assert!(
            matches!(troubles.as_slice(), [Trouble::TutorUnready(_)]),
            "the unready tutor was only logged"
        );
    }
}
