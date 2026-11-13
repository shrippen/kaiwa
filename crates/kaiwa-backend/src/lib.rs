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
pub mod pause;
pub mod profiles;
pub mod recordings;
pub mod reminder;
pub mod short;
pub mod talk;
pub mod views;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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
/// The grammar catalogue (ADR-009): explanations for the grammar view of a conversation.
const GRAMMAR: &[&str] = &[
    include_str!("../../../data/grammar/n5.json"),
    include_str!("../../../data/grammar/n4.json"),
];
const SCENARIOS: &[&str] = &[
    include_str!("../../../data/scenarios/n5-01-aisatsu.json"),
    include_str!("../../../data/scenarios/n5-02-jikoshoukai.json"),
    include_str!("../../../data/scenarios/n5-04-konbini.json"),
    include_str!("../../../data/scenarios/n5-05-kafe.json"),
    include_str!("../../../data/scenarios/n5-06-eki.json"),
    include_str!("../../../data/scenarios/n5-07-michi.json"),
    include_str!("../../../data/scenarios/n5-09-tomodachi.json"),
    include_str!("../../../data/scenarios/n5-10-yotei.json"),
    include_str!("../../../data/scenarios/n5-12-resutoran.json"),
    include_str!("../../../data/scenarios/n5-13-kaimono.json"),
    include_str!("../../../data/scenarios/n5-14-kazoku.json"),
    include_str!("../../../data/scenarios/n5-16-kinou.json"),
    include_str!("../../../data/scenarios/n5-03-jugyou.json"),
    include_str!("../../../data/scenarios/n5-11-ichinichi.json"),
    include_str!("../../../data/scenarios/n5-15-sumai.json"),
    include_str!("../../../data/scenarios/n5-08-onsen.json"),
    include_str!("../../../data/scenarios/n4-01-keiken.json"),
    include_str!("../../../data/scenarios/n4-02-byouin.json"),
    include_str!("../../../data/scenarios/n4-02-kaze.json"),
    include_str!("../../../data/scenarios/n4-03-fudousan.json"),
    include_str!("../../../data/scenarios/n4-04-omiyage.json"),
    include_str!("../../../data/scenarios/n4-04-tanjoubi.json"),
    include_str!("../../../data/scenarios/n4-05-mokuhyou.json"),
    include_str!("../../../data/scenarios/n4-05-natsuyasumi.json"),
    include_str!("../../../data/scenarios/n4-07-ryokan.json"),
    include_str!("../../../data/scenarios/n4-07-soudan.json"),
    include_str!("../../../data/scenarios/n4-06-baito.json"),
    include_str!("../../../data/scenarios/n4-08-tenki.json"),
    include_str!("../../../data/scenarios/n4-08-uwasa.json"),
    include_str!("../../../data/scenarios/n4-10-hoteru.json"),
    include_str!("../../../data/scenarios/n4-10-wasuremono.json"),
    include_str!("../../../data/scenarios/n4-11-tonari.json"),
    include_str!("../../../data/scenarios/n4-13-denwa.json"),
    include_str!("../../../data/scenarios/n4-13-uketsuke.json"),
    include_str!("../../../data/scenarios/n4-14-guchi.json"),
    include_str!("../../../data/scenarios/n4-09-kisetsu.json"),
    include_str!("../../../data/scenarios/n4-09-matsuri.json"),
    include_str!("../../../data/scenarios/n4-12-henpin.json"),
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
        "n5-04-konbini",
        include_str!("../../../qa/fixtures/n5-04-konbini.json"),
    ),
    (
        "n5-05-kafe",
        include_str!("../../../qa/fixtures/n5-05-kafe.json"),
    ),
    (
        "n5-06-eki",
        include_str!("../../../qa/fixtures/n5-06-eki.json"),
    ),
    (
        "n5-07-michi",
        include_str!("../../../qa/fixtures/n5-07-michi.json"),
    ),
    (
        "n5-09-tomodachi",
        include_str!("../../../qa/fixtures/n5-09-tomodachi.json"),
    ),
    (
        "n5-10-yotei",
        include_str!("../../../qa/fixtures/n5-10-yotei.json"),
    ),
    (
        "n5-12-resutoran",
        include_str!("../../../qa/fixtures/n5-12-resutoran.json"),
    ),
    (
        "n5-13-kaimono",
        include_str!("../../../qa/fixtures/n5-13-kaimono.json"),
    ),
    (
        "n5-14-kazoku",
        include_str!("../../../qa/fixtures/n5-14-kazoku.json"),
    ),
    (
        "n5-16-kinou",
        include_str!("../../../qa/fixtures/n5-16-kinou.json"),
    ),
    (
        "n5-03-jugyou",
        include_str!("../../../qa/fixtures/n5-03-jugyou.json"),
    ),
    (
        "n5-11-ichinichi",
        include_str!("../../../qa/fixtures/n5-11-ichinichi.json"),
    ),
    (
        "n5-15-sumai",
        include_str!("../../../qa/fixtures/n5-15-sumai.json"),
    ),
    (
        "n5-08-onsen",
        include_str!("../../../qa/fixtures/n5-08-onsen.json"),
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
        "n4-07-ryokan",
        include_str!("../../../qa/fixtures/n4-07-ryokan.json"),
    ),
    (
        "n4-07-soudan",
        include_str!("../../../qa/fixtures/n4-07-soudan.json"),
    ),
    (
        "n4-06-baito",
        include_str!("../../../qa/fixtures/n4-06-baito.json"),
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
        "n4-10-hoteru",
        include_str!("../../../qa/fixtures/n4-10-hoteru.json"),
    ),
    (
        "n4-10-wasuremono",
        include_str!("../../../qa/fixtures/n4-10-wasuremono.json"),
    ),
    (
        "n4-11-tonari",
        include_str!("../../../qa/fixtures/n4-11-tonari.json"),
    ),
    (
        "n4-13-denwa",
        include_str!("../../../qa/fixtures/n4-13-denwa.json"),
    ),
    (
        "n4-13-uketsuke",
        include_str!("../../../qa/fixtures/n4-13-uketsuke.json"),
    ),
    (
        "n4-14-guchi",
        include_str!("../../../qa/fixtures/n4-14-guchi.json"),
    ),
    (
        "n4-09-kisetsu",
        include_str!("../../../qa/fixtures/n4-09-kisetsu.json"),
    ),
    (
        "n4-09-matsuri",
        include_str!("../../../qa/fixtures/n4-09-matsuri.json"),
    ),
    (
        "n4-12-henpin",
        include_str!("../../../qa/fixtures/n4-12-henpin.json"),
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
    pub grammar: kaiwa_core::grammar::Catalogue,
    /// The catalogue's patterns, compiled once (grammar points in the learner model).
    pub grammar_matcher: kaiwa_core::grammar::Matcher,
    pub scenarios: BTreeMap<String, Scenario>,
    pub demo_data: bool,
    /// The keys of this profile ([`kaiwa_ai::secrets::KeyHome`]).
    keys: kaiwa_ai::secrets::Keys,
}

impl Backend {
    /// Opens the default profile, or the demo profile with `KAIWA_DEMO` set.
    pub fn open() -> anyhow::Result<Self> {
        #[cfg(any(test, feature = "demo"))]
        let demo_data = std::env::var_os("KAIWA_DEMO").is_some();
        #[cfg(not(any(test, feature = "demo")))]
        let demo_data = false;
        let own_dir = std::env::var_os("KAIWA_PROFILE").is_none() && !demo_data;
        let path = if let Some(p) = std::env::var_os("KAIWA_PROFILE") {
            PathBuf::from(p)
        } else if demo_data {
            let dir = std::env::temp_dir().join(format!("kaiwa-demo-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            dir.join("demo.kaiwa")
        } else {
            profiles::current_in(&kaiwa_store::profiles_dir())
        };
        let st = Self::open_at(path, demo_data)?;

        // The keys of before V-01 go to the profile open at the first start: one from the
        // profiles directory, not one named by KAIWA_PROFILE or the demo.
        if own_dir {
            st.migrate_keys();
        }
        Ok(st)
    }

    /// Removes the demo profile [`Backend::open`] made in the temp directory, when the app
    /// ends; a profile named by `KAIWA_PROFILE` stays.
    #[cfg(any(test, feature = "demo"))]
    pub fn discard_demo(&self) {
        let made_here = self.demo_data
            && std::env::var_os("KAIWA_PROFILE").is_none()
            && self.dir().parent() == Some(std::env::temp_dir().as_path());
        if made_here {
            let _ = std::fs::remove_dir_all(self.dir());
        }
    }

    /// Opens the profile at `path`; `demo` fills a new one with two weeks of history.
    pub fn open_at(path: PathBuf, demo: bool) -> anyhow::Result<Self> {
        // Tests never touch the real keyring.
        #[cfg(test)]
        let store = Arc::new(kaiwa_ai::secrets::MemoryKeys::default());
        #[cfg(not(test))]
        let store = Arc::new(kaiwa_ai::secrets::Keyring);
        Self::open_with(path, demo, store, kaiwa_store::keyring_dir().as_deref())
    }

    /// Like [`Backend::open_at`], with the keys in `store` for the profiles in `keyring_dir`
    /// (tests and `kaiwa-sim` pass a [`kaiwa_ai::secrets::MemoryKeys`]).
    pub fn open_with(
        path: PathBuf,
        demo: bool,
        store: Arc<dyn kaiwa_ai::secrets::KeyStore>,
        keyring_dir: Option<&Path>,
    ) -> anyhow::Result<Self> {
        let keys =
            kaiwa_ai::secrets::Keys::new(store, kaiwa_ai::secrets::KeyHome::of(&path, keyring_dir));
        #[cfg_attr(not(any(test, feature = "demo")), allow(unused_variables))]
        let fresh = !path.exists();
        let db = Store::open(&path)?;
        let curriculum = Curriculum::from_files(CURRICULUM)?;
        let grammar = kaiwa_core::grammar::Catalogue::from_files(GRAMMAR)?;
        let scenarios = SCENARIOS
            .iter()
            .map(|j| Scenario::from_json(j).map(|s| (s.id.clone(), s)))
            .collect::<Result<_, _>>()?;
        let st = Backend {
            store: db,
            keys,
            path,
            curriculum,
            grammar_matcher: grammar.matcher(),
            grammar,
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
    /// After a recorded session: each grammar point the learner used or got corrected on
    /// (/LF201/), into the learner model of the profile.
    pub fn track_grammar(&self, session: &kaiwa_core::Session, now: Moment) -> anyhow::Result<()> {
        let mut l = self.store.learning()?;
        self.grammar_matcher
            .track(&mut l.grammar, session.turns(), now.day());
        self.store.set_learning(&l)?;
        Ok(())
    }

    /// A new talk on `scenario` as this profile has it: its tutor, its amount of feedback, the
    /// difficulty the regulator set, the unit's grammar and the learner's own speaking level,
    /// which a free talk names to the character (/LF116/). With it: whether the answers are
    /// the recorded ones. The app and `kaiwa-sim` both start a talk here.
    pub fn new_talk(&self, scenario: Scenario) -> anyhow::Result<(kaiwa_core::Session, bool)> {
        let profile = self.store.profile()?;
        let learning = self.store.learning()?;
        let (provider, demo) = self.provider(&scenario.id);
        let grammar = self.unit_grammar(&scenario.id);
        let session =
            kaiwa_core::Session::new(scenario, provider, profile.feedback, &profile.ui_lang)
                .with_difficulty(learning.regulator.difficulty.clone())
                .with_grammar(grammar)
                .with_learner_level(learning.model.level(kaiwa_core::learner::Skill::Speaking));
        Ok((session, demo))
    }

    /// A talk continued from its kept `state` (/LF121/), with what the state does not keep:
    /// the tutor, the unit's grammar and the learner's level as it is now.
    pub fn continued_talk(
        &self,
        scenario: Scenario,
        state: kaiwa_core::session::SessionState,
    ) -> anyhow::Result<(kaiwa_core::Session, bool)> {
        let level = self
            .store
            .learning()?
            .model
            .level(kaiwa_core::learner::Skill::Speaking);
        let (provider, demo) = self.provider(&scenario.id);
        let grammar = self.unit_grammar(&scenario.id);
        let session = kaiwa_core::Session::restore(scenario, provider, state)
            .with_grammar(grammar)
            .with_learner_level(level);
        Ok((session, demo))
    }

    /// The grammar labels of a scenario's unit, for the tutor's instructions; none for free
    /// talk.
    pub fn unit_grammar(&self, scenario: &str) -> Vec<String> {
        self.curriculum
            .unit_of(scenario)
            .map(|u| u.grammar.clone())
            .unwrap_or_default()
    }

    /// The keys of this profile.
    pub(crate) fn keys(&self) -> &kaiwa_ai::secrets::Keys {
        &self.keys
    }

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
        match kaiwa_ai::provider_from(&self.keys, self.tutor_env()) {
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

/// A backend on a profile in a directory of its own under the temp directory, for tests and
/// `kaiwa-sim`; the directory goes when this is dropped.
#[cfg(any(test, feature = "demo"))]
pub struct TempBackend {
    backend: Option<Backend>,
    pub dir: PathBuf,
}

#[cfg(any(test, feature = "demo"))]
impl TempBackend {
    /// `name` tells the directories apart in the temp directory; `demo` fills the profile with
    /// the demo data.
    pub fn new(name: &str, demo: bool) -> anyhow::Result<Self> {
        static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("kaiwa-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let backend = Backend::open_at(dir.join("profile.kaiwa"), demo)?;
        Ok(Self {
            backend: Some(backend),
            dir,
        })
    }
}

#[cfg(any(test, feature = "demo"))]
impl std::ops::Deref for TempBackend {
    type Target = Backend;
    fn deref(&self) -> &Backend {
        self.backend.as_ref().expect("open until dropped")
    }
}

#[cfg(any(test, feature = "demo"))]
impl Drop for TempBackend {
    fn drop(&mut self) {
        // The profile closes first, then its directory goes.
        self.backend = None;
        let _ = std::fs::remove_dir_all(&self.dir);
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
