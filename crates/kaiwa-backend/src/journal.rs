//! The journal (ADR-010): what the learner does is written as an [`Event`] first and then
//! applied. Folding the journal later calls [`apply`] too, so live state and folded state
//! come from the same code.

use kaiwa_core::clock::Moment;
use kaiwa_core::session::SessionState;
use kaiwa_core::{Event, Session};
use kaiwa_store::Recorded;

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
            // The cause alone: the page says itself what was not saved.
            Self::NotSaved(e) | Self::NotApplied(e) => write!(f, "{e}"),
            Self::Spare => f.write_str("refused on a spare profile"),
        }
    }
}

impl std::error::Error for JournalError {}

/// What applying an event gave and what came with it.
#[derive(Default)]
pub(crate) struct Applied {
    /// The record of a talk that ended or was corrected.
    pub recorded: Option<Recorded>,
    /// What the event set off and could not do (tracking the grammar, deleting recordings);
    /// the event itself is applied.
    pub failed: Vec<anyhow::Error>,
}

/// An event that was journaled and applied.
pub(crate) struct Logged {
    /// The event's id; a talk's id when it began one.
    pub id: String,
    pub applied: Applied,
}

/// Journals `event`, then applies it. An event that cannot be written is not applied.
pub(crate) fn record(b: &Backend, event: &Event, at: Moment) -> Result<Logged, JournalError> {
    let stored = b
        .store
        .append_event(event, at)
        .map_err(|e| JournalError::NotSaved(e.into()))?;
    let applied = apply(b, &stored.id, event, at)?;
    Ok(Logged {
        id: stored.id,
        applied,
    })
}

/// Does what `event` (journal id `id`) says, through the commands live uses.
pub(crate) fn apply(
    b: &Backend,
    id: &str,
    event: &Event,
    at: Moment,
) -> Result<Applied, JournalError> {
    match event {
        Event::Review { text, grade } => {
            // A card that is gone changes nothing, as live.
            b.store
                .review(text, *grade, at)
                .map(|_| Applied::default())
                .map_err(not_saved)
        }
        Event::Setting { key, value } => {
            // A key this version does not know (from a newer device) changes nothing.
            let Some(key) = Key::parse(key) else {
                return Ok(Applied::default());
            };
            b.change_setting(key, value, at)?;
            Ok(Applied::default())
        }
        Event::TalkBegun { state } => {
            let begun = b.store.begin_session(id, state, at);
            begun.map(|()| Applied::default()).map_err(not_saved)
        }
        Event::TalkSaved { talk, state } => {
            let saved = b.store.save_session(talk, state);
            saved.map(|()| Applied::default()).map_err(not_saved)
        }
        Event::TalkFinished {
            talk,
            state,
            started,
            ended,
            day,
        } => {
            let session = restored(b, state)?;
            let curriculum = Some(&b.curriculum);
            let recorded = b
                .store
                .record_on(talk, &session, *started, *ended, *day, curriculum)
                .map_err(not_saved)?;
            let failed = [
                b.track_grammar(&session, *ended),
                b.purge_recordings(*ended).map(|_| ()),
            ];
            Ok(Applied {
                recorded: Some(recorded),
                failed: failed.into_iter().filter_map(Result::err).collect(),
            })
        }
        Event::TalkCorrected { talk, state } => {
            let session = restored(b, state)?;
            let recorded = b
                .correct_talk(talk, &session)
                .map_err(JournalError::NotSaved)?;
            Ok(Applied {
                recorded: Some(recorded),
                failed: Vec::new(),
            })
        }
    }
}

fn not_saved(e: kaiwa_store::StoreError) -> JournalError {
    JournalError::NotSaved(e.into())
}

/// The talk as its kept state has it again, with the scenario this version knows.
fn restored(b: &Backend, state: &SessionState) -> Result<Session, JournalError> {
    let scenario =
        b.scenarios.get(&state.scenario).cloned().ok_or_else(|| {
            JournalError::NotSaved(anyhow::anyhow!("no scenario {}", state.scenario))
        })?;
    let (session, _) = b
        .continued_talk(scenario, state.clone())
        .map_err(JournalError::NotSaved)?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use kaiwa_core::srs::{Grade, Item, ItemKind, Source};

    use kaiwa_core::session::Input;

    use super::*;
    use crate::TempBackend;
    use crate::flow::{Length, Talk};

    #[test]
    fn a_journal_error_reads_as_its_cause() {
        // The review page shows it after its own "not saved": no second prefix.
        let e = JournalError::NotSaved(anyhow::anyhow!("database is locked"));
        assert_eq!(e.to_string(), "database is locked");
    }

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
        let o = v.as_object_mut().unwrap();
        o.remove("exported_at");
        // The folded profile has no journal of its own.
        o.remove("events");
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
        live.set_setting("goal", "20", T0).unwrap();
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
            apply(&folded, &e.id, &e.event, e.at).unwrap();
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

    const SCENARIO: &str = "n5-02-jikoshoukai";
    const DAY: i64 = 86_400;

    fn fold(into: &Backend, events: &[kaiwa_store::Stored]) {
        for e in events {
            apply(into, &e.id, &e.event, e.at).unwrap();
        }
    }

    /// The export without what the journal does not hold yet: the costs (step 3) and the
    /// device settings.
    fn folded_export(b: &Backend) -> serde_json::Value {
        let mut v = export(b);
        let o = v.as_object_mut().unwrap();
        o.remove("costs");
        let p = v["profile"].as_object_mut().unwrap();
        p.remove("hands_free");
        p.remove("unload_after_min");
        v
    }

    /// Begins a talk on the recorded tutor and plays `turns` of its goals' example lines.
    fn play(b: &Backend, turns: usize) -> Talk {
        let opened = Talk::begin(b, SCENARIO, T0, Length::Short).unwrap();
        assert!(opened.failed.is_empty(), "{:?}", opened.failed);
        let mut talk = opened.talk;
        let goals = talk.session().goals().to_vec();
        for (i, g) in goals.iter().take(turns).enumerate() {
            talk.spend(60);
            let now = T0.plus_secs(60 * (i as i64 + 1));
            let turned = talk
                .say(b, &g.help.example, Input::Spoken, now)
                .unwrap()
                .unwrap();
            assert!(turned.failed.is_empty(), "{:?}", turned.failed);
        }
        talk
    }

    #[test]
    fn folding_the_journal_gives_the_live_talks_and_reviews() {
        let live = backend("talk-live");
        let mut talk = play(&live, usize::MAX);
        assert!(talk.finished());
        live.answer_review(WORDS[0], WORDS[0], false, T0.plus_secs(DAY));
        live.answer_review(WORDS[1], "x", false, T0.plus_secs(2 * DAY));
        // The learner marks a turn of the finished talk: the record is corrected.
        let marked = talk.mark_misrecognized(&live, 0).expect("marked");
        assert!(marked.failed.is_empty(), "{:?}", marked.failed);
        assert!(marked.recorded.is_some());

        let events = live.store.events().unwrap();
        let kinds: Vec<&str> = events
            .iter()
            .map(|e| match &e.event {
                Event::TalkBegun { .. } => "begun",
                Event::TalkSaved { .. } => "saved",
                Event::TalkFinished { .. } => "finished",
                Event::TalkCorrected { .. } => "corrected",
                Event::Review { .. } => "review",
                Event::Setting { .. } => "setting",
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "begun",
                "saved",
                "finished",
                "review",
                "review",
                "corrected"
            ]
        );
        assert_eq!(
            talk.id(),
            Some(events[0].id.clone()),
            "the start event's id"
        );

        let folded = backend("talk-folded");
        fold(&folded, &events);

        assert_eq!(folded_export(&live), folded_export(&folded));
        let rows = folded.store.sessions(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].finished);
    }

    #[test]
    fn a_talk_left_open_folds_to_the_same_open_talk() {
        let live = backend("open-live");
        let talk = play(&live, 1);
        talk.save(&live).unwrap();
        assert!(!talk.finished());

        let folded = backend("open-folded");
        fold(&folded, &live.store.events().unwrap());

        assert_eq!(folded_export(&live), folded_export(&folded));
        let open = folded.store.open_session().unwrap().expect("still open");
        assert_eq!(Some(open.id.clone()), talk.id());
        assert_eq!(folded.store.session_state(&open.id).unwrap().turns.len(), 1);
    }
}
