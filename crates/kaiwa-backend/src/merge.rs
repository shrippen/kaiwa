//! Sync between devices (ADR-010, step 4). A profile begins to sync with its state as the
//! journal's start (`Base`). Another device joins with an empty profile and takes that
//! journal. From then on each takes the other's new events and folds the journal again,
//! so both end with the same state:
//!
//! ```text
//!   device A                         device B (empty)
//!   share_base  [Base]
//!   practises   [Base a1]      ──►   merge: joins    [Base a1]
//!                                    practises       [Base a1 b1]
//!   merge       [Base a1 b1]   ◄──   journal
//!   practises   [Base a1 b1 a2] ──►  merge           [Base a1 b1 a2]
//! ```

use kaiwa_core::Event;
use kaiwa_core::clock::Moment;
use kaiwa_store::{StoreError, Stored, stored_of};
use serde_json::Value;

use crate::Backend;
use crate::journal::{self, JournalError, Pass};
use crate::keys::keep_device;

/// Why another device's journal was not taken. In every case the profile is as it was.
#[derive(Debug)]
pub enum MergeError {
    /// The journal holds events this version cannot read: a newer Kaiwa on the other
    /// device, or a damaged file.
    Unreadable,
    /// The journal has no start, or another one than this profile's: it belongs to another
    /// profile.
    Unrelated,
    /// This profile does not sync yet and has data; it joins only when empty, like an import.
    NotEmpty,
    /// The profile could not be opened; a fold would overwrite the real one's recordings.
    Spare,
    /// The profile could not be read or written.
    NotSaved(anyhow::Error),
}

impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable => f.write_str("the other journal cannot be read"),
            Self::Unrelated => f.write_str("the other journal belongs to another profile"),
            Self::NotEmpty => f.write_str("only an empty profile joins"),
            Self::Spare => f.write_str("refused on a spare profile"),
            Self::NotSaved(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for MergeError {}

impl From<StoreError> for MergeError {
    fn from(e: StoreError) -> Self {
        Self::NotSaved(e.into())
    }
}

impl From<JournalError> for MergeError {
    fn from(e: JournalError) -> Self {
        match e {
            JournalError::Spare => Self::Spare,
            JournalError::NotSaved(e) | JournalError::NotApplied(e) => Self::NotSaved(e),
        }
    }
}

impl Backend {
    /// Makes the profile's state the start of its journal, before its first sync; what the
    /// journal held is in that state. A profile that syncs already keeps its start.
    pub fn share_base(&self, now: Moment) -> Result<(), MergeError> {
        if self.spare {
            return Err(MergeError::Spare);
        }
        if base_of(&self.store.events()?).is_some() {
            return Ok(());
        }
        let mut state = self.store.export(now)?;
        if let Some(o) = state.as_object_mut() {
            o.remove("events");
            o.remove("exported_at");
        }
        self.store.start_journal(&Event::Base { state }, now)?;
        Ok(())
    }

    /// The journal as another device's [`Backend::merge`] takes it.
    pub fn journal(&self) -> anyhow::Result<Vec<Value>> {
        Ok(self.store.events_json()?)
    }

    /// Takes another device's journal: an empty profile joins it, a profile with the same
    /// start adds its new events. Then the journal is folded again in one transaction. How
    /// many events were new. Not while a talk is open in a front end: the fold rebuilds the
    /// talks from the journal.
    pub fn merge(&self, journal: &[Value], now: Moment) -> Result<usize, MergeError> {
        if self.spare {
            return Err(MergeError::Spare);
        }
        let theirs: Vec<Stored> = journal
            .iter()
            .map(stored_of)
            .collect::<Result<_, _>>()
            .map_err(|_| MergeError::Unreadable)?;
        let Some(start) = base_of(&theirs) else {
            return Err(MergeError::Unrelated);
        };
        let own = self.store.events()?;
        let joins = match base_of(&own) {
            None if !self.store.is_empty()? => return Err(MergeError::NotEmpty),
            None => Join::Join,
            Some(base) if base != start => return Err(MergeError::Unrelated),
            Some(_) => Join::Add,
        };
        let added = self.take_events(&theirs, joins, now)?;

        // The recordings as the folded profile keeps them (/LF712/); the merge stands.
        if added > 0
            && let Err(e) = self.purge_recordings(now)
        {
            self.trouble(crate::trouble::Trouble::RecordingsKept(e));
        }
        Ok(added)
    }

    /// Joins or adds `theirs` and folds again, in one transaction; how many were new.
    fn take_events(&self, theirs: &[Stored], how: Join, now: Moment) -> Result<usize, MergeError> {
        self.store.batch(|st| {
            let added = match how {
                Join::Join => {
                    st.replace_events(theirs)?;
                    theirs.len()
                }
                Join::Add => st.add_events(theirs)?,
            };
            if added == 0 {
                return Ok(0);
            }
            refold(self)?;
            self.arm_brake(now).map_err(MergeError::NotSaved)?;
            Ok(added)
        })
    }
}

/// How another device's journal comes in.
#[derive(Clone, Copy)]
enum Join {
    /// This profile is empty and takes it in place of its own.
    Join,
    /// Both share the start: the new events are added.
    Add,
}

/// The id of the journal's start, if it has one.
fn base_of(events: &[Stored]) -> Option<&str> {
    events
        .iter()
        .find(|e| matches!(e.event, Event::Base { .. }))
        .map(|e| e.id.as_str())
}

/// Folds the journal again from its start, inside the caller's transaction; this device's
/// own settings stay. An event that fails now (a correction a later talk overtook) is left
/// out, as each event is applied in its own savepoint.
fn refold(b: &Backend) -> Result<(), JournalError> {
    let device = b.store.profile()?;
    b.store.clear_folded()?;
    for e in b.store.events()? {
        let _ = b
            .store
            .batch(|_| journal::apply(b, &e.id, &e.event, e.at, Pass::Fold));
    }
    b.store.update_profile(|p| keep_device(p, &device))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use kaiwa_core::session::Input;
    use kaiwa_core::srs::{Item, ItemKind, Source};

    use super::*;
    use crate::TempBackend;
    use crate::flow::{Length, Talk};

    const T0: Moment = Moment {
        unix: 1_791_100_000,
        offset_min: 120,
    };
    const HOUR: i64 = 3_600;
    const SCENARIO: &str = "n5-02-jikoshoukai";
    const WORDS: [&str; 2] = ["駅", "袋"];

    /// The first device: two cards, then it begins to sync.
    fn first(name: &str) -> TempBackend {
        let b = TempBackend::new(&format!("merge-{name}"), false).unwrap();
        for w in WORDS {
            let item = Item::new(w, ItemKind::Word, Source::NewWord, T0);
            b.store.put_item(&item).unwrap();
        }
        b.share_base(T0).unwrap();
        b
    }

    fn empty(name: &str) -> TempBackend {
        TempBackend::new(&format!("merge-{name}"), false).unwrap()
    }

    /// A whole talk on the recorded tutor, begun at `at`.
    fn talk(b: &Backend, at: Moment) {
        let mut talk = Talk::begin(b, SCENARIO, at, Length::Short).unwrap().talk;
        let goals = talk.session().goals().to_vec();
        for (i, g) in goals.iter().enumerate() {
            talk.spend(60);
            let now = at.plus_secs(60 * (i as i64 + 1));
            talk.say(b, &g.help.example, Input::Spoken, now)
                .unwrap()
                .unwrap();
        }
        assert!(talk.finished());
    }

    fn export(b: &Backend) -> Value {
        b.store.export(T0).unwrap()
    }

    #[test]
    fn two_devices_practising_on_one_day_end_equal_with_the_sum() {
        let a = first("a");
        let b = empty("b");
        b.set_setting("handsFree", "true", T0).unwrap();
        assert_eq!(b.merge(&a.journal().unwrap(), T0).unwrap(), 1, "joins");
        assert_eq!(b.store.items().unwrap().len(), WORDS.len());

        // The same day, apart: a talk and a review on each.
        talk(&a, T0.plus_secs(HOUR));
        a.answer_review(WORDS[0], WORDS[0], false, T0.plus_secs(2 * HOUR));
        talk(&b, T0.plus_secs(3 * HOUR));
        b.answer_review(WORDS[1], WORDS[1], false, T0.plus_secs(4 * HOUR));
        let day = T0.day();
        let (da, db) = (a.store.day(day).unwrap(), b.store.day(day).unwrap());

        let from_a = a.journal().unwrap();
        assert!(a.merge(&b.journal().unwrap(), T0).unwrap() > 0);
        assert!(b.merge(&from_a, T0).unwrap() > 0);

        assert_eq!(export_but_device(&a), export_but_device(&b));
        let sum = a.store.day(day).unwrap();
        assert_eq!(sum.scenarios_done, da.scenarios_done + db.scenarios_done);
        assert_eq!(sum.reviews_done, da.reviews_done + db.reviews_done);
        assert_eq!(sum.speaking_secs, da.speaking_secs + db.speaking_secs);
        assert_eq!(sum.xp, da.xp + db.xp);
        assert_eq!(a.store.sessions(10).unwrap().len(), 2);

        // The device's own settings stay; a second merge brings nothing new.
        assert!(b.store.profile().unwrap().hands_free);
        assert!(!a.store.profile().unwrap().hands_free);
        assert_eq!(a.merge(&b.journal().unwrap(), T0).unwrap(), 0);
    }

    /// The export without the device's own settings.
    fn export_but_device(b: &Backend) -> Value {
        let mut v = export(b);
        v["profile"].as_object_mut().unwrap().remove("hands_free");
        v
    }

    #[test]
    fn a_talk_after_the_start_is_kept() {
        // The talk before the start keeps its id `<device>-<n>`: numbers must go on.
        let a = empty("after-start");
        a.set_setting("goal", "20", T0).unwrap();
        talk(&a, T0);
        a.share_base(T0).unwrap();
        let opened = Talk::begin(&a, SCENARIO, T0.plus_secs(HOUR), Length::Short).unwrap();
        assert!(opened.failed.is_empty(), "{:?}", opened.failed);
    }

    #[test]
    fn folding_keeps_the_recordings_the_learner_keeps() {
        // A talk ended while recordings were off; later the learner keeps them 30 days.
        let a = first("recordings");
        talk(&a, T0.plus_secs(HOUR));
        a.set_setting("keepRecordings", "30", T0.plus_secs(2 * HOUR))
            .unwrap();
        let kept = a
            .keep_recording("t1", 0, &[0.1; 1600], 16_000, T0.plus_secs(3 * HOUR))
            .unwrap()
            .expect("kept");

        let b = empty("recordings-b");
        b.merge(&a.journal().unwrap(), T0).unwrap();
        b.set_setting("goal", "20", T0.plus_secs(3 * HOUR)).unwrap();
        a.merge(&b.journal().unwrap(), T0.plus_secs(4 * HOUR))
            .unwrap();
        assert!(kept.exists(), "the fold deleted a kept recording");
    }

    #[test]
    fn the_start_holds_what_came_before() {
        let a = empty("before");
        a.set_setting("goal", "20", T0).unwrap();
        talk(&a, T0);
        let mut before = export(&a);
        a.share_base(T0).unwrap();
        let events = a.store.events().unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].event, Event::Base { .. }));

        // An import would write past the journal of a profile that syncs.
        assert!(!a.store.is_empty().unwrap());

        let b = empty("before-b");
        b.merge(&a.journal().unwrap(), T0).unwrap();
        let mut after = export(&b);
        for v in [&mut before, &mut after] {
            v.as_object_mut().unwrap().remove("events");
        }
        assert_eq!(before, after);
    }

    #[test]
    fn a_journal_of_another_profile_or_a_full_profile_is_refused() {
        let a = first("own");
        let other = first("other");
        assert!(matches!(
            a.merge(&other.journal().unwrap(), T0),
            Err(MergeError::Unrelated)
        ));

        // A profile with data that does not sync yet.
        let full = empty("full");
        talk(&full, T0);
        assert!(matches!(
            full.merge(&a.journal().unwrap(), T0),
            Err(MergeError::NotEmpty)
        ));

        // A journal without a start, and an event this version does not know.
        let fresh = empty("fresh");
        fresh.set_setting("goal", "20", T0).unwrap();
        assert!(matches!(
            a.merge(&fresh.journal().unwrap(), T0),
            Err(MergeError::Unrelated)
        ));
        let mut newer = a.journal().unwrap();
        newer[0]["body"]["kind"] = "from_a_newer_kaiwa".into();
        assert!(matches!(
            fresh.merge(&newer, T0),
            Err(MergeError::Unreadable)
        ));
    }
}
