//! The journal (ADR-010): what the learner does is written as an [`Event`] first and then
//! applied. Folding the journal later calls [`apply`] too, so live state and folded state
//! come from the same code.

use kaiwa_core::Event;
use kaiwa_core::clock::Moment;

use crate::Backend;
use crate::keys::Key;

/// Why an event was not journaled or not applied.
#[derive(Debug)]
pub enum JournalError {
    /// The profile could not be read or written: it is as it was.
    NotSaved(anyhow::Error),
    /// The event is saved, but what it sets off failed (today's goal, deleting recordings).
    NotApplied(anyhow::Error),
    /// The profile could not be opened; the event would delete recordings of the real one.
    Spare,
}

impl std::fmt::Display for JournalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSaved(e) => write!(f, "not saved: {e:#}"),
            Self::NotApplied(e) => write!(f, "saved, not applied: {e:#}"),
            Self::Spare => f.write_str("refused on a spare profile"),
        }
    }
}

impl std::error::Error for JournalError {}

/// Journals `event`, then applies it. An event that cannot be written is not applied.
pub(crate) fn record(b: &Backend, event: &Event, at: Moment) -> Result<(), JournalError> {
    b.store
        .append_event(event, at)
        .map_err(|e| JournalError::NotSaved(e.into()))?;
    apply(b, event, at)
}

/// Does what `event` says, through the commands live use.
pub(crate) fn apply(b: &Backend, event: &Event, at: Moment) -> Result<(), JournalError> {
    match event {
        Event::Review { text, grade } => {
            // A card that is gone changes nothing, as live.
            b.store
                .review(text, *grade, at)
                .map(|_| ())
                .map_err(|e| JournalError::NotSaved(e.into()))
        }
        Event::Setting { key, value } => {
            // A key this version does not know (from a newer device) changes nothing.
            let Some(key) = Key::parse(key) else {
                return Ok(());
            };
            b.change_setting(key, value, at)
        }
    }
}

#[cfg(test)]
mod tests {
    use kaiwa_core::srs::{Grade, Item, ItemKind, Source};

    use super::*;
    use crate::TempBackend;

    const T0: Moment = Moment {
        unix: 1_791_100_000,
        offset_min: 120,
    };
    const WORDS: [&str; 2] = ["駅", "袋"];

    /// A profile with two cards, as a profile about to join would have.
    fn backend(name: &str) -> TempBackend {
        let b = TempBackend::new(&format!("journal-{name}"), false).unwrap();
        for w in WORDS {
            let item = Item::new(w, ItemKind::Word, Source::NewWord, T0);
            b.store.put_item(&item).unwrap();
        }
        b
    }

    fn export(b: &Backend) -> serde_json::Value {
        let mut v = b.store.export(T0).unwrap();
        v.as_object_mut().unwrap().remove("exported_at");
        v
    }

    #[test]
    fn folding_the_journal_gives_the_live_profile() {
        let live = backend("live");
        let day = 86_400;
        for (i, grade) in [Grade::Good, Grade::Again, Grade::Easy]
            .into_iter()
            .enumerate()
        {
            let at = T0.plus_secs(i as i64 * day);
            live.answer_review(
                WORDS[i % 2],
                if grade == Grade::Again {
                    "x"
                } else {
                    WORDS[i % 2]
                },
                false,
                at,
            );
        }
        live.set_setting("goal", "25", T0).unwrap();
        live.set_setting("name", "Lena", T0.plus_secs(60)).unwrap();
        live.set_setting("script", "1", T0.plus_secs(day)).unwrap();
        live.set_setting("goal", "15", T0.plus_secs(2 * day))
            .unwrap();

        // The settings that do not go through the profile edit, and the one that purges.
        live.set_setting("costLimit", "500", T0.plus_secs(2 * day))
            .unwrap();
        live.set_setting("feedback", "much", T0.plus_secs(2 * day))
            .unwrap();
        live.set_setting("keepRecordings", "30", T0.plus_secs(2 * day))
            .unwrap();

        // Device settings change the profile but stay out of the journal.
        live.set_setting("handsFree", "true", T0).unwrap();
        live.set_setting("unloadAfter", "10", T0).unwrap();
        let events = live.store.events().unwrap();
        assert_eq!(events.len(), 10);
        assert!(events.iter().all(|e| !matches!(
            &e.event,
            Event::Setting { key, .. } if Key::parse(key).is_some_and(Key::per_device)
        )));

        let folded = backend("folded");
        for e in &events {
            apply(&folded, &e.event, e.at).unwrap();
        }

        // Only what the journal holds is folded: the device settings differ.
        let (mut a, mut b) = (export(&live), export(&folded));
        for v in [&mut a, &mut b] {
            let p = v["profile"].as_object_mut().unwrap();
            p.remove("hands_free");
            p.remove("unload_after_min");
        }
        assert_eq!(a, b);
        assert_eq!(
            folded.store.profile().unwrap().daily_goal,
            kaiwa_core::progress::DailyGoal::Minutes(15)
        );
    }
}
