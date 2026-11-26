//! Moves the devices' journal files (/LF700/, ADR-010): only bytes, no profile. Each device
//! writes its own file and reads the others', so no lock is needed and a folder another program
//! keeps in step (Syncthing, Nextcloud) works the same.
//!
//! ```text
//!   backend ── seal(journal) ──► Remote::put("<device>.kaiwa-sync")
//!   backend ◄─ open(bytes)  ◄── Remote::get(name)   for every name in Remote::list()
//! ```

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use age::secrecy::SecretString;

mod webdav;
pub use webdav::WebDav;

/// The ending of a journal file; other files in the place are left alone.
pub const FILE_END: &str = ".kaiwa-sync";

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("file: {0}")]
    Io(#[from] std::io::Error),
    /// The passphrase does not open the file, or the file is damaged.
    #[error("the passphrase does not open {0}")]
    Sealed(String),
    #[error("no file {0}")]
    Missing(String),
}

pub type Result<T> = std::result::Result<T, SyncError>;

/// A place that holds the devices' files.
pub trait Remote {
    /// The names of the journal files there.
    fn list(&self) -> Result<Vec<String>>;
    fn get(&self, name: &str) -> Result<Vec<u8>>;
    /// Writes `name` whole: a reader sees the old file or the new one, never half.
    fn put(&self, name: &str, bytes: &[u8]) -> Result<()>;
}

/// A folder on this computer, perhaps one another program keeps in step.
pub struct Folder {
    dir: PathBuf,
}

impl Folder {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }
}

impl Remote for Folder {
    fn list(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if name.ends_with(FILE_END) {
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }

    fn get(&self, name: &str) -> Result<Vec<u8>> {
        Ok(std::fs::read(self.dir.join(name))?)
    }

    fn put(&self, name: &str, bytes: &[u8]) -> Result<()> {
        // Written beside it, then renamed: the rename replaces the file at once. The
        // temporary name does not end in `FILE_END`, so no reader takes it.
        std::fs::create_dir_all(&self.dir)?;
        let part = self.dir.join(format!("{name}.part"));
        std::fs::write(&part, bytes)?;
        std::fs::rename(&part, self.dir.join(name))?;
        Ok(())
    }
}

/// Files in memory, for tests.
#[derive(Default)]
pub struct Memory {
    files: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl Remote for Memory {
    fn list(&self) -> Result<Vec<String>> {
        Ok(self.files.lock().unwrap().keys().cloned().collect())
    }

    fn get(&self, name: &str) -> Result<Vec<u8>> {
        let files = self.files.lock().unwrap();
        files
            .get(name)
            .cloned()
            .ok_or_else(|| SyncError::Missing(name.into()))
    }

    fn put(&self, name: &str, bytes: &[u8]) -> Result<()> {
        self.files
            .lock()
            .unwrap()
            .insert(name.into(), bytes.to_vec());
        Ok(())
    }
}

/// Seals `plain` with `passphrase` in the format of `age` (scrypt): the learner can open the
/// file with the `age` tool too (/LF710/).
pub fn seal(plain: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let encryptor = age::Encryptor::with_user_passphrase(secret(passphrase));
    let mut out = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut out)
        .map_err(|e| SyncError::Io(std::io::Error::other(e)))?;
    writer.write_all(plain)?;
    writer.finish()?;
    Ok(out)
}

/// Opens a file `seal` made; `name` names it in the error.
pub fn open(sealed: &[u8], passphrase: &str, name: &str) -> Result<Vec<u8>> {
    let wrong = || SyncError::Sealed(name.into());
    let decryptor = age::Decryptor::new(sealed).map_err(|_| wrong())?;
    let identity = age::scrypt::Identity::new(secret(passphrase));
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| wrong())?;
    let mut plain = Vec::new();
    reader.read_to_end(&mut plain).map_err(|_| wrong())?;
    Ok(plain)
}

fn secret(passphrase: &str) -> SecretString {
    SecretString::from(passphrase.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sealed_file_opens_with_its_passphrase_only() {
        let sealed = seal(b"journal", "kirschbluete").unwrap();
        assert!(!sealed.windows(7).any(|w| w == b"journal"), "plain text");
        assert_eq!(open(&sealed, "kirschbluete", "a").unwrap(), b"journal");
        assert!(matches!(
            open(&sealed, "pflaume", "a"),
            Err(SyncError::Sealed(_))
        ));
        assert!(matches!(
            open(b"no age file", "kirschbluete", "a"),
            Err(SyncError::Sealed(_))
        ));
    }

    #[test]
    fn a_folder_lists_only_journal_files() {
        let dir = std::env::temp_dir().join(format!("kaiwa-sync-folder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let folder = Folder::new(&dir);
        folder.put("b.kaiwa-sync", b"1").unwrap();
        folder.put("a.kaiwa-sync", b"2").unwrap();
        folder.put("a.kaiwa-sync", b"3").unwrap();
        std::fs::write(dir.join("notes.txt"), b"").unwrap();
        let names = folder.list().unwrap();
        let got = folder.get("a.kaiwa-sync").unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(names, ["a.kaiwa-sync", "b.kaiwa-sync"]);
        assert_eq!(got, b"3");
    }
}
