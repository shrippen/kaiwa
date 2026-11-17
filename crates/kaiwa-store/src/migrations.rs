//! The schema versions of the profile file.

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
];
