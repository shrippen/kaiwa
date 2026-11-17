//! Reading the parts of an export (`spec/export.schema.json`) that are rows of a table.

use kaiwa_core::Event;
use kaiwa_core::session::SessionState;
use rusqlite::params;

use crate::{Result, SessionRow, Store, StoreError, array};

impl Store {
    /// The talks of an export, with the ids they had.
    pub(crate) fn import_sessions(&self, v: &serde_json::Value) -> Result<()> {
        for s in array(v, "sessions")? {
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
        Ok(())
    }

    /// The journal of an export, as it was.
    pub(crate) fn import_events(&self, v: &serde_json::Value) -> Result<()> {
        let wrong = || StoreError::WrongFormat("events".into());
        for e in array(v, "events")? {
            let text = |k: &str| e[k].as_str().ok_or_else(wrong);
            let num = |k: &str| e[k].as_i64().ok_or_else(wrong);
            let body: Event = serde_json::from_value(e["body"].clone()).map_err(|_| wrong())?;
            self.db.execute(
                "INSERT INTO events (id, device, seq, at, offset_min, body) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    text("id")?,
                    text("device")?,
                    num("seq")?,
                    num("at")?,
                    num("offset_min")?,
                    serde_json::to_string(&body)?
                ],
            )?;
        }
        Ok(())
    }
}
