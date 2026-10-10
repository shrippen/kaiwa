//! What the app and the service `kaiwad` share (ADR-008): the profile (store), the curriculum,
//! the scenarios, the tutor and the content of every view as JSON.
//!
//! - The profile file is `$KAIWA_DATA/default.kaiwa` (see `kaiwa_store::profiles_dir`), or
//!   `KAIWA_PROFILE`.
//! - `KAIWA_DEMO=1` uses a fresh profile in a temporary directory, filled with two weeks of
//!   history, for screenshots and the demo (`KAIWA_LANG=en` for the English UI).
//! - `KAIWA_NOW=<unix>:<offset minutes>` fixes the clock (screenshots); otherwise it is the
//!   system time in the local time zone.

pub mod cost;
pub mod reminder;
pub mod short;
pub mod views;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use kaiwa_ai::ChatProvider;
use kaiwa_ai::mock::MockProvider;

use kaiwa_core::Scenario;
use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::curriculum::Curriculum;
use kaiwa_core::difficulty::{Difficulty, Regulator};
use kaiwa_core::learner::{LearnerModel, Skill};
use kaiwa_core::progress::DayRecord;
use kaiwa_core::srs::{Grade, Item, ItemKind, Source};
use kaiwa_store::{Learning, Profile, Store};

const CURRICULUM: &str = include_str!("../../../data/curriculum/n5.json");
const SCENARIOS: &[&str] = &[
    include_str!("../../../data/scenarios/n5-01-aisatsu.json"),
    include_str!("../../../data/scenarios/n5-02-jikoshoukai.json"),
    include_str!("../../../data/scenarios/n5-03-konbini.json"),
    include_str!("../../../data/scenarios/n5-04-kafe.json"),
    include_str!("../../../data/scenarios/n5-05-eki.json"),
    include_str!("../../../data/scenarios/n5-06-michi.json"),
    include_str!("../../../data/scenarios/n5-07-tomodachi.json"),
    include_str!("../../../data/scenarios/n5-08-yotei.json"),
    include_str!("../../../data/scenarios/n5-09-resutoran.json"),
    include_str!("../../../data/scenarios/n5-10-kaimono.json"),
    include_str!("../../../data/scenarios/n5-11-kazoku.json"),
    include_str!("../../../data/scenarios/n5-12-kinou.json"),
];

const FIXTURES: &[(&str, &str)] = &[
    (
        "n5-01-aisatsu",
        include_str!("../../../qa/fixtures/n5-01-aisatsu.json"),
    ),
    (
        "n5-02-jikoshoukai",
        include_str!("../../../qa/fixtures/n5-02-jikoshoukai.json"),
    ),
    (
        "n5-03-konbini",
        include_str!("../../../qa/fixtures/n5-03-konbini.json"),
    ),
    (
        "n5-04-kafe",
        include_str!("../../../qa/fixtures/n5-04-kafe.json"),
    ),
    (
        "n5-05-eki",
        include_str!("../../../qa/fixtures/n5-05-eki.json"),
    ),
    (
        "n5-06-michi",
        include_str!("../../../qa/fixtures/n5-06-michi.json"),
    ),
    (
        "n5-07-tomodachi",
        include_str!("../../../qa/fixtures/n5-07-tomodachi.json"),
    ),
    (
        "n5-08-yotei",
        include_str!("../../../qa/fixtures/n5-08-yotei.json"),
    ),
    (
        "n5-09-resutoran",
        include_str!("../../../qa/fixtures/n5-09-resutoran.json"),
    ),
    (
        "n5-10-kaimono",
        include_str!("../../../qa/fixtures/n5-10-kaimono.json"),
    ),
    (
        "n5-11-kazoku",
        include_str!("../../../qa/fixtures/n5-11-kazoku.json"),
    ),
    (
        "n5-12-kinou",
        include_str!("../../../qa/fixtures/n5-12-kinou.json"),
    ),
];

/// The recorded answers of a scenario (demo mode without a provider).
pub fn fixtures(scenario: &str) -> Option<&'static str> {
    FIXTURES
        .iter()
        .find(|(id, _)| *id == scenario)
        .map(|(_, f)| *f)
}

/// The profile with the curriculum and the scenarios: what the app and `kaiwad` work on.
pub struct Backend {
    pub store: Store,
    pub path: PathBuf,
    pub curriculum: Curriculum,
    pub scenarios: BTreeMap<String, Scenario>,
    pub demo_data: bool,
}

impl Backend {
    /// Opens the default profile, or the demo profile with `KAIWA_DEMO` set.
    pub fn open() -> anyhow::Result<Self> {
        let demo_data = std::env::var_os("KAIWA_DEMO").is_some();
        let path = if let Some(p) = std::env::var_os("KAIWA_PROFILE") {
            PathBuf::from(p)
        } else if demo_data {
            let dir = std::env::temp_dir().join(format!("kaiwa-demo-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            dir.join("demo.kaiwa")
        } else {
            kaiwa_store::profiles_dir().join("default.kaiwa")
        };
        Self::open_at(path, demo_data)
    }

    /// Opens the profile at `path`; `demo` fills a new one with two weeks of history.
    pub fn open_at(path: PathBuf, demo: bool) -> anyhow::Result<Self> {
        let fresh = !path.exists();
        let store = Store::open(&path)?;
        let curriculum = Curriculum::from_json(CURRICULUM)?;
        let scenarios = SCENARIOS
            .iter()
            .map(|j| Scenario::from_json(j).map(|s| (s.id.clone(), s)))
            .collect::<Result<_, _>>()?;
        let st = Backend {
            store,
            path,
            curriculum,
            scenarios,
            demo_data: demo,
        };
        if demo && fresh {
            seed(&st)?;
        }
        Ok(st)
    }
}

/// The tutor from the environment (`kaiwa_ai::provider_from_env`, for the owner's local
/// test); recorded answers otherwise (demo mode, `true`).
pub fn provider(scenario: &str) -> (Arc<dyn ChatProvider>, bool) {
    match kaiwa_ai::provider_from_env() {
        Ok(Some(p)) => return (p, false),
        Ok(None) => {}
        Err(e) => eprintln!("kaiwa: {e}; using recorded answers"),
    }
    let mock = fixtures(scenario)
        .and_then(|f| MockProvider::from_json(f).ok())
        .unwrap_or_default()
        .with_fallback("すみません、もう一度お願いします。");
    (Arc::new(mock), true)
}

/// Now, in the local time zone (or `KAIWA_NOW`).
pub fn now() -> Moment {
    if let Some(v) = std::env::var_os("KAIWA_NOW")
        && let Some((u, o)) = v.to_string_lossy().split_once(':')
        && let (Ok(u), Ok(o)) = (u.parse(), o.parse())
    {
        return Moment::new(u, o);
    }
    let local = chrono::Local::now();
    Moment::new(local.timestamp(), local.offset().local_minus_utc() / 60)
}

/// Two weeks of a beginner's history: units 1 and 2 done, unit 3 under way, words to review,
/// a few gaps. For screenshots and the demo.
fn seed(st: &Backend) -> anyhow::Result<()> {
    let now = now();
    let today = now.day();
    let profile = Profile {
        name: "Mara".into(),
        placed: true,
        ui_lang: if std::env::var("KAIWA_LANG").as_deref() == Ok("en") {
            "en".into()
        } else {
            "de".into()
        },
        ..Default::default()
    };
    st.store.set_profile(&profile)?;
    let model = LearnerModel::placed(&[
        (Skill::Listening, "n5_low"),
        (Skill::Speaking, "a0"),
        (Skill::Fluency, "a0"),
        (Skill::ReadingKana, "a0"),
        (Skill::ReadingKanji, "none"),
        (Skill::Register, "casual_only"),
    ]);
    let mut learning = Learning {
        regulator: Regulator::new(Difficulty::for_learner(&model)),
        model,
        ..Default::default()
    };
    learning.model.errors.insert(
        "particle".into(),
        kaiwa_core::learner::ErrorStat {
            corrected: 4,
            rate: 0.3,
        },
    );
    learning.model.errors.insert(
        "counter".into(),
        kaiwa_core::learner::ErrorStat {
            corrected: 3,
            rate: 0.22,
        },
    );
    for s in [0.74, 0.81, 0.86, 0.79, 0.9, 0.83, 0.78] {
        let model = learning.model.clone();
        learning.regulator.after_session(s, &model);
    }
    for s in ["n5-01-aisatsu", "n5-02-jikoshoukai"] {
        learning.course.scenarios_done.insert(s.into());
        learning.course.completions.insert(s.into(), 2);
    }
    learning
        .course
        .completions
        .insert("n5-03-konbini".into(), 1);
    st.store.set_learning(&learning)?;
    // Days: practised on 13 of the last 14 (one gap, covered by a freeze), two minutes today.
    let minutes = [6, 5, 0, 7, 5, 12, 6, 5, 6, 5, 8, 5, 6, 7];
    for (i, m) in minutes.iter().enumerate() {
        if *m == 0 {
            continue;
        }
        let d = today.plus(i as i64 - minutes.len() as i64);
        st.store.put_day(
            d,
            &DayRecord {
                speaking_secs: m * 60,
                scenarios_done: u32::from(*m >= 6),
                reviews_done: 4,
                xp: m * 18,
                ..Default::default()
            },
        )?;
    }
    st.store.put_day(
        today,
        &DayRecord {
            speaking_secs: 130,
            xp: 40,
            ..Default::default()
        },
    )?;
    let words = [
        (
            "コーヒーが好きです",
            ItemKind::Pattern,
            Some("particle"),
            "n5-02",
        ),
        (
            "ビールを二つください",
            ItemKind::Pattern,
            Some("counter"),
            "n5-03",
        ),
        ("おはようございます", ItemKind::Phrase, None, "n5-01"),
        ("天気", ItemKind::Word, None, "n5-01"),
        ("仕事", ItemKind::Word, None, "n5-02"),
        ("袋", ItemKind::Word, None, "n5-03"),
        ("温める", ItemKind::Word, None, "n5-03"),
        ("いくら", ItemKind::Word, None, "n5-03"),
    ];
    for (n, (w, kind, corr, unit)) in words.iter().enumerate() {
        let added = Day(today.0 - 9 + n as i64)
            .start(now.offset_min)
            .plus_secs(9 * 3600);
        let source = match corr {
            Some(k) => Source::Correction {
                kind: (*k).to_string(),
            },
            None => Source::NewWord,
        };
        let mut item = Item::new(*w, *kind, source, added);
        item.unit = Some((*unit).to_string());
        item.review(Grade::Good, added.plus_secs(86_400));
        if n % 3 == 0 {
            item.due = today;
        }
        st.store.put_item(&item)?;
    }
    Ok(())
}
