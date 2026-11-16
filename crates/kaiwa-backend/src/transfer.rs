//! Export and import of the profile as a JSON file (Lastenheft /LF710/).

use std::path::PathBuf;

use kaiwa_core::clock::Moment;
use kaiwa_store::StoreError;

use crate::Backend;

const FILE_URL: &str = "file://";

/// Why an import did not happen.
#[derive(Debug, PartialEq, Eq)]
pub enum ImportError {
    /// The file could not be read.
    Unreadable(String),
    /// The file is no export of Kaiwa.
    NotAnExport,
    /// The profile has data already; an import goes into a new one.
    NotEmpty,
    /// The profile could not be opened; nothing an import brings would be kept.
    Spare,
    /// The import failed and the profile is unchanged.
    Broken(String),
}

/// The file a front end names: a path, or a `file://` URL from a file dialog.
fn path_of(target: &str) -> PathBuf {
    PathBuf::from(target.strip_prefix(FILE_URL).unwrap_or(target))
}

impl Backend {
    /// Writes the export to `target`; the file it wrote.
    pub fn export_to(&self, target: &str, now: Moment) -> anyhow::Result<PathBuf> {
        let file = path_of(target);
        let export = self.store.export(now)?;
        std::fs::write(&file, serde_json::to_string_pretty(&export)?)?;
        Ok(file)
    }

    /// Reads an export from `target` into this (empty) profile; the file it read.
    pub fn import_from(&self, target: &str) -> Result<PathBuf, ImportError> {
        if self.spare {
            return Err(ImportError::Spare);
        }
        let file = path_of(target);
        let text =
            std::fs::read_to_string(&file).map_err(|e| ImportError::Unreadable(e.to_string()))?;
        let export = serde_json::from_str(&text).map_err(|_| ImportError::NotAnExport)?;
        self.store.import(&export).map_err(|e| match e {
            StoreError::NotEmpty => ImportError::NotEmpty,
            StoreError::WrongFormat(_) => ImportError::NotAnExport,
            e => ImportError::Broken(e.to_string()),
        })?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TempBackend;

    const T0: Moment = Moment {
        unix: 1_791_100_000,
        offset_min: 120,
    };

    fn backend(name: &str) -> TempBackend {
        TempBackend::new(&format!("xfer-{name}"), false).unwrap()
    }

    #[test]
    fn an_export_goes_into_a_new_profile() {
        let a = backend("from");
        a.set_setting("name", "Lena", T0).unwrap();
        a.set_setting("goal", "25", T0).unwrap();
        let file = a.dir.join("out.json");

        // A file:// URL from the file dialog works as a path.
        let url = format!("{FILE_URL}{}", file.display());
        assert_eq!(a.export_to(&url, T0).unwrap(), file);

        let b = backend("to");
        assert_eq!(b.import_from(file.to_str().unwrap()), Ok(file));
        let p = b.store.profile().unwrap();
        assert_eq!(p.name, "Lena");
        assert_eq!(p.daily_goal, kaiwa_core::progress::DailyGoal::Minutes(25));
    }

    #[test]
    fn a_second_import_is_refused() {
        let a = backend("full");
        let file = a.dir.join("out.json");
        let target = file.to_str().unwrap();
        a.export_to(target, T0).unwrap();
        assert!(a.import_from(target).is_ok());
        assert_eq!(a.import_from(target), Err(ImportError::NotEmpty));
    }

    #[test]
    fn a_file_that_is_no_export_is_named_so() {
        let b = backend("wrong");
        let text = b.dir.join("text.txt");
        std::fs::write(&text, "no json").unwrap();
        assert_eq!(
            b.import_from(text.to_str().unwrap()),
            Err(ImportError::NotAnExport)
        );

        // Valid JSON of another shape.
        let other = b.dir.join("other.json");
        std::fs::write(&other, r#"{"schema": "something-else"}"#).unwrap();
        assert_eq!(
            b.import_from(other.to_str().unwrap()),
            Err(ImportError::NotAnExport)
        );
    }

    #[test]
    fn a_missing_file_is_unreadable() {
        let b = backend("missing");
        let gone = b.dir.join("gone.json");
        assert!(matches!(
            b.import_from(gone.to_str().unwrap()),
            Err(ImportError::Unreadable(_))
        ));
    }
}
