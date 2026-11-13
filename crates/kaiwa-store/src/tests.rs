use std::sync::Arc;

use kaiwa_ai::ChatProvider;
use kaiwa_ai::mock::MockProvider;
use kaiwa_core::Scenario;
use kaiwa_core::session::Input;

use super::*;

fn scenario() -> Scenario {
    Scenario::from_json(include_str!(
        "../../../data/scenarios/n5-02-jikoshoukai.json"
    ))
    .unwrap()
}

fn curriculum() -> Curriculum {
    Curriculum::from_json(include_str!("../../../data/curriculum/n5.json")).unwrap()
}

fn provider() -> Arc<dyn ChatProvider> {
    Arc::new(
        MockProvider::from_json(include_str!("../../../qa/fixtures/n5-02-jikoshoukai.json"))
            .unwrap()
            .with_fallback("すみません、もう一度お願いします。"),
    )
}

fn at(day: i64, hour: i64) -> Moment {
    Day::from_ymd(2026, 10, 5)
        .plus(day)
        .start(120)
        .plus_secs(hour * 3600)
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kaiwa-store-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("test.kaiwa")
}

#[test]
fn a_new_profile_has_defaults_and_the_schema() {
    let s = Store::in_memory().unwrap();
    assert_eq!(s.schema_version().unwrap(), MIGRATIONS.len());
    let p = s.profile().unwrap();
    assert_eq!(p.daily_goal, DailyGoal::Minutes(5));
    assert_eq!(p.feedback, Amount::Little);
    assert_eq!(p.keep_recordings_days, None, "no recordings without opt-in");
    assert!(p.mora_hints, "mora hints are on by default (L-01)");
    let mut p = p;
    p.name = "Mara".into();
    p.gamification = false;
    s.set_profile(&p).unwrap();
    assert_eq!(s.profile().unwrap(), p);
}

#[test]
fn a_profile_from_before_the_mora_hints_reads() {
    let mut old = serde_json::to_value(Profile::default()).unwrap();
    old.as_object_mut().unwrap().remove("mora_hints");
    let p: Profile = serde_json::from_value(old).unwrap();
    assert!(p.mora_hints);
}

#[test]
fn a_placement_sets_the_feedback_until_the_learner_chose() {
    let n4 = || kaiwa_core::LearnerModel::placed(&[(kaiwa_core::learner::Skill::Speaking, "n4")]);
    let n3 = || kaiwa_core::LearnerModel::placed(&[(kaiwa_core::learner::Skill::Speaking, "n3")]);

    // Not chosen: the level sets it, again on a new placement.
    let s = Store::in_memory().unwrap();
    s.set_placement(n4()).unwrap();
    let p = s.profile().unwrap();
    assert_eq!(p.feedback, Amount::Normal);
    assert_eq!(p.feedback_source, FeedbackSource::Level);
    s.set_placement(n3()).unwrap();
    assert_eq!(s.profile().unwrap().feedback, Amount::Much);

    // Chosen: it stays, even when the choice is the old default.
    let s = Store::in_memory().unwrap();
    s.set_feedback(Amount::Little).unwrap();
    assert_eq!(s.profile().unwrap().feedback_source, FeedbackSource::Chosen);
    s.set_placement(n4()).unwrap();
    assert_eq!(s.profile().unwrap().feedback, Amount::Little);
}

/// A profile row as an older version wrote it: no `feedback_source`.
fn old_profile(amount: &str, placed: bool) -> serde_json::Value {
    let mut old = serde_json::to_value(Profile::default()).unwrap();
    let o = old.as_object_mut().unwrap();
    o.remove("feedback_source");
    o.insert("feedback".into(), amount.into());
    o.insert("placed".into(), placed.into());
    old
}

fn with_row(profile: &serde_json::Value, speaking: &str) -> Store {
    let s = Store::in_memory().unwrap();
    s.db.execute(
        "INSERT INTO settings (key, value) VALUES ('profile', ?1)",
        [profile.to_string()],
    )
    .unwrap();
    let mut l = s.learning().unwrap();
    l.model = kaiwa_core::LearnerModel::placed(&[(kaiwa_core::learner::Skill::Speaking, speaking)]);
    s.set_learning(&l).unwrap();
    s
}

#[test]
fn a_profile_from_before_the_feedback_source_reads() {
    // A pick is a pick.
    for amount in ["normal", "much"] {
        let p = with_row(&old_profile(amount, true), "n3")
            .profile()
            .unwrap();
        assert_eq!(p.feedback_source, FeedbackSource::Chosen, "{amount}");
    }
    // The old default after a placement: the level would have set it.
    let p = with_row(&old_profile("little", true), "n3")
        .profile()
        .unwrap();
    assert_eq!(
        (p.feedback, p.feedback_source),
        (Amount::Much, FeedbackSource::Level)
    );
    // The old default without a placement: nobody set it.
    let p = with_row(&old_profile("little", false), "n3")
        .profile()
        .unwrap();
    assert_eq!(
        (p.feedback, p.feedback_source),
        (Amount::Little, FeedbackSource::Default)
    );
}

#[test]
fn an_old_export_imports_with_the_same_rule() {
    let import = |amount: &str| {
        let src = with_row(&old_profile(amount, true), "n3");
        let mut export = src.export(at(1, 9)).unwrap();
        export["profile"] = old_profile(amount, true);
        let dst = Store::in_memory().unwrap();
        dst.import(&export).unwrap();
        dst.profile().unwrap()
    };
    assert_eq!(import("much").feedback_source, FeedbackSource::Chosen);
    let p = import("little");
    assert_eq!(
        (p.feedback, p.feedback_source),
        (Amount::Much, FeedbackSource::Level)
    );
}

#[test]
fn a_skipped_placement_leaves_the_feedback_default() {
    let s = Store::in_memory().unwrap();
    s.update_profile(|p| p.placed = true).unwrap();
    let p = s.profile().unwrap();
    assert_eq!(
        (p.feedback, p.feedback_source),
        (Amount::Little, FeedbackSource::Default)
    );
}

#[test]
fn an_interrupted_session_continues_where_it_stopped() {
    let path = temp("resume");
    {
        let store = Store::open(&path).unwrap();
        let mut session = Session::new(scenario(), provider(), Amount::Little, "de");
        let id = store.begin_session(&session.snapshot(), at(0, 9)).unwrap();
        assert!(store.open_session().unwrap().is_none(), "nothing said yet");
        session
            .turn("マラです。よろしくおねがいします。", Input::Spoken)
            .unwrap();
        store.save_session(id, &session.snapshot()).unwrap();
        // The plasmoid closes here.
    }
    let store = Store::open(&path).unwrap();
    let open = store.open_session().unwrap().unwrap();
    let state = store.session_state(open.id).unwrap();
    let session = Session::restore(scenario(), provider(), state);
    assert_eq!(
        session.current_goal().unwrap().id,
        "origin",
        "not from the start"
    );
    delete_profile(&path).unwrap();
    std::fs::remove_dir(path.parent().unwrap()).unwrap();
}

#[test]
fn recording_a_session_feeds_model_path_review_and_day() {
    let store = Store::in_memory().unwrap();
    let mut p = store.profile().unwrap();
    p.daily_goal = DailyGoal::Minutes(4);
    store.set_profile(&p).unwrap();
    let c = curriculum();
    let mut session = Session::new(scenario(), provider(), Amount::Little, "de");
    let start = at(0, 9);
    let id = store.begin_session(&session.snapshot(), start).unwrap();
    for (t, i) in [
        ("マラです。よろしくおねがいします。", Input::Spoken),
        ("ドイツのハンブルクから来ました。", Input::Spoken),
        ("仕事はプロデューサーです。", Input::Typed),
        ("コーヒーを好きです。", Input::Spoken),
    ] {
        session.turn(t, i).unwrap();
    }
    let r = store
        .record(id, &session, start, start.plus_secs(6 * 60), Some(&c))
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
    let second = Session::new(scenario(), provider(), Amount::Little, "de");
    let id2 = store.begin_session(&second.snapshot(), at(0, 20)).unwrap();
    let r2 = store
        .record(id2, &second, at(0, 20), at(0, 20).plus_secs(60), Some(&c))
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
    let third = Session::new(scenario(), provider(), Amount::Little, "de");
    let id3 = store.begin_session(&third.snapshot(), at(1, 23)).unwrap();
    let end = at(2, 0).plus_secs(8 * 60);
    let r3 = store
        .record_on(
            id3,
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
fn export_and_import_carry_everything() {
    let a = Store::in_memory().unwrap();
    let p = Profile {
        name: "Lena".into(),
        ..Default::default()
    };
    a.set_profile(&p).unwrap();
    let mut session = Session::new(scenario(), provider(), Amount::Little, "de");
    let id = a.begin_session(&session.snapshot(), at(0, 9)).unwrap();
    session.turn("コーヒーを好きです。", Input::Spoken).unwrap();
    a.record(
        id,
        &session,
        at(0, 9),
        at(0, 9).plus_secs(300),
        Some(&curriculum()),
    )
    .unwrap();
    a.add_pause(Pause {
        from: at(3, 0).day(),
        to: at(5, 0).day(),
    })
    .unwrap();
    a.add_dispute(&Dispute {
        at: at(0, 9),
        scenario: "n5-02-jikoshoukai".into(),
        learner: "コーヒーを好きです。".into(),
        original: "コーヒーを".into(),
        better: "コーヒーが".into(),
        explanation: "…".into(),
    })
    .unwrap();
    let json = a.export(at(1, 9)).unwrap();
    assert_eq!(json["schema"], EXPORT_SCHEMA);
    let text = serde_json::to_string(&json).unwrap();
    let b = Store::in_memory().unwrap();
    b.import(&serde_json::from_str(&text).unwrap()).unwrap();
    assert_eq!(b.export(at(1, 9)).unwrap(), json);
    assert!(matches!(b.import(&json), Err(StoreError::NotEmpty)));
    let mut wrong = json.clone();
    wrong["schema"] = "other/1".into();
    assert!(matches!(
        Store::in_memory().unwrap().import(&wrong),
        Err(StoreError::WrongFormat(_))
    ));
}

#[test]
fn import_keeps_that_the_learner_reviewed() {
    let a = Store::in_memory().unwrap();
    let mut session = Session::new(scenario(), provider(), Amount::Little, "de");
    let id = a.begin_session(&session.snapshot(), at(0, 9)).unwrap();
    session.turn("コーヒーを好きです。", Input::Spoken).unwrap();
    a.record(id, &session, at(0, 9), at(0, 9).plus_secs(300), None)
        .unwrap();
    words_before_a_break(&a, at(1, 9));
    for day in 2..=9 {
        a.review("ことば0", Grade::Good, at(day, 9)).unwrap();
    }

    // Reviewing daily is no break: the import must not start a refresher.
    let b = Store::in_memory().unwrap();
    b.import(&a.export(at(9, 10)).unwrap()).unwrap();
    assert_eq!(b.last_practice().unwrap(), a.last_practice().unwrap());
    assert!(!b.today(at(9, 11)).unwrap().refresher);
}

#[test]
fn import_rejects_a_file_missing_an_array() {
    let a = Store::in_memory().unwrap();
    // A profile unlike the default, so a profile left behind by the import would show.
    let mut p = a.profile().unwrap();
    p.name = "Lena".into();
    a.set_profile(&p).unwrap();
    words_before_a_break(&a, at(1, 9));
    let json = a.export(at(2, 9)).unwrap();

    for key in ["sessions", "items", "days", "pauses", "disputes"] {
        let mut part = json.clone();
        part.as_object_mut().unwrap().remove(key);
        let b = Store::in_memory().unwrap();
        assert!(b.import(&part).is_err(), "{key} missing was accepted");
        // Rolled back: nothing of the file stays.
        assert!(b.items().unwrap().is_empty(), "{key}: items stayed");
        assert_eq!(b.profile().unwrap(), Profile::default(), "{key}: profile");
        assert_eq!(b.last_practice().unwrap(), None, "{key}: practice");
    }
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
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        s.turn("マラです。よろしくおねがいします。", Input::Spoken)
            .unwrap();
        let start = at(day, 9);
        let id = store.begin_session(&s.snapshot(), start).unwrap();
        store
            .record(id, &s, start, start.plus_secs(minutes * 60), Some(&c))
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
    let late = Session::new(scenario(), provider(), Amount::Little, "de");
    let id = store.begin_session(&late.snapshot(), at(2, 23)).unwrap();
    let end = at(3, 0).plus_secs(180);
    store
        .record_on(id, &late, at(2, 23), end, at(2, 23).day(), Some(&c))
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
fn deleting_a_profile_leaves_no_file() {
    let path = temp("delete");
    {
        let s = Store::open(&path).unwrap();
        s.set_profile(&Profile::default()).unwrap();
    }
    let dir = path.parent().unwrap().to_path_buf();
    assert_eq!(profiles_in(&dir), vec![path.clone()]);
    delete_profile(&path).unwrap();
    assert!(
        std::fs::read_dir(&dir).unwrap().next().is_none(),
        "no database, no side files"
    );
    std::fs::remove_dir(&dir).unwrap();
}

#[test]
fn migration_2_moves_old_unit_ids_to_path_order() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch(super::MIGRATIONS[0]).unwrap();
    db.pragma_update(None, "user_version", 1).unwrap();
    db.execute(
        "INSERT INTO learning VALUES (1, ?1)",
        [r#"{"course":{"scenarios_done":["n5-13-jugyou","n5-03-konbini"],"skipped_units":["n5-12"],"reviews":{"n4-11":[3,2]}}}"#],
    )
    .unwrap();
    db.execute(
        "INSERT INTO sessions (scenario, started, offset_min, state) VALUES ('n5-16-onsen', 0, 0, '{\"scenario\":\"n5-16-onsen\"}')",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO items VALUES ('駅', 0, '{\"text\":\"駅\",\"unit\":\"n5-10\"}')",
        [],
    )
    .unwrap();
    let store = Store::init(db).unwrap();
    assert_eq!(store.schema_version().unwrap(), super::MIGRATIONS.len());
    let l: String = store
        .db
        .query_row("SELECT value FROM learning", [], |r| r.get(0))
        .unwrap();
    assert!(
        l.contains("\"n5-03-jugyou\"") && l.contains("\"n5-04-konbini\""),
        "{l}"
    );
    assert!(l.contains("\"n5-16\"") && l.contains("\"n4-13\""), "{l}");
    let (sc, state): (String, String) = store
        .db
        .query_row("SELECT scenario, state FROM sessions", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(sc, "n5-08-onsen");
    assert!(state.contains("n5-08-onsen"));
    let item: String = store
        .db
        .query_row("SELECT value FROM items", [], |r| r.get(0))
        .unwrap();
    assert!(item.contains("\"n5-13\""), "{item}");
    assert_eq!(
        super::renamed("2026-10-13 n5-131 an5-13"),
        "2026-10-13 n5-131 an5-13"
    );
}

#[test]
fn every_shown_correction_comes_back_as_a_mini_conversation() {
    // Four different corrections with much feedback: the fourth is past the summary's top three
    // and still goes into the review as a correction, with its line and old words (/LF230/).
    let store = Store::in_memory().unwrap();
    let mut session = Session::new(scenario(), provider(), Amount::Much, "de");
    let start = at(0, 9);
    let id = store.begin_session(&session.snapshot(), start).unwrap();
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
            id,
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

#[test]
fn migration_3_names_the_tutor_by_its_preset() {
    let stored = |provider: &str, base: &str| {
        let mut v = serde_json::to_value(Profile::default()).unwrap();
        v["name"] = "Mara".into();
        v["tutor"] =
            serde_json::json!({ "provider": provider, "model": "qwen3:14b", "base_url": base });
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch(super::MIGRATIONS[0]).unwrap();
        db.pragma_update(None, "user_version", 2).unwrap();
        db.execute(
            "INSERT INTO settings VALUES ('profile', ?1)",
            [v.to_string()],
        )
        .unwrap();
        Store::init(db).unwrap().profile().unwrap()
    };
    for (provider, base, after) in [
        ("openai", "", "ollama"),
        ("openai", "http://gpu.lan:8080/v1", "custom"),
        ("anthropic", "", "anthropic"),
    ] {
        let p = stored(provider, base);
        assert_eq!(p.tutor.provider, after);
        assert_eq!(p.tutor.model, "qwen3:14b");
        assert_eq!(p.tutor.base_url, base);
        assert_eq!(p.name, "Mara");
    }
}

#[test]
fn an_export_of_format_1_keeps_a_local_tutor_local() {
    let a = Store::in_memory().unwrap();
    let mut p = a.profile().unwrap();
    p.tutor.provider = "ollama".into();
    a.set_profile(&p).unwrap();
    let mut old = a.export(at(1, 9)).unwrap();
    old["schema"] = "kaiwa-export/1".into();
    for (base, after) in [("", "ollama"), ("http://gpu.lan:8080/v1", "custom")] {
        old["profile"]["tutor"] =
            serde_json::json!({ "provider": "openai", "model": "qwen3:14b", "base_url": base });
        let b = Store::in_memory().unwrap();
        b.import(&old).unwrap();
        assert_eq!(b.profile().unwrap().tutor.provider, after);
    }
    // An export from before the tutor setting has no tutor and still loads.
    old["profile"].as_object_mut().unwrap().remove("tutor");
    let b = Store::in_memory().unwrap();
    b.import(&old).unwrap();
    assert_eq!(b.profile().unwrap().tutor, kaiwa_ai::Tutor::default());
    old["schema"] = "kaiwa-export/0".into();
    assert!(Store::in_memory().unwrap().import(&old).is_err());
}

/// Thirty-four words met on day 0, the first answered on `last`.
fn words_before_a_break(store: &Store, last: Moment) {
    for n in 0..34 {
        let item = Item::new(
            format!("ことば{n}"),
            srs::ItemKind::Word,
            srs::Source::NewWord,
            at(0, 9),
        );
        store.put_item(&item).unwrap();
    }
    store.review("ことば0", Grade::Again, last).unwrap();
}

#[test]
fn after_a_break_ten_are_ten_and_the_rest_waits_for_the_next_days() {
    let store = Store::in_memory().unwrap();
    // Reviewing counts as practice (T-07): no conversation was ever held.
    words_before_a_break(&store, at(2, 9));
    assert_eq!(store.last_practice().unwrap(), Some(at(2, 9)));
    assert!(!store.today(at(5, 9)).unwrap().refresher);
    assert_eq!(store.today(at(5, 9)).unwrap().due, 34);

    let back = at(20, 9);
    let today = store.today(back).unwrap();
    assert!(today.refresher);
    assert_eq!(today.due, 10);
    // Each answer takes one off; none moves up.
    let first = store.due(back, 10).unwrap();
    store.review(&first[0].text, Grade::Good, back).unwrap();
    store.review(&first[1].text, Grade::Good, back).unwrap();
    let today = store.today(back.plus_secs(3600)).unwrap();
    assert_eq!(today.due, 8);
    assert!(today.refresher, "the whole day of the re-entry");
    // The next days bring ten each, not the pile.
    let next = store.today(at(21, 9)).unwrap();
    assert!(!next.refresher);
    assert_eq!(next.due, 8 + 10);
    assert_eq!(store.today(at(22, 9)).unwrap().due, 8 + 20);
}

#[test]
fn opening_the_app_in_a_break_does_not_use_the_break_up() {
    let store = Store::in_memory().unwrap();
    words_before_a_break(&store, at(2, 9));
    // Opened on day 10, nothing practised; the days after are no re-entry.
    assert_eq!(store.today(at(10, 9)).unwrap().due, 10);
    assert!(!store.today(at(16, 9)).unwrap().refresher);
    // Three weeks later the learner is back: welcome, ten, not the heap.
    let back = store.today(at(30, 9)).unwrap();
    assert!(back.refresher);
    assert_eq!(back.due, 10);
    assert_eq!(store.today(at(31, 9)).unwrap().due, 20);
}

#[test]
fn a_break_is_spread_once_also_when_nothing_is_practised_on_the_day_back() {
    let store = Store::in_memory().unwrap();
    words_before_a_break(&store, at(2, 9));
    assert_eq!(store.today(at(20, 9)).unwrap().due, 10);
    // Nothing answered: the day after is no second re-entry, the ten and the next ten wait.
    let next = store.today(at(21, 9)).unwrap();
    assert!(!next.refresher);
    assert_eq!(next.due, 20);
    // Practice, then another break: spread again.
    let first = store.due(at(21, 9), 1).unwrap();
    store
        .review(&first[0].text, Grade::Good, at(21, 9))
        .unwrap();
    let again = store.today(at(40, 9)).unwrap();
    assert!(again.refresher);
    assert_eq!(again.due, 10);
}

#[test]
fn a_running_pause_can_be_ended() {
    let store = Store::in_memory().unwrap();
    let today = at(10, 9).day();
    assert_eq!(store.paused_until(today).unwrap(), None);
    // One that began three days ago and one set today on top of it; one long over.
    for (from, to) in [(-3, 3), (0, 6), (-20, -15)] {
        store
            .add_pause(Pause {
                from: today.plus(from),
                to: today.plus(to),
            })
            .unwrap();
    }
    assert_eq!(store.paused_until(today).unwrap(), Some(today.plus(6)));
    store.end_pause(today).unwrap();
    assert_eq!(store.paused_until(today).unwrap(), None);
    // The days that were paused stay paused; today counts again.
    let mut left = store.pauses().unwrap();
    left.sort_by_key(|p| p.from);
    assert_eq!(
        left,
        vec![
            Pause {
                from: today.plus(-20),
                to: today.plus(-15)
            },
            Pause {
                from: today.plus(-3),
                to: today.plus(-1)
            },
        ]
    );
    assert_eq!(
        store.paused_until(today.plus(-2)).unwrap(),
        Some(today.plus(-1))
    );
    // Nothing running: ending changes nothing.
    store.end_pause(today).unwrap();
    assert_eq!(store.pauses().unwrap().len(), 2);
}

#[test]
fn a_new_placement_keeps_the_history() {
    use kaiwa_core::learner::{ErrorStat, Skill};
    let store = Store::in_memory().unwrap();

    // Mistakes, the learner's own corrections and pronunciation from talks so far.
    let mut l = store.learning().unwrap();
    l.model.errors.insert(
        "particle".into(),
        ErrorStat {
            corrected: 7,
            rate: 0.4,
        },
    );
    l.model.corrections_by_learner = 3;
    l.model.set(Skill::Pronunciation, 3.5);
    store.set_learning(&l).unwrap();

    // Placing again sets the levels it measured, nothing else (/LF200/, /LF202/).
    let placed = kaiwa_core::LearnerModel::placed(&[(Skill::Speaking, "n4")]);
    store.set_placement(placed.clone()).unwrap();
    let m = store.learning().unwrap().model;
    assert_eq!(m.level(Skill::Speaking), placed.level(Skill::Speaking));
    assert_eq!(m.errors["particle"].corrected, 7);
    assert_eq!(m.corrections_by_learner, 3);
    assert_eq!(m.level(Skill::Pronunciation), 3.5);
}

/// A profile row that does not parse (a bad import, a hand edit).
fn break_profile(s: &Store) {
    s.db.execute(
        "INSERT INTO settings (key, value) VALUES ('profile', '{\"gamification\": 3}')
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [],
    )
    .unwrap();
}

fn profile_row(s: &Store) -> String {
    s.db.query_row(
        "SELECT value FROM settings WHERE key = 'profile'",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn an_unreadable_profile_is_not_written_over() {
    let s = Store::in_memory().unwrap();
    break_profile(&s);
    let before = profile_row(&s);

    let r = s.update_profile(|p| p.name = "Mara".into());

    assert!(r.is_err(), "the read error comes back");
    assert_eq!(profile_row(&s), before, "the row stays as it was");
}

#[test]
fn update_profile_changes_a_readable_profile() {
    let s = Store::in_memory().unwrap();
    s.update_profile(|p| p.name = "Mara".into()).unwrap();
    assert_eq!(s.profile().unwrap().name, "Mara");
}

#[test]
fn a_failing_review_leaves_nothing_behind() {
    let s = Store::in_memory().unwrap();
    let text = "コーヒーが好きです";
    let item = Item::new(text, srs::ItemKind::Word, srs::Source::NewWord, at(0, 9));
    s.put_item(&item).unwrap();
    // The profile is read last: the writes before it must not stay.
    break_profile(&s);

    let r = s.review(text, Grade::Good, at(1, 9));

    assert!(r.is_err());
    assert_eq!(s.item(text).unwrap().unwrap(), item, "item unchanged");
    assert!(s.state_value::<Moment>("reviewed").unwrap().is_none());
}

#[test]
fn export_carries_the_costs_per_month() {
    let a = Store::in_memory().unwrap();
    a.set_state_value("cost.2026-10", &month(12.5, 3)).unwrap();
    a.set_state_value("cost.2026-11", &month(60.0, 8)).unwrap();
    a.set_state_value("reviewed", &at(0, 9)).unwrap();
    let json = a.export(at(1, 9)).unwrap();
    assert_eq!(json["costs"]["2026-11"]["cents"], 60.0);
    assert!(json["costs"].get("reviewed").is_none(), "only the costs");

    let b = Store::in_memory().unwrap();
    b.import(&json).unwrap();
    assert_eq!(b.export(at(1, 9)).unwrap()["costs"], json["costs"]);
}

#[test]
fn an_export_of_format_2_has_no_costs() {
    let a = Store::in_memory().unwrap();
    let mut old = a.export(at(1, 9)).unwrap();
    old["schema"] = "kaiwa-export/2".into();
    old.as_object_mut().unwrap().remove("costs");
    let b = Store::in_memory().unwrap();
    b.import(&old).unwrap();
    assert_eq!(b.export(at(1, 9)).unwrap()["costs"], serde_json::json!({}));

    // From format 3 on the costs are required, like the arrays.
    let mut cut = a.export(at(1, 9)).unwrap();
    cut.as_object_mut().unwrap().remove("costs");
    assert!(Store::in_memory().unwrap().import(&cut).is_err());
}

fn month(cents: f64, calls: u32) -> serde_json::Value {
    serde_json::json!({ "cents": cents, "calls": calls, "unknown_calls": 1, "sessions": 2, "paid": false })
}

#[test]
fn import_rejects_costs_that_are_no_months() {
    let a = Store::in_memory().unwrap();
    let json = a.export(at(1, 9)).unwrap();
    let bad = [
        ("2026-10", serde_json::json!({ "cents": 60.0 })),
        ("2026-1", month(1.0, 1)),
        ("x", month(1.0, 1)),
        ("2026-13", month(1.0, 1)),
    ];
    for (key, value) in bad {
        let mut part = json.clone();
        part["costs"] = serde_json::json!({ key: value });
        let b = Store::in_memory().unwrap();
        assert!(
            matches!(b.import(&part), Err(StoreError::WrongFormat(_))),
            "{key} was accepted"
        );
        assert!(b.state_values(COST_STATE_PREFIX).unwrap().is_empty());
    }
}

#[test]
fn import_adds_to_the_costs_already_booked() {
    let a = Store::in_memory().unwrap();
    a.set_state_value("cost.2026-11", &month(60.0, 8)).unwrap();
    let json = a.export(at(1, 9)).unwrap();

    // A placement with a cloud tutor booked costs before the import.
    let b = Store::in_memory().unwrap();
    b.set_state_value("cost.2026-11", &month(5.0, 2)).unwrap();
    b.import(&json).unwrap();
    let sum: serde_json::Value = b.state_value("cost.2026-11").unwrap().unwrap();
    assert_eq!(sum["cents"], 65.0);
    assert_eq!(sum["calls"], 10);
    assert_eq!(sum["unknown_calls"], 2);
    assert_eq!(sum["sessions"], 4);
}

#[test]
fn a_second_import_is_refused() {
    let a = Store::in_memory().unwrap();
    a.set_state_value("cost.2026-11", &month(60.0, 8)).unwrap();
    let json = a.export(at(1, 9)).unwrap();

    // The file has no sessions or items, so only a marker tells the profile took an import.
    let b = Store::in_memory().unwrap();
    b.import(&json).unwrap();
    assert!(matches!(b.import(&json), Err(StoreError::NotEmpty)));
    let sum: serde_json::Value = b.state_value("cost.2026-11").unwrap().unwrap();
    assert_eq!(sum["cents"], 60.0, "counted once");
}
