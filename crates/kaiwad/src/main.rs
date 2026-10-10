//! `kaiwad`: Kaiwa's session service on the session bus (ADR-008, Lastenheft /LF506/, /LF510/).
//! The plasmoid runs in `plasmashell` and keeps no models, audio or database there: it asks
//! this service, which owns the profile. It starts by D-Bus activation and quits after
//! [`IDLE`] without calls (/LL012/); the next call starts it again.
//!
//! Interface `org.shrippen.Kaiwa1` at `/org/shrippen/Kaiwa1`; every answer is JSON:
//!
//! - `Today()`: the start page (goal, streak, next scenario, due reviews).
//! - `Reviews(n)` and `AnswerReview(text, said, helped)`: five reviews (/LF502/).
//! - `StartShort()`, `Say(text, typed)`: a short talk of two goals (/LF501/), saved after every
//!   turn, so the app continues it (/LF507/); corrections in one line each (/LF503/).
//! - `OpenApp()`: the app, to see a talk in full (/LF503/).
//! - Signal `Changed()` after anything that changes the profile.
//!
//! Turns run on the bus thread: a slow tutor holds other calls until it answers. Speech (the
//! microphone and the models, /LF505/) follows with the shared audio path.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kaiwa_backend::short::ShortTalk;
use kaiwa_backend::{Backend, now, views};
use zbus::fdo;
use zbus::object_server::SignalEmitter;

pub const NAME: &str = "org.shrippen.Kaiwa1";
pub const PATH: &str = "/org/shrippen/Kaiwa1";
/// Quit after this long without a call.
const IDLE: Duration = Duration::from_secs(10 * 60);

fn secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

struct Inner {
    backend: Backend,
    short: Option<ShortTalk>,
}

struct Service {
    inner: Mutex<Inner>,
    last_call: Arc<AtomicU64>,
}

fn failed(e: impl std::fmt::Display) -> fdo::Error {
    fdo::Error::Failed(e.to_string())
}

impl Service {
    /// Notes the call (for the idle exit) and locks the profile.
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.last_call.store(secs(), Ordering::Relaxed);
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[zbus::interface(name = "org.shrippen.Kaiwa1")]
impl Service {
    async fn today(&self) -> fdo::Result<String> {
        views::today(&self.lock().backend, now())
            .map(|v| v.to_string())
            .map_err(failed)
    }

    async fn reviews(&self, n: u32) -> fdo::Result<String> {
        views::review(&self.lock().backend, now(), n as usize)
            .map(|v| v.to_string())
            .map_err(failed)
    }

    async fn answer_review(
        &self,
        text: &str,
        said: &str,
        helped: bool,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<String> {
        let v = self.lock().backend.answer_review(text, said, helped, now());
        let _ = Self::changed(&emitter).await;
        Ok(v.to_string())
    }

    async fn start_short(&self) -> fdo::Result<String> {
        let mut inner = self.lock();
        let talk = ShortTalk::start(&inner.backend, now()).map_err(failed)?;
        let v = talk.view();
        inner.short = Some(talk);
        Ok(v.to_string())
    }

    async fn say(
        &self,
        text: &str,
        typed: bool,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<String> {
        let v = {
            let mut guard = self.lock();
            let Inner { backend, short } = &mut *guard;
            let Some(talk) = short.as_mut() else {
                return Err(fdo::Error::Failed(
                    "no short talk; call StartShort first".into(),
                ));
            };
            let v = talk.say(backend, text, typed, now()).map_err(failed)?;
            if v["done"] == true {
                *short = None;
            }
            v
        };
        let _ = Self::changed(&emitter).await;
        Ok(v.to_string())
    }

    /// Opens the app (the plasmoid's "In der App ansehen", /LF503/); it shows the open
    /// session on its start page.
    async fn open_app(&self) -> fdo::Result<()> {
        self.last_call.store(secs(), Ordering::Relaxed);
        let app = std::env::var("KAIWA_APP").unwrap_or_else(|_| "kaiwa".into());
        std::process::Command::new(app)
            .spawn()
            .map(|_| ())
            .map_err(failed)
    }

    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

fn main() -> anyhow::Result<()> {
    let last_call = Arc::new(AtomicU64::new(secs()));
    let service = Service {
        inner: Mutex::new(Inner {
            backend: Backend::open()?,
            short: None,
        }),
        last_call: last_call.clone(),
    };
    let _conn = zbus::blocking::connection::Builder::session()?
        .name(NAME)?
        .serve_at(PATH, service)?
        .build()?;
    eprintln!("kaiwad: serving {NAME} at {PATH}");
    let idle = std::env::var("KAIWAD_IDLE_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(IDLE);
    loop {
        std::thread::sleep(Duration::from_secs(5).min(idle));
        if secs().saturating_sub(last_call.load(Ordering::Relaxed)) >= idle.as_secs() {
            eprintln!("kaiwad: idle, quitting");
            return Ok(());
        }
    }
}
