//! The flow of one talk, once (ARCHITECTURE.md): choose the scenario, start or continue,
//! take a turn, keep it, record the finished talk, book what it cost, keep the learner's
//! marks. The app's talk and the short talk both use it; a front end only shows the result.
//!
//! ```text
//!  start / resume ─► ask ─► Asking::run (no lock) ─► take ─► save, or record at the end
//!                                                             └─► book costs
//! ```

use kaiwa_ai::Correction;
use kaiwa_core::difficulty::Knob;
use kaiwa_core::session::{Answer, Asking};
use kaiwa_core::{Day, Input, Moment, Session, TurnOutcome};
use kaiwa_store::{Dispute, Recorded};

use crate::Backend;

/// The scenario when the path is played through.
const FALLBACK: &str = "n5-02-jikoshoukai";

/// The longest sitting that counts as time spent in one go.
const MAX_SITTING_SECS: i64 = 3600;

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
}

/// What a turn did, and whether it was kept.
pub struct Turned {
    /// What the learner said, as the session has it (trimmed, rōmaji as kana).
    pub said: String,
    pub outcome: TurnOutcome,
    /// The record of the finished talk; `None` while it goes on, and for an unstored talk.
    /// An error: the turn is in the session, but it was not kept.
    pub kept: anyhow::Result<Option<Recorded>>,
}

/// A mark of the learner, kept.
pub struct Marked {
    /// The disputed correction.
    pub correction: Option<Correction>,
    /// The corrected record, when the talk was finished.
    pub recorded: Option<Recorded>,
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
    /// The next scenario of the learner's path; the path played through gives self-introduction.
    pub fn next_scenario(b: &Backend) -> anyhow::Result<String> {
        let learning = b.store.learning()?;
        Ok(b.curriculum
            .next_scenario(&learning.course)
            .unwrap_or_else(|| FALLBACK.into()))
    }

    /// The open talk of the profile, continued; `None` when there is none.
    pub fn resume(b: &Backend, now: Moment) -> anyhow::Result<Option<Self>> {
        let Some(row) = b.store.open_session()? else {
            return Ok(None);
        };
        let state = b.store.session_state(row.id)?;
        let Some(scenario) = b.scenarios.get(&state.scenario).cloned() else {
            return Ok(None);
        };
        let (session, demo) = b.continued_talk(scenario, state)?;
        b.arm_brake(now)?;
        Ok(Some(Self {
            session,
            id: Some(row.id),
            began: now,
            demo,
            finished: false,
        }))
    }

    /// The open talk continued, or the next scenario of the path begun.
    pub fn open_or_next(b: &Backend, now: Moment, length: Length) -> anyhow::Result<Self> {
        if let Some(talk) = Self::resume(b, now)? {
            return Ok(talk);
        }
        Self::begin(b, &Self::next_scenario(b)?, now, length)
    }

    /// A new talk on `scenario`, stored at once.
    pub fn begin(b: &Backend, scenario: &str, now: Moment, length: Length) -> anyhow::Result<Self> {
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
        b.arm_brake(now)?;
        let id = b.store.begin_session(&session.snapshot(), now)?;
        Ok(Self {
            session,
            id: Some(id),
            began: now,
            demo,
            finished: false,
        })
    }

    /// A talk the profile does not keep (a placement stage): no saving, no record.
    pub fn unstored(
        b: &Backend,
        session: Session,
        demo: bool,
        now: Moment,
    ) -> anyhow::Result<Self> {
        b.arm_brake(now)?;
        Ok(Self {
            session,
            id: None,
            began: now,
            demo,
            finished: false,
        })
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

    /// Time the learner spent in the talk, counted into its record.
    pub fn spend(&mut self, secs: u32) {
        self.session.spend(secs);
    }

    /// Seconds from `since` to `now` as a sitting counts, at most an hour.
    pub fn sitting(since: Moment, now: Moment) -> u32 {
        let secs = (now.unix - since.unix).clamp(0, MAX_SITTING_SECS);
        u32::try_from(secs).unwrap_or(0)
    }

    /// What the tutor is asked about `text`; run it outside any lock.
    pub fn ask(&self, text: &str) -> Asking {
        self.session.ask(text)
    }

    /// The tutor's whole turn in one call, for a front end that has no lock to avoid.
    pub fn say(
        &mut self,
        b: &Backend,
        text: &str,
        input: Input,
        now: Moment,
    ) -> anyhow::Result<Turned> {
        let text = prepare(text, input);
        let answer = match self.ask(&text).run() {
            Ok(answer) => answer,
            Err(e) => {
                // What the failed call cost still counts.
                let _ = b.book_costs(now, false);
                return Err(e.into());
            }
        };
        Ok(self.take(b, &text, input, answer, now))
    }

    /// Takes the tutor's answer into the session, keeps the talk (the state, or the record
    /// once the goals are reached), then books the cost of the turn.
    pub fn take(
        &mut self,
        b: &Backend,
        text: &str,
        input: Input,
        answer: Answer,
        now: Moment,
    ) -> Turned {
        let outcome = self.session.take(text, input, answer);
        let kept = self.keep(b, now);
        let costs = b.book_costs(now, self.finished);
        // The save's error is the first to tell.
        let kept = kept.and_then(|recorded| costs.map(|_| recorded));
        Turned {
            said: text.to_string(),
            outcome,
            kept,
        }
    }

    /// The learner marks `turn` as not what was said (/LF113/) and the mark is kept.
    pub fn mark_misrecognized(
        &mut self,
        b: &Backend,
        turn: usize,
    ) -> anyhow::Result<Option<Marked>> {
        if !self.session.mark_misrecognized(turn) {
            return Ok(None);
        }
        let recorded = self.keep_marks(b)?;
        Ok(Some(Marked {
            correction: None,
            recorded,
        }))
    }

    /// The learner disputes the correction `better` of `turn` (/LF307/); it goes to the QA
    /// list and the mark is kept. `None` when there is no such correction.
    pub fn dispute(
        &mut self,
        b: &Backend,
        turn: usize,
        better: &str,
    ) -> anyhow::Result<Option<Marked>> {
        let Some(learner) = self.session.turns().get(turn).map(|t| t.learner.clone()) else {
            return Ok(None);
        };
        let Some(c) = self.session.dispute(turn, better) else {
            return Ok(None);
        };
        let listed = b.store.add_dispute(&Dispute {
            at: crate::now(),
            scenario: self.session.scenario.id.clone(),
            learner,
            original: c.original.clone(),
            better: c.better.clone(),
            explanation: c.explanation.clone(),
        });
        // The mark is kept even when the list is not.
        let recorded = self.keep_marks(b);
        listed?;
        Ok(Some(Marked {
            correction: Some(c),
            recorded: recorded?,
        }))
    }

    /// The state after a turn, or the record at the end of the talk.
    fn keep(&mut self, b: &Backend, now: Moment) -> anyhow::Result<Option<Recorded>> {
        let Some(id) = self.id else {
            return Ok(None);
        };
        if !self.session.done() {
            b.store.save_session(id, &self.session.snapshot())?;
            return Ok(None);
        }
        if self.finished {
            return Ok(None);
        }

        // The time spent over every sitting, counted for the day this one began (a talk
        // begun before midnight and finished after counts for the day it was continued).
        let begun = now.plus_secs(-i64::from(self.session.spent_secs()));
        let day: Day = self.began.day();
        let recorded =
            b.store
                .record_on(id, &self.session, begun, now, day, Some(&b.curriculum))?;
        self.finished = true;
        b.track_grammar(&self.session, now)?;
        Ok(Some(recorded))
    }

    /// Keeps the marks: the state of a running talk, the corrected record of a finished one.
    /// A correction that fails (a later talk was recorded) still saves the marks.
    fn keep_marks(&self, b: &Backend) -> anyhow::Result<Option<Recorded>> {
        let Some(id) = self.id else {
            return Ok(None);
        };
        if !self.finished {
            b.store.save_session(id, &self.session.snapshot())?;
            return Ok(None);
        }
        match b.correct_talk(id, &self.session) {
            Ok(recorded) => Ok(Some(recorded)),
            Err(e) => {
                b.store.save_session(id, &self.session.snapshot())?;
                Err(e)
            }
        }
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

    fn backend(name: &str) -> TempBackend {
        TempBackend::new(name, true).unwrap()
    }

    /// Says the goals' example lines until the talk is done.
    fn finish(b: &Backend, t: &mut Talk, now: Moment) -> Option<Recorded> {
        let goals = t.session().goals().to_vec();
        let mut recorded = None;
        for g in &goals {
            let turned = t.say(b, &g.help.example, Input::Spoken, now).unwrap();
            recorded = turned.kept.unwrap().or(recorded);
        }
        assert!(t.session().done());
        recorded
    }

    // The tests arm the process's meter, as the cost tests read it.
    fn meter() -> std::sync::MutexGuard<'static, ()> {
        crate::cost::tests::METER
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn the_next_scenario_is_the_paths_or_self_introduction() {
        let b = backend("flow-next");
        let on_path = b
            .curriculum
            .next_scenario(&b.store.learning().unwrap().course)
            .expect("an unplayed path has a next scenario");
        assert_eq!(Talk::next_scenario(&b).unwrap(), on_path);

        let mut l = b.store.learning().unwrap();
        l.course.scenarios_done = b.scenarios.keys().cloned().collect();
        b.store.set_learning(&l).unwrap();
        assert_eq!(Talk::next_scenario(&b).unwrap(), FALLBACK);
    }

    #[test]
    fn a_new_talk_is_stored_and_saved_after_a_turn() {
        let _m = meter();
        let b = backend("flow-save");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = Talk::begin(&b, SCENARIO, now, Length::Short).unwrap();
        assert_eq!(t.session().goals().len(), 2);
        assert!(!t.session().grammar().is_empty(), "the unit's grammar");
        let id = t.id().expect("stored at once");
        assert_eq!(b.store.session_state(id).unwrap().turns.len(), 0);

        let turned = t.say(&b, "はい", Input::Typed, now).unwrap();
        assert!(turned.kept.unwrap().is_none());
        assert_eq!(b.store.session_state(id).unwrap().turns.len(), 1);
    }

    #[test]
    fn a_full_talk_plays_the_goals_of_the_difficulty() {
        let b = backend("flow-full");
        let now = Moment::new(1_791_100_000, 120);
        let full = Talk::begin(&b, SCENARIO, now, Length::Full).unwrap();
        let short = Talk::begin(&b, SCENARIO, now, Length::Short).unwrap();
        assert!(full.session().goals().len() >= short.session().goals().len());
        assert!(Talk::begin(&b, "no-such", now, Length::Full).is_err());
    }

    #[test]
    fn a_finished_talk_is_recorded_with_its_spent_time_on_its_day() {
        let _m = meter();
        let b = backend("flow-record");
        let began = evening();
        let end = began.plus_secs(2 * 3600);
        let mut t = Talk::begin(&b, SCENARIO, began, Length::Short).unwrap();
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
        let _m = meter();
        let b = backend("flow-resume");
        let now = Moment::new(1_791_100_000, 120);
        assert!(Talk::resume(&b, now).unwrap().is_none());
        let mut t = Talk::open_or_next(&b, now, Length::Short).unwrap();
        t.say(&b, "はい", Input::Spoken, now).unwrap();

        let again = Talk::open_or_next(&b, now, Length::Full).unwrap();
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
        let _m = meter();
        let b = backend("flow-unstored");
        let now = Moment::new(1_791_100_000, 120);
        let (session, demo) = b
            .new_talk(b.scenarios.get(SCENARIO).cloned().unwrap())
            .unwrap();
        let mut t = Talk::unstored(&b, session, demo, now).unwrap();
        let sessions = b.store.sessions(10).unwrap().len();

        let goals = t.session().goals().to_vec();
        for g in &goals {
            let turned = t.say(&b, &g.help.example, Input::Spoken, now).unwrap();
            assert!(turned.kept.unwrap().is_none());
        }

        assert!(t.session().done());
        assert!(t.id().is_none() && !t.finished());
        assert_eq!(b.store.sessions(10).unwrap().len(), sessions);
        assert!(b.store.open_session().unwrap().is_none());
        let marked = t.mark_misrecognized(&b, 0).unwrap();
        assert!(marked.is_some_and(|m| m.recorded.is_none()));
    }

    #[test]
    fn a_failed_correction_still_saves_the_marks() {
        let _m = meter();
        let b = backend("flow-superseded");
        let now = Moment::new(1_791_100_000, 120);
        let mut first = Talk::begin(&b, SCENARIO, now, Length::Short).unwrap();
        finish(&b, &mut first, now);
        // A later talk is recorded: the first one can no longer be corrected.
        let mut later = Talk::begin(&b, SCENARIO, now.plus_secs(60), Length::Short).unwrap();
        finish(&b, &mut later, now.plus_secs(120));

        let err = first
            .mark_misrecognized(&b, 0)
            .err()
            .expect("no correction");

        let superseded = StoreError::Superseded(first.id().unwrap()).to_string();
        assert!(err.to_string().contains(&superseded), "{err:#}");
        let kept = b.store.session_state(first.id().unwrap()).unwrap();
        assert!(kept.turns[0].misrecognized, "the mark is saved");
    }

    #[test]
    fn a_dispute_is_listed_and_corrects_the_record() {
        let _m = meter();
        let b = backend("flow-dispute");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = Talk::begin(&b, SCENARIO, now, Length::Short).unwrap();
        finish(&b, &mut t, now);
        let found = t
            .session()
            .turns()
            .iter()
            .enumerate()
            .find_map(|(i, turn)| turn.corrections.first().map(|c| (i, c.better.clone())));
        let Some((turn, better)) = found else {
            // No correction in the recorded answers: nothing to dispute.
            assert!(t.dispute(&b, 0, "x").unwrap().is_none());
            return;
        };

        let marked = t.dispute(&b, turn, &better).unwrap().expect("disputed");

        assert!(marked.correction.is_some() && marked.recorded.is_some());
        assert_eq!(b.store.disputes().unwrap().len(), 1);
    }
}
