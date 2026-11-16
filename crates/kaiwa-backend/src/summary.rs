//! The summary page of a finished talk: what the end of the session did, in words
//! (/LF310/, /LF603/, /LF605/).

use kaiwa_core::Session;
use serde_json::{Value, json};

use crate::{Backend, Recorded};

impl Backend {
    /// The summary of a recorded talk, and why the profile could not be read if it could not:
    /// the page then shows the summary with the defaults and says so.
    pub fn recorded_view(&self, r: &Recorded, s: &Session, lang: &str) -> (Value, Option<String>) {
        let profile = self.profile();
        let unread = profile.as_ref().err().map(|e| e.to_string());
        let gamification = profile.map(|p| p.gamification).unwrap_or(true);
        let change = r.change.as_ref().map(|c| c.words(lang));
        let view = json!({
            "xp": if gamification { Some(r.xp) } else { None },
            "xpWhy": if gamification { s.summary().xp.explain(lang) } else { Vec::new() },
            "change": change,
            "newItems": r.new_items,
            "scenarioDone": r.scenario_done,
            "minutes": r.day.speaking_secs / 60,
        });
        (view, unread)
    }
}

#[cfg(test)]
mod tests {
    use kaiwa_core::progress::DayRecord;

    use super::*;
    use crate::TempBackend;

    const XP: u32 = 40;

    fn recorded() -> Recorded {
        Recorded {
            success: 1.0,
            change: None,
            scenario_done: false,
            new_items: 2,
            xp: XP,
            day: DayRecord::default(),
        }
    }

    fn session(b: &Backend) -> Session {
        let scenario = b.scenarios.values().next().unwrap().clone();
        b.new_talk(scenario).unwrap().0
    }

    #[test]
    fn xp_shows_only_with_gamification_on() {
        let b = TempBackend::new("summary-xp", false).unwrap();
        let s = session(&b);

        let (view, unread) = b.recorded_view(&recorded(), &s, "en");
        assert_eq!(view["xp"], XP);
        assert_eq!(view["newItems"], 2);
        assert!(unread.is_none());

        b.store.update_profile(|p| p.gamification = false).unwrap();
        let (view, unread) = b.recorded_view(&recorded(), &s, "en");
        assert!(view["xp"].is_null());
        assert!(view["xpWhy"].as_array().unwrap().is_empty());
        assert!(unread.is_none());
    }

    #[test]
    fn an_unreadable_profile_is_returned_and_the_summary_has_defaults() {
        let b = TempBackend::new("summary-unread", false).unwrap();
        let s = session(&b);
        let db = rusqlite::Connection::open(&b.path).unwrap();
        // The profile row holds a string, not a profile.
        db.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('profile', '\"unreadable\"')",
            [],
        )
        .unwrap();

        let (view, unread) = b.recorded_view(&recorded(), &s, "en");

        assert!(unread.is_some());
        assert_eq!(view["xp"], XP);
        assert_eq!(view["newItems"], 2);
    }
}
