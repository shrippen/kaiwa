//! Kaiwa's profile (Lastenheft /LD010/–/LD060/, /LF121/, /LF710/, /LF720/): one SQLite file per
//! profile, with versioned migrations.
//!
//! - **Profile** (/LD010/): name, UI language, goals, settings, placement, as one JSON value.
//! - **Learning state** (/LD020/): learner model, regulator with its history, course progress.
//! - **Sessions** (/LD030/): the full session state (turns, corrections, help), so an
//!   interrupted session continues where it stopped. Audio is never stored here (/LF712/).
//! - **Review items** (FSRS state), **days** (daily goal, XP), **pauses**, **disputes** (/LD060/).
//!
//! [`Store::record`] ends a session: it feeds the learner model and the regulator, moves the
//! path, adds the review items and books the day, so the app and the plasmoid share one rule.
//! [`Store::export`] writes everything as one JSON file (`spec/export.schema.json`),
//! [`Store::import`] reads it back, [`delete_profile`] removes every file of a profile.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use kaiwa_core::booking::FeedbackSource;
use kaiwa_core::booking::{book_placement, book_talk};
use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::curriculum::{CourseProgress, Curriculum};
use kaiwa_core::difficulty::{Change, Regulator};
use kaiwa_core::learner::LearnerModel;
use kaiwa_core::progress::{DailyGoal, DayRecord, Pause, Streak, XP_REVIEW, streak, turn_counts};
use kaiwa_core::session::{Session, SessionState};
use kaiwa_core::srs::{self, Grade, Item};
use kaiwa_core::{Lang, feedback::Amount};
use migrations::{MIGRATIONS, rename_unit_ids};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// The export format: 2 tutor preset (migration 3), 3 monthly costs (V-04), 4 journal and
/// text talk ids (ADR-010). Only the current format is read (no release yet).
pub const EXPORT_SCHEMA: &str = "kaiwa-export/4";

/// The device state of a month's costs is `cost.<year>-<month>`; the export keeps the months.
pub const COST_STATE_PREFIX: &str = "cost.";

/// Device state set by an import; a profile that took one takes no second (costs would double).
const IMPORTED_STATE: &str = "imported";
/// State: when a card was last reviewed (the break rule reads it).
const REVIEWED: &str = "reviewed";
/// State: the day the cards due after a break were spread.
const REENTERED: &str = "reentered";

/// What a month cost so far (/LF714/). The store owns the shape because the export carries it.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostMonth {
    pub cents: f64,
    pub calls: u32,
    pub unknown_calls: u32,
    /// Sessions that cost something (a talk with a local tutor is not one of them).
    pub sessions: u32,
    /// The talk going on has cost something; it counts as a session when it ends.
    #[serde(default)]
    pub paid: bool,
}

impl CostMonth {
    fn add(&mut self, other: &CostMonth) {
        self.cents += other.cents;
        self.calls += other.calls;
        self.unknown_calls += other.unknown_calls;
        self.sessions += other.sessions;
        self.paid |= other.paid;
    }
}

/// Whether `key` reads `YYYY-MM`.
fn is_month_key(key: &str) -> bool {
    let b = key.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    b.len() == 7
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && (b"01".as_slice()..=b"12".as_slice()).contains(&&b[5..7])
}

pub use kaiwa_core::choices::{TEXT_SCALE_STEPS, UNLOAD_STEPS};

/// The largest text size the app offers (/LL202/; owner, 10.11.2026: 160 % for now). Above
/// it the profile page no longer fits a small laptop's window (`tools/font-160.sh`). A profile
/// that holds more is shown and started at this size.
pub const TEXT_SCALE_MAX: f32 = TEXT_SCALE_STEPS[TEXT_SCALE_STEPS.len() - 1];

/// What a new profile unloads after: one of `UNLOAD_STEPS`.
pub const DEFAULT_UNLOAD: Option<u32> = UNLOAD_STEPS[1];

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("file: {0}")]
    Io(#[from] std::io::Error),
    #[error("the profile already has data; import only into a new profile")]
    NotEmpty,
    #[error("not a Kaiwa export ({0})")]
    WrongFormat(String),
    #[error("no session {0}")]
    NoSession(String),
    #[error("session {0} has no record to correct")]
    NoRecording(String),
    #[error("session {0} is not the newest talk; its record stays")]
    Superseded(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// How the character's text is shown (/LF110/).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Script {
    /// Kana and kanji with furigana.
    #[default]
    Furigana,
    /// Kana only.
    Kana,
    /// No text: listening only.
    Hidden,
}

/// The profile (/LD010/).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    /// Stored as "de" or "en" (/LL506/).
    pub ui_lang: Lang,
    pub daily_goal: DailyGoal,
    /// XP, streak and ring; off shows only the path (/LF605/).
    pub gamification: bool,
    pub feedback: Amount,
    #[serde(default)]
    pub feedback_source: FeedbackSource,
    pub script: Script,
    /// Silence after which a spoken turn ends, 1.2 to 5 s (/LF101/).
    pub end_pause_secs: f32,
    /// Keep recordings this many days; `None` keeps none (/LF712/).
    pub keep_recordings_days: Option<u32>,
    /// 0 system, 1 Kante, 2 Kante Light.
    pub style: u8,
    /// The placement ran or was skipped (/LF200/).
    pub placed: bool,
    /// Reminder time as minutes after midnight; `None` sends none (/LF520/, /LF620/).
    pub reminder: Option<u32>,
    /// Cost limit for a cloud tutor per month in euro cents; `None` without a limit (/LF714/).
    #[serde(default)]
    pub cost_limit_cents: Option<u32>,
    /// Hints on mora lengths after a spoken turn (/LF320/); on by default since the app path
    /// counts as reaching K-02 (L-01).
    #[serde(default = "yes")]
    pub mora_hints: bool,
    /// Easier listening (/LL204/): the character's voice louder, and clearer (more highs).
    #[serde(default)]
    pub louder: bool,
    #[serde(default)]
    pub clear: bool,
    /// Unload the speech models after this many idle minutes; `None` keeps them (/LL012/).
    #[serde(default = "default_unload")]
    pub unload_after_min: Option<u32>,
    /// Hands-free: the turn ends after the pause, listening starts again by itself (/LF100/).
    #[serde(default)]
    pub hands_free: bool,
    /// The whole interface, text and readings included, 1.0 to `TEXT_SCALE_MAX` (/LL202/); from
    /// the next start.
    #[serde(default = "one")]
    pub text_scale: f32,
    /// The tutor set up in the settings (Q-15); the key is in the system keyring.
    #[serde(default)]
    pub tutor: kaiwa_ai::Tutor,
    /// The microphone picked in the settings (a cpal device id); `None` is the system's
    /// default, and so is a device that is gone.
    #[serde(default)]
    pub microphone: Option<String>,
}

fn yes() -> bool {
    true
}

fn one() -> f32 {
    1.0
}

fn default_unload() -> Option<u32> {
    DEFAULT_UNLOAD
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: String::new(),
            ui_lang: Lang::default(),
            daily_goal: DailyGoal::default(),
            gamification: true,
            feedback: Amount::Little,
            feedback_source: FeedbackSource::Default,
            script: Script::Furigana,
            end_pause_secs: kaiwa_core::turn_end::DEFAULT_PAUSE,
            keep_recordings_days: None,
            style: 0,
            placed: false,
            reminder: None,
            cost_limit_cents: None,
            mora_hints: true,
            louder: false,
            clear: false,
            unload_after_min: default_unload(),
            hands_free: false,
            text_scale: 1.0,
            tutor: kaiwa_ai::Tutor::default(),
            microphone: None,
        }
    }
}

/// The learning state in one piece (/LD020/).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Learning {
    pub model: LearnerModel,
    pub regulator: Regulator,
    pub course: CourseProgress,
    /// Each grammar point's state by catalogue id (/LF201/): used, wrong, last day.
    #[serde(default)]
    pub grammar: std::collections::BTreeMap<String, kaiwa_core::grammar::PointState>,
}

impl Learning {
    /// What the finished talk `s` changes here, by the core's one rule: the regulator's
    /// change and whether the scenario is done in the course.
    fn book(&mut self, s: &Session, success: f64) -> (Option<Change>, bool) {
        let b = book_talk(
            &mut self.model,
            &mut self.regulator,
            &mut self.course,
            &s.evidence(),
            &s.scenario.id,
            success,
            s.done(),
        );
        (b.change, b.scenario_done)
    }
}

/// A session as listed (without its full state).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRow {
    /// The id of the talk's start event (`<device>-<seq>`); `old-<n>` before the journal.
    pub id: String,
    pub scenario: String,
    pub started: Moment,
    pub ended: Option<Moment>,
    pub finished: bool,
    pub success: Option<f64>,
    pub xp: u32,
    pub speaking_secs: u32,
}

/// A disputed correction (/LF307/, /LD060/).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dispute {
    pub at: Moment,
    pub scenario: String,
    pub learner: String,
    pub original: String,
    pub better: String,
    pub explanation: String,
}

/// What [`Store::record`] did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Recorded {
    pub success: f64,
    pub change: Option<Change>,
    pub scenario_done: bool,
    pub new_items: usize,
    pub xp: u32,
    pub day: DayRecord,
}

/// What [`Store::record_on`] stored beyond the result, to redo it with the marks the learner
/// sets on the summary ([`Store::correct`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Recording {
    /// The learning state before the talk (without its grammar tracking, see `correct`).
    learning: Learning,
    /// The day the talk's practice went to.
    day: Day,
    /// The keys of the review items this talk added.
    added: Vec<String>,
}

/// The corrections a summary puts into the review, with what the review shows for them.
fn corrected_of(summary: &kaiwa_core::session::Summary) -> Vec<srs::Corrected> {
    summary
        .corrected
        .iter()
        .map(|sh| srs::Corrected {
            better: sh.correction.better.clone(),
            kind: serde_json::to_value(sh.correction.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default(),
            original: summary
                .mistakes
                .get(&kaiwa_core::norm(&sh.correction.better))
                .cloned()
                .filter(|o| !o.is_empty()),
            cue: summary
                .cues
                .get(&kaiwa_core::norm(&sh.correction.better))
                .cloned(),
        })
        .collect()
}

/// The start page in one call (/LL020/).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Today {
    pub day: Day,
    pub record: DayRecord,
    pub goal_fraction: f64,
    pub streak: Streak,
    pub due: usize,
    pub refresher: bool,
    pub open_session: Option<SessionRow>,
}

/// Every profile file in a directory.
pub fn profiles_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "kaiwa"))
        .collect();
    v.sort();
    v
}

/// Where profiles live: `$KAIWA_DATA`, else `$XDG_DATA_HOME/kaiwa/profiles`, else
/// `~/.local/share/kaiwa/profiles`.
pub fn profiles_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("KAIWA_DATA") {
        return PathBuf::from(d);
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kaiwa/profiles")
}

/// The profiles directory whose profiles have keyring entries (/LF713/): the default one. With
/// `$KAIWA_DATA` set there is none: that directory holds other profiles, with the same file
/// names as the real ones.
pub fn keyring_dir() -> Option<PathBuf> {
    keyring_dir_for(std::env::var_os("KAIWA_DATA").is_some())
}

fn keyring_dir_for(data_set: bool) -> Option<PathBuf> {
    (!data_set).then(profiles_dir)
}

/// Removes a profile completely (/LF710/, /LL402/): the database and SQLite's side files.
pub fn delete_profile(path: &Path) -> Result<()> {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let mut p = path.as_os_str().to_owned();
        p.push(suffix);
        let p = PathBuf::from(p);
        if p.exists() {
            std::fs::remove_file(p)?;
        }
    }
    Ok(())
}

pub struct Store {
    db: Connection,
}

/// A database error as the store's.
fn db(e: rusqlite::Error) -> StoreError {
    StoreError::Db(e)
}

fn moment(unix: i64, offset: i64) -> Moment {
    Moment::new(unix, offset as i32)
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(db: Connection) -> Result<Self> {
        db.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = db.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            if i == 1 {
                rename_unit_ids(&tx)?;
            }
            tx.pragma_update(None, "user_version", i as i64 + 1)?;
            tx.commit()?;
        }
        Ok(Self { db })
    }

    /// Runs several writes as one transaction (fast, and all or nothing). Inside another
    /// batch it is a savepoint: a failure takes back only its own writes. The outermost takes
    /// the write lock first, so two processes on one file lose no change.
    pub fn batch<T, E: From<StoreError>>(
        &self,
        f: impl FnOnce(&Store) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E> {
        self.db.busy_timeout(shared::BUSY_WAIT).map_err(db)?;
        let outer = self.db.is_autocommit();
        let begin = if outer {
            "BEGIN IMMEDIATE"
        } else {
            "SAVEPOINT batch"
        };
        self.db.execute_batch(begin).map_err(db)?;
        let out = f(self);
        let end = match (&out, outer) {
            (Ok(_), true) => "COMMIT",
            (Ok(_), false) => "RELEASE batch",
            (Err(_), true) => "ROLLBACK",
            (Err(_), false) => "ROLLBACK TO batch; RELEASE batch",
        };
        let ended = self.db.execute_batch(end).map_err(db);
        // A failed commit leaves the transaction open: take it back.
        if ended.is_err() && outer && !self.db.is_autocommit() {
            let _ = self.db.execute_batch("ROLLBACK");
        }
        let out = out?;
        ended?;
        Ok(out)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, sql: &str, key: &str) -> Result<Option<T>> {
        let v: Option<String> = self
            .db
            .query_row(sql, params![key], |r| r.get(0))
            .optional()?;
        Ok(match v {
            Some(s) => Some(serde_json::from_str(&s)?),
            None => None,
        })
    }

    // ── Profile ──

    pub fn profile(&self) -> Result<Profile> {
        let raw: Option<serde_json::Value> =
            self.get_json("SELECT value FROM settings WHERE key = ?1", "profile")?;
        let Some(v) = raw else {
            return Ok(Profile::default());
        };
        Ok(serde_json::from_value(v)?)
    }

    /// Reads, changes and writes the profile. A profile that cannot be read is never written
    /// over: the read error comes back and the stored row stays.
    pub fn update_profile(&self, f: impl FnOnce(&mut Profile)) -> Result<Profile> {
        let mut p = self.profile()?;
        f(&mut p);
        self.set_profile(&p)?;
        Ok(p)
    }

    /// A small value of the device's state, e.g. when the last reminder went out. Only the
    /// costs (`cost.*`) are part of the export.
    pub fn state_value<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        self.get_json(
            "SELECT value FROM settings WHERE key = ?1",
            &format!("state.{key}"),
        )
    }

    /// The state values whose key starts with `prefix`, by the rest of the key.
    fn state_values(
        &self,
        prefix: &str,
    ) -> Result<std::collections::BTreeMap<String, serde_json::Value>> {
        let mut stmt = self
            .db
            .prepare("SELECT key, value FROM settings WHERE key LIKE ?1")?;
        let strip = format!("state.{prefix}");
        let rows: Vec<(String, String)> = stmt
            .query_map(params![format!("{strip}%")], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut out = std::collections::BTreeMap::new();
        for (k, v) in rows {
            let Some(rest) = k.strip_prefix(&strip) else {
                continue;
            };
            out.insert(rest.to_string(), serde_json::from_str(&v)?);
        }
        Ok(out)
    }

    pub fn set_state_value<T: Serialize>(&self, key: &str, v: &T) -> Result<()> {
        self.db.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![format!("state.{key}"), serde_json::to_string(v)?],
        )?;
        Ok(())
    }

    pub fn set_profile(&self, p: &Profile) -> Result<()> {
        self.db.execute(
            "INSERT INTO settings (key, value) VALUES ('profile', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![serde_json::to_string(p)?],
        )?;
        Ok(())
    }

    /// The learner picks the amount of feedback: a placement no longer changes it.
    pub fn set_feedback(&self, a: Amount) -> Result<Profile> {
        self.update_profile(|p| {
            p.feedback = a;
            p.feedback_source = FeedbackSource::Chosen;
        })
    }

    // ── Learning state ──

    pub fn learning(&self) -> Result<Learning> {
        let v: Option<String> = self
            .db
            .query_row("SELECT value FROM learning WHERE id = 1", [], |r| r.get(0))
            .optional()?;
        Ok(match v {
            Some(s) => serde_json::from_str(&s)?,
            None => Learning::default(),
        })
    }

    pub fn set_learning(&self, l: &Learning) -> Result<()> {
        self.db.execute(
            "INSERT INTO learning (id, value) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET value = excluded.value",
            params![serde_json::to_string(l)?],
        )?;
        Ok(())
    }

    /// After the placement (/LF200/): books what [`book_placement`] decides.
    pub fn set_placement(&self, placed: LearnerModel) -> Result<()> {
        let mut l = self.learning()?;
        let mut p = self.profile()?;
        let booked = book_placement(std::mem::take(&mut l.model), placed, p.feedback_source);
        l.model = booked.model;
        l.regulator = booked.regulator;
        self.set_learning(&l)?;
        for (day, mut record) in self.days()? {
            if record.restart() {
                self.put_day(day, &record)?;
            }
        }
        p.placed = true;
        if let Some(amount) = booked.feedback {
            p.feedback = amount;
            p.feedback_source = FeedbackSource::Level;
        }
        self.set_profile(&p)
    }

    // ── Sessions ──

    /// Starts the talk `id`, the id of its start event.
    pub fn begin_session(&self, id: &str, state: &SessionState, at: Moment) -> Result<()> {
        self.db.execute(
            "INSERT INTO sessions (id, scenario, started, offset_min, state) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id,
                state.scenario,
                at.unix,
                at.offset_min,
                serde_json::to_string(state)?
            ],
        )?;
        Ok(())
    }

    /// A finished session as it was, without recording it again (demo history, tests).
    pub fn put_finished_session(
        &self,
        state: &SessionState,
        started: Moment,
        ended: Moment,
        success: f64,
    ) -> Result<String> {
        let id = self.begin_local(state, started)?;
        self.db.execute(
            "UPDATE sessions SET ended = ?2, finished = 1, success = ?3 WHERE id = ?1",
            params![id, ended.unix, success],
        )?;
        Ok(id)
    }

    /// The finished sessions started at or after `from` (unix seconds), oldest first.
    pub fn finished_since(&self, from: i64) -> Result<Vec<SessionRow>> {
        let mut stmt = self.db.prepare(&format!(
            "{} WHERE finished = 1 AND started >= ?1 ORDER BY started",
            Self::ROW
        ))?;
        let rows = stmt
            .query_map(params![from], Self::row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// Keeps the state after every turn, so an interruption loses nothing (/LF121/).
    pub fn save_session(&self, id: &str, state: &SessionState) -> Result<()> {
        let n = self.db.execute(
            "UPDATE sessions SET state = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(state)?],
        )?;
        if n == 0 {
            return Err(StoreError::NoSession(id.into()));
        }
        Ok(())
    }

    pub fn session_state(&self, id: &str) -> Result<SessionState> {
        let s: Option<String> = self
            .db
            .query_row(
                "SELECT state FROM sessions WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?;
        let s = s.ok_or_else(|| StoreError::NoSession(id.into()))?;
        Ok(serde_json::from_str(&s)?)
    }

    fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<SessionRow> {
        let off: i64 = r.get(3)?;
        let ended: Option<i64> = r.get(4)?;
        Ok(SessionRow {
            id: r.get(0)?,
            scenario: r.get(1)?,
            started: moment(r.get(2)?, off),
            ended: ended.map(|e| moment(e, off)),
            finished: r.get::<_, i64>(5)? != 0,
            success: r.get(6)?,
            xp: r.get::<_, i64>(7)? as u32,
            speaking_secs: r.get::<_, i64>(8)? as u32,
        })
    }

    const ROW: &'static str = "SELECT id, scenario, started, offset_min, ended, finished, success, xp, speaking_secs FROM sessions";

    /// The newest session that was left unfinished and has at least one turn: the one to offer
    /// for continuing (/LF121/).
    pub fn open_session(&self) -> Result<Option<SessionRow>> {
        let mut stmt = self.db.prepare(&format!(
            "{} WHERE finished = 0 ORDER BY started DESC LIMIT 5",
            Self::ROW
        ))?;
        let rows: Vec<SessionRow> = stmt
            .query_map([], Self::row)?
            .collect::<rusqlite::Result<_>>()?;
        for r in rows {
            if !self.session_state(&r.id)?.turns.is_empty() {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }

    pub fn sessions(&self, limit: usize) -> Result<Vec<SessionRow>> {
        let mut stmt = self
            .db
            .prepare(&format!("{} ORDER BY started DESC LIMIT ?1", Self::ROW))?;
        let rows = stmt
            .query_map(params![limit as i64], Self::row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    // ── Disputes ──

    pub fn add_dispute(&self, d: &Dispute) -> Result<()> {
        self.db.execute(
            "INSERT INTO disputes (value) VALUES (?1)",
            params![serde_json::to_string(d)?],
        )?;
        Ok(())
    }

    pub fn disputes(&self) -> Result<Vec<Dispute>> {
        let mut stmt = self.db.prepare("SELECT value FROM disputes ORDER BY id")?;
        let v: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        v.iter()
            .map(|s| serde_json::from_str(s).map_err(StoreError::from))
            .collect()
    }
}

mod cards;
mod days;
mod events;
mod migrations;
mod shared;
mod talks;
pub use events::{Stored, stored_of};
#[cfg(test)]
mod tests;
#[cfg(test)]
mod text_ids;
mod transfer;
