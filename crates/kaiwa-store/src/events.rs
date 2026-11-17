//! The journal (ADR-010): everything the learner did, one row per event, in the order every
//! device folds them.

use kaiwa_core::session::SessionState;
use kaiwa_core::{Event, Moment};
use rusqlite::params;

use crate::{Result, Store};

/// Device state: this device's id in the journal, made once.
const DEVICE_STATE: &str = "device";
const DEVICE_ID_BYTES: usize = 8;

/// An event as the journal keeps it.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    /// `<device>-<seq>`.
    pub id: String,
    pub device: String,
    pub seq: i64,
    pub at: Moment,
    pub event: Event,
}

impl Store {
    /// This device's id: random, made on first use, never changed.
    pub fn device(&self) -> Result<String> {
        if let Some(id) = self.state_value(DEVICE_STATE)? {
            return Ok(id);
        }
        let id: String = self.db.query_row(
            &format!("SELECT lower(hex(randomblob({DEVICE_ID_BYTES})))"),
            [],
            |r| r.get(0),
        )?;
        self.set_state_value(DEVICE_STATE, &id)?;
        Ok(id)
    }

    /// Appends `event` as the next of this device.
    pub fn append_event(&self, event: &Event, at: Moment) -> Result<Stored> {
        let device = self.device()?;
        let seq: i64 = self.db.query_row(
            "SELECT coalesce(max(seq), 0) + 1 FROM events WHERE device = ?1",
            params![device],
            |r| r.get(0),
        )?;
        let id = format!("{device}-{seq}");
        self.db.execute(
            "INSERT INTO events (id, device, seq, at, offset_min, body) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                device,
                seq,
                at.unix,
                at.offset_min,
                serde_json::to_string(event)?
            ],
        )?;
        Ok(Stored {
            id,
            device,
            seq,
            at,
            event: event.clone(),
        })
    }

    /// Starts a talk outside the journal (demo history, tests, the simulation's setup); its id
    /// is `local-<random>`, which no journal id can equal.
    pub fn begin_local(&self, state: &SessionState, at: Moment) -> Result<String> {
        let id: String = self.db.query_row(
            &format!("SELECT 'local-' || lower(hex(randomblob({DEVICE_ID_BYTES})))"),
            [],
            |r| r.get(0),
        )?;
        self.begin_session(&id, state, at)?;
        Ok(id)
    }

    /// The journal as the export keeps it (`spec/export.schema.json`).
    pub fn events_json(&self) -> Result<Vec<serde_json::Value>> {
        let events = self.events()?;
        Ok(events
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "device": e.device,
                    "seq": e.seq,
                    "at": e.at.unix,
                    "offset_min": e.at.offset_min,
                    "body": e.event,
                })
            })
            .collect())
    }

    /// All events in fold order: time, then device, then number.
    pub fn events(&self) -> Result<Vec<Stored>> {
        let mut stmt = self.db.prepare(
            "SELECT id, device, seq, at, offset_min, body FROM events ORDER BY at, device, seq",
        )?;
        let rows: Vec<(String, String, i64, i64, i32, String)> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        rows.into_iter()
            .map(|(id, device, seq, at, offset, body)| {
                Ok(Stored {
                    id,
                    device,
                    seq,
                    at: Moment::new(at, offset),
                    event: serde_json::from_str(&body)?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use kaiwa_core::srs::Grade;

    use super::*;

    fn setting(value: &str) -> Event {
        Event::Setting {
            key: "goal".into(),
            value: value.into(),
        }
    }

    #[test]
    fn a_profile_from_before_the_journal_gets_the_table() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        for sql in &crate::MIGRATIONS[..4] {
            db.execute_batch(sql).unwrap();
        }
        db.pragma_update(None, "user_version", 4).unwrap();
        let s = Store::init(db).unwrap();
        assert!(s.events().unwrap().is_empty());
        s.append_event(&setting("10"), Moment::new(1, 0)).unwrap();
        assert_eq!(s.events().unwrap().len(), 1);
    }

    #[test]
    fn the_device_id_is_made_once() {
        let s = Store::in_memory().unwrap();
        let id = s.device().unwrap();
        assert_eq!(id.len(), 2 * DEVICE_ID_BYTES);
        assert_eq!(s.device().unwrap(), id);
    }

    #[test]
    fn numbers_run_per_device_and_events_come_back_in_order() {
        let s = Store::in_memory().unwrap();
        let device = s.device().unwrap();
        let late = Moment::new(2_000, 60);
        let early = Moment::new(1_000, 120);
        let a = s.append_event(&setting("10"), late).unwrap();
        let b = s
            .append_event(
                &Event::Review {
                    text: "駅".into(),
                    grade: Grade::Good,
                },
                early,
            )
            .unwrap();
        assert_eq!((a.seq, b.seq), (1, 2));
        assert_eq!(b.id, format!("{device}-2"));

        // Fold order is by time, not by number; at a tie devices sort by id. The own id is
        // random hex: "00" sorts before any of it, "gg" after.
        for foreign in ["00", "gg"] {
            s.db.execute(
                "INSERT INTO events VALUES (?1 || '-1', ?1, 1, 1000, 0, ?2)",
                params![foreign, serde_json::to_string(&setting("5")).unwrap()],
            )
            .unwrap();
        }
        let ids: Vec<String> = s.events().unwrap().into_iter().map(|e| e.id).collect();
        let want = vec!["00-1".to_string(), b.id, "gg-1".to_string(), a.id];
        assert_eq!(ids, want);
        assert_eq!(s.events().unwrap()[1].at, early);
    }
}
