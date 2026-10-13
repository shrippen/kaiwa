//! Checks that run against a real profile store (`kaiwa-store`) instead of a conversation:
//! resuming (/LF121/), export and deletion (/LF710/), no audio without opt-in (/LF712/) and the
//! load time with the full quantity structure (/LL020/). Each check builds its own profile in a
//! temporary directory, with the recorded tutor answers.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use kaiwa_ai::mock::MockProvider;
use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::curriculum::Curriculum;
use kaiwa_core::session::{Input, Session};
use kaiwa_core::srs::{Item, ItemKind, Source};
use kaiwa_core::{Amount, Scenario};
use kaiwa_store::{Store, delete_profile};

use crate::run::Outcome;

fn at(day: i64, secs: i64) -> Moment {
    Day::from_ymd(2026, 10, 5)
        .plus(day)
        .start(120)
        .plus_secs(secs)
}

fn temp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("kaiwa-sim-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

fn session(root: &Path) -> Result<Session> {
    let sc = Scenario::from_json(&std::fs::read_to_string(
        root.join("data/scenarios/n5-02-jikoshoukai.json"),
    )?)?;
    let mock = MockProvider::from_json(&std::fs::read_to_string(
        root.join("qa/fixtures/n5-02-jikoshoukai.json"),
    )?)?
    .with_fallback("すみません、もう一度お願いします。");
    Ok(Session::new(sc, Arc::new(mock), Amount::Little, "de"))
}

fn curriculum(root: &Path) -> Result<Curriculum> {
    Ok(Curriculum::from_dir(&root.join("data/curriculum"))?)
}

/// Three sessions in a profile on disk, like Lena's journey.
fn three_sessions(root: &Path, path: &Path) -> Result<Store> {
    let store = Store::open(path)?;
    let c = curriculum(root)?;
    for d in 0..3 {
        let mut s = session(root)?;
        let id = store.begin_session(&s.snapshot(), at(d, 9 * 3600))?;
        for t in ["マラです。よろしくおねがいします。", "コーヒーを好きです。"]
        {
            s.turn(t, Input::Spoken)?;
            store.save_session(id, &s.snapshot())?;
        }
        store.record(id, &s, at(d, 9 * 3600), at(d, 9 * 3600 + 400), Some(&c))?;
    }
    Ok(store)
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(files_under(&p));
        } else {
            out.push(p);
        }
    }
    out
}

pub fn check(id: &str, root: &Path) -> Option<Outcome> {
    let r = match id {
        "mara_resume" => resume(root),
        "lena_export" => export(root),
        "lena_delete" => delete(root),
        "lena_default_no_audio" => no_audio(root),
        "lena_stats_fast" => fast(root),
        "jonas_local_switch" => local_switch(root),
        _ => return None,
    };
    Some(match r {
        Ok(None) => Outcome::Pass,
        Ok(Some(why)) => Outcome::Fail(why),
        Err(e) => Outcome::Fail(format!("{e:#}")),
    })
}

/// A local model: the same answers as the cloud one, only slower.
struct Local(MockProvider);

impl kaiwa_ai::ChatProvider for Local {
    fn reply(&self, r: &kaiwa_ai::TurnRequest) -> Result<String, kaiwa_ai::AiError> {
        std::thread::sleep(std::time::Duration::from_millis(20));
        self.0.reply(r)
    }
    fn analyse(&self, r: &kaiwa_ai::TurnRequest) -> Result<kaiwa_ai::Analysis, kaiwa_ai::AiError> {
        std::thread::sleep(std::time::Duration::from_millis(20));
        self.0.analyse(r)
    }
}

/// /LL504/: the provider sits behind a trait; switching from the cloud (Android) to a local
/// model (PC) mid-course changes nothing in the learner model, the regulator or the course,
/// and the stored model names no provider. Latency and provider are all that differ.
fn local_switch(root: &Path) -> Result<Option<String>> {
    let sc = Scenario::from_json(&std::fs::read_to_string(
        root.join("data/scenarios/n5-02-jikoshoukai.json"),
    )?)?;
    let fixtures = std::fs::read_to_string(root.join("qa/fixtures/n5-02-jikoshoukai.json"))?;
    let mock = || -> Result<MockProvider> {
        Ok(MockProvider::from_json(&fixtures)?.with_fallback("すみません、もう一度お願いします。"))
    };
    let c = curriculum(root)?;
    let course = |name: &str, switch: bool| -> Result<Store> {
        let store = Store::open(&temp_dir(name).join("p.kaiwa"))?;
        for d in 0..4 {
            let provider: Arc<dyn kaiwa_ai::ChatProvider> = if switch && d >= 2 {
                Arc::new(Local(mock()?))
            } else {
                Arc::new(mock()?)
            };
            let mut s = Session::new(sc.clone(), provider, Amount::Little, "de");
            let id = store.begin_session(&s.snapshot(), at(d, 9 * 3600))?;
            for t in [
                "マラです。よろしくおねがいします。",
                "コーヒーを好きです。",
                "ドイツから来ました。",
            ] {
                s.turn(t, Input::Spoken)?;
            }
            store.record(id, &s, at(d, 9 * 3600), at(d, 9 * 3600 + 400), Some(&c))?;
        }
        Ok(store)
    };
    let (cloud, switched) = (
        course("switch-cloud", false)?,
        course("switch-local", true)?,
    );
    let (a, b) = (cloud.learning()?, switched.learning()?);
    let mut found = Vec::new();
    if a.model != b.model {
        found.push("the learner model differs after the switch".to_string());
    }
    if serde_json::to_value(&a.regulator)? != serde_json::to_value(&b.regulator)? {
        found.push("the regulator differs after the switch".into());
    }
    if serde_json::to_value(&a.course)? != serde_json::to_value(&b.course)? {
        found.push("the course differs after the switch".into());
    }
    let stored = serde_json::to_string(&serde_json::to_value(&b.model)?)?.to_lowercase();
    if ["provider", "anthropic", "openai", "model_name"]
        .iter()
        .any(|w| stored.contains(w))
    {
        found.push("the stored learner model names a provider".into());
    }
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

/// /LF121/: an interrupted session is offered and continues at the next unanswered goal.
fn resume(root: &Path) -> Result<Option<String>> {
    let dir = temp_dir("resume");
    let path = dir.join("mara.kaiwa");
    {
        let store = Store::open(&path)?;
        let mut s = session(root)?;
        let id = store.begin_session(&s.snapshot(), at(0, 8 * 3600 + 40 * 60))?;
        for t in [
            "マラです。よろしくおねがいします。",
            "ドイツのハンブルクから来ました。",
        ] {
            s.turn(t, Input::Spoken)?;
            store.save_session(id, &s.snapshot())?;
        }
        // The plasmoid closes in the middle of the character's answer: nothing more is saved.
    }
    let store = Store::open(&path)?;
    let Some(open) = store.open_session()? else {
        return Ok(Some("the interrupted session is not offered".into()));
    };
    let state = store.session_state(open.id)?;
    let s = session(root)?;
    let restored = Session::restore(s.scenario.clone(), Arc::new(MockProvider::default()), state);
    let goal = restored.current_goal().map(|g| g.id.clone());
    std::fs::remove_dir_all(&dir)?;
    Ok(match goal.as_deref() {
        Some("work") => None,
        other => Some(format!("continues at {other:?}, expected the third goal")),
    })
}

/// /LF710/: the export is valid JSON after the published schema and holds every part.
fn export(root: &Path) -> Result<Option<String>> {
    let dir = temp_dir("export");
    let store = three_sessions(root, &dir.join("lena.kaiwa"))?;
    let json = store.export(at(3, 9 * 3600))?;
    let text = serde_json::to_string_pretty(&json)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let schema: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("spec/export.schema.json"),
    )?)?;
    let validator = jsonschema::validator_for(&schema)?;
    let errors: Vec<String> = validator
        .iter_errors(&parsed)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect();
    std::fs::remove_dir_all(&dir)?;
    if !errors.is_empty() {
        return Ok(Some(errors.join("; ")));
    }
    for part in ["profile", "learning", "sessions", "items", "days"] {
        let empty = match &parsed[part] {
            serde_json::Value::Array(a) => a.is_empty(),
            serde_json::Value::Null => true,
            _ => false,
        };
        if empty {
            return Ok(Some(format!("{part} is empty")));
        }
    }
    Ok(None)
}

/// /LF710/, /LL402/: after deletion no file of the profile is left.
fn delete(root: &Path) -> Result<Option<String>> {
    let dir = temp_dir("delete");
    let path = dir.join("lena.kaiwa");
    drop(three_sessions(root, &path)?);
    delete_profile(&path)?;
    let left = files_under(&dir);
    std::fs::remove_dir_all(&dir)?;
    Ok((!left.is_empty()).then(|| format!("left behind: {left:?}")))
}

/// /LF712/: without the opt-in no audio file lands in the profile directory.
fn no_audio(root: &Path) -> Result<Option<String>> {
    let dir = temp_dir("audio");
    let store = three_sessions(root, &dir.join("lena.kaiwa"))?;
    if store.profile()?.keep_recordings_days.is_some() {
        return Ok(Some("recordings are kept by default".into()));
    }
    let audio: Vec<PathBuf> = files_under(&dir)
        .into_iter()
        .filter(|p| {
            p.extension().is_some_and(|e| {
                ["wav", "ogg", "opus", "flac", "mp3"].contains(&&*e.to_string_lossy())
            })
        })
        .collect();
    std::fs::remove_dir_all(&dir)?;
    Ok((!audio.is_empty()).then(|| format!("audio without opt-in: {audio:?}")))
}

/// /LL020/: start page, learning path and profile with 10 000 sessions and 20 000 items in ≤ 500 ms.
fn fast(root: &Path) -> Result<Option<String>> {
    let dir = temp_dir("fast");
    let path = dir.join("lena.kaiwa");
    {
        let store = Store::open(&path)?;
        let s = session(root)?;
        let state = s.snapshot();
        store.batch(|st| {
            for n in 0..10_000 {
                st.begin_session(&state, at(n / 4, (n % 4) * 3600))?;
            }
            for n in 0..20_000 {
                let mut item = Item::new(
                    format!("ことば{n}"),
                    ItemKind::Word,
                    Source::NewWord,
                    at(n / 20, 0),
                );
                item.unit = Some(format!("n5-{:02}", n % 12 + 1));
                st.put_item(&item)?;
            }
            Ok(())
        })?;
    }
    let c = curriculum(root)?;
    let t = Instant::now();
    let store = Store::open(&path)?;
    let today = store.today(at(2600, 9 * 3600))?;
    let learning = store.learning()?;
    let _path = c.path(&learning.course, "de");
    let _words = learning.model.describe("de");
    let _recent = store.sessions(30)?;
    let ms = t.elapsed().as_millis();
    std::fs::remove_dir_all(&dir)?;
    if today.due == 0 {
        return Ok(Some("the fixture has nothing due".into()));
    }
    if std::env::var_os("KAIWA_SIM_VERBOSE").is_some() {
        eprintln!("lena_stats_fast: {ms} ms");
    }
    Ok((ms > 500).then(|| format!("{ms} ms")))
}
