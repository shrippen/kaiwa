//! The profile for the whole process (`kaiwa_backend::Backend`); both QObjects (the app and
//! the conversation) reach it here.

use std::sync::{Mutex, MutexGuard, OnceLock};

pub use kaiwa_backend::{Backend as AppState, now, views};

static STATE: OnceLock<Mutex<AppState>> = OnceLock::new();

/// The state; opened on first use.
pub fn state() -> MutexGuard<'static, AppState> {
    STATE
        .get_or_init(|| Mutex::new(AppState::open().expect("the profile opens")))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
