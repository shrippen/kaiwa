//! What the app and, with M4 before 1.0, the service `kaiwad` share (ADR-008): the profile (store), the curriculum,
//! the scenarios, the tutor and the content of every view as JSON.
//!
//! - The profile file is the current one in `$KAIWA_DATA` (see `kaiwa_store::profiles_dir` and
//!   [`profiles`]), or `KAIWA_PROFILE`.
//! - With the build feature `demo` (off by default, never in a package): `KAIWA_DEMO=1` uses a
//!   fresh profile in a temporary directory, filled with two weeks of history, for screenshots
//!   (`KAIWA_LANG=en` for the English UI), and without a tutor the recorded answers stand in.
//!   Without the feature there is neither, and a missing tutor says so ([`kaiwa_ai::NoTutor`]).
//! - `KAIWA_NOW=<unix>:<offset minutes>` fixes the clock (screenshots); otherwise it is the
//!   system time in the local time zone.

pub mod cost;
#[cfg(any(test, feature = "demo"))]
mod demo;
pub mod latency;
pub mod lookup;
pub mod profiles;
pub mod recordings;
pub mod reminder;
pub mod short;
pub mod views;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use kaiwa_ai::ChatProvider;
#[cfg(any(test, feature = "demo"))]
use kaiwa_ai::mock::MockProvider;

use kaiwa_core::Scenario;
use kaiwa_core::clock::Moment;
use kaiwa_core::curriculum::Curriculum;
use kaiwa_store::Store;

/// The level files of the curriculum (ADR-009), joined by [`Curriculum::from_files`].
const CURRICULUM: &[&str] = &[
    include_str!("../../../data/curriculum/n5.json"),
    include_str!("../../../data/curriculum/n4.json"),
];
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
    include_str!("../../../data/scenarios/n5-13-jugyou.json"),
    include_str!("../../../data/scenarios/n5-14-ichinichi.json"),
    include_str!("../../../data/scenarios/n5-15-sumai.json"),
    include_str!("../../../data/scenarios/n5-16-onsen.json"),
    include_str!("../../../data/scenarios/n4-01-keiken.json"),
    include_str!("../../../data/scenarios/n4-02-byouin.json"),
    include_str!("../../../data/scenarios/n4-02-kaze.json"),
    include_str!("../../../data/scenarios/n4-03-fudousan.json"),
    include_str!("../../../data/scenarios/n4-04-omiyage.json"),
    include_str!("../../../data/scenarios/n4-04-tanjoubi.json"),
    include_str!("../../../data/scenarios/n4-05-mokuhyou.json"),
    include_str!("../../../data/scenarios/n4-05-natsuyasumi.json"),
    include_str!("../../../data/scenarios/n4-06-ryokan.json"),
    include_str!("../../../data/scenarios/n4-06-soudan.json"),
    include_str!("../../../data/scenarios/n4-07-baito.json"),
    include_str!("../../../data/scenarios/n4-08-tenki.json"),
    include_str!("../../../data/scenarios/n4-08-uwasa.json"),
    include_str!("../../../data/scenarios/n4-09-hoteru.json"),
    include_str!("../../../data/scenarios/n4-09-wasuremono.json"),
    include_str!("../../../data/scenarios/n4-10-tonari.json"),
    include_str!("../../../data/scenarios/n4-11-denwa.json"),
    include_str!("../../../data/scenarios/n4-11-uketsuke.json"),
    include_str!("../../../data/scenarios/n4-12-guchi.json"),
    include_str!("../../../data/scenarios/n4-13-kisetsu.json"),
    include_str!("../../../data/scenarios/n4-13-matsuri.json"),
    include_str!("../../../data/scenarios/n4-14-henpin.json"),
    // Free talk on a topic, outside the path (/LF116/).
    include_str!("../../../data/scenarios/frei-shumi.json"),
    include_str!("../../../data/scenarios/frei-tabemono.json"),
    include_str!("../../../data/scenarios/frei-ryokou.json"),
    include_str!("../../../data/scenarios/frei-mainichi.json"),
];

#[cfg(any(test, feature = "demo"))]
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
    (
        "n5-13-jugyou",
        include_str!("../../../qa/fixtures/n5-13-jugyou.json"),
    ),
    (
        "n5-14-ichinichi",
        include_str!("../../../qa/fixtures/n5-14-ichinichi.json"),
    ),
    (
        "n5-15-sumai",
        include_str!("../../../qa/fixtures/n5-15-sumai.json"),
    ),
    (
        "n5-16-onsen",
        include_str!("../../../qa/fixtures/n5-16-onsen.json"),
    ),
    (
        "n4-01-keiken",
        include_str!("../../../qa/fixtures/n4-01-keiken.json"),
    ),
    (
        "n4-02-byouin",
        include_str!("../../../qa/fixtures/n4-02-byouin.json"),
    ),
    (
        "n4-02-kaze",
        include_str!("../../../qa/fixtures/n4-02-kaze.json"),
    ),
    (
        "n4-03-fudousan",
        include_str!("../../../qa/fixtures/n4-03-fudousan.json"),
    ),
    (
        "n4-04-omiyage",
        include_str!("../../../qa/fixtures/n4-04-omiyage.json"),
    ),
    (
        "n4-04-tanjoubi",
        include_str!("../../../qa/fixtures/n4-04-tanjoubi.json"),
    ),
    (
        "n4-05-mokuhyou",
        include_str!("../../../qa/fixtures/n4-05-mokuhyou.json"),
    ),
    (
        "n4-05-natsuyasumi",
        include_str!("../../../qa/fixtures/n4-05-natsuyasumi.json"),
    ),
    (
        "n4-06-ryokan",
        include_str!("../../../qa/fixtures/n4-06-ryokan.json"),
    ),
    (
        "n4-06-soudan",
        include_str!("../../../qa/fixtures/n4-06-soudan.json"),
    ),
    (
        "n4-07-baito",
        include_str!("../../../qa/fixtures/n4-07-baito.json"),
    ),
    (
        "n4-08-tenki",
        include_str!("../../../qa/fixtures/n4-08-tenki.json"),
    ),
    (
        "n4-08-uwasa",
        include_str!("../../../qa/fixtures/n4-08-uwasa.json"),
    ),
    (
        "n4-09-hoteru",
        include_str!("../../../qa/fixtures/n4-09-hoteru.json"),
    ),
    (
        "n4-09-wasuremono",
        include_str!("../../../qa/fixtures/n4-09-wasuremono.json"),
    ),
    (
        "n4-10-tonari",
        include_str!("../../../qa/fixtures/n4-10-tonari.json"),
    ),
    (
        "n4-11-denwa",
        include_str!("../../../qa/fixtures/n4-11-denwa.json"),
    ),
    (
        "n4-11-uketsuke",
        include_str!("../../../qa/fixtures/n4-11-uketsuke.json"),
    ),
    (
        "n4-12-guchi",
        include_str!("../../../qa/fixtures/n4-12-guchi.json"),
    ),
    (
        "n4-13-kisetsu",
        include_str!("../../../qa/fixtures/n4-13-kisetsu.json"),
    ),
    (
        "n4-13-matsuri",
        include_str!("../../../qa/fixtures/n4-13-matsuri.json"),
    ),
    (
        "n4-14-henpin",
        include_str!("../../../qa/fixtures/n4-14-henpin.json"),
    ),
];

/// The recorded answers of a scenario (demo mode without a provider).
#[cfg(any(test, feature = "demo"))]
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
        #[cfg(any(test, feature = "demo"))]
        let demo_data = std::env::var_os("KAIWA_DEMO").is_some();
        #[cfg(not(any(test, feature = "demo")))]
        let demo_data = false;
        let path = if let Some(p) = std::env::var_os("KAIWA_PROFILE") {
            PathBuf::from(p)
        } else if demo_data {
            let dir = std::env::temp_dir().join(format!("kaiwa-demo-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            dir.join("demo.kaiwa")
        } else {
            profiles::current_in(&kaiwa_store::profiles_dir())
        };
        Self::open_at(path, demo_data)
    }

    /// Opens the profile at `path`; `demo` fills a new one with two weeks of history.
    pub fn open_at(path: PathBuf, demo: bool) -> anyhow::Result<Self> {
        #[cfg_attr(not(any(test, feature = "demo")), allow(unused_variables))]
        let fresh = !path.exists();
        let store = Store::open(&path)?;
        let curriculum = Curriculum::from_files(CURRICULUM)?;
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
        #[cfg(any(test, feature = "demo"))]
        if demo && fresh {
            demo::seed(&st)?;
        }
        // Recordings past their days go when the profile opens (/LF712/).
        if let Err(e) = st.purge_recordings(now()) {
            eprintln!("kaiwa: {e:#}");
        }
        Ok(st)
    }
}

/// Whether this build has the demo (the build feature `demo`, for screenshots; never in a
/// package, `packaging/check-release.sh` checks the binary).
pub const DEMO: bool = cfg!(any(test, feature = "demo"));

/// What only a demo build contains; the package check looks for it.
#[cfg(any(test, feature = "demo"))]
#[used]
static DEMO_MARKER: [u8; 16] = *b"kaiwa:demo-build";

impl Backend {
    /// The tutor of the profile (Q-15).
    pub fn tutor(&self) -> kaiwa_ai::Tutor {
        self.store.profile().map(|p| p.tutor).unwrap_or_default()
    }

    /// The tutor's settings as the providers read them, the environment on top (local test).
    pub fn tutor_env(&self) -> impl Fn(&str) -> Option<String> + use<> {
        let tutor = self.tutor();
        move |k: &str| tutor.lookup(|k| std::env::var(k).ok())(k)
    }

    /// The tutor for a conversation in `scenario`: the one set up in the settings (or the
    /// environment). Without one, a demo build replays the recorded answers (`true`); any other
    /// build says that no tutor is set up.
    pub fn provider(&self, scenario: &str) -> (Arc<dyn ChatProvider>, bool) {
        match kaiwa_ai::provider_from(self.tutor_env()) {
            Ok(Some(p)) => return (p, false),
            Ok(None) => {}
            Err(e) => eprintln!("kaiwa: {e}"),
        }
        #[cfg(any(test, feature = "demo"))]
        {
            let mock = fixtures(scenario)
                .and_then(|f| MockProvider::from_json(f).ok())
                .unwrap_or_default()
                .with_fallback("すみません、もう一度お願いします。");
            (Arc::new(mock), true)
        }
        #[cfg(not(any(test, feature = "demo")))]
        {
            let _ = scenario;
            (Arc::new(kaiwa_ai::NoTutor), false)
        }
    }
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
