//! Export and import of the whole profile (/LF710/, `spec/export.schema.json`).

use kaiwa_core::session::SessionState;
use rusqlite::params;

use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::srs::Item;

use crate::events::{Stored, stored_of};
use crate::{
    COST_STATE_PREFIX, CostMonth, EXPORT_SCHEMA, IMPORTED_STATE, REVIEWED, Result, SessionRow,
    Store, StoreError, is_month_key,
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
        if !self.is_empty()? {
            return Err(StoreError::NotEmpty);
        }
        self.batch(|st| {
            st.load_state(v)?;
            st.import_events(v)?;
            st.set_state_value(IMPORTED_STATE, &true)
        })
    }

    /// No talk, no card, no import and no sync yet: an import or another device's journal
    /// may come in. A file without sessions or items leaves no trace but the import's
    /// marker. A profile that syncs folds its state from the journal, so an import would be
    /// lost there.
    pub fn is_empty(&self) -> Result<bool> {
        let talks: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
        let imported: Option<bool> = self.state_value(IMPORTED_STATE)?;
        let syncs: bool = self.db.query_row(
            "SELECT EXISTS (SELECT 1 FROM events WHERE json_extract(body, '$.kind') = 'base')",
            [],
            |r| r.get(0),
        )?;
        Ok(talks == 0 && self.items()?.is_empty() && imported.is_none() && !syncs)
    }

    /// Reads the state of an export (all but its journal) into this profile: the import, and
    /// the journal's start (ADR-010 `Base`) when it is folded.
    pub fn load_state(&self, v: &serde_json::Value) -> Result<()> {
        self.set_profile(&serde_json::from_value(v["profile"].clone())?)?;
        self.set_learning(&serde_json::from_value(v["learning"].clone())?)?;
        self.import_sessions(v)?;
        let mut reviewed: Option<Moment> = None;
        for i in array(v, "items")? {
            let item: Item = serde_json::from_value(i.clone())?;
            reviewed = reviewed.max(item.last_review);
            self.put_item(&item)?;
        }
        // `state` is not exported; without this, reviewing since the last talk reads as a break.
        if let Some(at) = reviewed {
            self.set_state_value(REVIEWED, &at)?;
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
        let events: Vec<Stored> = array(v, "events")?
            .iter()
            .map(stored_of)
            .collect::<Result<_>>()?;
        self.add_events(&events).map(|_| ())
    }
}

/// A required array of an export; missing is an error, so a cut-off file imports nothing.
fn array<'a>(v: &'a serde_json::Value, key: &str) -> Result<&'a Vec<serde_json::Value>> {
    v[key]
        .as_array()
        .ok_or_else(|| StoreError::WrongFormat(key.into()))
}
