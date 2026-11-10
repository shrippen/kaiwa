//! The cost brake in the profile (Lastenheft /LF714/): the month's estimated cost of a cloud
//! tutor, booked from the process meter (`kaiwa_ai::cost`) after every turn, the average per
//! session, and the limit the learner sets. A local tutor costs nothing and shows no costs.

use kaiwa_ai::cost::{Spent, meter};
use kaiwa_core::clock::Moment;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::Backend;

/// What a month cost so far.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Month {
    pub cents: f64,
    pub calls: u32,
    pub unknown_calls: u32,
    /// Sessions that called a cloud tutor.
    pub sessions: u32,
}

fn key(now: Moment) -> String {
    let (y, m) = now.day().month();
    format!("cost.{y:04}-{m:02}")
}

impl Backend {
    pub fn month_cost(&self, now: Moment) -> anyhow::Result<Month> {
        Ok(self.store.state_value(&key(now))?.unwrap_or_default())
    }

    /// Books what the meter counted since the last booking; `session_ended` counts a session.
    pub fn book_costs(&self, now: Moment, session_ended: bool) -> anyhow::Result<Month> {
        self.book(meter().take(), now, session_ended)
    }

    fn book(&self, spent: Spent, now: Moment, session_ended: bool) -> anyhow::Result<Month> {
        let mut m = self.month_cost(now)?;
        m.cents += spent.cents;
        m.calls += spent.calls;
        m.unknown_calls += spent.unknown_calls;
        if session_ended && m.calls > 0 {
            m.sessions += 1;
        }
        self.store.set_state_value(&key(now), &m)?;
        self.arm_brake(now)?;
        Ok(m)
    }

    /// Sets the meter's budget to what the limit leaves this month.
    pub fn arm_brake(&self, now: Moment) -> anyhow::Result<()> {
        let limit = self.store.profile()?.cost_limit_cents;
        let spent = self.month_cost(now)?.cents;
        meter().set_budget(limit.map(|l| (f64::from(l) - spent).max(0.0)));
        Ok(())
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
        let text = match (m.calls, de) {
            (0, true) => "Diesen Monat noch keine Kosten.".to_string(),
            (0, false) => "No costs this month yet.".to_string(),
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
        assert!(meter().exhausted(), "the meter refuses further calls");
        // A new month starts at zero.
        let next = Moment::new(1_791_100_000 + 31 * 86_400, 120);
        assert!(!b.over_budget(next).unwrap());
        b.arm_brake(next).unwrap();
        assert!(!meter().exhausted());
    }
}
