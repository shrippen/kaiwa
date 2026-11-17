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

    /// Runs several writes as one transaction (fast, and all or nothing).
    pub fn batch<T>(&self, f: impl FnOnce(&Store) -> Result<T>) -> Result<T> {
        let tx = self.db.unchecked_transaction()?;
        let out = f(self)?;
        tx.commit()?;
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

    /// Ends a session: learner model, regulator, path, review items and the day (/LF201/,
    /// /LF210/, /LF220/, /LF230/, /LF601/).
    pub fn record(
        &self,
        id: &str,
        s: &Session,
        started: Moment,
        ended: Moment,
        curriculum: Option<&Curriculum>,
    ) -> Result<Recorded> {
        self.record_on(id, s, started, ended, started.day(), curriculum)
    }

    /// As [`Store::record`], for a talk of several sittings (/LF121/): `started` is the end
    /// minus all the time spent in it, and `day` the day its last sitting began, which gets
    /// the practice (a talk begun yesterday and finished after midnight counts for today).
    pub fn record_on(
        &self,
        id: &str,
        s: &Session,
        started: Moment,
        ended: Moment,
        day: Day,
        curriculum: Option<&Curriculum>,
    ) -> Result<Recorded> {
        let stats = s.stats();
        let success = stats.success();
        let summary = s.summary();
        let mut l = self.learning()?;
        let learning_before = l.clone();
        // Where the day started from, for the week's progress per skill (/LF604/).
        let levels_before: std::collections::BTreeMap<_, _> = kaiwa_core::learner::SKILLS
            .iter()
            .map(|k| (*k, l.model.level(*k)))
            .collect();
        let (change, scenario_done) = l.book(s, success);

        let unit = curriculum
            .and_then(|c| c.unit_of(&s.scenario.id))
            .map(|u| u.id.clone());
        let mut items = self.items()?;
        let before = items.len();
        let corrections = corrected_of(&summary);
        srs::add_from_summary(
            &mut items,
            &summary.review,
            &corrections,
            unit.as_deref(),
            ended,
        );
        for item in &items[before..] {
            self.put_item(item)?;
        }
        let recording = Recording {
            learning: learning_before,
            day,
            added: items[before..]
                .iter()
                .map(|i| kaiwa_core::norm(&i.text))
                .collect(),
        };

        // A turn whose analysis had gaps was spoken too (T-11), so the share and the day's
        // typed turns count all turns.
        let typed = turn_counts(s.turns()).1;
        let speaking = s.spoken_secs((ended.unix - started.unix) as f64);
        let xp = summary.xp.xp;
        let mut rec = self.day(day)?;
        rec.keep_goal(day, ended.day(), self.profile()?.daily_goal);
        rec.add_talk(
            levels_before,
            speaking,
            typed,
            xp,
            scenario_done || s.done(),
        );
        self.put_day(day, &rec)?;
        self.set_learning(&l)?;
        self.db.execute(
            "UPDATE sessions SET ended = ?2, finished = 1, success = ?3, xp = ?4, speaking_secs = ?5, state = ?6,
             recording = ?7 WHERE id = ?1",
            params![
                id,
                ended.unix,
                success,
                xp,
                speaking,
                serde_json::to_string(&s.snapshot())?,
                serde_json::to_string(&recording)?
            ],
        )?;
        Ok(Recorded {
            success,
            change,
            scenario_done,
            new_items: items.len() - before,
            xp,
            day: rec,
        })
    }

    /// The learner marked turns of a recorded talk on the summary (V-05): redoes what
    /// [`Store::record_on`] stored from `s` as it is now. The learner model, the regulator and
    /// the course are replayed from before the talk, the review items this talk added and the
    /// marks no longer justify are removed, the stored talk and its success are updated; all
    /// in one transaction. `retrack` redoes the grammar tracking on the replayed learning
    /// state, with the moment the talk ended. Only the newest talk can be corrected: a later
    /// one built on its result (`Superseded`).
    pub fn correct<F: FnOnce(&mut Learning, Moment)>(
        &self,
        id: &str,
        s: &Session,
        retrack: F,
    ) -> Result<Recorded> {
        self.batch(|st| st.correct_in(id, s, retrack))
    }

    fn correct_in<F: FnOnce(&mut Learning, Moment)>(
        &self,
        id: &str,
        s: &Session,
        retrack: F,
    ) -> Result<Recorded> {
        let row = self
            .db
            .query_row(
                &format!("{} WHERE id = ?1", Self::ROW),
                params![id],
                Self::row,
            )
            .optional()?
            .ok_or_else(|| StoreError::NoSession(id.into()))?;
        let stored: Option<String> = self.db.query_row(
            "SELECT recording FROM sessions WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        let (Some(stored), true, Some(ended)) = (stored, row.finished, row.ended) else {
            return Err(StoreError::NoRecording(id.into()));
        };
        let mut recording: Recording = serde_json::from_str(&stored)?;
        // The newest is the one recorded last: a talk begun earlier but finished later is.
        let later: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM sessions
             WHERE finished = 1 AND (ended > ?2 OR (ended = ?2 AND rowid > (SELECT rowid FROM sessions WHERE id = ?1)))",
            params![id, ended.unix],
            |r| r.get(0),
        )?;
        if later > 0 {
            return Err(StoreError::Superseded(id.into()));
        }

        // The learner side as `record_on` made it, from the evidence with the marks.
        let stats = s.stats();
        let success = stats.success();
        let mut l = recording.learning.clone();
        let (change, scenario_done) = l.book(s, success);
        retrack(&mut l, ended);
        self.set_learning(&l)?;

        // The review items: what this talk added stays only if the summary still has it.
        let summary = s.summary();
        let review: Vec<String> = summary.review.iter().map(|r| kaiwa_core::norm(r)).collect();
        let (keep, drop): (Vec<String>, Vec<String>) = recording
            .added
            .into_iter()
            .partition(|key| review.contains(key));
        for key in &drop {
            self.db
                .execute("DELETE FROM items WHERE key = ?1", params![key])?;
        }
        // A card that stays is built from the turns that stay, as if the marked turn had
        // been marked before the end.
        let corrections = corrected_of(&summary);
        for key in &keep {
            let Some(mut item) = self.item(key)? else {
                continue;
            };
            let found = corrections
                .iter()
                .find(|c| kaiwa_core::norm(&c.better) == *key);
            (item.kind, item.source) = srs::card_kind(&item.text, found);
            item.cue = found.and_then(|c| c.cue.clone());
            item.original = found.and_then(|c| c.original.clone());
            self.put_item(&item)?;
        }
        recording.added = keep;

        self.db.execute(
            "UPDATE sessions SET success = ?2, state = ?3, recording = ?4 WHERE id = ?1",
            params![
                id,
                success,
                serde_json::to_string(&s.snapshot())?,
                serde_json::to_string(&recording)?
            ],
        )?;
        Ok(Recorded {
            success,
            change,
            scenario_done,
            new_items: recording.added.len(),
            xp: row.xp,
            day: self.day(recording.day)?,
        })
    }

    // ── Review ──

    pub fn items(&self) -> Result<Vec<Item>> {
        let mut stmt = self.db.prepare("SELECT value FROM items")?;
        let v: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        v.iter()
            .map(|s| serde_json::from_str(s).map_err(StoreError::from))
            .collect()
    }

    pub fn put_item(&self, item: &Item) -> Result<()> {
        self.db.execute(
            "INSERT INTO items (key, due, value) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET due = excluded.due, value = excluded.value",
            params![
                kaiwa_core::norm(&item.text),
                item.due.0,
                serde_json::to_string(item)?
            ],
        )?;
        Ok(())
    }

    /// Adds an item by hand, e.g. from the word lookup (/LF112/).
    pub fn add_item(&self, item: &Item) -> Result<bool> {
        let n = self.db.execute(
            "INSERT OR IGNORE INTO items (key, due, value) VALUES (?1, ?2, ?3)",
            params![
                kaiwa_core::norm(&item.text),
                item.due.0,
                serde_json::to_string(item)?
            ],
        )?;
        Ok(n > 0)
    }

    /// When the learner last practised: the newest conversation or review answer (T-07:
    /// reviewing counts as practice).
    pub fn last_practice(&self) -> Result<Option<Moment>> {
        let r: Option<(i64, i64)> = self
            .db
            .query_row(
                "SELECT COALESCE(ended, started), offset_min FROM sessions ORDER BY started DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let talked = r.map(|(u, o)| moment(u, o));
        // Kept by `review`, so the start page does not read every item for it (/LL020/).
        let reviewed: Option<Moment> = self.state_value("reviewed")?;
        Ok(talked.max(reviewed))
    }

    /// The day of the last re-entry after a break, if there was one.
    pub fn reentered(&self) -> Result<Option<Day>> {
        self.state_value("reentered")
    }

    /// The re-entry after a break (/LF231/, T-07), when [`srs::new_break`] says it is one: ten
    /// items stay due, the rest is spread over the next days. The day is kept, so the
    /// refresher lasts the whole day whatever the learner opens first.
    pub fn reenter(&self, now: Moment) -> Result<()> {
        if !srs::new_break(now, self.last_practice()?, self.reentered()?) {
            return Ok(());
        }
        let mut items = self.items()?;
        let before: Vec<Day> = items.iter().map(|i| i.due).collect();
        srs::spread_after_break(&mut items, now);
        // One transaction: a long break moves thousands of items, and all or none.
        let write = |st: &Store| -> Result<()> {
            for (item, was) in items.iter().zip(&before) {
                if item.due != *was {
                    st.put_item(item)?;
                }
            }
            st.set_state_value("reentered", &now.day())
        };
        if self.db.is_autocommit() {
            self.batch(write)
        } else {
            write(self)
        }
    }

    /// The items to review now (/LF230/, /LF231/).
    pub fn due(&self, now: Moment, limit: usize) -> Result<Vec<Item>> {
        let items = self.items()?;
        Ok(srs::due(&items, now, limit).into_iter().cloned().collect())
    }

    /// Like [`Store::due`], but only the items of `unit`.
    pub fn due_in(&self, unit: &str, now: Moment, limit: usize) -> Result<Vec<Item>> {
        let items: Vec<Item> = self
            .items()?
            .into_iter()
            .filter(|i| i.unit.as_deref() == Some(unit))
            .collect();
        Ok(srs::due(&items, now, limit).into_iter().cloned().collect())
    }

    /// The review item of `text`, if there is one.
    pub fn item(&self, text: &str) -> Result<Option<Item>> {
        self.get_json(
            "SELECT value FROM items WHERE key = ?1",
            &kaiwa_core::norm(text),
        )
    }

    /// One review answer: FSRS, the unit's review record, the day.
    pub fn review(&self, text: &str, grade: Grade, now: Moment) -> Result<Option<Item>> {
        // One transaction: a failure part-way leaves no half a review.
        self.batch(|s| s.review_in(text, grade, now))
    }

    fn review_in(&self, text: &str, grade: Grade, now: Moment) -> Result<Option<Item>> {
        let key = kaiwa_core::norm(text);
        let Some(mut item): Option<Item> =
            self.get_json("SELECT value FROM items WHERE key = ?1", &key)?
        else {
            return Ok(None);
        };
        item.review(grade, now);
        self.put_item(&item)?;
        self.set_state_value("reviewed", &now)?;
        if let Some(unit) = &item.unit {
            let mut l = self.learning()?;
            l.course.reviewed(unit, grade != Grade::Again);
            self.set_learning(&l)?;
        }
        let mut rec = self.day(now.day())?;
        rec.goal = Some(self.profile()?.daily_goal);
        rec.reviews_done += 1;
        rec.xp += XP_REVIEW;
        self.put_day(now.day(), &rec)?;
        Ok(Some(item))
    }

    // ── Days, streak, pauses ──

    pub fn day(&self, d: Day) -> Result<DayRecord> {
        let v: Option<String> = self
            .db
            .query_row("SELECT value FROM days WHERE day = ?1", params![d.0], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(match v {
            Some(s) => serde_json::from_str(&s)?,
            None => DayRecord::default(),
        })
    }

    pub fn put_day(&self, d: Day, r: &DayRecord) -> Result<()> {
        self.db.execute(
            "INSERT INTO days (day, value) VALUES (?1, ?2)
             ON CONFLICT(day) DO UPDATE SET value = excluded.value",
            params![d.0, serde_json::to_string(r)?],
        )?;
        Ok(())
    }

    /// The learner chose another daily goal on `today`: a day already begun takes it over, so
    /// it is judged by the goal it ended with, tomorrow as today (/LF602/). Days before keep
    /// theirs.
    pub fn set_day_goal(&self, today: Day, goal: DailyGoal) -> Result<()> {
        // From today on: after a flight west "today" can lie before a day already begun.
        for (day, mut rec) in self.days()?.split_off(&today) {
            rec.goal = Some(goal);
            self.put_day(day, &rec)?;
        }
        Ok(())
    }

    pub fn days(&self) -> Result<BTreeMap<Day, DayRecord>> {
        let mut stmt = self.db.prepare("SELECT day, value FROM days")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        rows.into_iter()
            .map(|(d, v)| Ok((Day(d), serde_json::from_str(&v)?)))
            .collect()
    }

    pub fn pauses(&self) -> Result<Vec<Pause>> {
        let mut stmt = self.db.prepare("SELECT from_day, to_day FROM pauses")?;
        let v = stmt
            .query_map([], |r| {
                Ok(Pause {
                    from: Day(r.get(0)?),
                    to: Day(r.get(1)?),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(v)
    }

    /// Until when a pause runs on `today`: the latest end of the pauses that hold it.
    pub fn paused_until(&self, today: Day) -> Result<Option<Day>> {
        Ok(self
            .pauses()?
            .iter()
            .filter(|p| p.from <= today && today <= p.to)
            .map(|p| p.to)
            .max())
    }

    /// Ends the pauses running on `today` (T-08): they held up to yesterday, today counts
    /// again; one that began today is gone. Pauses that are over stay as they were.
    pub fn end_pause(&self, today: Day) -> Result<()> {
        self.batch(|st| {
            st.db.execute(
                "DELETE FROM pauses WHERE from_day = ?1 AND to_day >= ?1",
                params![today.0],
            )?;
            st.db.execute(
                "UPDATE pauses SET to_day = ?1 - 1 WHERE from_day < ?1 AND to_day >= ?1",
                params![today.0],
            )?;
            Ok(())
        })
    }

    /// Pause mode (holiday, illness): the streak neither counts nor breaks (/LF602/).
    pub fn add_pause(&self, p: Pause) -> Result<()> {
        self.db.execute(
            "INSERT INTO pauses (from_day, to_day) VALUES (?1, ?2)",
            params![p.from.0, p.to.0],
        )?;
        Ok(())
    }

    /// The start page (/LL020/).
    pub fn today(&self, now: Moment) -> Result<Today> {
        let profile = self.profile()?;
        let days = self.days()?;
        let pauses = self.pauses()?;
        let day = now.day();
        let record = days.get(&day).cloned().unwrap_or_default();
        let items = self.items()?;
        Ok(Today {
            day,
            goal_fraction: record.fraction(profile.daily_goal),
            record,
            streak: streak(&days, profile.daily_goal, &pauses, day),
            due: srs::due(&items, now, usize::MAX).len(),
            // The day of the re-entry is the refresher's, whatever was opened since.
            refresher: self.reentered()? == Some(day),
            open_session: self.open_session()?,
        })
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

mod events;
mod migrations;
mod shared;
pub use events::Stored;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod text_ids;
mod transfer;
