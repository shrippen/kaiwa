//! Export and import of the whole profile (/LF710/, `spec/export.schema.json`).

use kaiwa_core::Event;
use kaiwa_core::session::SessionState;
use rusqlite::params;

use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::srs::Item;

use crate::{
    COST_STATE_PREFIX, CostMonth, EXPORT_SCHEMA, IMPORTED_STATE, Result, SessionRow, Store,
    StoreError, is_month_key,
};

impl Store {
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
            let state = self.session_state(&r.id)?;
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
            "costs": self.state_values(COST_STATE_PREFIX)?,
            "events": self.events_json()?,
        }))
    }

    /// Reads an export into this (new, empty) profile.
    pub fn import(&self, v: &serde_json::Value) -> Result<()> {
        if v["schema"].as_str() != Some(EXPORT_SCHEMA) {
            return Err(StoreError::WrongFormat(v["schema"].to_string()));
        }
        let profile = v["profile"].clone();
        let empty: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
        // A file without sessions or items leaves no trace but the marker.
        let imported: Option<bool> = self.state_value(IMPORTED_STATE)?;
        if empty > 0 || !self.items()?.is_empty() || imported.is_some() {
            return Err(StoreError::NotEmpty);
        }
        let tx = self.db.unchecked_transaction()?;
        self.set_profile(&serde_json::from_value(profile)?)?;
        self.set_learning(&serde_json::from_value(v["learning"].clone())?)?;
        self.import_sessions(v)?;
        self.import_events(v)?;
        let mut reviewed: Option<Moment> = None;
        for i in array(v, "items")? {
            let item: Item = serde_json::from_value(i.clone())?;
            reviewed = reviewed.max(item.last_review);
            self.put_item(&item)?;
        }
        // `state` is not exported; without this, reviewing since the last talk reads as a break.
        if let Some(at) = reviewed {
            self.set_state_value("reviewed", &at)?;
        }
        for d in array(v, "days")? {
            let n = d["n"]
                .as_i64()
                .ok_or_else(|| StoreError::WrongFormat("day".into()))?;
            self.put_day(Day(n), &serde_json::from_value(d["record"].clone())?)?;
        }
        for p in array(v, "pauses")? {
            self.add_pause(serde_json::from_value(p.clone())?)?;
        }
        for d in array(v, "disputes")? {
            self.add_dispute(&serde_json::from_value(d.clone())?)?;
        }
        // A placement may have booked costs before the import: the file's month adds to them.
        let wrong = || StoreError::WrongFormat("costs".into());
        for (month, m) in v["costs"].as_object().ok_or_else(wrong)? {
            let add: CostMonth = serde_json::from_value(m.clone()).map_err(|_| wrong())?;
            if !is_month_key(month) {
                return Err(wrong());
            }
            let key = format!("{COST_STATE_PREFIX}{month}");
            let mut sum: CostMonth = self.state_value(&key)?.unwrap_or_default();
            sum.add(&add);
            self.set_state_value(&key, &sum)?;
        }
        self.set_state_value(IMPORTED_STATE, &true)?;
        tx.commit()?;
        Ok(())
    }

    /// The talks of an export, with the ids they had.
    fn import_sessions(&self, v: &serde_json::Value) -> Result<()> {
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
    fn import_events(&self, v: &serde_json::Value) -> Result<()> {
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

/// A required array of an export; missing is an error, so a cut-off file imports nothing.
pub(crate) fn array<'a>(v: &'a serde_json::Value, key: &str) -> Result<&'a Vec<serde_json::Value>> {
    v[key]
        .as_array()
        .ok_or_else(|| StoreError::WrongFormat(key.into()))
}
