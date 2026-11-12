//! Several profiles on one device (Lastenheft /LF720/), e.g. a family: one file each in the
//! profiles directory, fully separate (progress, review, settings, costs). The file `current`
//! names the one in use; the app follows it (and `kaiwad` with M4).

use std::path::{Path, PathBuf};

use kaiwa_store::{Store, profiles_in};
use serde_json::{Value, json};

use crate::Backend;

const CURRENT: &str = "current";
const DEFAULT: &str = "default.kaiwa";

/// The profile in use in `dir` (`default.kaiwa` until another is chosen).
pub fn current_in(dir: &Path) -> PathBuf {
    std::fs::read_to_string(dir.join(CURRENT))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.contains('/') && s.ends_with(".kaiwa"))
        .map(|f| dir.join(f))
        .filter(|p| p.exists())
        .unwrap_or_else(|| dir.join(DEFAULT))
}

/// Makes `file` (a name in `dir`) the profile in use.
pub fn set_current(dir: &Path, file: &str) -> anyhow::Result<PathBuf> {
    anyhow::ensure!(
        !file.contains('/') && file.ends_with(".kaiwa"),
        "not a profile file: {file}"
    );
    let path = dir.join(file);
    anyhow::ensure!(path.exists(), "no profile {file}");
    std::fs::write(dir.join(CURRENT), file)?;
    Ok(path)
}

/// A file name for a new profile called `name`: letters and digits, unique in `dir`.
fn file_for(dir: &Path, name: &str) -> String {
    let base: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() {
        "profil".to_string()
    } else {
        base
    };
    let mut n = 1;
    loop {
        let f = if n == 1 {
            format!("{base}.kaiwa")
        } else {
            format!("{base}-{n}.kaiwa")
        };
        if !dir.join(&f).exists() {
            return f;
        }
        n += 1;
    }
}

/// Creates a profile called `name` (UI language taken over from `lang`) and makes it current.
pub fn create(dir: &Path, name: &str, lang: &str) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let file = file_for(dir, name);
    let store = Store::open(&dir.join(&file))?;
    let mut p = store.profile()?;
    p.name = name.trim().to_string();
    p.ui_lang = lang.to_string();
    store.set_profile(&p)?;
    drop(store);
    set_current(dir, &file)
}

impl Backend {
    /// Deletes every piece of data of this profile (/LF710/): recordings, then the database
    /// files; an empty profile with the same language takes its place.
    pub fn wipe(&mut self) -> anyhow::Result<()> {
        // The emptied profile keeps the language the page is in.
        let lang = self.store.profile().map(|p| p.ui_lang).ok();
        if let Err(e) = self.delete_recordings() {
            eprintln!("kaiwa: {e:#}");
        }

        // Close the file before removing it. Whether removing works or not, the file is opened
        // again: on a failure what follows must not go to memory, lost at the next start.
        // Only when opening itself fails (a full disk) does the store stay in memory, and the
        // error says so.
        self.store = Store::in_memory()?;
        let deleted = kaiwa_store::delete_profile(&self.path);
        self.store = Store::open(&self.path)?;
        deleted?;

        if let Some(lang) = lang {
            let mut p = self.store.profile()?;
            p.ui_lang = lang;
            self.store.set_profile(&p)?;
        }
        Ok(())
    }

    /// The directory of this profile and its siblings.
    pub fn dir(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Every profile in the directory: file, name, whether it is this one.
    pub fn profiles(&self) -> Vec<Value> {
        let mut files = profiles_in(&self.dir());
        if !files.contains(&self.path) {
            files.push(self.path.clone());
        }
        files
            .iter()
            .map(|f| {
                let name = if *f == self.path {
                    self.store.profile().map(|p| p.name).unwrap_or_default()
                } else {
                    Store::open(f)
                        .and_then(|s| s.profile())
                        .map(|p| p.name)
                        .unwrap_or_default()
                };
                let file = f
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                json!({
                    "file": file,
                    "name": if name.is_empty() { file.trim_end_matches(".kaiwa").to_string() } else { name },
                    "current": *f == self.path,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_failed_delete_keeps_the_profile_on_disk() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("kaiwa-wipe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(DEFAULT);
        let mut b = Backend::open_at(path.clone(), false).unwrap();

        // A directory the files cannot be removed from. Root may write there anyway: then
        // there is nothing to show.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        if std::fs::write(dir.join("probe"), b"").is_ok() {
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
            drop(b);
            std::fs::remove_dir_all(&dir).unwrap();
            return;
        }
        assert!(b.wipe().is_err());
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();

        // What happens next still lands in the file, not in memory lost at the next start.
        let mut p = b.store.profile().unwrap();
        p.name = "Lena".into();
        b.store.set_profile(&p).unwrap();
        assert_eq!(Store::open(&path).unwrap().profile().unwrap().name, "Lena");

        drop(b);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn profiles_are_separate_and_one_is_current() {
        let dir = std::env::temp_dir().join(format!("kaiwa-profiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(current_in(&dir), dir.join(DEFAULT));
        let mara = Backend::open_at(current_in(&dir), false).unwrap();
        let mut p = mara.store.profile().unwrap();
        p.name = "Mara".into();
        p.daily_goal = kaiwa_core::progress::DailyGoal::Minutes(10);
        mara.store.set_profile(&p).unwrap();

        let tom = create(&dir, "Tom Müller", "en").unwrap();
        assert_eq!(tom, dir.join("tom-m-ller.kaiwa"));
        assert_eq!(current_in(&dir), tom);
        let tom = Backend::open_at(tom, false).unwrap();
        let tp = tom.store.profile().unwrap();
        assert_eq!(
            (tp.name.as_str(), tp.ui_lang.as_str()),
            ("Tom Müller", "en")
        );
        assert_ne!(tp.daily_goal, p.daily_goal, "settings stay per profile");
        assert!(!tp.placed, "a new profile starts with the placement");

        let list = tom.profiles();
        assert_eq!(list.len(), 2);
        assert!(
            list.iter()
                .any(|v| v["name"] == "Mara" && v["current"] == false)
        );
        assert_eq!(
            create(&dir, "Tom Müller", "de").unwrap(),
            dir.join("tom-m-ller-2.kaiwa")
        );
        set_current(&dir, "default.kaiwa").unwrap();
        assert_eq!(current_in(&dir), dir.join(DEFAULT));
        assert!(set_current(&dir, "../x.kaiwa").is_err());
        drop((mara, tom));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
