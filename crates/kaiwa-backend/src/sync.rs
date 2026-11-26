//! Sync over a place of files (/LF700/, ADR-010 step 5). Each device keeps one sealed file
//! there with its whole journal; syncing reads the others', merges them and writes its own
//! again. On its first sync the learner says whether the device begins (its state is the
//! journal's start) or joins with an empty profile.
//!
//! ```text
//!   folder/  3f9a….kaiwa-sync   (PC)     c07e….kaiwa-sync   (phone)
//!   sync_now on the PC:  open + merge c07e…  ─►  seal + put 3f9a…
//! ```

use kaiwa_ai::secrets::{KeyError, SYNC_SECRETS};
use kaiwa_core::clock::Moment;
use kaiwa_store::SyncPlace;
use kaiwa_sync::{FILE_END, Folder, Remote, WebDav};
use serde_json::{Value, json};

use crate::Backend;
use crate::merge::MergeError;

/// The shape of a journal file; a device that reads a higher one refuses it (ADR-010).
const FORMAT: u64 = 1;

/// How a device that does not sync yet takes part; the learner says it once (ADR-010).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// This device's state is where the journal begins.
    Here,
    /// An empty profile takes the journal of the devices already in the place.
    Join,
}

/// A secret of the sync, kept in the keyring (/LF713/).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    /// Seals the files (`age`).
    Passphrase,
    /// The WebDAV folder's password.
    Password,
}

impl Secret {
    /// Its name in the keyring.
    fn name(self) -> &'static str {
        match self {
            Self::Passphrase => SYNC_SECRETS[0],
            Self::Password => SYNC_SECRETS[1],
        }
    }
}

/// Why a sync stopped. What was merged before the error stays; this device's file is not
/// written.
#[derive(Debug)]
pub enum SyncError {
    /// No place or no passphrase is set up.
    NotSetUp,
    /// The keyring holds a secret of the sync but cannot be read (locked, not running).
    Keyring(String),
    /// A WebDAV address without `https` to another computer: the password would go out
    /// readable.
    Plain,
    /// "Begin here" in a place where devices sync already: join instead.
    Taken,
    /// "Join" with a profile that has data and no device in the place yet.
    Alone,
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
            Self::NotSetUp => f.write_str("no place or no passphrase set up"),
            Self::Keyring(e) => write!(f, "the keyring cannot be read: {e}"),
            Self::Plain => f.write_str("a WebDAV address needs https"),
            Self::Taken => f.write_str("devices sync in this place already"),
            Self::Alone => f.write_str("no device to join in this place"),
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

/// A sync in stages (ADR-010): [`Backend::sync_job`] and [`Backend::sync_take`] need the
/// profile; [`SyncJob::fetch`] and [`SyncJob::send`] move and seal files, which takes
/// seconds (scrypt, the network), and need no backend. A front end holds its profile only
/// for the two short stages:
///
/// ```text
///   sync_job (profile) ─► fetch (files) ─► sync_take (profile) ─► send (files)
/// ```
pub struct SyncJob {
    place: Box<dyn Remote + Send>,
    passphrase: String,
    /// This device's file.
    own: String,
    start: Start,
}

/// The other devices' journals, opened.
pub struct Fetched(Vec<(String, Vec<Value>)>);

/// What goes out after the merge: this device's file, or nothing while an empty profile waits.
pub struct Outgoing {
    plain: Option<Vec<u8>>,
    synced: Synced,
}

impl SyncJob {
    /// Reads and opens the other devices' files.
    pub fn fetch(&self) -> Result<Fetched, SyncError> {
        fetch(&*self.place, &self.own, &self.passphrase)
    }

    /// Seals and writes this device's file.
    pub fn send(&self, out: Outgoing) -> Result<Synced, SyncError> {
        send(&*self.place, &self.own, &self.passphrase, out)
    }
}

impl Backend {
    /// Syncs with the place set up in the settings, all stages at once. `start` counts only
    /// while the device does not sync yet. A front end calls it while no talk is open: the
    /// merge rebuilds the talks.
    pub fn sync_now(&self, start: Start, now: Moment) -> Result<Synced, SyncError> {
        let job = self.sync_job(start)?;
        let fetched = job.fetch()?;
        let out = self.sync_take(&job, fetched, now)?;
        job.send(out)
    }

    /// The first stage: the place, the passphrase and this device's file from the settings.
    pub fn sync_job(&self, start: Start) -> Result<SyncJob, SyncError> {
        let passphrase = self.sync_secret(Secret::Passphrase)?;
        let profile = self.store.profile().map_err(|e| not_saved(e.into()))?;
        let place: Box<dyn Remote + Send> = match profile.sync {
            SyncPlace::None => return Err(SyncError::NotSetUp),
            SyncPlace::Folder { path } => Box::new(Folder::new(path)),
            SyncPlace::WebDav { url, .. } if url.is_empty() => return Err(SyncError::NotSetUp),
            SyncPlace::WebDav { url, .. } if !sealed_way(&url) => return Err(SyncError::Plain),
            SyncPlace::WebDav { url, user } => {
                let password = self.sync_secret(Secret::Password)?;
                Box::new(WebDav::new(&url, &user, &password))
            }
        };
        Ok(SyncJob {
            place,
            passphrase,
            own: self.own_file()?,
            start,
        })
    }

    /// The third stage: the first sync as the learner chose, then the other journals merged;
    /// what goes out.
    pub fn sync_take(
        &self,
        job: &SyncJob,
        fetched: Fetched,
        now: Moment,
    ) -> Result<Outgoing, SyncError> {
        self.take(fetched, job.start, now)
    }

    fn own_file(&self) -> Result<String, SyncError> {
        let device = self.store.device().map_err(|e| not_saved(e.into()))?;
        Ok(format!("{device}{FILE_END}"))
    }

    fn take(&self, fetched: Fetched, start: Start, now: Moment) -> Result<Outgoing, SyncError> {
        let devices = fetched.0.len();
        if !self.syncs().map_err(not_saved)? && self.first_sync(start, devices == 0, now)? {
            let synced = Synced {
                taken: 0,
                devices: 0,
            };
            return Ok(Outgoing {
                plain: None,
                synced,
            });
        }
        let mut taken = 0;
        for (_, journal) in &fetched.0 {
            taken += self.merge(journal, now)?;
        }
        let file = json!({ "format": FORMAT, "events": self.journal().map_err(not_saved)? });
        let plain = serde_json::to_vec(&file).map_err(|e| not_saved(e.into()))?;
        Ok(Outgoing {
            plain: Some(plain),
            synced: Synced { taken, devices },
        })
    }

    /// Keeps a secret of the sync in the keyring; a profile without a keyring home (the
    /// environment's, a test's) keeps it for this run only. Spaces around it are cut, as
    /// both ways keep it, so every device seals with the same passphrase.
    pub fn set_sync_secret(&self, secret: Secret, value: &str) -> Result<(), KeyError> {
        if self.spare {
            return Err(KeyError::Spare);
        }
        let value = value.trim();
        if value.is_empty() {
            return Err(KeyError::Empty);
        }
        if self.keys.keyring_home() {
            return self.keys.store(secret.name(), value);
        }
        self.sync_secrets
            .lock()
            .unwrap()
            .insert(secret.name(), value.to_string());
        Ok(())
    }

    /// Whether `secret` is kept; the keyring's error when it cannot be read.
    pub(crate) fn sync_secret_kept(&self, secret: Secret) -> Result<bool, String> {
        match self.sync_secret(secret) {
            Ok(_) => Ok(true),
            Err(SyncError::Keyring(e)) => Err(e),
            Err(_) => Ok(false),
        }
    }

    fn sync_secret(&self, secret: Secret) -> Result<String, SyncError> {
        if let Some(v) = self.sync_secrets.lock().unwrap().get(secret.name()) {
            return Ok(v.clone());
        }
        self.keys
            .secret(secret.name())
            .map_err(SyncError::Keyring)?
            .ok_or(SyncError::NotSetUp)
    }

    /// All stages with any place (tests).
    #[cfg(test)]
    pub(crate) fn sync_with(
        &self,
        place: &dyn Remote,
        passphrase: &str,
        start: Start,
        now: Moment,
    ) -> Result<Synced, SyncError> {
        let own = self.own_file()?;
        let fetched = fetch(place, &own, passphrase)?;
        let out = self.take(fetched, start, now)?;
        send(place, &own, passphrase, out)
    }

    /// The first sync of a device as the learner chose; whether it waits. An empty profile
    /// that joins and finds no device yet waits: in a folder another program keeps in step
    /// the first device's file may come later.
    fn first_sync(&self, start: Start, alone: bool, now: Moment) -> Result<bool, SyncError> {
        match (start, alone) {
            (Start::Here, true) => {
                self.share_base(now)?;
                Ok(false)
            }
            (Start::Here, false) => Err(SyncError::Taken),
            (Start::Join, true) if self.store.is_empty().map_err(|e| not_saved(e.into()))? => {
                Ok(true)
            }
            (Start::Join, true) => Err(SyncError::Alone),
            (Start::Join, false) => Ok(false),
        }
    }
}

/// Whether the login goes out sealed: `https`, or `http` to this computer only.
pub(crate) fn sealed_way(url: &str) -> bool {
    let url = url.to_ascii_lowercase();
    let Some(rest) = url.strip_prefix("http://") else {
        return url.starts_with("https://");
    };
    let host = rest.split(['/', ':']).next().unwrap_or_default();
    matches!(host, "localhost" | "127.0.0.1")
}

fn not_saved(e: anyhow::Error) -> SyncError {
    SyncError::Merge(MergeError::NotSaved(e))
}

/// All stages under a front end's lock on its backend: `lock` is taken for the two short
/// stages only and given back before the slow ones (fetch, send).
pub fn sync_locked<G: std::ops::Deref<Target = Backend>>(
    lock: impl Fn() -> G,
    start: Start,
    now: Moment,
) -> Result<Synced, SyncError> {
    // Each lock in a statement of its own: a guard lives to the end of its statement, and a
    // second lock inside that statement would wait for it for good.
    let job = lock().sync_job(start)?;
    let fetched = job.fetch()?;
    let out = lock().sync_take(&job, fetched, now)?;
    job.send(out)
}

/// Reads and opens every other device's file in `place`.
fn fetch(place: &dyn Remote, own: &str, passphrase: &str) -> Result<Fetched, SyncError> {
    let mut out = Vec::new();
    for name in place.list()?.into_iter().filter(|n| n != own) {
        let journal = read(place, &name, passphrase)?;
        out.push((name, journal));
    }
    Ok(Fetched(out))
}

/// Seals and writes `own`, unless the sync waits.
fn send(
    place: &dyn Remote,
    own: &str,
    passphrase: &str,
    out: Outgoing,
) -> Result<Synced, SyncError> {
    if let Some(plain) = out.plain {
        place.put(own, &kaiwa_sync::seal(&plain, passphrase)?)?;
    }
    Ok(out.synced)
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
    use kaiwa_core::learner::{LearnerModel, Skill};
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

    /// A device with a card: it has data of its own.
    fn with_card(name: &str) -> TempBackend {
        let b = backend(name);
        let item = Item::new("駅", ItemKind::Word, Source::NewWord, T0);
        b.store.put_item(&item).unwrap();
        b
    }

    #[test]
    fn two_devices_sync_through_one_place() {
        let place = Memory::default();
        let pc = with_card("pc");
        let first = pc.sync_with(&place, PASS, Start::Here, T0).unwrap();
        assert_eq!(
            first,
            Synced {
                taken: 0,
                devices: 0
            }
        );

        // The phone joins with an empty profile and reviews the card.
        let phone = backend("phone");
        let joined = phone.sync_with(&place, PASS, Start::Join, T0).unwrap();
        assert_eq!(joined.devices, 1);
        phone.answer_review("駅", "駅", false, T0.plus_secs(60));
        phone.sync_with(&place, PASS, Start::Join, T0).unwrap();

        let back = pc.sync_with(&place, PASS, Start::Here, T0).unwrap();
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
    fn a_device_with_only_a_placement_begins_when_told() {
        // Only the placement: no talk, no card, so the profile counts as empty for a join.
        let place = Memory::default();
        let pc = backend("placed-pc");
        let model = LearnerModel::placed(&[(Skill::Speaking, "n4")]);
        pc.set_placement(model).unwrap();
        pc.sync_with(&place, PASS, Start::Here, T0).unwrap();
        assert_eq!(place.list().unwrap().len(), 1, "the PC wrote no file");

        let phone = backend("placed-phone");
        phone.sync_with(&place, PASS, Start::Join, T0).unwrap();
        assert!(phone.store.profile().unwrap().placed);
    }

    #[test]
    fn an_empty_device_waits_for_the_first_ones_file() {
        // A folder another program keeps in step: the phone's copy has not got the PC's file
        // yet when the phone syncs.
        let (at_pc, at_phone) = (Memory::default(), Memory::default());
        let pc = with_card("wait-pc");
        pc.sync_with(&at_pc, PASS, Start::Here, T0).unwrap();
        let phone = backend("wait-phone");
        let alone = phone.sync_with(&at_phone, PASS, Start::Join, T0).unwrap();
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
        phone.sync_with(&at_phone, PASS, Start::Join, T0).unwrap();
        copy(&at_phone, &at_pc);
        pc.sync_with(&at_pc, PASS, Start::Here, T0).unwrap();
        assert_eq!(phone.store.items().unwrap().len(), 1);
    }

    #[test]
    fn a_start_that_does_not_fit_the_place_writes_nothing() {
        let place = Memory::default();
        let full = with_card("alone");
        assert!(matches!(
            full.sync_with(&place, PASS, Start::Join, T0),
            Err(SyncError::Alone)
        ));
        assert!(place.list().unwrap().is_empty());

        with_card("taken-pc")
            .sync_with(&place, PASS, Start::Here, T0)
            .unwrap();
        let second = backend("taken");
        assert!(matches!(
            second.sync_with(&place, PASS, Start::Here, T0),
            Err(SyncError::Taken)
        ));
        assert_eq!(place.list().unwrap().len(), 1);
    }

    #[test]
    fn a_wrong_passphrase_or_a_newer_file_writes_nothing() {
        let place = Memory::default();
        let pc = with_card("pass-pc");
        pc.sync_with(&place, PASS, Start::Here, T0).unwrap();
        let phone = backend("pass-phone");
        assert!(matches!(
            phone.sync_with(&place, "pflaume", Start::Join, T0),
            Err(SyncError::Place(kaiwa_sync::SyncError::Sealed(_)))
        ));
        assert_eq!(place.list().unwrap().len(), 1, "the phone wrote its file");

        let newer = json!({ "format": FORMAT + 1, "events": [] });
        let sealed = kaiwa_sync::seal(newer.to_string().as_bytes(), PASS).unwrap();
        place.put(&format!("later{FILE_END}"), &sealed).unwrap();
        assert!(matches!(
            pc.sync_with(&place, PASS, Start::Here, T0),
            Err(SyncError::Newer(_))
        ));
    }

    #[test]
    fn a_sync_under_the_front_ends_lock_takes_it_only_for_the_short_stages() {
        // The app keeps its backend in a mutex and locks it per call: a stage that took the
        // lock twice in one thread would hang for good.
        let dir = std::env::temp_dir().join(format!("kaiwa-sync-locked-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let folder = dir.join("shared");
        std::fs::create_dir_all(&folder).unwrap();
        let b = Backend::open_at(dir.join("p.kaiwa"), false).unwrap();
        let item = Item::new("駅", ItemKind::Word, Source::NewWord, T0);
        b.store.put_item(&item).unwrap();
        b.set_setting("syncFolder", folder.to_str().unwrap(), T0)
            .unwrap();
        b.set_sync_secret(Secret::Passphrase, PASS).unwrap();
        let shared = std::sync::Arc::new(std::sync::Mutex::new(b));

        let (done, finished) = std::sync::mpsc::channel();
        let state = shared.clone();
        std::thread::spawn(move || {
            let r = sync_locked(|| state.lock().unwrap(), Start::Here, T0);
            done.send(r.is_ok()).unwrap();
        });
        let ok = finished.recv_timeout(std::time::Duration::from_secs(60));
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(ok, Ok(true), "the sync hung or failed");
    }

    #[test]
    fn files_move_while_the_backend_is_free() {
        // The slow stages run in another thread while the profile takes a setting.
        let b = with_card("free");
        let folder = b.dir.join("shared");
        std::fs::create_dir_all(&folder).unwrap();
        b.set_setting("syncFolder", folder.to_str().unwrap(), T0)
            .unwrap();
        b.set_sync_secret(Secret::Passphrase, PASS).unwrap();
        let job = b.sync_job(Start::Here).unwrap();
        let fetching = std::thread::spawn(move || {
            let fetched = job.fetch();
            (job, fetched)
        });
        b.set_setting("goal", "20", T0).unwrap();
        let (job, fetched) = fetching.join().unwrap();
        let out = b.sync_take(&job, fetched.unwrap(), T0).unwrap();
        let sending = std::thread::spawn(move || job.send(out));
        b.set_setting("goal", "15", T0).unwrap();
        sending.join().unwrap().unwrap();
        assert_eq!(Folder::new(&folder).list().unwrap().len(), 1);
    }

    #[test]
    fn webdav_without_https_goes_only_to_this_computer() {
        assert!(sealed_way("https://cloud.example.org/dav/"));
        assert!(sealed_way("http://localhost:8080/dav/"));
        assert!(!sealed_way("http://cloud.example.org/dav/"));
        assert!(!sealed_way("ftp://cloud.example.org/"));
        let b = backend("plain");
        b.set_setting("syncUrl", "http://cloud.example.org/dav/", T0)
            .unwrap();
        b.set_sync_secret(Secret::Passphrase, PASS).unwrap();
        assert!(matches!(b.sync_now(Start::Here, T0), Err(SyncError::Plain)));
    }

    #[test]
    fn a_passphrase_is_kept_without_its_spaces() {
        let b = backend("spaces");
        b.set_sync_secret(Secret::Passphrase, " kirschbluete ")
            .unwrap();
        assert_eq!(b.sync_secret(Secret::Passphrase).unwrap(), PASS);
    }

    #[test]
    fn a_locked_keyring_is_named_not_a_missing_passphrase() {
        struct Locked;
        impl kaiwa_ai::secrets::KeyStore for Locked {
            fn get(&self, _: &str) -> Result<Option<String>, String> {
                Err("locked".into())
            }
            fn set(&self, _: &str, _: &str) -> Result<(), String> {
                Err("locked".into())
            }
            fn delete(&self, _: &str) -> Result<(), String> {
                Ok(())
            }
        }
        let b = backend("locked");
        let dir = b.dir.join("profiles");
        std::fs::create_dir_all(&dir).unwrap();
        let keyed = Backend::open_with(
            dir.join("p.kaiwa"),
            false,
            std::sync::Arc::new(Locked),
            Some(&dir),
        )
        .unwrap();
        keyed
            .set_setting("syncFolder", dir.to_str().unwrap(), T0)
            .unwrap();
        let r = keyed.sync_now(Start::Here, T0);
        assert!(matches!(r, Err(SyncError::Keyring(_))), "{r:?}");
        let view = crate::sync_view::sync_view(&keyed, kaiwa_core::Lang::De);
        let text = view["text"].as_str().unwrap();
        assert!(text.contains("Schlüsselbund"), "{text}");
    }

    #[test]
    fn sync_now_takes_the_place_and_the_passphrase_of_the_settings() {
        let pc = with_card("settings");
        assert!(matches!(
            pc.sync_now(Start::Here, T0),
            Err(SyncError::NotSetUp)
        ));
        let folder = pc.dir.join("shared");
        std::fs::create_dir_all(&folder).unwrap();
        pc.set_setting("syncFolder", folder.to_str().unwrap(), T0)
            .unwrap();
        assert!(matches!(
            pc.sync_now(Start::Here, T0),
            Err(SyncError::NotSetUp)
        ));
        // A temporary profile has no keyring home: the passphrase is kept for this run.
        pc.set_sync_secret(Secret::Passphrase, PASS).unwrap();
        pc.sync_now(Start::Here, T0).unwrap();
        assert_eq!(Folder::new(&folder).list().unwrap().len(), 1);
        assert!(pc.store.events().unwrap().iter().all(|e| !matches!(
            &e.event,
            kaiwa_core::Event::Setting { key, .. } if key.starts_with("sync")
        )));
    }
}
