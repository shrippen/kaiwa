//! Checks of single rules of the engine, each with its own small session against the recorded
//! answers: XP against gaming (/LF603/), help when stuck (/LF213/), typed turns and rōmaji
//! (/LF120/), no locks after mistakes (/LF606/), placement settings and the profile in words
//! (/LF211/, /LF202/), disputes (/LF307/), misrecognitions (/LF113/), the skip offer (/LF215/)
//! and what leaves the device (/LF711/).

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use kaiwa_ai::mock::MockProvider;
use kaiwa_core::curriculum::{CourseProgress, Curriculum};
use kaiwa_core::difficulty::{Difficulty, Knob, learner_says};
use kaiwa_core::learner::{LearnerModel, Skill, level_index};
use kaiwa_core::placement::{Placement, Reading, STAGES, StageResult, TURNS_PER_STAGE};
use kaiwa_core::romaji;
use kaiwa_core::session::{Input, Session};
use kaiwa_core::{Amount, Regulator, Scenario};
use serde_json::Value;

use crate::run::Outcome;

fn session(root: &Path, scenario: &str) -> Result<Session> {
    let sc = Scenario::from_json(
        &std::fs::read_to_string(root.join(format!("data/scenarios/{scenario}.json")))
            .with_context(|| scenario.to_string())?,
    )?;
    let mock = MockProvider::from_json(&std::fs::read_to_string(
        root.join(format!("qa/fixtures/{scenario}.json")),
    )?)?
    .with_fallback("すみません、もう一度お願いします。");
    Ok(Session::new(sc, Arc::new(mock), Amount::Little, "de"))
}

/// The persona's placement (/LF200/), played against the simulated learner: each stage gives
/// the expected share of good turns at the persona's true level (as in `days`), rounded to
/// whole turns; register slips follow the persona's register.
pub fn simulate_placement(persona: &Value) -> Placement {
    let truth = |k: &str| {
        persona["start_level"][k]
            .as_str()
            .and_then(level_index)
            .unwrap_or(0.0)
    };
    let (listening, speaking, register) =
        (truth("listening"), truth("speaking"), truth("register"));
    let n = f64::from(TURNS_PER_STAGE);
    let turns = |p: f64| (p * n).round() / n;
    let mut pl = Placement::default();
    while let Some(i) = pl.next_stage() {
        let level = STAGES[i].1;
        let speak = turns(crate::days::p_success(speaking, level));
        let slips = match register as u32 {
            0 | 1 => 0.75,
            2 => 0.25,
            _ => 0.0,
        };
        pl.add(StageResult {
            level,
            turns: TURNS_PER_STAGE,
            listening: turns(crate::days::p_success(listening, level)),
            speaking: speak,
            fluency: speak,
            register_mistakes: (slips * n).round() as u32,
        });
    }
    let says = |k: &str, fluent: &str| {
        let (t, f) = (truth(k), level_index(fluent).unwrap_or(4.0));
        if t >= f {
            Reading::Fluent
        } else if t > 0.0 {
            Reading::Some
        } else {
            Reading::None
        }
    };
    pl.kana = says("reading_kana", "n3");
    pl.kanji = says("reading_kanji", "n5");
    pl
}

fn placed(persona: &Value) -> LearnerModel {
    simulate_placement(persona).result()
}

fn persona(root: &Path, id: &str) -> Result<Value> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        root.join(format!("qa/personas/{id}.json")),
    )?)?)
}

pub fn check(id: &str, root: &Path, p: &Value) -> Option<Outcome> {
    let r = match id {
        "jonas_xp_fair" => xp_fair(root),
        "jonas_scenario_stuck" => stuck(root),
        "jonas_typed_marked" => typed(root),
        "hinnerk_no_hearts" => no_hearts(root),
        "selin_settings_split" => settings_split(p),
        "hinnerk_tts_slow" => tts_slow(root, p),
        "selin_profile_visible" => profile_visible(p),
        "theo_dispute" => dispute(root),
        "emi_misrecognition" => misrecognition(root),
        "emi_skip" => skip(root, p),
        "lena_transparency" => transparency(),
        "selin_win_mixed" => mixed(),
        "selin_profile_split" => profile_split(p),
        "emi_ceiling" => ceiling(p),
        "selin_casual_ok" => casual_ok(root),
        "theo_free_talk" => free_talk(p),
        "jonas_cost_limit" => cost_limit(),
        _ => return None,
    };
    Some(match r {
        Ok(None) => Outcome::Pass,
        Ok(Some(why)) => Outcome::Fail(why),
        Err(e) => Outcome::Fail(format!("{e:#}")),
    })
}

/// A provider's answer with a bill: one call of the dearest model in the price list.
struct Billing;

impl kaiwa_ai::transport::Transport for Billing {
    fn post_json(
        &self,
        _: &str,
        _: &[(&str, String)],
        _: &Value,
    ) -> Result<Value, kaiwa_ai::AiError> {
        Ok(serde_json::json!({
            "model": "claude-opus-5-5",
            "usage": { "input_tokens": 20_000, "output_tokens": 2_000 }
        }))
    }
}

/// /LF714/: Jonas talks with a cloud tutor on the way and a local one on the PC. The settings
/// name the month's estimated cost and the cost per talk; nothing brakes before he sets a limit
/// (I-06); at his limit the cloud tutor stops answering and the local one goes on; local talks
/// cost nothing and do not thin out the cost per talk; the next month starts at zero.
fn cost_limit() -> Result<Option<String>> {
    use kaiwa_ai::cost::{Metered, meter};
    use kaiwa_ai::transport::Transport;
    // The meter is the process's: one run of this check at a time.
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let b = kaiwa_backend::TempBackend::new("sim-cost", false)?;
    let now = kaiwa_core::Day::from_ymd(2026, 10, 20)
        .start(120)
        .plus_secs(8 * 3600);
    let (cloud, local) = (Metered::new(Billing, true), Metered::new(Billing, false));
    let call = |t: &Metered<Billing>| t.post_json("u", &[], &Value::Null);
    let mut found = Vec::new();
    meter().take();

    // No limit until he sets one: a talk of 20 calls goes through.
    if b.store.profile()?.cost_limit_cents.is_some() {
        found.push("a limit is preset".into());
    }
    b.arm_brake(now)?;
    for _ in 0..20 {
        if call(&cloud).is_err() {
            found.push("braked without a limit".into());
            break;
        }
        b.book_costs(now, false)?;
    }
    b.book_costs(now, true)?;
    let first = b.month_cost(now)?.cents;
    let view = b.cost_view(now)?;
    let euro = format!("{:.2} €", first / 100.0).replace('.', ",");
    let text = view["text"].as_str().unwrap_or_default();
    if first <= 0.0 || !text.contains(&euro) || !text.contains("pro Gespräch") {
        found.push(format!("the month's cost ({euro}) is not named: {text}"));
    }
    // A talk on the PC, local: free, and not a talk the cost is spread over.
    for _ in 0..20 {
        call(&local)?;
        b.book_costs(now, false)?;
    }
    b.book_costs(now, true)?;
    let view = b.cost_view(now)?;
    if b.month_cost(now)?.cents != first {
        found.push("a local talk cost something".into());
    }
    if view["perSession"].as_f64() != Some(first) {
        found.push(format!(
            "the cost per talk counts the local one: {} instead of {first}",
            view["perSession"]
        ));
    }

    // His own limit, a little above what the month cost so far: the cloud tutor stops there.
    let limit = (first / 100.0).ceil() as u32 * 100 + 200;
    let mut profile = b.store.profile()?;
    profile.cost_limit_cents = Some(limit);
    b.store.set_profile(&profile)?;
    b.arm_brake(now)?;
    let mut refused = false;
    for _ in 0..40 {
        match call(&cloud) {
            Ok(_) => {
                b.book_costs(now, false)?;
            }
            Err(kaiwa_ai::AiError::Budget) => {
                refused = true;
                break;
            }
            Err(e) => found.push(format!("{e}")),
        }
    }
    let spent = b.month_cost(now)?.cents;
    let one_call = first / 20.0;
    if !refused {
        found.push(format!(
            "no brake at {spent:.0} ct with a limit of {limit} ct"
        ));
    } else if spent < f64::from(limit) || spent > f64::from(limit) + one_call {
        found.push(format!("braked at {spent:.0} ct, the limit is {limit} ct"));
    }
    if !b.over_budget(now)? || b.cost_view(now)?["over"] != true {
        found.push("the settings do not show the limit as reached".into());
    }
    if call(&local).is_err() {
        found.push("at the limit the local tutor does not answer either".into());
    }
    b.book_costs(now, true)?;

    // November starts at zero.
    let next = kaiwa_core::Day::from_ymd(2026, 11, 1)
        .start(60)
        .plus_secs(8 * 3600);
    b.arm_brake(next)?;
    if b.month_cost(next)?.cents != 0.0 || b.over_budget(next)? || call(&cloud).is_err() {
        found.push("the new month does not start at zero".into());
    }
    meter().take();
    meter().set_budget(None);
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

/// A tutor that reaches every goal named to it.
struct Listening {
    goals: Vec<String>,
}

impl kaiwa_ai::ChatProvider for Listening {
    fn reply(&self, _: &kaiwa_ai::TurnRequest) -> Result<String, kaiwa_ai::AiError> {
        Ok("そうですか。".into())
    }

    fn analyse(&self, _: &kaiwa_ai::TurnRequest) -> Result<kaiwa_ai::Analysis, kaiwa_ai::AiError> {
        Ok(kaiwa_ai::Analysis {
            understood: true,
            goals_met: self.goals.clone(),
            corrections: Vec::new(),
            new_words: Vec::new(),
            reading: String::new(),
        })
    }
}

/// /LF116/: the start page offers free talks on topics, outside the learning path; a free
/// talk started or continued as the app does it tells the character the persona's own level;
/// and finished free talks leave the path where it was, while the day counts them as practice.
fn free_talk(p: &Value) -> Result<Option<String>> {
    let b = kaiwa_backend::TempBackend::new("sim-free", false)?;
    let now = kaiwa_core::Day::from_ymd(2026, 10, 5)
        .start(120)
        .plus_secs(19 * 3600);
    let mut found = Vec::new();

    // Offered on the start page, in both languages, none of them a scenario of the path.
    let mut topics = Vec::new();
    for lang in ["de", "en"] {
        let mut profile = b.store.profile()?;
        profile.ui_lang = lang.into();
        b.store.set_profile(&profile)?;
        let today = kaiwa_backend::views::today(&b, now)?;
        let free: Vec<(String, String)> = today["free"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|f| {
                let text = |k: &str| f[k].as_str().unwrap_or_default().to_string();
                (text("id"), text("topic"))
            })
            .collect();
        if free.len() < 2 {
            found.push(format!("{} free topics offered ({lang})", free.len()));
        }
        for (id, topic) in &free {
            if topic.is_empty() || topic.contains(':') {
                found.push(format!("{id}: topic {topic:?} ({lang})"));
            }
            if b.curriculum.unit_of(id).is_some() {
                found.push(format!("{id} is a scenario of the path"));
            }
        }
        topics.push(free);
    }
    if topics[0]
        .iter()
        .map(|t| &t.1)
        .eq(topics[1].iter().map(|t| &t.1))
    {
        found.push("the topics read the same in German and English".into());
    }
    let Some((id, _)) = topics[0].first().cloned() else {
        return Ok(Some(found.join("; ")));
    };
    let scenario = b.scenarios.get(&id).cloned().with_context(|| id.clone())?;

    // His level: the profile holds his placement, and a talk started the way the app starts
    // it (`Backend::new_talk`) names the level of his `start_level.speaking` to the
    // character. So does the same talk continued later.
    let model = simulate_placement(p).result();
    let own = model.level(Skill::Speaking);
    b.store.set_placement(model.clone())?;
    let want = match p["start_level"]["speaking"].as_str().unwrap_or_default() {
        "none" | "a0" | "n5_low" | "n5" => "N5",
        "n4_low" | "n4" => "N4",
        "n3" => "N3",
        "n2" => "N2",
        _ => "N1",
    };
    let (started, _) = b.new_talk(scenario.clone())?;
    let (continued, _) = b.continued_talk(scenario.clone(), started.snapshot())?;
    for (how, told) in [
        ("started", started.reply_instructions()),
        ("continued", continued.reply_instructions()),
    ] {
        if !told.contains(&format!("JLPT {want})")) && !told.contains(&format!("JLPT {want} ")) {
            found.push(format!(
                "speaking {own:.1}, a talk {how} does not tell the character JLPT {want}: {}",
                told.lines().nth(1).unwrap_or_default()
            ));
        }
    }
    let goals: Vec<String> = scenario.goals.iter().map(|g| g.id.clone()).collect();
    let talk = || -> Result<Session> {
        let tutor = Arc::new(Listening {
            goals: goals.clone(),
        });
        let mut s = Session::new(scenario.clone(), tutor, Amount::Normal, "en")
            .with_difficulty(Difficulty::for_learner(&model));
        s.turn("映画音楽をよく聞きます。", Input::Spoken)?;
        Ok(s)
    };

    // Outside the path: free talks with every goal reached, more often than a scenario of the
    // path needs to count as done, leave the path as it was; the day counts them.
    let before = b.store.learning()?.course;
    let next = b.curriculum.next_scenario(&before);
    for k in 0..=i64::from(kaiwa_core::curriculum::COMPLETIONS) {
        let s = talk()?;
        if !s.done() {
            found.push("the free talk does not end with its goals".into());
            break;
        }
        let start = now.plus_secs(k * 900);
        let sid = b.store.begin_session(&s.snapshot(), start)?;
        b.store
            .record(sid, &s, start, start.plus_secs(300), Some(&b.curriculum))?;
    }
    let after = b.store.learning()?.course;
    if b.curriculum.next_scenario(&after) != next
        || b.curriculum.path(&after, "de") != b.curriculum.path(&before, "de")
    {
        found.push("free talks moved the learning path".into());
    }
    let day = b.store.day(now.day())?;
    if day.speaking_secs == 0 || day.scenarios_done == 0 {
        found.push(format!("the day does not count the free talks: {day:?}"));
    }
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

/// /LF603/: one-word answers, rōmaji, repeats and reading out help earn at most half the XP of
/// a normally spoken session, and the summary says why.
fn xp_fair(root: &Path) -> Result<Option<String>> {
    let mut mara = session(root, "n5-06-eki")?;
    let examples: Vec<String> = mara
        .scenario
        .goals
        .iter()
        .map(|g| g.help.example.clone())
        .collect();
    for e in &examples {
        mara.turn(e, Input::Spoken)?;
    }
    let mut jonas = session(root, "n5-06-eki")?;
    jonas.turn("はい", Input::Spoken)?;
    jonas.turn("いいですね", Input::Spoken)?;
    jonas.turn(&romaji::to_kana("kyouto made"), Input::Typed)?;
    jonas.turn("はい", Input::Spoken)?;
    for e in &examples {
        while jonas.help().is_some_and(|h| h.level < 3) {}
        jonas.turn(e, Input::Spoken)?;
    }
    let (m, j) = (mara.summary().xp, jonas.summary().xp);
    if f64::from(j.xp) > 0.5 * f64::from(m.xp) {
        return Ok(Some(format!("Jonas {} XP, Mara {} XP", j.xp, m.xp)));
    }
    if j.explain("de").is_empty() {
        return Ok(Some("the summary does not say why".into()));
    }
    Ok(None)
}

/// /LF213/: after three evasive answers the task gets easier (help with a sentence start).
fn stuck(root: &Path) -> Result<Option<String>> {
    let mut s = session(root, "n5-06-eki")?;
    s.turn("はい", Input::Spoken)?;
    s.turn("いいですね", Input::Spoken)?;
    let third = s.turn("ええと", Input::Spoken)?;
    Ok(match third.help {
        Some(h) if h.level >= 2 => None,
        Some(h) => Some(format!("help only at level {}", h.level)),
        None => Some("no help after three evasive answers".into()),
    })
}

/// /LF120/: typed turns are marked and add no speaking time; rōmaji is shown as kana.
fn typed(root: &Path) -> Result<Option<String>> {
    let kana = romaji::to_kana("kyouto made onegaishimasu");
    if romaji::has_romaji(&kana) {
        return Ok(Some(format!("rōmaji left in {kana}")));
    }
    let mut s = session(root, "n5-06-eki")?;
    s.turn(&kana, Input::Typed)?;
    let st = s.stats();
    if st.typed_turns != 1 || st.speaking_secs != 0 {
        return Ok(Some(format!(
            "typed {}, speaking {} s",
            st.typed_turns, st.speaking_secs
        )));
    }
    Ok((s.summary().typed_turns != 1).then(|| "the summary does not count it as typed".into()))
}

/// /LF606/: ten mistakes in a row lock nothing; the next right answer still counts.
fn no_hearts(root: &Path) -> Result<Option<String>> {
    let mut s = session(root, "n5-02-jikoshoukai")?;
    for _ in 0..10 {
        s.turn("コーヒーを好きです。", Input::Spoken)?;
    }
    let out = s.turn("マラです。よろしくおねがいします。", Input::Spoken)?;
    if out.goals_met_now.is_empty() {
        return Ok(Some(
            "after ten mistakes the right answer no longer counts".into(),
        ));
    }
    let c = Curriculum::from_dir(&root.join("data/curriculum"))?;
    Ok(c.next_scenario(&CourseProgress::default())
        .is_none()
        .then(|| "the path is locked".into()))
}

/// /LF211/: after Selin's placement: speed at least 1.0, furigana on every kanji, kana aid on.
fn settings_split(p: &Value) -> Result<Option<String>> {
    let m = placed(p);
    let d = Difficulty::for_learner(&m);
    let mut why = Vec::new();
    if d.speed() < 1.0 {
        why.push(format!("speed {}", d.speed()));
    }
    if d.get(Knob::Furigana) != 0 {
        why.push("furigana not on every kanji".to_string());
    }
    if !Difficulty::kana_aid(&m) {
        why.push("no kana aid".to_string());
    }
    Ok((!why.is_empty()).then(|| why.join(", ")))
}

/// /LF211/, /LL204/: a beginner like Hinnerk starts at 0.8 or slower, the app speaks at the
/// difficulty's pace, and "louder" and "clear" sit on the conversation page itself.
fn tts_slow(root: &Path, p: &Value) -> Result<Option<String>> {
    let d = Difficulty::for_learner(&placed(p));
    let mut why = Vec::new();
    if d.speed() > 0.8 {
        why.push(format!("starts at {}", d.speed()));
    }
    let app = std::fs::read_to_string(root.join("app/src/conversation.rs"))?;
    if !app.contains("difficulty.speed()") {
        why.push("the voice ignores the difficulty's pace".into());
    }
    let page = std::fs::read_to_string(root.join("app/qml/ConversationPage.qml"))?;
    for (word, call) in [
        ("Lauter", "conv.setSound(checked, conv.clear)"),
        ("Klar", "conv.setSound(conv.louder, checked)"),
    ] {
        if !page.contains(&format!("page.t(\"{word}\"")) || !page.contains(call) {
            why.push(format!("no '{word}' on the conversation page"));
        }
    }
    Ok((!why.is_empty()).then(|| why.join(", ")))
}

/// /LF202/: the profile names strengths and weaknesses in words; "too easy" for listening
/// lifts the speed one step.
fn profile_visible(p: &Value) -> Result<Option<String>> {
    let mut m = placed(p);
    let words = m.describe("de");
    let listening = &words
        .iter()
        .find(|(s, _)| *s == Skill::Listening)
        .context("listening")?
        .1;
    let kanji = &words
        .iter()
        .find(|(s, _)| *s == Skill::ReadingKanji)
        .context("kanji")?
        .1;
    if !listening.contains("N4") || !kanji.contains("noch nicht") {
        return Ok(Some(format!("{listening} / {kanji}")));
    }
    let mut r = Regulator::new(Difficulty::for_learner(&m));
    let before = r.difficulty.get(Knob::Speed);
    let change = learner_says(&mut m, &mut r, Skill::Listening, true);
    Ok(match change {
        Some(c) if c.to == before + 1 && c.knob == Knob::Speed => None,
        Some(c) => Some(format!("{:?} {} → {}", c.knob, c.from, c.to)),
        None if before + 1 >= Knob::Speed.steps() => None,
        None => Some("no change".into()),
    })
}

/// /LF307/, /LD060/: a dispute is stored, the correction stays marked, it no longer counts.
fn dispute(root: &Path) -> Result<Option<String>> {
    let store = kaiwa_store::Store::in_memory()?;
    let mut s = session(root, "n5-02-jikoshoukai")?;
    s.turn("コーヒーを好きです。", Input::Spoken)?;
    let c = s
        .dispute(0, "コーヒーが好きです")
        .context("no correction to dispute")?;
    store.add_dispute(&kaiwa_store::Dispute {
        at: kaiwa_core::Moment::new(0, 0),
        scenario: s.scenario.id.clone(),
        learner: "コーヒーを好きです。".into(),
        original: c.original,
        better: c.better,
        explanation: c.explanation,
    })?;
    let t = &s.turns()[0];
    if store.disputes()?.len() != 1 {
        return Ok(Some("not stored".into()));
    }
    if t.corrections.is_empty() || t.disputed.is_empty() {
        return Ok(Some("deleted instead of marked".into()));
    }
    Ok((!s.stats().mistakes.is_empty()).then(|| "still counted as a mistake".into()))
}

/// /LF113/: a turn marked as misrecognised adds no mistake to the learner model.
fn misrecognition(root: &Path) -> Result<Option<String>> {
    let mut s = session(root, "n5-02-jikoshoukai")?;
    s.turn("コーヒーを好きです。", Input::Spoken)?;
    s.mark_misrecognized(0);
    let e = s.evidence();
    if !e.mistakes.is_empty() {
        return Ok(Some(format!("mistakes {:?}", e.mistakes)));
    }
    Ok(s.summary()
        .review
        .iter()
        .any(|r| r.contains("が好き"))
        .then(|| "the correction still goes into the review".into()))
}

/// /LF215/: after Emi's placement, skipping is offered at once, up to the keigo path (the first
/// unit with a keigo scenario, n4-13); not so for a beginner.
fn skip(root: &Path, p: &Value) -> Result<Option<String>> {
    let c = Curriculum::from_dir(&root.join("data/curriculum"))?;
    let mut keigo_units = std::collections::BTreeSet::new();
    for u in &c.units {
        for s in &u.scenarios {
            let text = std::fs::read_to_string(root.join(format!("data/scenarios/{s}.json")))?;
            if kaiwa_core::Scenario::from_json(&text)?.register
                == kaiwa_core::scenario::Register::Keigo
            {
                keigo_units.insert(u.id.clone());
            }
        }
    }
    let keigo = |u: &kaiwa_core::curriculum::Unit| keigo_units.contains(&u.id);
    let emi = c.placement_skip(&placed(p), &CourseProgress::default(), keigo);
    let mara = c.placement_skip(
        &placed(&persona(root, "mara")?),
        &CourseProgress::default(),
        keigo,
    );
    if emi.is_empty() {
        return Ok(Some("no skip offered after the placement".into()));
    }
    if !mara.is_empty() {
        return Ok(Some("a beginner is offered to skip".into()));
    }
    // The offer ends right before the first keigo unit on the path.
    let first_keigo = c.units.iter().position(keigo);
    let last = c.units.iter().position(|u| Some(&u.id) == emi.last());
    Ok(match (first_keigo, last) {
        (Some(k), Some(l)) if l + 1 == k => None,
        (Some(k), _) => Some(format!(
            "the offer ends at {:?}, not right before the keigo unit {}",
            emi.last(),
            c.units[k].id
        )),
        (None, _) => None,
    })
}

/// /LF711/: only local parts: nothing leaves the device.
fn transparency() -> Result<Option<String>> {
    let f = kaiwa_ai::privacy::flows(|_| None, "de");
    Ok(f.iter()
        .find(|x| x.cloud || x.sends != "nichts")
        .map(|x| format!("{} sends {}", x.part, x.provider)))
}

/// /LF711/: a cloud tutor sends text; recognition and voice stay local.
fn mixed() -> Result<Option<String>> {
    let env = |k: &str| (k == "KAIWA_PROVIDER").then(|| "anthropic".to_string());
    let f = kaiwa_ai::privacy::flows(env, "de");
    let ok = f.len() == 3
        && !f[0].cloud
        && f[0].sends == "nichts"
        && f[1].cloud
        && f[1].sends.starts_with("Text")
        && !f[2].cloud;
    Ok((!ok).then(|| format!("{f:?}")))
}

/// /LF200/, /LF201/: Selin's uneven profile: listening ≥ n4_low, speaking ≤ n5, no kanji,
/// casual only, each within one step of the truth.
fn profile_split(p: &Value) -> Result<Option<String>> {
    let m = placed(p);
    let mut why = Vec::new();
    let near = |s: Skill, k: &str| {
        let t = p["start_level"][k]
            .as_str()
            .and_then(level_index)
            .unwrap_or(0.0);
        (m.level(s) - t).abs() <= 1.0
    };
    if m.level(Skill::Listening) < 4.0 || !near(Skill::Listening, "listening") {
        why.push(format!("listening {:.1}", m.level(Skill::Listening)));
    }
    if m.level(Skill::Speaking) > 3.0 || !near(Skill::Speaking, "speaking") {
        why.push(format!("speaking {:.1}", m.level(Skill::Speaking)));
    }
    if m.level(Skill::ReadingKanji) != 0.0 {
        why.push("kanji".into());
    }
    if m.level(Skill::Register) != 1.0 {
        why.push(format!("register {}", m.level(Skill::Register)));
    }
    Ok((!why.is_empty()).then(|| why.join(", ")))
}

/// /LF200/, /LF201/: Emi reaches the top of the scale within 8 minutes.
fn ceiling(p: &Value) -> Result<Option<String>> {
    let pl = simulate_placement(p);
    let m = pl.result();
    let mut why = Vec::new();
    for s in [Skill::Listening, Skill::Speaking] {
        if m.level(s) < 7.0 {
            why.push(format!("{s:?} {:.1}", m.level(s)));
        }
    }
    if m.level(Skill::Register) != 3.0 {
        why.push(format!("register {}", m.level(Skill::Register)));
    }
    if m.level(Skill::ReadingKanji) != 3.0 {
        why.push(format!("kanji {}", m.level(Skill::ReadingKanji)));
    }
    let minutes = (pl.results.len() as u32 * TURNS_PER_STAGE).div_ceil(2);
    if minutes > 8 {
        why.push(format!("{minutes} min"));
    }
    Ok((!why.is_empty()).then(|| why.join(", ")))
}

/// /LF303/: between friends the plain form is right: a register correction that only asks
/// for politeness is no mistake there (the recorded tutor flags it anyway, on purpose).
fn casual_ok(root: &Path) -> Result<Option<String>> {
    let mut s = session(root, "n5-09-tomodachi")?;
    let out = s.turn("うん、行く！", Input::Spoken)?;
    if !s.stats().mistakes.is_empty() {
        return Ok(Some(format!("counted: {:?}", s.stats().mistakes)));
    }
    Ok(out
        .shown
        .iter()
        .any(|x| !x.correction.also_possible)
        .then(|| "shown as a mistake".into()))
}
