//! The flow of one talk, once (ARCHITECTURE.md): choose the scenario, start or continue,
//! take a turn, keep it, record the finished talk, book what it cost, keep the learner's
//! marks. The app's talk and the short talk both use it; a front end only shows the result.
//!
//! ```text
//!  start / resume ─► ask ─► Asking::run (no lock) ─► take ─► save, or record at the end
//!                                                             └─► book costs
//! ```
//!
//! What was booked is never lost to a later failure: a result carries what was done and,
//! apart from it, the failures that came with it (`failed`), which the front end shows.

use kaiwa_ai::Correction;
use kaiwa_core::difficulty::Knob;
use kaiwa_core::session::{Answer, Asking};
use kaiwa_core::{Day, Input, Moment, Session, TurnOutcome};
use kaiwa_store::{Dispute, Recorded};

use crate::Backend;

mod aids;
pub use aids::{Fetched, Hear, Heard, Helped, Translated, Translation, VoiceState};

/// The scenario when the path is played through.
const FALLBACK: &str = "n5-02-jikoshoukai";

/// The longest sitting that counts as time spent in one go.
const MAX_SITTING_SECS: u32 = 3600;

/// How many goals a talk plays.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Length {
    /// What the learner's difficulty says.
    Full,
    /// Two goals (/LF501/).
    Short,
}

/// A talk in progress, stored in the profile or not (a placement stage is not).
pub struct Talk {
    session: Session,
    id: Option<i64>,
    began: Moment,
    demo: bool,
    finished: bool,
    /// A turn is with the tutor ([`Talk::ask`] until it is taken or failed).
    asking: bool,
}

/// A talk that opened, with what went wrong on the way: the brake not armed, the talk not
/// stored (it then runs unstored), the path unreadable (the default scenario stands).
pub struct Opened {
    pub talk: Talk,
    pub failed: Vec<anyhow::Error>,
}

/// What a turn did, and what went wrong with keeping it.
pub struct Turned {
    /// What the learner said, as the session has it (trimmed, rōmaji as kana).
    pub said: String,
    pub outcome: TurnOutcome,
    /// The record of the talk when this turn finished it.
    pub recorded: Option<Recorded>,
    /// Not saved, not tracked, costs not booked. The turn is in the session regardless.
    pub failed: Vec<anyhow::Error>,
}

/// A mark of the learner, kept.
pub struct Marked {
    /// The disputed correction.
    pub correction: Option<Correction>,
    /// The corrected record, when the talk was finished.
    pub recorded: Option<Recorded>,
    /// The mark is in the session regardless; this is what could not be kept.
    pub failed: Vec<anyhow::Error>,
}

/// What the learner typed or said as the session takes it (/LF120/): trimmed, and typed
/// rōmaji as kana ("watashi" becomes "わたし").
pub fn prepare(text: &str, input: Input) -> String {
    let text = text.trim();
    if input == Input::Typed && kaiwa_core::romaji::has_romaji(text) {
        return kaiwa_core::romaji::to_kana(text);
    }
    text.to_string()
}

impl Talk {
    /// The next scenario of the learner's path; the path played through gives
    /// self-introduction, and so does a profile whose course cannot be read (reported).
    pub fn next_scenario(b: &Backend) -> (String, Vec<anyhow::Error>) {
        let (learning, failed) = match b.store.learning() {
            Ok(l) => (l, Vec::new()),
            Err(e) => (Default::default(), vec![e.into()]),
        };
        let id = b
            .curriculum
            .next_scenario(&learning.course)
            .unwrap_or_else(|| FALLBACK.into());
        (id, failed)
    }

    /// The open talk of the profile, continued; `None` when there is none.
    pub fn resume(b: &Backend, now: Moment) -> anyhow::Result<Option<Opened>> {
        let Some(row) = b.store.open_session()? else {
            return Ok(None);
        };
        let state = b.store.session_state(row.id)?;
        let Some(scenario) = b.scenarios.get(&state.scenario).cloned() else {
            return Ok(None);
        };
        let (session, demo) = b.continued_talk(scenario, state)?;
        let failed = b.arm_brake(now).err().into_iter().collect();
        let talk = Self {
            session,
            id: Some(row.id),
            began: now,
            demo,
            finished: false,
            asking: false,
        };
        Ok(Some(Opened { talk, failed }))
    }

    /// The open talk continued, or the next scenario of the path begun.
    pub fn open_or_next(b: &Backend, now: Moment, length: Length) -> anyhow::Result<Opened> {
        if let Some(opened) = Self::resume(b, now)? {
            return Ok(opened);
        }
        let (id, mut failed) = Self::next_scenario(b);
        let mut opened = Self::begin(b, &id, now, length)?;
        failed.append(&mut opened.failed);
        opened.failed = failed;
        Ok(opened)
    }

    /// A new talk on `scenario`, stored at once; when it cannot be stored it runs unstored.
    pub fn begin(
        b: &Backend,
        scenario: &str,
        now: Moment,
        length: Length,
    ) -> anyhow::Result<Opened> {
        let scenario = b
            .scenarios
            .get(scenario)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no scenario {scenario}"))?;
        let (mut session, demo) = b.new_talk(scenario)?;
        if length == Length::Short {
            let mut difficulty = session.difficulty.clone();
            difficulty.set(Knob::Goals, 0);
            session = session.with_difficulty(difficulty);
        }
        let mut failed: Vec<anyhow::Error> = b.arm_brake(now).err().into_iter().collect();
        let id = match b.store.begin_session(&session.snapshot(), now) {
            Ok(id) => Some(id),
            Err(e) => {
                failed.push(e.into());
                None
            }
        };
        let talk = Self {
            session,
            id,
            began: now,
            demo,
            finished: false,
            asking: false,
        };
        Ok(Opened { talk, failed })
    }

    /// A talk the profile does not keep (a placement stage): no saving, no record.
    pub fn unstored(b: &Backend, session: Session, demo: bool, now: Moment) -> Opened {
        let talk = Self {
            session,
            id: None,
            began: now,
            demo,
            finished: false,
            asking: false,
        };
        let failed = b.arm_brake(now).err().into_iter().collect();
        Opened { talk, failed }
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn id(&self) -> Option<i64> {
        self.id
    }

    pub fn demo(&self) -> bool {
        self.demo
    }

    /// Whether the talk was recorded.
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Time the learner spent in the talk, counted into its record: at most an hour per
    /// sitting, for a window left open.
    pub fn spend(&mut self, secs: u32) {
        self.session.spend(secs.min(MAX_SITTING_SECS));
    }

    /// Seconds from `since` to `now` as a sitting counts, at most an hour.
    pub fn sitting(since: Moment, now: Moment) -> u32 {
        let secs = (now.unix - since.unix).clamp(0, i64::from(MAX_SITTING_SECS));
        u32::try_from(secs).unwrap_or(0)
    }

    /// Whether a turn with `text` may go to the tutor: something was said and the talk is
    /// not recorded yet.
    pub fn may_say(&self, text: &str) -> bool {
        !text.trim().is_empty() && !self.finished
    }

    /// Keeps the state of a running talk: after help, an aid, a translation, a failed
    /// call. A turn is kept by [`Talk::take`]. Nothing for an unstored or finished talk.
    pub fn save(&self, b: &Backend) -> anyhow::Result<()> {
        let Some(id) = self.id.filter(|_| !self.finished) else {
            return Ok(());
        };
        b.store.save_session(id, &self.session.snapshot())?;
        Ok(())
    }

    /// What the tutor is asked about `text`; run it outside any lock. Help and listening aids
    /// do not count until the answer is taken ([`Talk::take`]) or the call failed
    /// ([`Talk::failed_call`]).
    pub fn ask(&mut self, text: &str) -> Asking {
        self.asking = true;
        self.session.ask(text)
    }

    /// The tutor's whole turn in one call, for a front end that has no lock to avoid.
    /// `None` when [`Talk::may_say`] says no. The caller adds the time spent first
    /// ([`Talk::spend`]). A failed call is the error; its cost is booked and the state
    /// saved (the session may have changed since the last save).
    pub fn say(
        &mut self,
        b: &Backend,
        text: &str,
        input: Input,
        now: Moment,
    ) -> anyhow::Result<Option<Turned>> {
        let text = prepare(text, input);
        if !self.may_say(&text) {
            return Ok(None);
        }
        let answer = match self.ask(&text).run() {
            Ok(answer) => answer,
            Err(e) => {
                let e = anyhow::Error::from(e);
                return Err(self
                    .failed_call(b, now)
                    .into_iter()
                    .fold(e, |e, f| e.context(f)));
            }
        };
        Ok(Some(self.take(b, &text, input, answer, now)))
    }

    /// The tutor did not answer: books the cost and keeps the state. What failed on top of
    /// that is returned; a front end that runs the call itself ([`Talk::ask`]) calls this.
    pub fn failed_call(&mut self, b: &Backend, now: Moment) -> Vec<anyhow::Error> {
        self.asking = false;
        let booked = b.book_costs(now, false).err();
        booked.into_iter().chain(self.save(b).err()).collect()
    }

    /// Takes the tutor's answer into the session, keeps the talk (the state, or the record
    /// once the goals are reached), then books the cost of the turn. The caller adds the
    /// time spent in the talk before ([`Talk::spend`]): the record counts it.
    pub fn take(
        &mut self,
        b: &Backend,
        text: &str,
        input: Input,
        answer: Answer,
        now: Moment,
    ) -> Turned {
        self.asking = false;
        let outcome = self.session.take(text, input, answer);
        let mut failed = Vec::new();
        let recorded = self.keep(b, now, &mut failed);
        failed.extend(b.book_costs(now, self.finished).err());
        Turned {
            said: text.to_string(),
            outcome,
            recorded,
            failed,
        }
    }

    /// The learner marks `turn` as not what was said (/LF113/) and the mark is kept.
    /// `None` when there is no such turn.
    pub fn mark_misrecognized(&mut self, b: &Backend, turn: usize) -> Option<Marked> {
        if !self.session.mark_misrecognized(turn) {
            return None;
        }
        let mut failed = Vec::new();
        let recorded = self.keep_marks(b, &mut failed);
        Some(Marked {
            correction: None,
            recorded,
            failed,
        })
    }

    /// The learner disputes the correction `better` of `turn` (/LF307/); it goes to the QA
    /// list and the mark is kept. `None` when there is no such correction.
    pub fn dispute(&mut self, b: &Backend, turn: usize, better: &str) -> Option<Marked> {
        let learner = self.session.turns().get(turn)?.learner.clone();
        let c = self.session.dispute(turn, better)?;
        let mut failed = Vec::new();
        let listed = b.store.add_dispute(&Dispute {
            at: crate::now(),
            scenario: self.session.scenario.id.clone(),
            learner,
            original: c.original.clone(),
            better: c.better.clone(),
            explanation: c.explanation.clone(),
        });
        failed.extend(listed.err().map(anyhow::Error::from));
        // The mark is kept even when the list is not.
        let recorded = self.keep_marks(b, &mut failed);
        Some(Marked {
            correction: Some(c),
            recorded,
            failed,
        })
    }

    /// The state after a turn, or the record at the end of the talk.
    fn keep(
        &mut self,
        b: &Backend,
        now: Moment,
        failed: &mut Vec<anyhow::Error>,
    ) -> Option<Recorded> {
        let id = self.id?;
        if !self.session.done() {
            failed.extend(
                b.store
                    .save_session(id, &self.session.snapshot())
                    .err()
                    .map(Into::into),
            );
            return None;
        }
        if self.finished {
            return None;
        }

        // The time spent over every sitting, counted for the day this one began (a talk
        // begun before midnight and finished after counts for the day it was continued).
        let begun = now.plus_secs(-i64::from(self.session.spent_secs()));
        let day: Day = self.began.day();
        let recorded = b
            .store
            .record_on(id, &self.session, begun, now, day, Some(&b.curriculum));
        let recorded = match recorded {
            Ok(r) => r,
            Err(e) => {
                failed.push(e.into());
                return None;
            }
        };
        self.finished = true;
        failed.extend(b.track_grammar(&self.session, now).err());
        Some(recorded)
    }

    /// Keeps the marks: the state of a running talk, the corrected record of a finished one.
    /// A correction that fails (a later talk was recorded) still saves the marks.
    fn keep_marks(&self, b: &Backend, failed: &mut Vec<anyhow::Error>) -> Option<Recorded> {
        let id = self.id?;
        if self.finished {
            match b.correct_talk(id, &self.session) {
                Ok(recorded) => return Some(recorded),
                Err(e) => failed.push(e),
            }
        }
        failed.extend(
            b.store
                .save_session(id, &self.session.snapshot())
                .err()
                .map(Into::into),
        );
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TempBackend;
    use kaiwa_store::StoreError;

    const SCENARIO: &str = "n5-02-jikoshoukai";

    /// 23:00 on some day, UTC: two hours on it is the next day.
    fn evening() -> Moment {
        Moment::new(1_791_100_000 - 1_791_100_000 % 86_400 + 23 * 3600, 0)
    }

    fn open(b: &Backend, scenario: &str, now: Moment, length: Length) -> Talk {
        let opened = Talk::begin(b, scenario, now, length).unwrap();
        assert!(opened.failed.is_empty(), "{:?}", opened.failed);
        opened.talk
    }

    fn backend(name: &str) -> TempBackend {
        TempBackend::new(name, true).unwrap()
    }

    /// Says the goals' example lines until the talk is done.
    fn finish(b: &Backend, t: &mut Talk, now: Moment) -> Option<Recorded> {
        let goals = t.session().goals().to_vec();
        let mut recorded = None;
        for g in &goals {
            let turned = say(b, t, &g.help.example, now);
            assert!(turned.failed.is_empty(), "{:?}", turned.failed);
            recorded = turned.recorded.or(recorded);
        }
        assert!(t.session().done());
        recorded
    }

    fn say(b: &Backend, t: &mut Talk, text: &str, now: Moment) -> Turned {
        t.say(b, text, Input::Spoken, now)
            .unwrap()
            .expect("may say")
    }

    #[test]
    fn the_next_scenario_is_the_paths_or_self_introduction() {
        let b = backend("flow-next");
        let on_path = b
            .curriculum
            .next_scenario(&b.store.learning().unwrap().course)
            .expect("an unplayed path has a next scenario");
        let (id, failed) = Talk::next_scenario(&b);
        assert_eq!(id, on_path);
        assert!(failed.is_empty());

        let mut l = b.store.learning().unwrap();
        l.course.scenarios_done = b.scenarios.keys().cloned().collect();
        b.store.set_learning(&l).unwrap();
        assert_eq!(Talk::next_scenario(&b).0, FALLBACK);
    }

    #[test]
    fn a_new_talk_is_stored_and_saved_after_a_turn() {
        let b = backend("flow-save");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = open(&b, SCENARIO, now, Length::Short);
        assert_eq!(t.session().goals().len(), 2);
        assert!(!t.session().grammar().is_empty(), "the unit's grammar");
        let id = t.id().expect("stored at once");
        assert_eq!(b.store.session_state(id).unwrap().turns.len(), 0);

        let turned = t.say(&b, "はい", Input::Typed, now).unwrap().unwrap();
        assert!(turned.recorded.is_none() && turned.failed.is_empty());
        assert_eq!(b.store.session_state(id).unwrap().turns.len(), 1);
    }

    #[test]
    fn a_sitting_counts_at_most_an_hour() {
        // A window left open overnight books an hour, not the night.
        let b = backend("flow-sitting");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = open(&b, SCENARIO, now, Length::Short);
        t.spend(8 * 3600);
        t.spend(60);
        assert_eq!(t.session().spent_secs(), 3600 + 60);
    }

    #[test]
    fn a_full_talk_plays_the_goals_of_the_difficulty() {
        let b = backend("flow-full");
        let now = Moment::new(1_791_100_000, 120);
        let full = open(&b, SCENARIO, now, Length::Full);
        let short = open(&b, SCENARIO, now, Length::Short);
        assert!(full.session().goals().len() >= short.session().goals().len());
        assert!(Talk::begin(&b, "no-such", now, Length::Full).is_err());
    }

    #[test]
    fn a_finished_talk_is_recorded_with_its_spent_time_on_its_day() {
        let b = backend("flow-record");
        let began = evening();
        let end = began.plus_secs(2 * 3600);
        let mut t = open(&b, SCENARIO, began, Length::Short);
        t.spend(600);
        let before = b.store.day(began.day()).unwrap().speaking_secs;

        let r = finish(&b, &mut t, end).expect("recorded at the end");

        assert!(t.finished());
        assert_ne!(began.day(), end.day());
        let spoken = b.store.day(began.day()).unwrap().speaking_secs - before;
        assert_eq!(spoken, r.day.speaking_secs - before);
        assert_eq!(spoken, 600, "the time spent, on the day it began");
        let row = (b.store.sessions(100).unwrap().into_iter())
            .find(|r| Some(r.id) == t.id())
            .expect("the talk's row");
        assert_eq!(row.ended, Some(end));
        assert!(row.finished);
    }

    #[test]
    fn an_open_talk_is_continued() {
        let b = backend("flow-resume");
        let now = Moment::new(1_791_100_000, 120);
        assert!(Talk::resume(&b, now).unwrap().is_none());
        let mut t = Talk::open_or_next(&b, now, Length::Short).unwrap().talk;
        say(&b, &mut t, "はい", now);

        let again = Talk::open_or_next(&b, now, Length::Full).unwrap().talk;
        assert_eq!(again.id(), t.id());
        assert_eq!(again.session().turns().len(), 1);
        assert_eq!(again.session().goals().len(), 2, "its own goals stay");
    }

    #[test]
    fn typed_romaji_becomes_kana_after_trimming() {
        assert_eq!(prepare("  watashi ", Input::Typed), "わたし");
        assert_eq!(prepare(" はい ", Input::Typed), "はい");
        assert_eq!(prepare(" watashi ", Input::Spoken), "watashi");
    }

    #[test]
    fn an_unstored_talk_saves_nothing() {
        let b = backend("flow-unstored");
        let now = Moment::new(1_791_100_000, 120);
        let (session, demo) = b
            .new_talk(b.scenarios.get(SCENARIO).cloned().unwrap())
            .unwrap();
        let mut t = Talk::unstored(&b, session, demo, now).talk;
        let sessions = b.store.sessions(10).unwrap().len();

        let goals = t.session().goals().to_vec();
        for g in &goals {
            let turned = say(&b, &mut t, &g.help.example, now);
            assert!(turned.recorded.is_none() && turned.failed.is_empty());
        }

        assert!(t.session().done());
        assert!(t.id().is_none() && !t.finished());
        assert_eq!(b.store.sessions(10).unwrap().len(), sessions);
        assert!(b.store.open_session().unwrap().is_none());
        let marked = t.mark_misrecognized(&b, 0).expect("marked");
        assert!(marked.recorded.is_none() && marked.failed.is_empty());
    }

    #[test]
    fn a_failed_correction_still_saves_the_marks() {
        let b = backend("flow-superseded");
        let now = Moment::new(1_791_100_000, 120);
        let mut first = open(&b, SCENARIO, now, Length::Short);
        finish(&b, &mut first, now);
        // A later talk is recorded: the first one can no longer be corrected.
        let mut later = open(&b, SCENARIO, now.plus_secs(60), Length::Short);
        finish(&b, &mut later, now.plus_secs(120));

        let marked = first.mark_misrecognized(&b, 0).expect("marked");

        let superseded = StoreError::Superseded(first.id().unwrap()).to_string();
        assert_eq!(marked.failed.len(), 1, "only the correction failed");
        assert!(marked.failed[0].to_string().contains(&superseded));
        assert!(marked.recorded.is_none());
        let kept = b.store.session_state(first.id().unwrap()).unwrap();
        assert!(kept.turns[0].misrecognized, "the mark is saved");
    }

    #[test]
    fn a_dispute_is_listed_and_corrects_the_record() {
        let b = backend("flow-dispute");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = open(&b, SCENARIO, now, Length::Short);
        finish(&b, &mut t, now);
        let found = t
            .session()
            .turns()
            .iter()
            .enumerate()
            .find_map(|(i, turn)| turn.corrections.first().map(|c| (i, c.better.clone())));
        let Some((turn, better)) = found else {
            // No correction in the recorded answers: nothing to dispute.
            assert!(t.dispute(&b, 0, "x").is_none());
            return;
        };

        let marked = t.dispute(&b, turn, &better).expect("disputed");

        assert!(marked.correction.is_some() && marked.recorded.is_some());
        assert!(marked.failed.is_empty());
        assert_eq!(b.store.disputes().unwrap().len(), 1);
    }

    /// Makes the month's cost entry unreadable: arming the brake and booking fail.
    fn break_costs(b: &Backend, now: Moment) {
        let (y, m) = now.day().month();
        let key = format!("{}{y:04}-{m:02}", kaiwa_store::COST_STATE_PREFIX);
        b.store.set_state_value(&key, &"unreadable").unwrap();
    }

    #[test]
    fn a_talk_opens_when_the_brake_cannot_be_armed() {
        let b = backend("flow-brake");
        let now = Moment::new(1_791_100_000, 120);
        break_costs(&b, now);

        let opened = Talk::begin(&b, SCENARIO, now, Length::Short).unwrap();

        assert_eq!(opened.failed.len(), 1, "reported");
        assert!(opened.talk.id().is_some(), "still stored");
    }

    #[test]
    fn a_booked_finish_survives_a_failed_cost_booking() {
        let b = backend("flow-booked");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = open(&b, SCENARIO, now, Length::Short);
        break_costs(&b, now);

        let goals = t.session().goals().to_vec();
        let mut last = None;
        for g in &goals {
            last = Some(say(&b, &mut t, &g.help.example, now));
        }

        let last = last.unwrap();
        assert!(last.recorded.is_some(), "the record is not lost");
        assert!(t.finished());
        assert_eq!(last.failed.len(), 1, "the cost booking is reported");
    }

    #[test]
    fn nothing_is_said_empty_or_after_the_end() {
        let b = backend("flow-guard");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = open(&b, SCENARIO, now, Length::Short);
        assert!(!t.may_say("  "));
        assert!(t.say(&b, " \n", Input::Typed, now).unwrap().is_none());
        assert!(t.session().turns().is_empty());

        finish(&b, &mut t, now);

        assert!(!t.may_say("はい"));
        assert!(t.say(&b, "はい", Input::Spoken, now).unwrap().is_none());
    }

    #[test]
    fn a_change_outside_a_turn_is_saved_on_request() {
        let b = backend("flow-save-outside");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = open(&b, SCENARIO, now, Length::Short);
        say(&b, &mut t, "はい", now);
        t.spend(42);

        t.save(&b).unwrap();

        let kept = b.store.session_state(t.id().unwrap()).unwrap();
        assert_eq!(kept.spent_secs, 42);
    }
}
