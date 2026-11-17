//! Test setup for the QA driver `kaiwa-sim` (AUDIT.md finding 6).
//!
//! The simulation must put a profile into states no front end can reach: a finished
//! session of last month, a day record, a learner model with a planted mistake. These
//! need the raw rows, which the backend keeps private. Instead of handing out the
//! store, this module names each thing the simulation may set or read. It exists only
//! with the feature `sim`, which no front end enables; a package never contains it.

use kaiwa_core::Session;
use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::curriculum::Curriculum;
use kaiwa_core::progress::DayRecord;
use kaiwa_core::session::SessionState;
use kaiwa_core::srs::Item;
use kaiwa_store::{Dispute, Learning, Profile, Recorded, Result, SessionRow, Today};

use crate::Backend;

impl Backend {
    /// The rows of this profile, for test setup.
    pub fn seed(&self) -> Seed<'_> {
        Seed(self)
    }
}

/// What a simulation may set and read besides the backend's commands.
pub struct Seed<'a>(&'a Backend);

impl Seed<'_> {
    pub fn set_profile(&self, p: &Profile) -> Result<()> {
        self.0.store.set_profile(p)
    }

    pub fn set_feedback(&self, a: kaiwa_core::Amount) -> Result<Profile> {
        self.0.store.set_feedback(a)
    }

    pub fn learning(&self) -> Result<Learning> {
        self.0.store.learning()
    }

    pub fn set_learning(&self, l: &Learning) -> Result<()> {
        self.0.store.set_learning(l)
    }

    pub fn set_state_value<T: serde::Serialize>(&self, key: &str, v: &T) -> Result<()> {
        self.0.store.set_state_value(key, v)
    }

    pub fn begin_session(&self, state: &SessionState, at: Moment) -> Result<String> {
        self.0.store.begin_local(state, at)
    }

    pub fn save_session(&self, id: &str, state: &SessionState) -> Result<()> {
        self.0.store.save_session(id, state)
    }

    pub fn profile(&self) -> Result<Profile> {
        self.0.store.profile()
    }

    pub fn disputes(&self) -> Result<Vec<Dispute>> {
        self.0.store.disputes()
    }

    pub fn record(
        &self,
        id: &str,
        s: &Session,
        started: Moment,
        ended: Moment,
        curriculum: Option<&Curriculum>,
    ) -> Result<Recorded> {
        self.0.store.record(id, s, started, ended, curriculum)
    }

    pub fn put_finished_session(
        &self,
        state: &SessionState,
        started: Moment,
        ended: Moment,
        success: f64,
    ) -> Result<String> {
        self.0
            .store
            .put_finished_session(state, started, ended, success)
    }

    pub fn sessions(&self, limit: usize) -> Result<Vec<SessionRow>> {
        self.0.store.sessions(limit)
    }

    pub fn finished_since(&self, from: i64) -> Result<Vec<SessionRow>> {
        self.0.store.finished_since(from)
    }

    pub fn session_state(&self, id: &str) -> Result<SessionState> {
        self.0.store.session_state(id)
    }

    pub fn items(&self) -> Result<Vec<Item>> {
        self.0.store.items()
    }

    pub fn item(&self, text: &str) -> Result<Option<Item>> {
        self.0.store.item(text)
    }

    /// What is due, after spreading a break as the review page does.
    pub fn due(&self, now: Moment, limit: usize) -> anyhow::Result<Vec<Item>> {
        self.0.due(None, now, limit)
    }

    pub fn put_item(&self, item: &Item) -> Result<()> {
        self.0.store.put_item(item)
    }

    pub fn day(&self, d: Day) -> Result<DayRecord> {
        self.0.store.day(d)
    }

    pub fn days(&self) -> Result<std::collections::BTreeMap<Day, DayRecord>> {
        self.0.store.days()
    }

    pub fn put_day(&self, d: Day, r: &DayRecord) -> Result<()> {
        self.0.store.put_day(d, r)
    }

    pub fn paused_until(&self, today: Day) -> Result<Option<Day>> {
        self.0.store.paused_until(today)
    }

    /// The start page's numbers, after spreading a break as the start page does.
    pub fn today(&self, now: Moment) -> anyhow::Result<Today> {
        self.0.opened_today(now)
    }

    pub fn export(&self, now: Moment) -> Result<serde_json::Value> {
        self.0.store.export(now)
    }
}
