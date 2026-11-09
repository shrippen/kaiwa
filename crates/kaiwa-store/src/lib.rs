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

use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::curriculum::{CourseProgress, Curriculum};
use kaiwa_core::difficulty::{Change, Difficulty, Regulator};
use kaiwa_core::feedback::Amount;
use kaiwa_core::learner::LearnerModel;
use kaiwa_core::progress::{DailyGoal, DayRecord, Pause, Streak, XP_REVIEW, streak};
use kaiwa_core::session::{Session, SessionState};
use kaiwa_core::srs::{self, Grade, Item};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub const EXPORT_SCHEMA: &str = "kaiwa-export/1";

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
    NoSession(i64),
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
    /// "de" or "en" (/LL506/).
    pub ui_lang: String,
    pub daily_goal: DailyGoal,
    /// XP, streak and ring; off shows only the path (/LF605/).
    pub gamification: bool,
    pub feedback: Amount,
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
    #[serde(default = "ten_minutes")]
    pub unload_after_min: Option<u32>,
    /// Hands-free: the turn ends after the pause, listening starts again by itself (/LF100/).
    #[serde(default)]
    pub hands_free: bool,
    /// The whole interface, text and readings included, 1.0 to 2.0 (/LL202/); from the next start.
    #[serde(default = "one")]
    pub text_scale: f32,
    /// The tutor set up in the settings (Q-15); the key is in the system keyring.
    #[serde(default)]
    pub tutor: kaiwa_ai::Tutor,
}

fn yes() -> bool {
    true
}

fn one() -> f32 {
    1.0
}

fn ten_minutes() -> Option<u32> {
    Some(10)
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: String::new(),
            ui_lang: "de".into(),
            daily_goal: DailyGoal::default(),
            gamification: true,
            feedback: Amount::Little,
            script: Script::Furigana,
            end_pause_secs: 1.2,
            keep_recordings_days: None,
            style: 0,
            placed: false,
            reminder: None,
            cost_limit_cents: None,
            mora_hints: true,
            louder: false,
            clear: false,
            unload_after_min: ten_minutes(),
            hands_free: false,
            text_scale: 1.0,
            tutor: kaiwa_ai::Tutor::default(),
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

/// A session as listed (without its full state).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: i64,
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

/// Schema versions; each entry migrates from the version before.
const MIGRATIONS: &[&str] = &[
    // 1
    "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
     CREATE TABLE learning (id INTEGER PRIMARY KEY CHECK (id = 1), value TEXT NOT NULL);
     CREATE TABLE sessions (
        id INTEGER PRIMARY KEY,
        scenario TEXT NOT NULL,
        started INTEGER NOT NULL,
        offset_min INTEGER NOT NULL,
        ended INTEGER,
        finished INTEGER NOT NULL DEFAULT 0,
        success REAL,
        xp INTEGER NOT NULL DEFAULT 0,
        speaking_secs INTEGER NOT NULL DEFAULT 0,
        state TEXT NOT NULL);
     CREATE INDEX sessions_started ON sessions(started);
     CREATE TABLE items (key TEXT PRIMARY KEY, due INTEGER NOT NULL, value TEXT NOT NULL);
     CREATE INDEX items_due ON items(due);
     CREATE TABLE days (day INTEGER PRIMARY KEY, value TEXT NOT NULL);
     CREATE TABLE pauses (from_day INTEGER NOT NULL, to_day INTEGER NOT NULL);
     CREATE TABLE disputes (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
    // 2: unit and scenario ids in path order (2026-10-14, before the first release); the
    // rewrite of the stored ids is `rename_unit_ids`.
    "SELECT 1;",
    // 3: tutor presets (2026-11-09, before the first release): "openai" meant any
    // OpenAI-compatible server and is now the provider OpenAI; without a server it was
    // Ollama on this computer, with one it is "custom".
    "UPDATE settings SET value = json_set(value, '$.tutor.provider',
        CASE WHEN coalesce(json_extract(value, '$.tutor.base_url'), '') = ''
             THEN 'ollama' ELSE 'custom' END)
     WHERE key = 'profile' AND json_extract(value, '$.tutor.provider') = 'openai';",
];

/// Migration 2: unit ids before → after (scenario ids carry the unit id as prefix). Frozen;
/// one pass per text, so n5-13 → n5-03 and n5-03 → n5-04 do not chain.
const UNIT_IDS_2: &[(&str, &str)] = &[
    ("n5-13", "n5-03"),
    ("n5-03", "n5-04"),
    ("n5-04", "n5-05"),
    ("n5-05", "n5-06"),
    ("n5-06", "n5-07"),
    ("n5-16", "n5-08"),
    ("n5-07", "n5-09"),
    ("n5-08", "n5-10"),
    ("n5-14", "n5-11"),
    ("n5-09", "n5-12"),
    ("n5-10", "n5-13"),
    ("n5-11", "n5-14"),
    ("n5-12", "n5-16"),
    ("n4-07", "n4-06"),
    ("n4-06", "n4-07"),
    ("n4-13", "n4-09"),
    ("n4-09", "n4-10"),
    ("n4-10", "n4-11"),
    ("n4-14", "n4-12"),
    ("n4-11", "n4-13"),
    ("n4-12", "n4-14"),
];

/// Every unit id token (`n5-13`, also inside `n5-13-jugyou`) in `text`, renamed in one pass.
fn renamed(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < b.len() {
        let token = b.get(i..i + 5).filter(|t| {
            (t[0] == b'n')
                && (t[1] == b'4' || t[1] == b'5')
                && t[2] == b'-'
                && t[3].is_ascii_digit()
                && t[4].is_ascii_digit()
        });
        let before_ok = i == 0 || !b[i - 1].is_ascii_alphanumeric();
        let after_ok = b.get(i + 5).is_none_or(|c| !c.is_ascii_digit());
        if let (Some(t), true, true) = (token, before_ok, after_ok) {
            let t = std::str::from_utf8(t).unwrap_or_default();
            if let Some((_, new)) = UNIT_IDS_2.iter().find(|(old, _)| *old == t) {
                out.push_str(new);
                i += 5;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap_or_default();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Rewrites the stored ids of migration 2 in every JSON text of the profile.
fn rename_unit_ids(db: &Connection) -> Result<()> {
    for (table, key, cols) in [
        ("learning", "id", &["value"][..]),
        ("sessions", "id", &["scenario", "state"][..]),
        ("items", "key", &["value"][..]),
        ("days", "day", &["value"][..]),
        ("disputes", "id", &["value"][..]),
    ] {
        for col in cols {
            let rows: Vec<(rusqlite::types::Value, String)> = db
                .prepare(&format!("SELECT {key}, {col} FROM {table}"))?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            for (k, v) in rows {
                let new = renamed(&v);
                if new != v {
                    db.execute(
                        &format!("UPDATE {table} SET {col} = ?1 WHERE {key} = ?2"),
                        params![new, k],
                    )?;
                }
            }
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

    pub fn schema_version(&self) -> Result<usize> {
        let v: i64 = self
            .db
            .pragma_query_value(None, "user_version", |r| r.get(0))?;
        Ok(v as usize)
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
        Ok(self
            .get_json("SELECT value FROM settings WHERE key = ?1", "profile")?
            .unwrap_or_default())
    }

    /// A small value of the device's state (not part of the export), e.g. when the last
    /// reminder went out.
    pub fn state_value<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        self.get_json(
            "SELECT value FROM settings WHERE key = ?1",
            &format!("state.{key}"),
        )
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

    /// After the placement (/LF200/): the model and the difficulty that fits it.
    pub fn set_placement(&self, model: LearnerModel) -> Result<()> {
        let mut l = self.learning()?;
        l.regulator = Regulator::new(Difficulty::for_learner(&model));
        l.model = model;
        self.set_learning(&l)?;
        let mut p = self.profile()?;
        p.placed = true;
        self.set_profile(&p)
    }

    // ── Sessions ──

    pub fn begin_session(&self, state: &SessionState, at: Moment) -> Result<i64> {
        self.db.execute(
            "INSERT INTO sessions (scenario, started, offset_min, state) VALUES (?1, ?2, ?3, ?4)",
            params![
                state.scenario,
                at.unix,
                at.offset_min,
                serde_json::to_string(state)?
            ],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// A finished session as it was, without recording it again (demo history, tests).
    pub fn put_finished_session(
        &self,
        state: &SessionState,
        started: Moment,
        ended: Moment,
        success: f64,
    ) -> Result<i64> {
        let id = self.begin_session(state, started)?;
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
    pub fn save_session(&self, id: i64, state: &SessionState) -> Result<()> {
        let n = self.db.execute(
            "UPDATE sessions SET state = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(state)?],
        )?;
        if n == 0 {
            return Err(StoreError::NoSession(id));
        }
        Ok(())
    }

    pub fn session_state(&self, id: i64) -> Result<SessionState> {
        let s: Option<String> = self
            .db
            .query_row(
                "SELECT state FROM sessions WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?;
        let s = s.ok_or(StoreError::NoSession(id))?;
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
            if !self.session_state(r.id)?.turns.is_empty() {
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

    /// Drops an unfinished session the learner does not want to continue.
    pub fn discard_session(&self, id: i64) -> Result<()> {
        self.db.execute(
            "DELETE FROM sessions WHERE id = ?1 AND finished = 0",
            params![id],
        )?;
        Ok(())
    }

    /// Ends a session: learner model, regulator, path, review items and the day (/LF201/,
    /// /LF210/, /LF220/, /LF230/, /LF601/).
    pub fn record(
        &self,
        id: i64,
        s: &Session,
        started: Moment,
        ended: Moment,
        curriculum: Option<&Curriculum>,
    ) -> Result<Recorded> {
        let stats = s.stats();
        let success = stats.success();
        let summary = s.summary();
        let mut l = self.learning()?;
        l.model.update(&s.evidence());
        let change = if stats.turns > 0 {
            l.regulator.after_session(success, &l.model)
        } else {
            None
        };
        let scenario_done = s.done() && l.course.complete(&s.scenario.id, success);

        let unit = curriculum
            .and_then(|c| c.unit_of(&s.scenario.id))
            .map(|u| u.id.clone());
        let mut items = self.items()?;
        let before = items.len();
        let corrections: Vec<srs::Corrected> = summary
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
            .collect();
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

        // Practice time: the session's wall time, at most an hour, in the share of spoken turns.
        let wall = (ended.unix - started.unix).clamp(0, 3600) as f64;
        let spoken_share = if stats.turns == 0 {
            0.0
        } else {
            f64::from(stats.turns - stats.typed_turns) / f64::from(stats.turns)
        };
        let speaking = (wall * spoken_share).round() as u32;
        let xp = summary.xp.xp;
        let day = started.day();
        let mut rec = self.day(day)?;
        rec.speaking_secs += speaking;
        rec.typed_turns += stats.typed_turns;
        rec.xp += xp;
        if scenario_done || s.done() {
            rec.scenarios_done += 1;
        }
        self.put_day(day, &rec)?;
        self.set_learning(&l)?;
        self.db.execute(
            "UPDATE sessions SET ended = ?2, finished = 1, success = ?3, xp = ?4, speaking_secs = ?5, state = ?6
             WHERE id = ?1",
            params![
                id,
                ended.unix,
                success,
                xp,
                speaking,
                serde_json::to_string(&s.snapshot())?
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

    pub fn last_practice(&self) -> Result<Option<Moment>> {
        let r: Option<(i64, i64)> = self
            .db
            .query_row(
                "SELECT COALESCE(ended, started), offset_min FROM sessions ORDER BY started DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        Ok(r.map(|(u, o)| moment(u, o)))
    }

    /// The items to review now (/LF230/, /LF231/).
    pub fn due(&self, now: Moment, limit: usize) -> Result<Vec<Item>> {
        let items = self.items()?;
        let last = self.last_practice()?;
        Ok(srs::due(&items, now, last, limit)
            .into_iter()
            .cloned()
            .collect())
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
        let key = kaiwa_core::norm(text);
        let Some(mut item): Option<Item> =
            self.get_json("SELECT value FROM items WHERE key = ?1", &key)?
        else {
            return Ok(None);
        };
        item.review(grade, now);
        self.put_item(&item)?;
        if let Some(unit) = &item.unit {
            let mut l = self.learning()?;
            l.course.reviewed(unit, grade != Grade::Again);
            self.set_learning(&l)?;
        }
        let mut rec = self.day(now.day())?;
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
        let last = self.last_practice()?;
        Ok(Today {
            day,
            goal_fraction: record.fraction(profile.daily_goal),
            record,
            streak: streak(&days, profile.daily_goal, &pauses, day),
            due: srs::due(&items, now, last, usize::MAX).len(),
            refresher: srs::refresher(now, last),
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

    // ── Export, import (/LF710/) ──

    /// Everything of the profile as one JSON value (`spec/export.schema.json`).
    pub fn export(&self, now: Moment) -> Result<serde_json::Value> {
        let mut stmt = self
            .db
            .prepare(&format!("{} ORDER BY started", Self::ROW))?;
        let rows: Vec<SessionRow> = stmt
            .query_map([], Self::row)?
            .collect::<rusqlite::Result<_>>()?;
        let mut sessions = Vec::new();
        for r in rows {
            let state = self.session_state(r.id)?;
            sessions.push(serde_json::json!({ "row": r, "state": state }));
        }
        let days: Vec<serde_json::Value> = self
            .days()?
            .into_iter()
            .map(|(d, r)| serde_json::json!({ "day": d.iso(), "n": d.0, "record": r }))
            .collect();
        Ok(serde_json::json!({
            "schema": EXPORT_SCHEMA,
            "exported_at": now,
            "profile": self.profile()?,
            "learning": self.learning()?,
            "sessions": sessions,
            "items": self.items()?,
            "days": days,
            "pauses": self.pauses()?,
            "disputes": self.disputes()?,
        }))
    }

    /// Reads an export into this (new, empty) profile.
    pub fn import(&self, v: &serde_json::Value) -> Result<()> {
        if v["schema"] != EXPORT_SCHEMA {
            return Err(StoreError::WrongFormat(v["schema"].to_string()));
        }
        let empty: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
        if empty > 0 || !self.items()?.is_empty() {
            return Err(StoreError::NotEmpty);
        }
        let tx = self.db.unchecked_transaction()?;
        self.set_profile(&serde_json::from_value(v["profile"].clone())?)?;
        self.set_learning(&serde_json::from_value(v["learning"].clone())?)?;
        for s in v["sessions"].as_array().into_iter().flatten() {
            let row: SessionRow = serde_json::from_value(s["row"].clone())?;
            let state: SessionState = serde_json::from_value(s["state"].clone())?;
            self.db.execute(
                "INSERT INTO sessions (id, scenario, started, offset_min, ended, finished, success, xp, speaking_secs, state)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    row.id,
                    row.scenario,
                    row.started.unix,
                    row.started.offset_min,
                    row.ended.map(|e| e.unix),
                    i64::from(row.finished),
                    row.success,
                    row.xp,
                    row.speaking_secs,
                    serde_json::to_string(&state)?
                ],
            )?;
        }
        for i in v["items"].as_array().into_iter().flatten() {
            self.put_item(&serde_json::from_value(i.clone())?)?;
        }
        for d in v["days"].as_array().into_iter().flatten() {
            let n = d["n"]
                .as_i64()
                .ok_or_else(|| StoreError::WrongFormat("day".into()))?;
            self.put_day(Day(n), &serde_json::from_value(d["record"].clone())?)?;
        }
        for p in v["pauses"].as_array().into_iter().flatten() {
            self.add_pause(serde_json::from_value(p.clone())?)?;
        }
        for d in v["disputes"].as_array().into_iter().flatten() {
            self.add_dispute(&serde_json::from_value(d.clone())?)?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
