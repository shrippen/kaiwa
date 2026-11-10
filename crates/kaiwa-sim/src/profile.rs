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

/// A directory of its own under the temp directory; it goes when this is dropped, also when a
/// check stops early. Two runs of a check side by side (tests) get two directories.
pub(crate) struct TempDir(PathBuf);

impl std::ops::Deref for TempDir {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn temp_dir(name: &str) -> TempDir {
    static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("kaiwa-sim-{}-{name}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    TempDir(d)
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
        "emi_srs_keigo" => srs_keigo(root),
        "hinnerk_own_profile" => own_profile(root),
        _ => return None,
    };
    Some(match r {
        Ok(None) => Outcome::Pass,
        Ok(Some(why)) => Outcome::Fail(why),
        Err(e) => Outcome::Fail(format!("{e:#}")),
    })
}

/// /LF720/: Hinnerk's grandson tries the app in a profile of his own on the same computer.
/// It starts from zero in the language of the interface, takes nothing of Hinnerk's and gives
/// nothing to it: his settings (text size, no gamification, cost limit), level, path, review,
/// days and the month's costs are as before, whatever the grandson does. The list names both
/// with exactly one in use; the profile chosen last is the one the app opens next time; a
/// name cannot leave the profiles' directory.
fn own_profile(root: &Path) -> Result<Option<String>> {
    use kaiwa_backend::{Backend, profiles, views};
    let dir = temp_dir("own-profile");
    let c = curriculum(root)?;
    let now = at(3, 19 * 3600);
    let talk = |b: &Backend, day: i64| -> Result<()> {
        let mut s = session(root)?;
        let id = b.store.begin_session(&s.snapshot(), at(day, 9 * 3600))?;
        for t in ["マラです。よろしくおねがいします。", "コーヒーを好きです。"]
        {
            s.turn(t, Input::Spoken)?;
        }
        b.store
            .record(id, &s, at(day, 9 * 3600), at(day, 9 * 3600 + 400), Some(&c))?;
        Ok(())
    };
    // Everything the app shows of a profile, and what it keeps.
    let all = |b: &Backend| -> Result<serde_json::Value> {
        let mut profile = views::profile(b)?;
        // The list of profiles and the keyring are the device's, not this profile's.
        for shared in ["profiles", "key", "tutor", "path"] {
            profile[shared] = serde_json::Value::Null;
        }
        Ok(serde_json::json!({
            "profile": profile,
            "today": views::today(b, now)?,
            "path": views::path(b)?,
            "review": views::review(b, now, 50)?,
            "learner": views::learner(b)?,
            "stats": views::stats(b, now)?,
            "export": b.store.export(now)?,
            "cost": b.month_cost(now)?,
        }))
    };
    let mut found = Vec::new();

    // Hinnerk: his settings, two talks, a month's cost.
    let hinnerk = Backend::open_at(profiles::current_in(&dir), false)?;
    let mut p = hinnerk.store.profile()?;
    p.name = "Hinnerk".into();
    p.text_scale = 1.6;
    p.gamification = false;
    p.cost_limit_cents = Some(500);
    hinnerk.store.set_profile(&p)?;
    talk(&hinnerk, 0)?;
    talk(&hinnerk, 1)?;
    hinnerk.store.set_state_value(
        "cost.2026-10",
        &kaiwa_backend::cost::Month {
            cents: 42.0,
            calls: 12,
            sessions: 2,
            ..Default::default()
        },
    )?;
    let before = all(&hinnerk)?;
    if before["cost"]["cents"] != 42.0 {
        found.push(format!("the month's cost is not his: {}", before["cost"]));
    }
    let his_file = hinnerk.path.clone();
    drop(hinnerk);

    // The grandson's own profile: from zero, German like the interface, now the one in use.
    let file = profiles::create(&dir, "Ole", "de")?;
    if profiles::current_in(&dir) != file {
        found.push("the new profile is not the one in use".into());
    }
    let ole = Backend::open_at(file.clone(), false)?;
    let op = ole.store.profile()?;
    let fresh = kaiwa_store::Profile::default();
    if op.name != "Ole" || op.ui_lang != "de" {
        found.push(format!("the new profile: {} / {}", op.name, op.ui_lang));
    }
    if op.text_scale != fresh.text_scale
        || op.gamification != fresh.gamification
        || op.cost_limit_cents.is_some()
        || op.placed
    {
        found.push("the new profile took over settings".into());
    }
    if !ole.store.items()?.is_empty()
        || !ole.store.days()?.is_empty()
        || !ole.store.sessions(10)?.is_empty()
        || ole.month_cost(now)?.cents != 0.0
        || ole.store.learning()?.course != Default::default()
    {
        found.push("the new profile does not start from zero".into());
    }
    let list = ole.profiles();
    let names: Vec<&str> = list.iter().filter_map(|v| v["name"].as_str()).collect();
    if names.len() != 2 || !names.contains(&"Hinnerk") || !names.contains(&"Ole") {
        found.push(format!("the list of profiles: {names:?}"));
    }
    if list.iter().filter(|v| v["current"] == true).count() != 1
        || !list
            .iter()
            .any(|v| v["current"] == true && v["name"] == "Ole")
    {
        found.push("not exactly the grandson's profile is marked as in use".into());
    }
    // He plays with it: talks, other settings.
    talk(&ole, 2)?;
    talk(&ole, 3)?;
    let mut op = ole.store.profile()?;
    op.ui_lang = "en".into();
    op.text_scale = 1.0;
    op.gamification = true;
    op.cost_limit_cents = Some(2000);
    ole.store.set_profile(&op)?;
    drop(ole);

    // Back to Hinnerk's: as he left it. And the app opens the profile chosen last.
    let name = his_file
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    profiles::set_current(&dir, &name)?;
    if profiles::current_in(&dir) != his_file {
        found.push("the profile chosen last is not the one opened".into());
    }
    let hinnerk = Backend::open_at(profiles::current_in(&dir), false)?;
    let after = all(&hinnerk)?;
    if after != before {
        let part = before
            .as_object()
            .into_iter()
            .flatten()
            .find(|(k, v)| after[k.as_str()] != **v)
            .map(|(k, _)| k.clone())
            .unwrap_or_default();
        found.push(format!("Hinnerk's profile changed: {part}"));
    }
    drop(hinnerk);
    // A name with a path in it stays a file in the directory.
    let odd = profiles::create(&dir, "../../Ole", "de")?;
    if odd.parent() != Some(&*dir) {
        found.push(format!(
            "a profile outside the directory: {}",
            odd.display()
        ));
    }
    if profiles::set_current(&dir, "../x.kaiwa").is_ok() {
        found.push("a profile outside the directory can be chosen".into());
    }
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

/// /LF230/: Emi's register corrections come back the next day as a mini conversation, not as a
/// flash card: the item carries the character's line she answered, and an answer that uses the
/// humble form in a sentence counts as right. The reception scene (keigo) with its recorded
/// answers stands in for the interview, which has no scenario yet.
fn srs_keigo(root: &Path) -> Result<Option<String>> {
    let dir = temp_dir("srs-keigo");
    let store = Store::open(&dir.join("emi.kaiwa"))?;
    let sc = Scenario::from_json(&std::fs::read_to_string(
        root.join("data/scenarios/n4-13-uketsuke.json"),
    )?)?;
    let mock = MockProvider::from_json(&std::fs::read_to_string(
        root.join("qa/fixtures/n4-13-uketsuke.json"),
    )?)?;
    // Emi wants much feedback: style and register corrections are shown.
    let mut s = Session::new(sc, Arc::new(mock), Amount::Much, "de");
    let id = store.begin_session(&s.snapshot(), at(0, 9 * 3600))?;
    s.turn(
        "ミュラーと申します。二時に小野様とお約束しています。",
        Input::Spoken,
    )?;
    store.record(
        id,
        &s,
        at(0, 9 * 3600),
        at(0, 9 * 3600 + 300),
        Some(&curriculum(root)?),
    )?;
    let due = store.due(at(1, 9 * 3600), 10)?;
    let Some(item) = due.iter().find(|i| i.text == "お約束しております") else {
        return Ok(Some(format!(
            "the humble form is not due the next day: {:?}",
            due.iter().map(|i| &i.text).collect::<Vec<_>>()
        )));
    };
    if !matches!(&item.source, Source::Correction { kind } if kind == "register") {
        return Ok(Some(format!(
            "not a register correction: {:?}",
            item.source
        )));
    }
    match item.cue.as_deref() {
        Some("いらっしゃいませ。") => {}
        other => return Ok(Some(format!("no mini conversation, cue {other:?}"))),
    }
    if !item.answered("ミュラーと申します。二時に小野様とお約束しております。")
    {
        return Ok(Some(
            "a sentence with the humble form does not count".into(),
        ));
    }
    if item.answered("ミュラーと申します。二時に小野様とお約束しています。")
    {
        return Ok(Some("the old form counts as right".into()));
    }
    Ok(None)
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
    let course = |name: &str, switch: bool| -> Result<(Store, TempDir)> {
        let dir = temp_dir(name);
        let store = Store::open(&dir.join("p.kaiwa"))?;
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
        Ok((store, dir))
    };
    let ((cloud, _cloud_dir), (switched, _switched_dir)) = (
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
    // The fastest of three loads: the limit is for the app, not for a computer that is busy
    // with something else at that moment (a build next to the tests made this check fail).
    let mut ms = u128::MAX;
    let mut due = 0;
    for _ in 0..3 {
        let t = Instant::now();
        let store = Store::open(&path)?;
        let today = store.today(at(2600, 9 * 3600))?;
        let learning = store.learning()?;
        let _path = c.path(&learning.course, "de");
        let _words = learning.model.describe("de");
        let _recent = store.sessions(30)?;
        ms = ms.min(t.elapsed().as_millis());
        due = today.due;
    }
    if due == 0 {
        return Ok(Some("the fixture has nothing due".into()));
    }
    if std::env::var_os("KAIWA_SIM_VERBOSE").is_some() {
        eprintln!("lena_stats_fast: {ms} ms");
    }
    Ok((ms > 500).then(|| format!("{ms} ms")))
}
