//! Reminders (Lastenheft /LF520/, /LF620/, /LL300/): off by default; at the chosen time, at
//! most one a day, only while today's goal is open and no pause runs. The text invites and
//! never blames: no missed days, no loss. "Später" moves the same reminder once by an hour.
//! The plasmoid (M4, before 1.0) asks for it every minute and shows it as a notification;
//! until then the setting is hidden.

use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::progress::blameless;
use serde_json::{Value, json};

use crate::Backend;

/// "Später" moves the reminder this far.
pub const SNOOZE_SECS: i64 = 3600;

fn minute_of_day(now: Moment) -> u32 {
    ((now.unix + i64::from(now.offset_min) * 60).rem_euclid(86_400) / 60) as u32
}

impl Backend {
    /// The reminder to show now, if one is due; it counts as sent.
    pub fn reminder(&self, now: Moment) -> anyhow::Result<Option<Value>> {
        let p = self.store.profile()?;
        let Some(at) = p.reminder else {
            return Ok(None);
        };
        let today = now.day();
        let snoozed: Option<i64> = self
            .store
            .state_value::<Option<i64>>("remind_again")?
            .flatten();
        let due = match snoozed {
            Some(t) => now.unix >= t,
            None => {
                let sent: Option<Day> = self.store.state_value("reminded")?;
                sent != Some(today) && minute_of_day(now) >= at
            }
        };
        if !due {
            return Ok(None);
        }
        let t = self.store.today(now)?;
        let paused = self
            .store
            .pauses()?
            .iter()
            .any(|x| x.from <= today && today <= x.to);
        self.store.set_state_value("reminded", &today)?;
        self.store
            .set_state_value::<Option<i64>>("remind_again", &None)?;
        if paused || t.record.met(p.daily_goal) {
            return Ok(None);
        }
        let de = p.ui_lang != "en";
        let learning = self.store.learning()?;
        let next = self
            .curriculum
            .next_scenario(&learning.course)
            .and_then(|id| self.scenarios.get(&id))
            .map(|s| s.title.get(&p.ui_lang).to_string());
        let text = match (next, de) {
            (Some(n), true) => format!("Fünf Minuten Japanisch? Als Nächstes wartet: {n}."),
            (Some(n), false) => format!("Five minutes of Japanese? Next up: {n}."),
            (None, true) => "Fünf Minuten Japanisch?".to_string(),
            (None, false) => "Five minutes of Japanese?".to_string(),
        };
        debug_assert!(blameless(&text));
        Ok(Some(json!({
            "title": "Kaiwa",
            "text": text,
            "now": if de { "Jetzt 5 min" } else { "5 min now" },
            "later": if de { "Später" } else { "Later" },
        })))
    }

    /// "Später": the same reminder once more in an hour (still one a day).
    pub fn snooze(&self, now: Moment) -> anyhow::Result<()> {
        let sent: Option<Day> = self.store.state_value("snoozed")?;
        if sent == Some(now.day()) {
            return Ok(());
        }
        self.store.set_state_value("snoozed", &now.day())?;
        self.store
            .set_state_value("remind_again", &Some(now.unix + SNOOZE_SECS))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaiwa_core::progress::BLAME_WORDS;

    fn backend(name: &str, reminder: Option<u32>) -> Backend {
        let dir = std::env::temp_dir().join(format!("kaiwa-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let b = Backend::open_at(dir.join("t.kaiwa"), false).unwrap();
        let mut p = b.store.profile().unwrap();
        p.reminder = reminder;
        b.store.set_profile(&p).unwrap();
        b
    }

    /// 2026-10-04 at `h`:00 in UTC+2.
    fn at(h: i64) -> Moment {
        let day = Day::from_ymd(2026, 10, 4);
        day.start(120).plus_secs(h * 3600)
    }

    #[test]
    fn off_by_default() {
        let b = backend("rem-off", None);
        assert!(b.reminder(at(20)).unwrap().is_none());
    }

    #[test]
    fn once_a_day_at_the_chosen_time_without_blame() {
        let b = backend("rem-once", Some(18 * 60));
        assert!(b.reminder(at(17)).unwrap().is_none(), "not before the time");
        let r = b.reminder(at(18)).unwrap().expect("due at 18:00");
        let text = r["text"].as_str().unwrap();
        assert!(blameless(text), "{text}");
        assert!(!BLAME_WORDS.iter().any(|w| text.to_lowercase().contains(w)));
        assert!(b.reminder(at(19)).unwrap().is_none(), "only one a day");
        // The next day again.
        let next = at(18).plus_secs(86_400);
        assert!(b.reminder(next).unwrap().is_some());
    }

    #[test]
    fn later_moves_it_once_by_an_hour() {
        let b = backend("rem-later", Some(18 * 60));
        assert!(b.reminder(at(18)).unwrap().is_some());
        b.snooze(at(18)).unwrap();
        assert!(b.reminder(at(18).plus_secs(1800)).unwrap().is_none());
        assert!(b.reminder(at(19)).unwrap().is_some());
        b.snooze(at(19)).unwrap();
        assert!(
            b.reminder(at(20)).unwrap().is_none(),
            "snoozed only once a day"
        );
    }

    #[test]
    fn none_when_the_goal_is_met_or_paused() {
        let b = backend("rem-met", Some(18 * 60));
        let day = at(18).day();
        b.store
            .put_day(
                day,
                &kaiwa_core::progress::DayRecord {
                    speaking_secs: 3600,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(b.reminder(at(18)).unwrap().is_none());
        let b = backend("rem-pause", Some(18 * 60));
        b.store
            .add_pause(kaiwa_core::progress::Pause { from: day, to: day })
            .unwrap();
        assert!(b.reminder(at(18)).unwrap().is_none());
    }
}
