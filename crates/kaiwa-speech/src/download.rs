//! The offered download of the speech models (F-08, /LL401/): only after the learner asked for
//! it, with source, size and licence shown first. Every file is checked against its SHA-256
//! before it takes its place; a broken or stopped download leaves nothing behind but a
//! `.part` file, which the next try overwrites.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

use crate::models::{KANA_CTC_URL, KOKORO, KOKORO_URL, REAZONSPEECH, REAZONSPEECH_URL, models_dir};

/// A file to fetch: its name, size in bytes and SHA-256.
#[derive(Debug, Clone, Copy)]
pub struct Remote {
    pub name: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}

/// Where a model comes from: single files under a base URL, or one `.tar.bz2` archive of which
/// only some files are kept.
#[derive(Debug, Clone, Copy)]
pub enum Source {
    Files {
        base: &'static str,
        files: &'static [Remote],
    },
    Archive {
        archive: Remote,
        url: &'static str,
        keep: &'static [&'static str],
    },
}

/// A model the app can offer to download.
#[derive(Debug, Clone, Copy)]
pub struct Model {
    /// Its directory under [`models_dir`].
    pub dir: &'static str,
    pub name: &'static str,
    pub licence: &'static str,
    /// Where the files come from, for the learner (host and project).
    pub origin: &'static str,
    /// Bytes on the disk afterwards.
    pub disk_bytes: u64,
    pub source: Source,
}

impl Model {
    /// Bytes to download.
    pub fn download_bytes(&self) -> u64 {
        match self.source {
            Source::Files { files, .. } => files.iter().map(|f| f.bytes).sum(),
            Source::Archive { archive, .. } => archive.bytes,
        }
    }

    pub fn path(&self) -> PathBuf {
        models_dir().join(self.dir)
    }

    /// Whether every file the app needs is there.
    pub fn present(&self) -> bool {
        let dir = self.path();
        match self.source {
            Source::Files { files, .. } => files.iter().all(|f| dir.join(f.name).exists()),
            Source::Archive { keep, .. } => keep.iter().all(|f| dir.join(f).exists()),
        }
    }
}

const KANA_CTC_FILES: &[Remote] = &[
    Remote {
        name: "model.int8.onnx",
        bytes: 356_148_324,
        sha256: "a1f2bab0730eb63eaedcc24f79d23d976a64fdecf86b8cf0ccc5b10bf14126ee",
    },
    Remote {
        name: "vocab.json",
        bytes: 1_100,
        sha256: "b7e677690af7557a65ae5dd9b922b0c5660d902463be90d443c24493fe6735b1",
    },
    Remote {
        name: "thresholds.json",
        bytes: 53,
        sha256: "d0230faf644690f9fef4e4b783b386a486aaff4747d545443269b15775e9153f",
    },
];

const KOKORO_FILES: &[Remote] = &[
    Remote {
        name: "kokoro-v1.0.int8.onnx",
        bytes: 92_361_271,
        sha256: "6e742170d309016e5891a994e1ce1559c702a2ccd0075e67ef7157974f6406cb",
    },
    Remote {
        name: "voices-v1.0.bin",
        bytes: 28_214_398,
        sha256: "bca610b8308e8d99f32e6fe4197e7ec01679264efed0cac9140fe9c29f1fbf7d",
    },
];

/// The three models: speech recognition, the interim voice, the mora check.
pub const MODELS: [Model; 3] = [
    Model {
        dir: REAZONSPEECH,
        name: "ReazonSpeech",
        licence: "Apache-2.0",
        origin: "github.com/k2-fsa/sherpa-onnx",
        disk_bytes: 160_372_200,
        source: Source::Archive {
            archive: Remote {
                name: "sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01.tar.bz2",
                bytes: 713_097_333,
                sha256: "e0981d0d5d7b446d41010831b59091ebf57d2aa7b79980f67ca37af460b5842d",
            },
            url: REAZONSPEECH_URL,
            keep: &[
                "encoder-epoch-99-avg-1.int8.onnx",
                "decoder-epoch-99-avg-1.int8.onnx",
                "joiner-epoch-99-avg-1.int8.onnx",
                "tokens.txt",
            ],
        },
    },
    Model {
        dir: KOKORO,
        name: "Kokoro",
        licence: "Apache-2.0",
        origin: "github.com/thewh1teagle/kokoro-onnx",
        disk_bytes: 120_575_669,
        source: Source::Files {
            base: KOKORO_URL,
            files: KOKORO_FILES,
        },
    },
    Model {
        dir: crate::mora::MODEL,
        name: "kaiwa-kana-ctc",
        licence: "Apache-2.0",
        origin: "huggingface.co/shrippen/kaiwa-kana-ctc",
        disk_bytes: 356_149_477,
        source: Source::Files {
            base: KANA_CTC_URL,
            files: KANA_CTC_FILES,
        },
    },
];

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("download failed: {0}")]
    Http(String),
    #[error("{0}: the file is not the expected one (checksum)")]
    Checksum(String),
    #[error("{0}: missing in the archive")]
    NotInArchive(String),
    #[error("stopped")]
    Stopped,
    #[error("{0}")]
    Io(#[from] io::Error),
}

/// Counts what passes through and hashes it; stops when `stop` is set.
struct Meter<'a, R> {
    inner: R,
    hash: Sha256,
    done: u64,
    progress: &'a dyn Fn(u64),
    stop: &'a AtomicBool,
}

impl<R: Read> Read for Meter<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.stop.load(Ordering::Relaxed) {
            // Not `Interrupted`: io::copy would retry it forever.
            return Err(io::Error::other("stopped"));
        }
        let n = self.inner.read(buf)?;
        self.hash.update(&buf[..n]);
        self.done += n as u64;
        (self.progress)(self.done);
        Ok(n)
    }
}

fn get(url: &str) -> Result<impl Read, DownloadError> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| DownloadError::Http(e.to_string()))?;
    Ok(response
        .into_body()
        .into_with_config()
        .limit(u64::MAX)
        .reader())
}

fn hex(hash: Sha256) -> String {
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn stopped(e: io::Error, stop: &AtomicBool) -> DownloadError {
    if stop.load(Ordering::Relaxed) {
        DownloadError::Stopped
    } else {
        DownloadError::Io(e)
    }
}

/// Fetches `file` from `url` into `dir`: first as `.part`, renamed once size and checksum fit.
fn fetch_file(
    url: &str,
    file: &Remote,
    dir: &Path,
    offset: u64,
    progress: &dyn Fn(u64),
    stop: &AtomicBool,
) -> Result<(), DownloadError> {
    let part = dir.join(format!("{}.part", file.name));
    let report = |n: u64| progress(offset + n);
    let mut meter = Meter {
        inner: get(url)?,
        hash: Sha256::new(),
        done: 0,
        progress: &report,
        stop,
    };
    let mut out = File::create(&part)?;
    io::copy(&mut meter, &mut out).map_err(|e| stopped(e, stop))?;
    out.flush()?;
    if meter.done != file.bytes || hex(meter.hash) != file.sha256 {
        let _ = fs::remove_file(&part);
        return Err(DownloadError::Checksum(file.name.into()));
    }
    fs::rename(&part, dir.join(file.name))?;
    Ok(())
}

/// Downloads `model` into its directory. `progress` gets the bytes so far (of
/// [`Model::download_bytes`]); setting `stop` ends it early with [`DownloadError::Stopped`].
pub fn download(
    model: &Model,
    progress: &dyn Fn(u64),
    stop: &AtomicBool,
) -> Result<(), DownloadError> {
    let dir = model.path();
    fs::create_dir_all(&dir)?;
    match model.source {
        Source::Files { base, files } => {
            let mut offset = 0;
            for f in files {
                if dir.join(f.name).exists() {
                    offset += f.bytes;
                    progress(offset);
                    continue;
                }
                fetch_file(
                    &format!("{base}{}", f.name),
                    f,
                    &dir,
                    offset,
                    progress,
                    stop,
                )?;
                offset += f.bytes;
            }
            Ok(())
        }
        Source::Archive { archive, url, keep } => unpack(url, &archive, keep, &dir, progress, stop),
    }
}

/// Streams the archive, keeps the files named in `keep` (as `.part` until the archive's
/// checksum fits), and drops the rest.
fn unpack(
    url: &str,
    archive: &Remote,
    keep: &[&str],
    dir: &Path,
    progress: &dyn Fn(u64),
    stop: &AtomicBool,
) -> Result<(), DownloadError> {
    let mut meter = Meter {
        inner: get(url)?,
        hash: Sha256::new(),
        done: 0,
        progress,
        stop,
    };
    let mut kept = Vec::new();
    {
        let mut tar = tar::Archive::new(bzip2::read::BzDecoder::new(&mut meter));
        for entry in tar.entries().map_err(|e| stopped(e, stop))? {
            let mut entry = entry.map_err(|e| stopped(e, stop))?;
            let path = entry.path().map_err(|e| stopped(e, stop))?.into_owned();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            // Only files at the archive's top directory (not test_wavs/…).
            if path.components().count() != 2 || !keep.contains(&name) {
                continue;
            }
            let part = dir.join(format!("{name}.part"));
            let mut out = File::create(&part)?;
            io::copy(&mut entry, &mut out).map_err(|e| stopped(e, stop))?;
            kept.push(name.to_string());
        }
    }
    // Read to the end, so the checksum covers the whole archive.
    io::copy(&mut meter, &mut io::sink()).map_err(|e| stopped(e, stop))?;
    let ok = meter.done == archive.bytes && hex(meter.hash) == archive.sha256;
    for name in keep {
        let part = dir.join(format!("{name}.part"));
        if !ok {
            let _ = fs::remove_file(&part);
        }
    }
    if !ok {
        return Err(DownloadError::Checksum(archive.name.into()));
    }
    if let Some(missing) = keep.iter().find(|k| !kept.iter().any(|n| n == *k)) {
        return Err(DownloadError::NotInArchive((*missing).into()));
    }
    for name in keep {
        fs::rename(dir.join(format!("{name}.part")), dir.join(name))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_file_has_a_checksum_and_a_size() {
        for m in MODELS {
            let files: Vec<Remote> = match m.source {
                Source::Files { files, .. } => files.to_vec(),
                Source::Archive { archive, .. } => vec![archive],
            };
            for f in files {
                assert_eq!(f.sha256.len(), 64, "{}", f.name);
                assert!(
                    f.sha256.chars().all(|c| c.is_ascii_hexdigit()),
                    "{}",
                    f.name
                );
                assert!(f.bytes > 0, "{}", f.name);
            }
            assert!(m.download_bytes() > 0 && m.disk_bytes > 0, "{}", m.name);
        }
    }

    #[test]
    fn the_kana_model_is_pinned_to_a_revision() {
        let Source::Files { base, .. } = MODELS[2].source else {
            panic!("files")
        };
        assert!(
            base.contains("/resolve/") && !base.contains("/main/"),
            "{base}"
        );
    }

    #[test]
    fn a_meter_counts_hashes_and_stops() {
        let stop = AtomicBool::new(false);
        let seen = std::cell::Cell::new(0);
        let report = |n: u64| seen.set(n);
        let mut m = Meter {
            inner: &b"kaiwa"[..],
            hash: Sha256::new(),
            done: 0,
            progress: &report,
            stop: &stop,
        };
        io::copy(&mut m, &mut io::sink()).unwrap();
        assert_eq!(seen.get(), 5);
        assert_eq!(
            hex(m.hash),
            "cd597638157f09960a3e8f46d4eaba62c61b663c16602e5613bdbf1f88fd472a"
        );
        stop.store(true, Ordering::Relaxed);
        let mut m = Meter {
            inner: &b"kaiwa"[..],
            hash: Sha256::new(),
            done: 0,
            progress: &report,
            stop: &stop,
        };
        assert!(io::copy(&mut m, &mut io::sink()).is_err());
    }
}
