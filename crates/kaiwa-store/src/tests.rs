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
    assert!(
        !p.mora_hints,
        "mora hints are a preview until K-02 holds in the app"
    );
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
    assert!(!p.mora_hints);
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
