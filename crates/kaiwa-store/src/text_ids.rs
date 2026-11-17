//! Talk ids as text (ADR-010 step 2) and the journal in the export.

use kaiwa_core::session::Session;
use kaiwa_core::{Amount, Event, Moment};

use crate::Store;
use crate::tests::{at, provider, scenario, schema_version};

#[test]
fn migration_7_makes_talk_ids_text() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    for sql in &crate::MIGRATIONS[..6] {
        db.execute_batch(sql).unwrap();
    }
    db.pragma_update(None, "user_version", 6).unwrap();
    let state = serde_json::to_string(
        &Session::new(scenario(), provider(), Amount::Little, "de").snapshot(),
    )
    .unwrap();
    // One finished talk with a stored recording, one open.
    db.execute(
        "INSERT INTO sessions (id, scenario, started, offset_min, ended, finished, success, xp, speaking_secs, state, recording)
         VALUES (1, 'n5-02-jikoshoukai', 1000, 120, 1300, 1, 0.75, 40, 200, ?1, '{\"r\":1}')",
        [&state],
    )
    .unwrap();
    db.execute(
        "INSERT INTO sessions (id, scenario, started, offset_min, state) VALUES (2, 'n5-03-x', 2000, 60, ?1)",
        [&state],
    )
    .unwrap();

    let s = Store::init(db).unwrap();

    assert_eq!(schema_version(&s), crate::MIGRATIONS.len());
    let rows = s.sessions(10).unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["old-2", "old-1"], "newest first");
    let (open, done) = (&rows[0], &rows[1]);
    assert_eq!(open.scenario, "n5-03-x");
    assert_eq!(open.started, Moment::new(2000, 60));
    assert!(!open.finished && open.ended.is_none() && open.success.is_none());
    assert_eq!(done.started, Moment::new(1000, 120));
    assert_eq!(done.ended, Some(Moment::new(1300, 120)));
    assert!(done.finished);
    assert_eq!(
        (done.success, done.xp, done.speaking_secs),
        (Some(0.75), 40, 200)
    );
    assert_eq!(
        s.session_state("old-1").unwrap().scenario,
        "n5-02-jikoshoukai"
    );
    let recording: String =
        s.db.query_row(
            "SELECT recording FROM sessions WHERE id = 'old-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(recording, "{\"r\":1}");
    let index: i64 =
        s.db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'sessions_started'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(index, 1);
}

#[test]
fn an_export_carries_the_journal_and_imports_it_back() {
    let a = Store::in_memory().unwrap();
    let event = Event::Setting {
        key: "goal".into(),
        value: "20".into(),
    };
    a.append_event(&event, at(0, 9)).unwrap();
    let json = a.export(at(1, 9)).unwrap();
    assert_eq!(json["schema"], "kaiwa-export/4");
    assert_eq!(json["events"][0]["body"]["kind"], "setting");

    let b = Store::in_memory().unwrap();
    b.import(&json).unwrap();

    assert_eq!(b.events().unwrap(), a.events().unwrap());
}
