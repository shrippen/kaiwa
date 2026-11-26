//! Sync over a place of files (/LF700/, ADR-010 step 5). Each device keeps one sealed file
//! there with its whole journal; syncing reads the others', merges them and writes its own
//! again. The first device makes its state the journal's start; another joins with an empty
//! profile.
//!
//! ```text
//!   folder/  3f9a….kaiwa-sync   (PC)     c07e….kaiwa-sync   (phone)
//!   sync_now on the PC:  open + merge c07e…  ─►  seal + put 3f9a…
//! ```

use std::path::Path;

use kaiwa_core::clock::Moment;
use kaiwa_sync::{FILE_END, Folder, Remote};
use serde_json::{Value, json};

use crate::Backend;
use crate::merge::MergeError;

/// The shape of a journal file; a device that reads a higher one refuses it (ADR-010).
const FORMAT: u64 = 1;

/// Why a sync stopped. What was merged before the error stays; this device's file is not
/// written.
#[derive(Debug)]
pub enum SyncError {
    /// The place could not be read or written, or the passphrase does not open a file.
    Place(kaiwa_sync::SyncError),
    /// A file is from a newer Kaiwa; this one does not fold its events.
    Newer(String),
    /// A file is no journal file.
    Broken(String),
    /// The journal of a file was not taken.
    Merge(MergeError),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Place(e) => write!(f, "{e}"),
            Self::Newer(name) => write!(f, "{name} is from a newer Kaiwa"),
            Self::Broken(name) => write!(f, "{name} is no journal file"),
            Self::Merge(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<kaiwa_sync::SyncError> for SyncError {
    fn from(e: kaiwa_sync::SyncError) -> Self {
        Self::Place(e)
    }
}

impl From<MergeError> for SyncError {
    fn from(e: MergeError) -> Self {
        Self::Merge(e)
    }
}

/// What a sync did.
#[derive(Debug, PartialEq, Eq)]
pub struct Synced {
    /// Events of other devices that were new here.
    pub taken: usize,
    /// The other devices' files read.
    pub devices: usize,
}

impl Backend {
    /// Syncs with the other devices' files in `folder`, sealed with `passphrase`. A front
    /// end calls it while no talk is open: the merge rebuilds the talks.
    pub fn sync_now(
        &self,
        folder: &Path,
        passphrase: &str,
        now: Moment,
    ) -> Result<Synced, SyncError> {
        self.sync_with(&Folder::new(folder), passphrase, now)
    }

    /// [`Backend::sync_now`] with any place.
    pub(crate) fn sync_with(
        &self,
        place: &dyn Remote,
        passphrase: &str,
        now: Moment,
    ) -> Result<Synced, SyncError> {
        let not_saved = |e: anyhow::Error| SyncError::Merge(MergeError::NotSaved(e));
        let own = format!(
            "{}{FILE_END}",
            self.store.device().map_err(|e| not_saved(e.into()))?
        );
        let others: Vec<String> = place.list()?.into_iter().filter(|n| *n != own).collect();

        // Alone in the place: this device's state is where the journal begins. An empty
        // profile waits instead: the first device's file may not have reached this copy of
        // the folder yet, and a start of its own would never merge with that one.
        if others.is_empty() {
            if self.store.is_empty().map_err(|e| not_saved(e.into()))? {
                return Ok(Synced {
                    taken: 0,
                    devices: 0,
                });
            }
            self.share_base(now)?;
        }
        let mut taken = 0;
        for name in &others {
            let journal = read(place, name, passphrase)?;
            taken += self.merge(&journal, now)?;
        }

        let file = json!({ "format": FORMAT, "events": self.journal().map_err(not_saved)? });
        let plain = serde_json::to_vec(&file).map_err(|e| not_saved(e.into()))?;
        place.put(&own, &kaiwa_sync::seal(&plain, passphrase)?)?;
        Ok(Synced {
            taken,
            devices: others.len(),
        })
    }
}

/// The events of another device's file.
fn read(place: &dyn Remote, name: &str, passphrase: &str) -> Result<Vec<Value>, SyncError> {
    let plain = kaiwa_sync::open(&place.get(name)?, passphrase, name)?;
    let broken = || SyncError::Broken(name.into());
    let file: Value = serde_json::from_slice(&plain).map_err(|_| broken())?;
    let format = file["format"].as_u64().ok_or_else(broken)?;
    if format > FORMAT {
        return Err(SyncError::Newer(name.into()));
    }
    file["events"].as_array().cloned().ok_or_else(broken)
}

#[cfg(test)]
mod tests {
    use kaiwa_core::srs::{Item, ItemKind, Source};
    use kaiwa_sync::Memory;

    use super::*;
    use crate::TempBackend;

    const T0: Moment = Moment {
        unix: 1_791_100_000,
        offset_min: 120,
    };
    const PASS: &str = "kirschbluete";

    fn backend(name: &str) -> TempBackend {
        TempBackend::new(&format!("sync-{name}"), false).unwrap()
    }

    #[test]
    fn two_devices_sync_through_one_place() {
        let place = Memory::default();
        let pc = backend("pc");
        let item = Item::new("駅", ItemKind::Word, Source::NewWord, T0);
        pc.store.put_item(&item).unwrap();
        let first = pc.sync_with(&place, PASS, T0).unwrap();
        assert_eq!(
            first,
            Synced {
                taken: 0,
                devices: 0
            }
        );

        // The phone joins with an empty profile and reviews the card.
        let phone = backend("phone");
        assert_eq!(phone.sync_with(&place, PASS, T0).unwrap().devices, 1);
        phone.answer_review("駅", "駅", false, T0.plus_secs(60));
        phone.sync_with(&place, PASS, T0).unwrap();

        let back = pc.sync_with(&place, PASS, T0).unwrap();
        assert_eq!(
            back,
            Synced {
                taken: 1,
                devices: 1
            }
        );
        assert_eq!(pc.store.day(T0.day()).unwrap().reviews_done, 1);
    }

    #[test]
    fn an_empty_device_waits_for_the_first_ones_file() {
        // A folder another program keeps in step: the phone's copy has not got the PC's file
        // yet when the phone syncs.
        let (at_pc, at_phone) = (Memory::default(), Memory::default());
        let pc = backend("wait-pc");
        let item = Item::new("駅", ItemKind::Word, Source::NewWord, T0);
        pc.store.put_item(&item).unwrap();
        pc.sync_with(&at_pc, PASS, T0).unwrap();
        let phone = backend("wait-phone");
        let alone = phone.sync_with(&at_phone, PASS, T0).unwrap();
        assert_eq!(
            alone,
            Synced {
                taken: 0,
                devices: 0
            }
        );
        assert!(
            at_phone.list().unwrap().is_empty(),
            "the empty phone wrote a file"
        );

        // The copies meet: the phone joins, the PC takes the phone's file.
        let copy = |from: &Memory, to: &Memory| {
            for name in from.list().unwrap() {
                to.put(&name, &from.get(&name).unwrap()).unwrap();
            }
        };
        copy(&at_pc, &at_phone);
        phone.sync_with(&at_phone, PASS, T0).unwrap();
        copy(&at_phone, &at_pc);
        pc.sync_with(&at_pc, PASS, T0).unwrap();
        assert_eq!(phone.store.items().unwrap().len(), 1);
    }

    #[test]
    fn a_wrong_passphrase_or_a_newer_file_writes_nothing() {
        let place = Memory::default();
        let pc = backend("pass-pc");
        let item = Item::new("駅", ItemKind::Word, Source::NewWord, T0);
        pc.store.put_item(&item).unwrap();
        pc.sync_with(&place, PASS, T0).unwrap();
        let phone = backend("pass-phone");
        assert!(matches!(
            phone.sync_with(&place, "pflaume", T0),
            Err(SyncError::Place(kaiwa_sync::SyncError::Sealed(_)))
        ));
        assert_eq!(place.list().unwrap().len(), 1, "the phone wrote its file");

        let newer = json!({ "format": FORMAT + 1, "events": [] });
        let sealed = kaiwa_sync::seal(newer.to_string().as_bytes(), PASS).unwrap();
        place.put(&format!("later{FILE_END}"), &sealed).unwrap();
        assert!(matches!(
            pc.sync_with(&place, PASS, T0),
            Err(SyncError::Newer(_))
        ));
    }
}
