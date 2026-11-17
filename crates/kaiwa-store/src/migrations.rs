//! The schema versions of the profile file, and the rewrite of migration 2.

use rusqlite::{Connection, params};

use crate::Result;

/// Schema versions; each entry migrates from the version before.
pub(super) const MIGRATIONS: &[&str] = &[
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
    // 4: what the end of a talk stored, to take back when the summary marks a turn (V-05,
    // 2026-11-13); NULL for talks recorded before.
    "ALTER TABLE sessions ADD COLUMN recording TEXT;",
    // 5: the journal (ADR-010): one row per act of the learner, id `<device>-<seq>`.
    "CREATE TABLE events (
        id TEXT PRIMARY KEY,
        device TEXT NOT NULL,
        seq INTEGER NOT NULL,
        at INTEGER NOT NULL,
        offset_min INTEGER NOT NULL,
        body TEXT NOT NULL);",
    // 6: where the amount of feedback comes from (V-02), for profiles from before it
    // (2026-11-17, before the first release). Any amount but "little" was a pick; "little"
    // was the fixed default, and a skipped placement cannot be told from a real one.
    "UPDATE settings SET value = json_set(value, '$.feedback_source',
        CASE WHEN json_extract(value, '$.feedback') IN ('normal', 'much')
             THEN 'chosen' ELSE 'default' END)
     WHERE key = 'profile' AND json_type(value, '$.feedback_source') IS NULL;",
    // 7: talk ids as text (ADR-010 step 2, 2026-11-17, before the first release); talks from
    // before the journal keep their number as `old-<n>`.
    "CREATE TABLE sessions_new (
        id TEXT PRIMARY KEY,
        scenario TEXT NOT NULL,
        started INTEGER NOT NULL,
        offset_min INTEGER NOT NULL,
        ended INTEGER,
        finished INTEGER NOT NULL DEFAULT 0,
        success REAL,
        xp INTEGER NOT NULL DEFAULT 0,
        speaking_secs INTEGER NOT NULL DEFAULT 0,
        state TEXT NOT NULL,
        recording TEXT);
     INSERT INTO sessions_new
        SELECT 'old-' || id, scenario, started, offset_min, ended, finished, success, xp,
               speaking_secs, state, recording
        FROM sessions;
     DROP TABLE sessions;
     ALTER TABLE sessions_new RENAME TO sessions;
     CREATE INDEX sessions_started ON sessions(started);",
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
pub(super) fn renamed(text: &str) -> String {
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
pub(super) fn rename_unit_ids(db: &Connection) -> Result<()> {
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
