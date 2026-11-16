//! The journal (ADR-010): what the learner does is written as an [`Event`] first and then
//! applied. Folding the journal later calls [`apply`] too, so live state and folded state
//! come from the same code.

use kaiwa_core::Event;
use kaiwa_core::clock::Moment;

use crate::Backend;
use crate::settings::SettingError;

/// Settings that belong to this device, not to the profile: applied live, never journaled, so
/// a second device keeps its own tutor and microphone. The owner decides the final list
/// (ADR-010, "Offen"); `louder` and `clear` are set by `set_sound`, not as a setting.
pub(crate) const DEVICE_SETTINGS: &[&str] = &[
    "tutorProvider",
    "tutorModel",
    "tutorBase",
    "microphone",
    "unloadAfter",
    "handsFree",
];

/// Journals `event`, then applies it. An event that cannot be written is not applied.
pub(crate) fn record(b: &Backend, event: &Event, at: Moment) -> Result<(), SettingError> {
    b.store
        .append_event(event, at)
        .map_err(|e| SettingError::NotSaved(e.into()))?;
    apply(b, event, at)
}

/// Does what `event` says, through the commands live use.
pub fn apply(b: &Backend, event: &Event, at: Moment) -> Result<(), SettingError> {
    match event {
        Event::Review { text, grade } => {
            // A card that is gone changes nothing, as live.
            b.store
                .review(text, *grade, at)
                .map(|_| ())
                .map_err(|e| SettingError::NotSaved(e.into()))
        }
        Event::Setting { key, value } => b.change_setting(key, value, at),
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

        // Device settings change the profile but stay out of the journal.
        live.set_setting("handsFree", "true", T0).unwrap();
        live.set_setting("unloadAfter", "10", T0).unwrap();
        let events = live.store.events().unwrap();
        assert_eq!(events.len(), 7);
        assert!(events.iter().all(|e| !matches!(
            &e.event,
            Event::Setting { key, .. } if DEVICE_SETTINGS.contains(&key.as_str())
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
