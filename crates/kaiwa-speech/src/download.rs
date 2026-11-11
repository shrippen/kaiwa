//! The offered download of the speech models (F-08, /LL401/): only after the learner asked for
//! it, with source, size and licence shown first. Every file is checked against its SHA-256
//! before it takes its place; a broken or stopped download leaves nothing behind but a
//! `.part` file, which the next try overwrites.
//!
//! The connection is read on a thread of its own (T-09, owner 10.11.2026): a connection that
//! hangs gave no data and no error, "Abbrechen" was only looked at between two reads, and so a
//! hanging download could not be stopped and blocked every other until the app started again.
//! Now `STALL` without data counts as broken, and stopping takes effect within `TICK`. There is
//! no limit on the whole download: a large model on a slow line may take its time.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use ureq::unversioned::resolver::DefaultResolver;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, DefaultConnector, NextTimeout, Transport, time,
};

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
    /// An archive built into the model on this computer after the download (UniDic: the
    /// source dictionary into lindera's format); `build(archive, dir)` removes the archive.
    Built {
        archive: Remote,
        url: &'static str,
        build: fn(&Path, &Path) -> Result<(), String>,
        made: &'static [&'static str],
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
            Source::Archive { archive, .. } | Source::Built { archive, .. } => archive.bytes,
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
            Source::Built { made, .. } => made.iter().all(|f| dir.join(f).exists()),
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

/// The models: speech recognition, the interim voice, the mora check, the dictionary
/// (kaiwa-data, L-06; not speech, but downloaded the same way, M-03) and the readings for
/// furigana (UniDic, ADR-004).
pub const MODELS: [Model; 5] = [
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
    Model {
        dir: kaiwa_lang::PACKAGE,
        name: "kaiwa-data",
        licence: "CC BY-SA 4.0",
        origin: "github.com/shrippen/kaiwa-data",
        disk_bytes: DICT_DISK,
        source: Source::Archive {
            archive: Remote {
                name: "kaiwa-data-0.2.0.tar.bz2",
                bytes: 47_794_085,
                sha256: "c9cb6effe946f7ab3ffec021b9e18127571cb057a03da28efbf12e29474e63df",
            },
            url: "https://github.com/shrippen/kaiwa-data/releases/download/v0.2.0/kaiwa-data-0.2.0.tar.bz2",
            keep: &["kaiwa-dict.sqlite", "NOTICE", "LICENSE-DATA"],
        },
    },
    Model {
        dir: kaiwa_lang::unidic::DIR,
        name: "UniDic",
        licence: "BSD-3-Clause",
        origin: "lindera.dev (UniDic 2.1.2, NINJAL)",
        disk_bytes: kaiwa_lang::unidic::DISK_BYTES,
        source: Source::Built {
            archive: Remote {
                name: kaiwa_lang::unidic::ARCHIVE,
                bytes: kaiwa_lang::unidic::ARCHIVE_BYTES,
                sha256: kaiwa_lang::unidic::ARCHIVE_SHA256,
            },
            url: kaiwa_lang::unidic::URL,
            build: kaiwa_lang::unidic::build,
            made: kaiwa_lang::unidic::BUILT,
        },
    },
];

/// The unpacked dictionary with its licence files.
const DICT_DISK: u64 = 158_474_240 + 1_442 + 20_138;

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
    #[error("no data for {0} s")]
    Stalled(u64),
    #[error("building it failed: {0}")]
    Build(String),
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

/// Without data for this long the connection counts as broken.
pub const STALL: Duration = Duration::from_secs(30);
/// How often a waiting read looks whether the learner stopped the download.
const TICK: Duration = Duration::from_millis(200);
const CHUNK: usize = 64 * 1024;

/// A source read on a thread of its own: the reader here waits for its chunks, at most
/// `stall` for each, and looks at `stop` meanwhile. A source that hangs keeps its thread
/// until the connection gives up; it holds nothing else and ends with the first chunk nobody
/// takes.
struct Patient<'a> {
    chunks: mpsc::Receiver<Result<Vec<u8>, DownloadError>>,
    chunk: Vec<u8>,
    at: usize,
    stall: Duration,
    stop: &'a AtomicBool,
    over: bool,
    /// Why it gave up. The readers between here and the caller (bzip2, tar) wrap an error in
    /// their own, so the reason is kept here and the error only says that there is one.
    why: Why,
}

/// Why a download's source gave up, shared with whoever started it.
type Why = std::rc::Rc<std::cell::RefCell<Option<DownloadError>>>;

impl<'a> Patient<'a> {
    /// `open` runs on the thread: connecting may hang like reading.
    fn new<R: Read>(
        open: impl FnOnce() -> Result<R, DownloadError> + Send + 'static,
        stall: Duration,
        stop: &'a AtomicBool,
    ) -> Self {
        // A few chunks ahead at most: the download is not read into memory.
        let (tx, chunks) = mpsc::sync_channel(4);
        std::thread::spawn(move || {
            let mut source = match open() {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            loop {
                let mut buf = vec![0; CHUNK];
                let sent = match source.read(&mut buf) {
                    // An empty chunk is the end.
                    Ok(n) => {
                        buf.truncate(n);
                        let end = n == 0;
                        tx.send(Ok(buf)).is_ok() && !end
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => true,
                    Err(e) => {
                        let _ = tx.send(Err(DownloadError::Io(e)));
                        false
                    }
                };
                if !sent {
                    return;
                }
            }
        });
        Self {
            chunks,
            chunk: Vec::new(),
            at: 0,
            stall,
            stop,
            over: false,
            why: Why::default(),
        }
    }

    fn give_up(&self, why: DownloadError) -> io::Error {
        // Not `Interrupted`: io::copy would retry it forever.
        let e = io::Error::other(why.to_string());
        *self.why.borrow_mut() = Some(why);
        e
    }
}

impl Read for Patient<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.at == self.chunk.len() && !self.over {
            let since = Instant::now();
            self.chunk = loop {
                if self.stop.load(Ordering::Relaxed) {
                    return Err(self.give_up(DownloadError::Stopped));
                }
                match self.chunks.recv_timeout(TICK.min(self.stall)) {
                    Ok(Ok(chunk)) => break chunk,
                    Ok(Err(why)) => return Err(self.give_up(why)),
                    Err(mpsc::RecvTimeoutError::Timeout) if since.elapsed() < self.stall => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        return Err(self.give_up(DownloadError::Stalled(self.stall.as_secs())));
                    }
                    // The thread is gone without an end or an error: it panicked.
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        return Err(
                            self.give_up(DownloadError::Http("the download thread ended".into()))
                        );
                    }
                }
            };
            self.at = 0;
            self.over = self.chunk.is_empty();
        }
        let n = buf.len().min(self.chunk.len() - self.at);
        buf[..n].copy_from_slice(&self.chunk[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

fn get<'a>(url: &str, stop: &'a AtomicBool) -> Patient<'a> {
    let url = url.to_string();
    Patient::new(move || open(&url, STALL), STALL, stop)
}

/// The body of `url`. Finding the host, connecting, the response's head and every read of the
/// body get `limit` each: without, a server that never answers, or goes silent mid-body, held
/// its thread for good after `Patient` gave up (U-02, owner 11.11.2026). The body has no limit
/// as a whole (a large model on a slow line takes its time).
fn open(url: &str, limit: Duration) -> Result<impl Read + use<>, DownloadError> {
    let config = ureq::Agent::config_builder()
        .timeout_resolve(Some(limit))
        .timeout_connect(Some(limit))
        .timeout_recv_response(Some(limit))
        .build();
    let connector = DefaultConnector::new().chain(Idle(limit));
    let agent = ureq::Agent::with_parts(config, connector, DefaultResolver::default());
    let response = agent
        .get(url)
        .call()
        .map_err(|e| DownloadError::Http(e.to_string()))?;
    Ok(response
        .into_body()
        .into_with_config()
        .limit(u64::MAX)
        .reader())
}

/// Puts a limit on every wait of ureq's connections: ureq limits only the whole body. The
/// TCP connection under TLS takes the limit as its socket's read timeout.
#[derive(Debug)]
struct Idle(Duration);

impl Connector<Box<dyn Transport>> for Idle {
    type Out = IdleTransport;

    fn connect(
        &self,
        _: &ConnectionDetails,
        chained: Option<Box<dyn Transport>>,
    ) -> Result<Option<Self::Out>, ureq::Error> {
        Ok(chained.map(|inner| IdleTransport {
            inner,
            limit: self.0,
        }))
    }
}

#[derive(Debug)]
struct IdleTransport {
    inner: Box<dyn Transport>,
    limit: Duration,
}

impl IdleTransport {
    /// The shorter of ureq's next timeout and the limit.
    fn clamp(&self, next: NextTimeout) -> NextTimeout {
        if *next.after <= self.limit {
            return next;
        }
        NextTimeout {
            after: time::Duration::Exact(self.limit),
            reason: next.reason,
        }
    }
}

impl Transport for IdleTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        self.inner.buffers()
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), ureq::Error> {
        let timeout = self.clamp(timeout);
        self.inner.transmit_output(amount, timeout)
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, ureq::Error> {
        let timeout = self.clamp(timeout);
        self.inner.await_input(timeout)
    }

    fn is_open(&mut self) -> bool {
        self.inner.is_open()
    }

    fn is_tls(&self) -> bool {
        self.inner.is_tls()
    }
}

fn hex(hash: Sha256) -> String {
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// What a failed read of the download means: stopped by the learner, the source's own reason
/// (no data, the request failed, the connection broke), or the disk.
fn stopped(e: io::Error, stop: &AtomicBool, why: &Why) -> DownloadError {
    if stop.load(Ordering::Relaxed) {
        return DownloadError::Stopped;
    }
    why.borrow_mut().take().unwrap_or(DownloadError::Io(e))
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
    let source = get(url, stop);
    let why = source.why.clone();
    let mut meter = Meter {
        inner: source,
        hash: Sha256::new(),
        done: 0,
        progress: &report,
        stop,
    };
    let mut out = File::create(&part)?;
    io::copy(&mut meter, &mut out).map_err(|e| stopped(e, stop, &why))?;
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
        Source::Built {
            archive,
            url,
            build,
            ..
        } => {
            let file = dir.join(archive.name);
            if !file.exists() {
                fetch_file(url, &archive, &dir, 0, progress, stop)?;
            }
            progress(archive.bytes);
            build(&file, &dir).map_err(DownloadError::Build)
        }
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
    let source = get(url, stop);
    let why = source.why.clone();
    let mut meter = Meter {
        inner: source,
        hash: Sha256::new(),
        done: 0,
        progress,
        stop,
    };
    let mut kept = Vec::new();
    {
        let mut tar = tar::Archive::new(bzip2::read::BzDecoder::new(&mut meter));
        for entry in tar.entries().map_err(|e| stopped(e, stop, &why))? {
            let mut entry = entry.map_err(|e| stopped(e, stop, &why))?;
            let path = entry
                .path()
                .map_err(|e| stopped(e, stop, &why))?
                .into_owned();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            // Only files at the archive's top directory (not test_wavs/…).
            if path.components().count() != 2 || !keep.contains(&name) {
                continue;
            }
            let part = dir.join(format!("{name}.part"));
            let mut out = File::create(&part)?;
            io::copy(&mut entry, &mut out).map_err(|e| stopped(e, stop, &why))?;
            kept.push(name.to_string());
        }
    }
    // Read to the end, so the checksum covers the whole archive.
    io::copy(&mut meter, &mut io::sink()).map_err(|e| stopped(e, stop, &why))?;
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
    fn a_server_that_never_answers_lets_its_thread_go() {
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model.onnx", server.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let failed = open(&url, Duration::from_millis(300)).is_err();
            let _ = tx.send(failed);
        });

        // Accepted, never answered: the request must give up by itself.
        let _held = server.accept().unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(true));
    }

    #[test]
    fn a_server_silent_mid_body_lets_its_thread_go() {
        use std::io::Write as _;
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model.onnx", server.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut body = Vec::new();
            let r = open(&url, Duration::from_millis(300))
                .and_then(|mut b| b.read_to_end(&mut body).map_err(DownloadError::Io));
            let _ = tx.send((r.is_err(), body.len()));
        });

        // A head promising 1000 bytes, 10 of them, then silence with the connection open.
        let (mut held, _) = server.accept().unwrap();
        let mut request = [0; 1024];
        let _ = held.read(&mut request).unwrap();
        held.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\n0123456789")
            .unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok((true, 10)));
    }

    #[test]
    fn every_file_has_a_checksum_and_a_size() {
        for m in MODELS {
            let files: Vec<Remote> = match m.source {
                Source::Files { files, .. } => files.to_vec(),
                Source::Archive { archive, .. } | Source::Built { archive, .. } => vec![archive],
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
    fn the_dictionary_is_the_release_this_kaiwa_reads() {
        let m = MODELS[3];
        assert_eq!(m.dir, kaiwa_lang::PACKAGE);
        let Source::Archive { archive, url, keep } = m.source else {
            panic!("archive")
        };
        let version = kaiwa_lang::PACKAGE.trim_start_matches("kaiwa-data-");
        assert!(
            url.ends_with(&format!("/v{version}/{}", archive.name)),
            "{url}"
        );
        assert_eq!(archive.name, format!("{}.tar.bz2", kaiwa_lang::PACKAGE));
        assert!(keep.contains(&"kaiwa-dict.sqlite") && keep.contains(&"NOTICE"));
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

    /// A source that gives `first`, then nothing for `hang`, then `rest`.
    struct Hanging {
        first: Option<Vec<u8>>,
        hang: Duration,
        rest: Option<Vec<u8>>,
    }

    impl Read for Hanging {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let part = match self.first.take() {
                Some(p) => p,
                None => match self.rest.take() {
                    Some(p) => {
                        std::thread::sleep(self.hang);
                        p
                    }
                    None => return Ok(0),
                },
            };
            buf[..part.len()].copy_from_slice(&part);
            Ok(part.len())
        }
    }

    fn hanging(hang_ms: u64) -> impl FnOnce() -> Result<Hanging, DownloadError> + Send + 'static {
        move || {
            Ok(Hanging {
                first: Some(b"kai".to_vec()),
                hang: Duration::from_millis(hang_ms),
                rest: Some(b"wa".to_vec()),
            })
        }
    }

    #[test]
    fn a_slow_source_is_read_to_the_end() {
        let stop = AtomicBool::new(false);
        let mut p = Patient::new(hanging(150), Duration::from_secs(5), &stop);
        let mut all = Vec::new();
        p.read_to_end(&mut all).unwrap();
        assert_eq!(all, b"kaiwa");
        // More than one chunk, and more than the reader asks for at once.
        let big = vec![7u8; CHUNK * 3 + 5];
        let source = big.clone();
        let mut p = Patient::new(
            move || Ok(io::Cursor::new(source)),
            Duration::from_secs(5),
            &stop,
        );
        let mut all = Vec::new();
        let mut small = [0u8; 1000];
        loop {
            let n = p.read(&mut small).unwrap();
            if n == 0 {
                break;
            }
            all.extend_from_slice(&small[..n]);
        }
        assert_eq!(all, big);
    }

    #[test]
    fn a_source_without_data_counts_as_broken() {
        let stop = AtomicBool::new(false);
        let mut p = Patient::new(hanging(60_000), Duration::from_millis(300), &stop);
        let mut buf = [0u8; 8];
        assert_eq!(p.read(&mut buf).unwrap(), 3);
        let began = Instant::now();
        let e = p.read(&mut buf).unwrap_err();
        assert!(began.elapsed() < Duration::from_secs(5), "waited too long");
        // Through readers that wrap the error, as bzip2 and tar do.
        let wrapped = io::Error::new(io::ErrorKind::InvalidData, e);
        assert!(matches!(
            stopped(wrapped, &stop, &p.why),
            DownloadError::Stalled(0)
        ));
    }

    #[test]
    fn a_hanging_download_stops_when_asked() {
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let asked = stop.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            asked.store(true, Ordering::Relaxed);
        });
        // Far below the stall: only the stop can end this.
        let mut p = Patient::new(hanging(60_000), Duration::from_secs(600), &stop);
        let began = Instant::now();
        let e = io::copy(&mut p, &mut io::sink()).unwrap_err();
        assert!(began.elapsed() < Duration::from_secs(5), "waited too long");
        assert!(matches!(stopped(e, &stop, &p.why), DownloadError::Stopped));
    }

    #[test]
    fn a_request_that_fails_says_why() {
        let stop = AtomicBool::new(false);
        let mut p = Patient::new(
            || -> Result<io::Empty, DownloadError> { Err(DownloadError::Http("404".into())) },
            Duration::from_secs(5),
            &stop,
        );
        let e = p.read(&mut [0u8; 8]).unwrap_err();
        assert!(matches!(stopped(e, &stop, &p.why), DownloadError::Http(m) if m == "404"));
        // A full disk is no reason of the source's.
        let disk = io::Error::other("no space left");
        assert!(matches!(stopped(disk, &stop, &p.why), DownloadError::Io(_)));
    }
}
