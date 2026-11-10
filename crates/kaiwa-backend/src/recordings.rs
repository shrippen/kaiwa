//! Recordings of the learner's turns (Lastenheft /LF712/): none without the opt-in. With it,
//! each spoken turn is kept as a WAV file next to the profile (`recordings/<profile>/`), named
//! by the time it was said, and deleted after the chosen number of days; turning the setting
//! off deletes them at once. The transcripts stay in the sessions. Deleting the profile
//! deletes them too.

use std::path::{Path, PathBuf};

use kaiwa_core::clock::Moment;

use crate::Backend;

impl Backend {
    /// Where this profile's recordings live.
    pub fn recordings_dir(&self) -> PathBuf {
        let stem = self
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "profile".into());
        self.dir().join("recordings").join(stem)
    }

    /// Keeps a spoken turn if the learner opted in; the file, or `None`.
    pub fn keep_recording(
        &self,
        session: i64,
        turn: usize,
        samples: &[f32],
        sample_rate: u32,
        now: Moment,
    ) -> anyhow::Result<Option<PathBuf>> {
        if self.store.profile()?.keep_recordings_days.is_none() {
            return Ok(None);
        }
        let dir = self.recordings_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}-s{session}-t{turn}.wav", now.unix));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec)?;
        for s in samples {
            w.write_sample((s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)?;
        }
        w.finalize()?;
        Ok(Some(path))
    }

    /// Deletes the recordings past the kept days (all of them without the opt-in); how many.
    pub fn purge_recordings(&self, now: Moment) -> anyhow::Result<usize> {
        let keep = self.store.profile()?.keep_recordings_days;
        purge(&self.recordings_dir(), keep, now)
    }

    /// Deletes every recording of this profile (with the profile, /LF710/).
    pub fn delete_recordings(&self) -> anyhow::Result<()> {
        let dir = self.recordings_dir();
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }
}

fn purge(dir: &Path, keep_days: Option<u32>, now: Moment) -> anyhow::Result<usize> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(0);
    };
    let cutoff = keep_days.map(|d| now.unix - i64::from(d) * 86_400);
    let mut n = 0;
    for e in entries.flatten() {
        let p = e.path();
        let said: Option<i64> = p
            .file_name()
            .and_then(|f| f.to_str())
            .and_then(|f| f.split('-').next())
            .and_then(|t| t.parse().ok());
        let old = match (cutoff, said) {
            (None, _) => true,
            (Some(c), Some(t)) => t < c,
            (Some(_), None) => false,
        };
        if old && p.extension().is_some_and(|x| x == "wav") {
            std::fs::remove_file(&p)?;
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend(name: &str, keep: Option<u32>) -> crate::TempBackend {
        let b = crate::TempBackend::new(&format!("rec-{name}"), false).unwrap();
        let mut p = b.store.profile().unwrap();
        p.keep_recordings_days = keep;
        b.store.set_profile(&p).unwrap();
        b
    }

    const T0: Moment = Moment {
        unix: 1_791_100_000,
        offset_min: 120,
    };

    #[test]
    fn nothing_without_the_opt_in() {
        let b = backend("off", None);
        assert!(
            b.keep_recording(1, 0, &[0.1; 1600], 16_000, T0)
                .unwrap()
                .is_none()
        );
        assert!(!b.recordings_dir().exists());
    }

    #[test]
    fn kept_for_the_chosen_days_then_gone() {
        let b = backend("keep", Some(7));
        let f = b
            .keep_recording(1, 0, &[0.1; 1600], 16_000, T0)
            .unwrap()
            .unwrap();
        assert!(f.exists());
        assert_eq!(b.purge_recordings(T0.plus_secs(6 * 86_400)).unwrap(), 0);
        assert_eq!(b.purge_recordings(T0.plus_secs(8 * 86_400)).unwrap(), 1);
        assert!(!f.exists());
    }

    #[test]
    fn turning_it_off_deletes_them() {
        let b = backend("toggle", Some(30));
        let f = b
            .keep_recording(1, 0, &[0.1; 1600], 16_000, T0)
            .unwrap()
            .unwrap();
        let mut p = b.store.profile().unwrap();
        p.keep_recordings_days = None;
        b.store.set_profile(&p).unwrap();
        b.purge_recordings(T0).unwrap();
        assert!(!f.exists());
    }
}
