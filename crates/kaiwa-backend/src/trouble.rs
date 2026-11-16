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
        let spare = Self::assemble(db, keys, path, false).expect("the built-in data loads");
        spare.trouble(Trouble::Unopened(e));
        spare
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
}
