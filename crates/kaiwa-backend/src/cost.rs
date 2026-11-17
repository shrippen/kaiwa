//! The cost brake in the profile (Lastenheft /LF714/): the month's estimated cost of a cloud
//! tutor, booked from the backend's meter (`kaiwa_ai::cost`) after every turn, the average per
//! session, and the limit the learner sets. A local tutor costs nothing and shows no costs.

use kaiwa_ai::cost::Spent;
use kaiwa_core::Event;
use kaiwa_core::clock::Moment;
use kaiwa_store::COST_STATE_PREFIX;
use serde_json::{Value, json};

use crate::Backend;
use crate::journal::{self, JournalError};

pub use kaiwa_store::CostMonth as Month;

fn key(now: Moment) -> String {
    let (y, m) = now.day().month();
    format!("{COST_STATE_PREFIX}{y:04}-{m:02}")
}

impl Backend {
    pub fn month_cost(&self, now: Moment) -> anyhow::Result<Month> {
        Ok(self.store.state_value(&key(now))?.unwrap_or_default())
    }

    /// Books what the meter counted since the last booking; `session_ended` counts a session.
    pub fn book_costs(&self, now: Moment, session_ended: bool) -> anyhow::Result<Month> {
        self.book(self.meter.take(), now, session_ended)
    }

    fn book(&self, spent: Spent, now: Moment, session_ended: bool) -> anyhow::Result<Month> {
        // Nothing spent and no talk to count: nothing to journal.
        if spent.calls == 0 && spent.unknown_calls == 0 && !session_ended {
            self.arm_brake(now)?;
            return self.month_cost(now);
        }
        let event = Event::CostsBooked {
            cents: spent.cents,
            calls: spent.calls,
            unknown_calls: spent.unknown_calls,
            talk_ended: session_ended,
        };
        journal::record(self, &event, now)?;
        self.month_cost(now)
    }

    /// What a journaled booking does: the month of `now` adds the costs.
    pub(crate) fn add_costs(
        &self,
        spent: Spent,
        now: Moment,
        talk_ended: bool,
    ) -> anyhow::Result<()> {
        self.store.update_state_value(&key(now), |m: &mut Month| {
            m.cents += spent.cents;
            m.calls += spent.calls;
            m.unknown_calls += spent.unknown_calls;
            m.paid |= spent.cents > 0.0;
            if talk_ended && std::mem::take(&mut m.paid) {
                m.sessions += 1;
            }
        })?;
        self.arm_brake(now)
    }

    /// Sets the meter's budget to what the limit leaves this month.
    pub fn arm_brake(&self, now: Moment) -> anyhow::Result<()> {
        let limit = self.store.profile()?.cost_limit_cents;
        let spent = self.month_cost(now)?.cents;
        self.meter
            .set_budget(limit.map(|l| (f64::from(l) - spent).max(0.0)));
        Ok(())
    }

    /// Stores the limit and re-arms the brake at once (/LF714/).
    pub fn set_cost_limit(&self, limit: Option<u32>, now: Moment) -> Result<(), JournalError> {
        self.store
            .update_profile(|p| p.cost_limit_cents = limit)
            .map_err(|e| JournalError::NotSaved(e.into()))?;
        self.arm_brake(now).map_err(JournalError::NotApplied)
    }

    /// Whether the limit is reached (a new talk with a cloud tutor would be refused).
    pub fn over_budget(&self, now: Moment) -> anyhow::Result<bool> {
        let limit = self.store.profile()?.cost_limit_cents;
        Ok(limit
            .is_some_and(|l| self.month_cost(now).map(|m| m.cents).unwrap_or(0.0) >= f64::from(l)))
    }

    /// For the settings: the month, the average per session, the limit, in words.
    pub fn cost_view(&self, now: Moment) -> anyhow::Result<Value> {
        let p = self.store.profile()?;
        let de = p.ui_lang != "en";
        let m = self.month_cost(now)?;
        let euro = |c: f64| {
            let s = format!("{:.2} €", c / 100.0);
            if de { s.replace('.', ",") } else { s }
        };
        let per_session = (m.sessions > 0).then(|| m.cents / f64::from(m.sessions));
        let text = match (m.cents > 0.0, de) {
            (false, true) => "Diesen Monat noch keine Kosten.".to_string(),
            (false, false) => "No costs this month yet.".to_string(),
            (_, true) => format!(
                "Diesen Monat etwa {}{}.",
                euro(m.cents),
                per_session
                    .map(|c| format!(", etwa {} pro Gespräch", euro(c)))
                    .unwrap_or_default()
            ),
            (_, false) => format!(
                "About {} this month{}.",
                euro(m.cents),
                per_session
                    .map(|c| format!(", about {} per talk", euro(c)))
                    .unwrap_or_default()
            ),
        };
        let unknown = match (m.unknown_calls, de) {
            (0, _) => String::new(),
            (n, true) => format!(
                "{n} Aufrufe ohne bekannten Preis sind wie das teuerste bekannte Modell gezählt, die Bremse greift eher zu früh."
            ),
            (n, false) => format!(
                "{n} calls without a known price are counted like the dearest known model, so the limit rather holds too early."
            ),
        };
        Ok(json!({
            "cents": m.cents,
            "perSession": per_session,
            "limit": p.cost_limit_cents,
            "over": self.over_budget(now)?,
            "text": text,
            "unknown": unknown,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend(name: &str) -> crate::TempBackend {
        crate::TempBackend::new(name, false).unwrap()
    }

    #[test]
    fn two_backends_booking_at_once_lose_nothing() {
        // Two windows on one profile file: every booking lands in the month.
        const BOOKINGS: u32 = 25;
        const WINDOWS: usize = 2;
        let b = backend("cost-race");
        let path = b.dir.join("profile.kaiwa");
        let now = Moment::new(1_791_100_000, 120);
        // Opened first, released together, so the bookings overlap.
        let start = std::sync::Arc::new(std::sync::Barrier::new(WINDOWS));
        let threads: Vec<_> = (0..WINDOWS)
            .map(|_| {
                let other = crate::Backend::open_at(path.clone(), false).unwrap();
                let start = start.clone();
                std::thread::spawn(move || {
                    start.wait();
                    for _ in 0..BOOKINGS {
                        let spent = Spent {
                            cents: 1.0,
                            calls: 1,
                            unknown_calls: 0,
                        };
                        other.book(spent, now, false).unwrap();
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }

        let m = b.month_cost(now).unwrap();
        assert_eq!(m.calls, WINDOWS as u32 * BOOKINGS);
        assert_eq!(m.cents, f64::from(WINDOWS as u32 * BOOKINGS));
    }

    #[test]
    fn a_new_limit_brakes_or_frees_at_once() {
        let b = backend("cost-limit");
        let now = Moment::new(1_791_100_000, 120);
        let spent = Spent {
            cents: 60.0,
            calls: 8,
            unknown_calls: 0,
        };
        b.book(spent, now, false).unwrap();
        assert!(!b.meter().exhausted(), "no limit, no brake");

        b.set_cost_limit(Some(50), now).unwrap();
        assert!(b.meter().exhausted(), "a limit below the spent cents holds");

        b.set_cost_limit(Some(100), now).unwrap();
        assert!(!b.meter().exhausted(), "a raised limit frees the next turn");

        b.set_cost_limit(None, now).unwrap();
        assert!(!b.meter().exhausted());
    }

    #[test]
    fn a_brake_that_cannot_arm_is_not_applied_but_the_limit_is_saved() {
        let b = backend("cost-unarmed");
        let now = Moment::new(1_791_100_000, 120);
        // The month's row holds a string, not a month: the brake cannot read what is spent.
        let db = rusqlite::Connection::open(&b.path).unwrap();
        db.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, '\"unreadable\"')",
            [format!("state.{}", key(now))],
        )
        .unwrap();

        let r = b.set_cost_limit(Some(50), now);

        assert!(matches!(r, Err(JournalError::NotApplied(_))), "{r:?}");
        assert_eq!(b.store.profile().unwrap().cost_limit_cents, Some(50));
    }

    #[test]
    fn the_month_adds_up_and_the_limit_brakes() {
        let b = backend("cost");
        let now = Moment::new(1_791_100_000, 120);
        let mut p = b.store.profile().unwrap();
        p.cost_limit_cents = Some(100);
        b.store.set_profile(&p).unwrap();
        let spent = Spent {
            cents: 60.0,
            calls: 8,
            unknown_calls: 0,
        };
        b.book(spent.clone(), now, true).unwrap();
        assert!(!b.over_budget(now).unwrap());
        let v = b.cost_view(now).unwrap();
        assert_eq!(v["perSession"], 60.0);
        assert!(v["text"].as_str().unwrap().contains("0,60 €"), "{v}");
        b.book(spent, now, true).unwrap();
        assert!(b.over_budget(now).unwrap());
        assert!(b.meter().exhausted(), "the meter refuses further calls");
        // Talks with a tutor on this computer cost nothing and do not thin out the average.
        let local = Spent {
            cents: 0.0,
            calls: 6,
            unknown_calls: 0,
        };
        b.book(local, now, true).unwrap();
        assert_eq!(b.cost_view(now).unwrap()["perSession"], 60.0);
        // A new month starts at zero.
        let next = Moment::new(1_791_100_000 + 31 * 86_400, 120);
        assert!(!b.over_budget(next).unwrap());
        b.arm_brake(next).unwrap();
        assert!(!b.meter().exhausted());
    }

    #[test]
    fn an_import_keeps_the_months_costs_and_the_brake() {
        let a = backend("cost-export");
        let now = Moment::new(1_791_100_000, 120);
        a.set_cost_limit(Some(100), now).unwrap();
        let spent = Spent {
            cents: 60.0,
            calls: 8,
            unknown_calls: 0,
        };
        a.book(spent, now, true).unwrap();
        let json = a.store.export(now).unwrap();

        // V-04: a new profile starts with the costs of the month, not with the full limit.
        let b = backend("cost-import");
        b.store.import(&json).unwrap();
        assert_eq!(b.month_cost(now).unwrap(), a.month_cost(now).unwrap());
        let book = Spent {
            cents: 40.0,
            calls: 1,
            unknown_calls: 0,
        };
        b.book(book, now, false).unwrap();
        assert!(
            b.over_budget(now).unwrap(),
            "60 imported + 40 booked reach 100"
        );
        assert!(b.meter().exhausted());
    }
}
