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
}

#[test]
fn recording_a_session_feeds_model_path_review_and_day() {
    let store = Store::in_memory().unwrap();
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
    // The next day the item is due and can be reviewed.
    let due = store.due(at(1, 9), 10).unwrap();
    assert!(due.iter().any(|i| i.text == "コーヒーが好きです"));
    let reviewed = store
        .review("コーヒーが好きです", Grade::Good, at(1, 9))
        .unwrap()
        .unwrap();
    assert!(reviewed.due > at(1, 9).day());
    assert_eq!(store.day(at(1, 9).day()).unwrap().reviews_done, 1);
    assert_eq!(store.learning().unwrap().course.reviews["n5-02"], (1, 1));
    let mut p = store.profile().unwrap();
    p.daily_goal = DailyGoal::Minutes(4);
    store.set_profile(&p).unwrap();
    let today = store.today(at(1, 10)).unwrap();
    assert_eq!(today.streak.days, 1, "yesterday met, today not over yet");
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
