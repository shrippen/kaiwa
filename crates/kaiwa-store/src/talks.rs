//! Booking a finished talk and correcting it afterwards (/LF201/, /LF230/).

use crate::*;

impl Store {
    /// Ends a session: learner model, regulator, path, review items and the day (/LF201/,
    /// /LF210/, /LF220/, /LF230/, /LF601/).
    pub fn record(
        &self,
        id: &str,
        s: &Session,
        started: Moment,
        ended: Moment,
        curriculum: Option<&Curriculum>,
    ) -> Result<Recorded> {
        self.record_on(id, s, started, ended, started.day(), curriculum)
    }

    /// As [`Store::record`], for a talk of several sittings (/LF121/): `started` is the end
    /// minus all the time spent in it, and `day` the day its last sitting began, which gets
    /// the practice (a talk begun yesterday and finished after midnight counts for today).
    pub fn record_on(
        &self,
        id: &str,
        s: &Session,
        started: Moment,
        ended: Moment,
        day: Day,
        curriculum: Option<&Curriculum>,
    ) -> Result<Recorded> {
        let stats = s.stats();
        let success = stats.success();
        let summary = s.summary();
        let mut l = self.learning()?;
        let learning_before = l.clone();
        // Where the day started from, for the week's progress per skill (/LF604/).
        let levels_before: std::collections::BTreeMap<_, _> = kaiwa_core::learner::SKILLS
            .iter()
            .map(|k| (*k, l.model.level(*k)))
            .collect();
        let (change, scenario_done) = l.book(s, success);

        let unit = curriculum
            .and_then(|c| c.unit_of(&s.scenario.id))
            .map(|u| u.id.clone());
        let mut items = self.items()?;
        let before = items.len();
        let corrections = corrected_of(&summary);
        srs::add_from_summary(
            &mut items,
            &summary.review,
            &corrections,
            unit.as_deref(),
            ended,
        );
        for item in &items[before..] {
            self.put_item(item)?;
        }
        let recording = Recording {
            learning: learning_before,
            day,
            added: items[before..]
                .iter()
                .map(|i| kaiwa_core::norm(&i.text))
                .collect(),
        };

        // A turn whose analysis had gaps was spoken too (T-11), so the share and the day's
        // typed turns count all turns.
        let typed = turn_counts(s.turns()).1;
        let speaking = s.spoken_secs((ended.unix - started.unix) as f64);
        let xp = summary.xp.xp;
        let mut rec = self.day(day)?;
        rec.keep_goal(day, ended.day(), self.profile()?.daily_goal);
        rec.add_talk(
            levels_before,
            speaking,
            typed,
            xp,
            scenario_done || s.done(),
        );
        self.put_day(day, &rec)?;
        self.set_learning(&l)?;
        self.db.execute(
            "UPDATE sessions SET ended = ?2, finished = 1, success = ?3, xp = ?4, speaking_secs = ?5, state = ?6,
             recording = ?7 WHERE id = ?1",
            params![
                id,
                ended.unix,
                success,
                xp,
                speaking,
                serde_json::to_string(&s.snapshot())?,
                serde_json::to_string(&recording)?
            ],
        )?;
        Ok(Recorded {
            success,
            change,
            scenario_done,
            new_items: items.len() - before,
            xp,
            day: rec,
        })
    }

    /// The learner marked turns of a recorded talk on the summary (V-05): redoes what
    /// [`Store::record_on`] stored from `s` as it is now. The learner model, the regulator and
    /// the course are replayed from before the talk, the review items this talk added and the
    /// marks no longer justify are removed, the stored talk and its success are updated; all
    /// in one transaction. `retrack` redoes the grammar tracking on the replayed learning
    /// state, with the moment the talk ended. Only the newest talk can be corrected: a later
    /// one built on its result (`Superseded`).
    pub fn correct<F: FnOnce(&mut Learning, Moment)>(
        &self,
        id: &str,
        s: &Session,
        retrack: F,
    ) -> Result<Recorded> {
        self.batch(|st| st.correct_in(id, s, retrack))
    }

    fn correct_in<F: FnOnce(&mut Learning, Moment)>(
        &self,
        id: &str,
        s: &Session,
        retrack: F,
    ) -> Result<Recorded> {
        let row = self
            .db
            .query_row(
                &format!("{} WHERE id = ?1", Self::ROW),
                params![id],
                Self::row,
            )
            .optional()?
            .ok_or_else(|| StoreError::NoSession(id.into()))?;
        let stored: Option<String> = self.db.query_row(
            "SELECT recording FROM sessions WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        let (Some(stored), true, Some(ended)) = (stored, row.finished, row.ended) else {
            return Err(StoreError::NoRecording(id.into()));
        };
        let mut recording: Recording = serde_json::from_str(&stored)?;
        // The newest is the one recorded last: a talk begun earlier but finished later is.
        let later: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM sessions
             WHERE finished = 1 AND (ended > ?2 OR (ended = ?2 AND rowid > (SELECT rowid FROM sessions WHERE id = ?1)))",
            params![id, ended.unix],
            |r| r.get(0),
        )?;
        if later > 0 {
            return Err(StoreError::Superseded(id.into()));
        }

        // The learner side as `record_on` made it, from the evidence with the marks.
        let stats = s.stats();
        let success = stats.success();
        let mut l = recording.learning.clone();
        let (change, scenario_done) = l.book(s, success);
        retrack(&mut l, ended);
        self.set_learning(&l)?;

        // The review items: what this talk added stays only if the summary still has it.
        let summary = s.summary();
        let review: Vec<String> = summary.review.iter().map(|r| kaiwa_core::norm(r)).collect();
        let (keep, drop): (Vec<String>, Vec<String>) = recording
            .added
            .into_iter()
            .partition(|key| review.contains(key));
        for key in &drop {
            self.db
                .execute("DELETE FROM items WHERE key = ?1", params![key])?;
        }
        // A card that stays is built from the turns that stay, as if the marked turn had
        // been marked before the end.
        let corrections = corrected_of(&summary);
        for key in &keep {
            let Some(mut item) = self.item(key)? else {
                continue;
            };
            let found = corrections
                .iter()
                .find(|c| kaiwa_core::norm(&c.better) == *key);
            (item.kind, item.source) = srs::card_kind(&item.text, found);
            item.cue = found.and_then(|c| c.cue.clone());
            item.original = found.and_then(|c| c.original.clone());
            self.put_item(&item)?;
        }
        recording.added = keep;

        self.db.execute(
            "UPDATE sessions SET success = ?2, state = ?3, recording = ?4 WHERE id = ?1",
            params![
                id,
                success,
                serde_json::to_string(&s.snapshot())?,
                serde_json::to_string(&recording)?
            ],
        )?;
        Ok(Recorded {
            success,
            change,
            scenario_done,
            new_items: recording.added.len(),
            xp: row.xp,
            day: self.day(recording.day)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use kaiwa_core::session::Input;

    #[test]
    fn time_spoken_counts_when_the_analysis_had_gaps() {
        // jonas_analysis_gaps (T-11): such a turn is no evidence of the level, but the time
        // spoken is practice and counts for the day.
        let store = Store::in_memory().unwrap();
        let mut talk = Session::new(scenario(), provider(), Amount::Little, Lang::De);
        let start = at(0, 9);
        let id = store.begin_local(&talk.snapshot(), start).unwrap();
        talk.turn("マラです。よろしくおねがいします。", Input::Spoken)
            .unwrap();
        let mut state = talk.snapshot();
        state.turns[0].incomplete = true;
        let talk = Session::restore(scenario(), provider(), state);

        let r = store
            .record(&id, &talk, start, start.plus_secs(60), None)
            .unwrap();
        assert_eq!(r.day.speaking_secs, 60, "one minute, the one turn spoken");
    }

    #[test]
    fn recording_a_session_feeds_model_path_review_and_day() {
        let store = Store::in_memory().unwrap();
        let mut p = store.profile().unwrap();
        p.daily_goal = DailyGoal::Minutes(4);
        store.set_profile(&p).unwrap();
        let c = curriculum();
        let mut session = Session::new(scenario(), provider(), Amount::Little, Lang::De);
        let start = at(0, 9);
        let id = store.begin_local(&session.snapshot(), start).unwrap();
        for (t, i) in [
            ("マラです。よろしくおねがいします。", Input::Spoken),
            ("ドイツのハンブルクから来ました。", Input::Spoken),
            ("仕事はプロデューサーです。", Input::Typed),
            ("コーヒーを好きです。", Input::Spoken),
        ] {
            session.turn(t, i).unwrap();
        }
        let r = store
            .record(&id, &session, start, start.plus_secs(6 * 60), Some(&c))
            .unwrap();
        assert!(r.new_items > 0);
        assert_eq!(r.day.typed_turns, 1);
        assert_eq!(
            r.day.speaking_secs, 270,
            "six minutes, three of four turns spoken"
        );
        assert!(r.day.met(DailyGoal::Minutes(4)));
        let items = store.items().unwrap();
        let liked = items
            .iter()
            .find(|i| i.text == "コーヒーが好きです")
            .unwrap();
        assert_eq!(liked.unit.as_deref(), Some("n5-02"));
        assert!(matches!(liked.source, srs::Source::Correction { .. }));
        let l = store.learning().unwrap();
        assert_eq!(l.regulator.success.len(), 1);
        assert_eq!(l.course.completions.get("n5-02-jikoshoukai"), Some(&1));
        let rows = store.sessions(10).unwrap();
        assert!(rows[0].finished && rows[0].xp > 0);
        assert!(store.open_session().unwrap().is_none());
        // The day keeps where it started from, for the week's progress (/LF604/): the levels
        // before the first talk, not those after it, and a second talk leaves them.
        let before = kaiwa_core::LearnerModel::default();
        let kept = &r.day.levels_at_start;
        assert_eq!(kept.len(), kaiwa_core::learner::SKILLS.len());
        assert!(kept.iter().all(|(s, l)| *l == before.level(*s)), "{kept:?}");
        assert!(
            kaiwa_core::learner::SKILLS
                .iter()
                .any(|s| l.model.level(*s) != before.level(*s)),
            "the talk moved the model"
        );
        let second = Session::new(scenario(), provider(), Amount::Little, Lang::De);
        let id2 = store.begin_local(&second.snapshot(), at(0, 20)).unwrap();
        let r2 = store
            .record(&id2, &second, at(0, 20), at(0, 20).plus_secs(60), Some(&c))
            .unwrap();
        assert_eq!(&r2.day.levels_at_start, kept);
        // A new placement is no progress: the days forget where they started from.
        store
            .set_placement(kaiwa_core::LearnerModel::placed(&[(
                kaiwa_core::learner::Skill::Speaking,
                "n4",
            )]))
            .unwrap();
        assert!(
            store
                .day(at(0, 9).day())
                .unwrap()
                .levels_at_start
                .is_empty()
        );
        // A talk of two sittings, the second after midnight: all its time, on the second day.
        let third = Session::new(scenario(), provider(), Amount::Little, Lang::De);
        let id3 = store.begin_local(&third.snapshot(), at(1, 23)).unwrap();
        let end = at(2, 0).plus_secs(8 * 60);
        let r3 = store
            .record_on(
                &id3,
                &third,
                end.plus_secs(-15 * 60),
                end,
                end.day(),
                Some(&c),
            )
            .unwrap();
        assert_eq!(store.day(at(2, 0).day()).unwrap(), r3.day);
        assert_eq!(
            store.day(at(1, 23).day()).unwrap(),
            Default::default(),
            "nothing on the day before"
        );
        // The next day the item is due and can be reviewed.
        let due = store.due(at(1, 9), 10).unwrap();
        assert!(due.iter().any(|i| i.text == "コーヒーが好きです"));
        let reviewed = store
            .review("コーヒーが好きです", Grade::Good, at(1, 9))
            .unwrap()
            .unwrap();
        assert!(reviewed.due > at(1, 9).day());
        assert_eq!(store.day(at(1, 9).day()).unwrap().reviews_done, 1);
        assert_eq!(store.learning().unwrap().course.reviews["n5-02"], [true]);
        let today = store.today(at(1, 10)).unwrap();
        assert_eq!(today.streak.days, 1, "yesterday met, today not over yet");
        // A higher goal chosen now does not undo yesterday (/LF602/): the day keeps its goal.
        let mut p = store.profile().unwrap();
        p.daily_goal = DailyGoal::Minutes(10);
        store.set_profile(&p).unwrap();
        assert_eq!(store.today(at(1, 10)).unwrap().streak.days, 1);
    }

    #[test]
    fn a_goal_changed_during_the_day_is_the_days_goal() {
        let store = Store::in_memory().unwrap();
        let c = curriculum();
        let goal = |m: u32| {
            let mut p = store.profile().unwrap();
            p.daily_goal = DailyGoal::Minutes(m);
            store.set_profile(&p).unwrap();
            // As the settings do it: the day begun takes the new goal over.
            store
                .set_day_goal(at(1, 12).day(), DailyGoal::Minutes(m))
                .unwrap();
        };
        let talk = |day: i64, minutes: i64| {
            let mut s = Session::new(scenario(), provider(), Amount::Little, Lang::De);
            s.turn("マラです。よろしくおねがいします。", Input::Spoken)
                .unwrap();
            let start = at(day, 9);
            let id = store.begin_local(&s.snapshot(), start).unwrap();
            store
                .record(&id, &s, start, start.plus_secs(minutes * 60), Some(&c))
                .unwrap();
        };
        // Goal ten minutes, met yesterday. Today six minutes, then the goal is lowered to five.
        let mut p = store.profile().unwrap();
        p.daily_goal = DailyGoal::Minutes(10);
        store.set_profile(&p).unwrap();
        talk(0, 11);
        talk(1, 6);
        goal(5);
        let today = store.today(at(1, 20)).unwrap();
        assert!(today.streak.today_met);
        assert_eq!(today.streak.days, 2);
        // The next morning the day still counts, and no freeze was needed for it.
        let next = store.today(at(2, 8)).unwrap();
        assert_eq!((next.streak.days, next.streak.freezes_used.len()), (2, 0));
        // The other way round: raised to twenty during the day, the day is missed tomorrow too.
        goal(20);
        assert!(!store.today(at(1, 20)).unwrap().streak.today_met);
        let next = store.today(at(2, 8)).unwrap();
        assert_eq!(next.streak.freezes_used, vec![at(1, 9).day()]);
        // Yesterday keeps the goal it had.
        assert_eq!(
            store.day(at(0, 9).day()).unwrap().goal,
            Some(DailyGoal::Minutes(10))
        );
        // Also when a talk begun before midnight ends after it and the goal was changed in
        // between: the talk counts for the day it began, the goal for today.
        goal(10);
        talk(2, 11);
        let mut p = store.profile().unwrap();
        p.daily_goal = DailyGoal::Minutes(30);
        store.set_profile(&p).unwrap();
        store
            .set_day_goal(at(3, 0).day(), DailyGoal::Minutes(30))
            .unwrap();
        let late = Session::new(scenario(), provider(), Amount::Little, Lang::De);
        let id = store.begin_local(&late.snapshot(), at(2, 23)).unwrap();
        let end = at(3, 0).plus_secs(180);
        store
            .record_on(&id, &late, at(2, 23), end, at(2, 23).day(), Some(&c))
            .unwrap();
        assert_eq!(
            store.day(at(2, 23).day()).unwrap().goal,
            Some(DailyGoal::Minutes(10)),
            "the day it began keeps its goal"
        );
        // After a flight west the day begun lies ahead of "today": it takes the new goal too.
        store
            .set_day_goal(at(0, 9).day(), DailyGoal::Minutes(7))
            .unwrap();
        assert_eq!(
            store.day(at(1, 9).day()).unwrap().goal,
            Some(DailyGoal::Minutes(7))
        );
    }

    #[test]
    fn every_shown_correction_comes_back_as_a_mini_conversation() {
        // Four different corrections with much feedback: the fourth is past the summary's top three
        // and still goes into the review as a correction, with its line and old words (/LF230/).
        let store = Store::in_memory().unwrap();
        let mut session = Session::new(scenario(), provider(), Amount::Much, Lang::De);
        let start = at(0, 9);
        let id = store.begin_local(&session.snapshot(), start).unwrap();
        for t in [
            "コーヒーを好きです。",
            "わたしはマラです。わたしはプロデューサーです。",
            "ビール二ください。",
            "きのう東京に行きます。",
        ] {
            session.turn(t, Input::Spoken).unwrap();
        }
        assert!(session.summary().corrected.len() >= 4);
        store
            .record(
                &id,
                &session,
                start,
                start.plus_secs(300),
                Some(&curriculum()),
            )
            .unwrap();
        for (better, old) in [
            ("コーヒーが好きです", "コーヒーを好きです"),
            (
                "マラです。プロデューサーをしています。",
                "わたしはマラですわたしはプロデューサーです",
            ),
            ("ビールを二つください", "ビール二ください"),
            ("きのう東京に行きました", "きのう東京に行きます"),
        ] {
            let item = store.item(better).unwrap().expect(better);
            assert!(
                matches!(item.source, srs::Source::Correction { .. }),
                "{better}: {:?}",
                item.source
            );
            assert!(item.cue.is_some(), "{better} has no line");
            assert_eq!(item.original.as_deref(), Some(old), "{better}");
        }
    }

    /// The talk of `recording_a_session_...`: it ends on a turn with a particle mistake.
    fn talk_ending_in_a_mistake() -> Session {
        let mut session = Session::new(scenario(), provider(), Amount::Little, Lang::De);
        for (t, i) in [
            ("マラです。よろしくおねがいします。", Input::Spoken),
            ("ドイツのハンブルクから来ました。", Input::Spoken),
            ("仕事はプロデューサーです。", Input::Typed),
            ("コーヒーを好きです。", Input::Spoken),
        ] {
            session.turn(t, i).unwrap();
        }
        assert!(session.done());
        session
    }

    #[derive(Clone, Copy)]
    enum Mark {
        Misrecognized,
        Dispute,
    }

    fn mark(session: &mut Session, how: Mark) {
        match how {
            Mark::Misrecognized => assert!(session.mark_misrecognized(3)),
            Mark::Dispute => assert!(session.dispute(3, "コーヒーが好きです").is_some()),
        }
    }

    /// V-05: marking after the end corrects what the end stored: the same profile as if the
    /// talk had been marked before it ended.
    fn marking_after_the_end(how: Mark) {
        let c = curriculum();
        let (start, end) = (at(0, 9), at(0, 9).plus_secs(6 * 60));
        let liked = |s: &Store| {
            s.items()
                .unwrap()
                .iter()
                .any(|i| i.text == "コーヒーが好きです")
        };

        let store = Store::in_memory().unwrap();
        let mut session = talk_ending_in_a_mistake();
        let id = store.begin_local(&session.snapshot(), start).unwrap();
        store.record(&id, &session, start, end, Some(&c)).unwrap();
        assert!(liked(&store));
        assert!(
            store
                .learning()
                .unwrap()
                .model
                .errors
                .contains_key("particle")
        );

        mark(&mut session, how);
        let r = store.correct(&id, &session, |_, _| {}).unwrap();
        assert!(!liked(&store), "the review item of that turn is gone");
        assert!(
            !store
                .learning()
                .unwrap()
                .model
                .errors
                .contains_key("particle"),
            "the error is no longer in the learner model"
        );

        // The same as marking before the end.
        let expected = Store::in_memory().unwrap();
        let mut marked = talk_ending_in_a_mistake();
        let i2 = expected.begin_local(&marked.snapshot(), start).unwrap();
        mark(&mut marked, how);
        let r2 = expected.record(&i2, &marked, start, end, Some(&c)).unwrap();
        assert_eq!(store.learning().unwrap(), expected.learning().unwrap());
        assert_eq!(store.items().unwrap(), expected.items().unwrap());
        assert_eq!(r.new_items, r2.new_items);
        assert_eq!(r.success, r2.success);
        assert_eq!(r.day, r2.day);
        let row = &store.sessions(1).unwrap()[0];
        assert_eq!(Some(r.success), row.success);
        assert_eq!(
            store.session_state(&id).unwrap().turns[3].misrecognized,
            matches!(how, Mark::Misrecognized),
            "the stored talk has the mark"
        );
    }

    #[test]
    fn a_turn_marked_misrecognised_after_the_end_leaves_the_record() {
        marking_after_the_end(Mark::Misrecognized);
    }

    #[test]
    fn a_turn_disputed_after_the_end_leaves_the_record() {
        marking_after_the_end(Mark::Dispute);
    }

    #[test]
    fn a_correction_keeps_the_items_the_profile_had_before_the_talk() {
        let c = curriculum();
        let store = Store::in_memory().unwrap();
        let mut earlier = Item::new(
            "コーヒーが好きです",
            srs::ItemKind::Pattern,
            srs::Source::NewWord,
            at(0, 8),
        );
        earlier.unit = Some("n5-01".into());
        store.put_item(&earlier).unwrap();

        let mut session = talk_ending_in_a_mistake();
        let id = store.begin_local(&session.snapshot(), at(0, 9)).unwrap();
        store
            .record(&id, &session, at(0, 9), at(0, 10), Some(&c))
            .unwrap();
        session.mark_misrecognized(3);
        store.correct(&id, &session, |_, _| {}).unwrap();
        assert!(
            store.items().unwrap().contains(&earlier),
            "not made by this talk, so not taken back"
        );
    }

    #[test]
    fn only_the_newest_talk_can_be_corrected() {
        let c = curriculum();
        let store = Store::in_memory().unwrap();
        let mut first = talk_ending_in_a_mistake();
        let id = store.begin_local(&first.snapshot(), at(0, 9)).unwrap();
        store
            .record(&id, &first, at(0, 9), at(0, 10), Some(&c))
            .unwrap();
        let later = Session::new(scenario(), provider(), Amount::Little, Lang::De);
        let id2 = store.begin_local(&later.snapshot(), at(1, 9)).unwrap();
        store
            .record(&id2, &later, at(1, 9), at(1, 10), Some(&c))
            .unwrap();

        first.mark_misrecognized(3);
        let learning = store.learning().unwrap();
        assert!(matches!(
            store.correct(&id, &first, |_, _| {}),
            Err(StoreError::Superseded(_))
        ));
        assert_eq!(store.learning().unwrap(), learning, "nothing changed");

        // The newest is the one recorded last, not the one begun last (/LF121/): A is begun,
        // left, B begun and finished, then A continued and finished.
        let store = Store::in_memory().unwrap();
        let mut a = talk_ending_in_a_mistake();
        let mut b = talk_ending_in_a_mistake();
        let id_a = store.begin_local(&a.snapshot(), at(0, 9)).unwrap();
        let id_b = store.begin_local(&b.snapshot(), at(0, 10)).unwrap();
        store
            .record(&id_b, &b, at(0, 10), at(0, 11), Some(&c))
            .unwrap();
        store
            .record(&id_a, &a, at(0, 9), at(0, 12), Some(&c))
            .unwrap();
        b.mark_misrecognized(3);
        assert!(matches!(
            store.correct(&id_b, &b, |_, _| {}),
            Err(StoreError::Superseded(_))
        ));
        a.mark_misrecognized(3);
        store.correct(&id_a, &a, |_, _| {}).unwrap();
    }

    /// Marking turn 0 of a talk whose turn 2 has the same correction: the card that stays is the
    /// one of turn 2 (its original, its line), as if turn 0 had been marked before the end.
    #[test]
    fn a_card_that_stays_comes_from_the_turns_that_stay() {
        let c = curriculum();
        let (start, end) = (at(0, 9), at(0, 9).plus_secs(6 * 60));
        let talk = || {
            let mut s = Session::new(scenario(), provider(), Amount::Little, Lang::De);
            for t in [
                "コーヒーを好きです。",
                "マラです。よろしくおねがいします。",
                "コーヒーを好きです。",
                "ドイツのハンブルクから来ました。",
                "仕事はプロデューサーです。",
            ] {
                s.turn(t, Input::Spoken).unwrap();
            }
            s
        };

        let store = Store::in_memory().unwrap();
        let mut session = talk();
        let id = store.begin_local(&session.snapshot(), start).unwrap();
        store.record(&id, &session, start, end, Some(&c)).unwrap();
        session.mark_misrecognized(0);
        store.correct(&id, &session, |_, _| {}).unwrap();

        let expected = Store::in_memory().unwrap();
        let mut marked = talk();
        let i2 = expected.begin_local(&marked.snapshot(), start).unwrap();
        marked.mark_misrecognized(0);
        expected.record(&i2, &marked, start, end, Some(&c)).unwrap();
        let liked = |s: &Store| {
            s.item("コーヒーが好きです")
                .unwrap()
                .expect("the card stays")
        };
        assert_eq!(liked(&store), liked(&expected));
        assert_ne!(liked(&expected).cue, None, "a line to answer");
    }
}
