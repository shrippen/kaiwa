//! `kaiwa-sim run`: plays a persona's journey in text mode against the engine.
//!
//! A journey script (`qa/scripts/<journey>.json`) lists what the learner does: silence, asking
//! for help, saying or typing a sentence (with the ids of the errors it plants). The engine
//! is the real `kaiwa_core::Session`; the tutor answers from recorded fixtures
//! (`qa/fixtures/<scenario>.json`, ADR-004: live providers are tried in the owner's local test).
//! Afterwards every check of the journey runs: automated ones pass or fail, the others are
//! listed as not automated yet (GUI, audio or hardware needed).
//!
//! Some checks judge the tutor itself (false alarms, hit rate, explanations, a word asked for
//! in German). With recorded answers they would only check the recordings, so they run only
//! with `--live` (the owner's local test, `spec/LOKALER-TEST.md`) and are listed as "live only"
//! otherwise ([`LIVE`]). A script with `"live_only": true` is played only with `--live`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use kaiwa_ai::Severity;
use kaiwa_ai::mock::MockProvider;
use kaiwa_core::session::HelpStep;
use kaiwa_core::{Amount, Input, Scenario, Session, Summary, TurnOutcome, norm};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Script {
    pub journey: String,
    pub scenario: String,
    /// The recorded answers; a live-only script has none.
    #[serde(default)]
    pub fixtures: String,
    /// Played only with a real tutor (`--live`).
    #[serde(default)]
    pub live_only: bool,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Step {
    Silence {
        silence: f64,
    },
    Help {
        help: u8,
    },
    Say {
        say: String,
        input: String,
        #[serde(default)]
        plants: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
pub enum Event {
    Silence {
        secs: f64,
        help_offered: bool,
    },
    Help(HelpStep),
    Turn {
        say: String,
        input: Input,
        plants: Vec<String>,
        outcome: Box<TurnOutcome>,
    },
}

/// What a check sees: the persona, the session's settings, the events and the summary.
pub struct Run {
    pub persona: Value,
    pub amount: Amount,
    pub scenario: Scenario,
    pub events: Vec<Event>,
    pub summary: Summary,
    /// After the journey, every line of the character translated (/LF111/): the line and the
    /// translation, or why there is none.
    pub translations: Vec<(String, Result<String, String>)>,
    /// The aids the session counted after the translations.
    pub aids: u32,
    pub root: std::path::PathBuf,
    /// Played against a real tutor (`--live`).
    pub live: bool,
    /// A call to the real tutor failed (a limit, the network): the run stopped there, and every
    /// check that needs it fails with this reason.
    pub tutor_error: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "result", content = "detail", rename_all = "snake_case")]
pub enum Outcome {
    Pass,
    Fail(String),
    /// Judges the tutor: runs only with `--live` ([`LIVE`]).
    LiveOnly,
    NotAutomated,
}

/// One journey in the report (/LL604/): every check with its outcome and the requirements it
/// tests; for journeys over simulated days the course of success and difficulty.
#[derive(Debug, Serialize)]
pub struct JourneyReport {
    pub persona: String,
    pub journey: String,
    pub checks: BTreeMap<String, Outcome>,
    pub requirements: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub curve: Vec<CurvePoint>,
}

/// A session of a long journey: its day, its success and the sum of the difficulty's steps.
#[derive(Debug, Serialize)]
pub struct CurvePoint {
    pub day: u32,
    pub success: f64,
    pub level: u32,
}

fn requirements_of(journey: &Value) -> BTreeMap<String, Vec<String>> {
    journey["checks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| {
            let ids = c["requirements"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|r| r.as_str().map(String::from))
                .collect();
            (c["id"].as_str().unwrap_or_default().to_string(), ids)
        })
        .collect()
}

fn amount_of(persona: &Value) -> Amount {
    match persona["learning"]["feedback_preference"].as_str() {
        Some("much") => Amount::Much,
        Some("normal") => Amount::Normal,
        _ => Amount::Little,
    }
}

/// Plays a script. `live`: the tutor from the environment (`kaiwa_ai::provider_from_env`)
/// instead of the recorded answers, for the owner's local test.
pub fn play(root: &Path, persona: &Value, script: &Script, live: bool) -> Result<Run> {
    let provider: Arc<dyn kaiwa_ai::ChatProvider> = if live {
        kaiwa_ai::provider_from_env()
            .map_err(anyhow::Error::msg)?
            .context("--live needs KAIWA_PROVIDER (see app/README.md)")?
    } else {
        let fixtures_path = root.join(format!("qa/fixtures/{}.json", script.fixtures));
        let fixtures = std::fs::read_to_string(&fixtures_path)
            .with_context(|| fixtures_path.display().to_string())?;
        Arc::new(
            MockProvider::from_json(&fixtures)?.with_fallback("すみません、もう一度お願いします。"),
        )
    };
    play_with(root, persona, script, provider, live)
}

/// Plays a script against `provider`; `live` marks the run as one with a real tutor.
pub fn play_with(
    root: &Path,
    persona: &Value,
    script: &Script,
    provider: Arc<dyn kaiwa_ai::ChatProvider>,
    live: bool,
) -> Result<Run> {
    let scenario_path = root.join(format!("data/scenarios/{}.json", script.scenario));
    let scenario = Scenario::from_json(
        &std::fs::read_to_string(&scenario_path)
            .with_context(|| scenario_path.display().to_string())?,
    )?;
    let amount = amount_of(persona);
    let mut session = Session::new(
        scenario.clone(),
        provider,
        amount,
        persona["profile"]["ui_language"].as_str().unwrap_or("de"),
    );
    let mut events = Vec::new();
    let mut tutor_error = None;
    for step in &script.steps {
        match step {
            Step::Silence { silence } => events.push(Event::Silence {
                secs: *silence,
                help_offered: session.offer_help(Duration::from_secs_f64(*silence)),
            }),
            Step::Help { help } => {
                let h = session
                    .help()
                    .context("help asked with every goal reached")?;
                if h.level != *help {
                    bail!(
                        "{}: help reached level {}, the script expects {}",
                        script.journey,
                        h.level,
                        help
                    );
                }
                events.push(Event::Help(h));
            }
            Step::Say { say, input, plants } => {
                let input = if input == "typed" {
                    Input::Typed
                } else {
                    Input::Spoken
                };
                let outcome = match session.turn(say, input) {
                    Ok(o) => o,
                    Err(e) if live => {
                        tutor_error = Some(format!("{e:#}"));
                        break;
                    }
                    Err(e) => return Err(e.into()),
                };
                events.push(Event::Turn {
                    say: say.clone(),
                    input,
                    plants: plants.clone(),
                    outcome: Box::new(outcome),
                });
            }
        }
    }
    let summary = session.summary();
    let lines: Vec<String> = std::iter::once(kaiwa_core::plain(&scenario.opening))
        .chain(turns_of(&events).map(|o| kaiwa_core::plain(&o.reply)))
        .collect();
    let translations = lines
        .into_iter()
        .map(|l| {
            let t = session.translate(&l).map_err(|e| e.to_string());
            (l, t)
        })
        .collect();
    let aids = session.stats().aids;
    Ok(Run {
        persona: persona.clone(),
        amount,
        scenario,
        events,
        summary,
        translations,
        aids,
        root: root.to_path_buf(),
        live,
        tutor_error,
    })
}

/// The checks that judge the tutor, not Kaiwa's code: only with `--live`.
pub const LIVE: &[&str] = &[
    "mara_codeswitch",
    "mara_explain_de",
    "theo_fp",
    "theo_tp",
    "emi_keigo_detect",
    "mara_own_furigana",
];

/// Whether an explanation reads as German: a German function word or an umlaut, and no English
/// one. A heuristic for the local test; the owner reads the explanations too.
fn reads_german(text: &str) -> bool {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphabetic() || is_japanese(c))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let has = |list: &[&str]| words.iter().any(|w| list.contains(&w.as_str()));
    let german = has(&[
        "der", "die", "das", "ist", "nicht", "man", "mit", "für", "und", "ein", "eine", "statt",
        "hier", "wird", "heißt", "sagt", "bei", "nach", "vor", "verlangt", "braucht",
    ]) || text.chars().any(|c| "äöüßÄÖÜ".contains(c));
    let english = has(&["the", "is", "you", "use", "with", "this", "should"]);
    german && !english
}

/// The checks of [`LIVE`] against a run with a real tutor.
pub fn live_check(id: &str, run: &Run) -> Option<Outcome> {
    let fail = |m: String| Some(Outcome::Fail(m));
    let real = |s: &&kaiwa_core::feedback::Shown| !s.correction.also_possible;
    match id {
        // /LF122/: asked in German for a word, the character gives the Japanese one and asks
        // the learner to say it (言って…, 言えますか, 言いましょう; と言います only names it).
        // どうぞ alone is not counted: it also greets (どうぞよろしく, 宜しく, どうぞ、よろしく),
        // and a wrong Fail shows the reply where a wrong Pass would hide it. Hearing it is the
        // local test's.
        "mara_codeswitch" => {
            let (_, o) = turns(run).find(|(p, _)| p.iter().any(|x| x == "mara_codeswitch"))?;
            let reply = kaiwa_core::plain(&o.reply);
            let asks = ["言って", "言えますか", "言いましょう"]
                .iter()
                .any(|f| reply.contains(f));
            if !reply.contains("プロデューサー") {
                fail(format!("no プロデューサー in: {reply}"))
            } else if !asks {
                fail(format!("does not ask to say it: {reply}"))
            } else {
                Some(Outcome::Pass)
            }
        }
        // /LL112/: every explanation in German, with a Japanese example.
        // /LF110/, ADR-004: every line of the learner's with kanji gets the tutor's readings
        // (the dictionary's are only the first guess).
        "mara_own_furigana" => {
            let kanji = |c: char| ('\u{4e00}'..='\u{9fff}').contains(&c);
            for e in &run.events {
                if let Event::Turn { say, outcome, .. } = e
                    && say.chars().any(kanji)
                    && outcome.own_markup.is_empty()
                {
                    return fail(format!("no reading from the tutor for: {say}"));
                }
            }
            Some(Outcome::Pass)
        }
        "mara_explain_de" => {
            for (_, o) in turns(run) {
                for s in &o.shown {
                    let e = &s.correction.explanation;
                    if !e.chars().any(is_japanese) {
                        return fail(format!("no Japanese example: {e}"));
                    }
                    if !reads_german(e) {
                        return fail(format!("not German: {e}"));
                    }
                }
            }
            Some(Outcome::Pass)
        }
        // /LL110/: at most 2 of the correct sentences get a correction of "wrong" or worse.
        "theo_fp" => {
            let alarms: Vec<String> = turns(run)
                .filter(|(p, _)| p.is_empty())
                .filter(|(_, o)| {
                    o.shown
                        .iter()
                        .filter(real)
                        .any(|s| s.correction.severity >= Severity::Wrong)
                })
                .map(|(_, o)| {
                    o.shown
                        .iter()
                        .map(|s| format!("{} → {}", s.correction.original, s.correction.better))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .collect();
            if alarms.len() <= 2 {
                Some(Outcome::Pass)
            } else {
                fail(format!(
                    "{} false alarms: {}",
                    alarms.len(),
                    alarms.join("; ")
                ))
            }
        }
        // /LL111/: at least 5 of the 6 planted errors found, theo_transitive as "wrong".
        "theo_tp" => {
            let planted: Vec<_> = turns(run).filter(|(p, _)| !p.is_empty()).collect();
            let found = planted
                .iter()
                .filter(|(_, o)| o.shown.iter().any(|s| real(&s)))
                .count();
            let transitive = planted
                .iter()
                .find(|(p, _)| p.iter().any(|x| x == "theo_transitive"))
                .is_some_and(|(_, o)| {
                    o.shown
                        .iter()
                        .filter(real)
                        .any(|s| s.correction.severity == Severity::Wrong)
                });
            match (found >= 5, transitive) {
                (true, true) => Some(Outcome::Pass),
                (false, _) => fail(format!("{found} of {} planted errors found", planted.len())),
                (true, false) => fail("theo_transitive not as 'wrong'".into()),
            }
        }
        // /LF300/, /LF302/, /LL111/: every planted register mistake found; 二重敬語 as
        // "unnatural", ご苦労様 as "wrong" with a reason.
        "emi_keigo_detect" => {
            for (p, o) in turns(run).filter(|(p, _)| !p.is_empty()) {
                let id = &p[0];
                let found: Vec<_> = o.shown.iter().filter(real).collect();
                if found.is_empty() {
                    return fail(format!("{id} not found"));
                }
                let want = match id.as_str() {
                    "emi_double_keigo" => Some(Severity::Unnatural),
                    "emi_gokurou" => Some(Severity::Wrong),
                    _ => None,
                };
                // Any of the turn's corrections may carry it (much feedback shows style too).
                let hit = found
                    .iter()
                    .find(|s| want.is_none_or(|w| s.correction.severity == w));
                let Some(s) = hit else {
                    let got: Vec<_> = found.iter().map(|s| s.correction.severity).collect();
                    return fail(format!("{id} as {got:?}"));
                };
                if id == "emi_gokurou" && s.correction.explanation.trim().is_empty() {
                    return fail("emi_gokurou without a reason".into());
                }
            }
            Some(Outcome::Pass)
        }
        _ => None,
    }
}

fn turns_of(events: &[Event]) -> impl Iterator<Item = &TurnOutcome> {
    events.iter().filter_map(|e| match e {
        Event::Turn { outcome, .. } => Some(outcome.as_ref()),
        _ => None,
    })
}

fn turns(run: &Run) -> impl Iterator<Item = (&Vec<String>, &TurnOutcome)> {
    run.events.iter().filter_map(|e| match e {
        Event::Turn {
            plants, outcome, ..
        } => Some((plants, outcome.as_ref())),
        _ => None,
    })
}

fn severity(name: &str) -> Option<Severity> {
    serde_json::from_value(Value::String(name.into())).ok()
}

/// The right forms of a planted error ("A／B" lists alternatives).
fn right_forms(persona: &Value, id: &str) -> Option<(Severity, Vec<String>)> {
    let e = persona["language_errors"]
        .as_array()?
        .iter()
        .find(|e| e["id"] == id)?;
    let sev = severity(e["expected_severity"].as_str()?)?;
    let forms = e["right"].as_str()?.split('／').map(norm).collect();
    Some((sev, forms))
}

fn is_japanese(c: char) -> bool {
    ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c)
}

/// The automated form of a check, by its id. `None`: not automated yet.
pub fn check(id: &str, run: &Run) -> Option<Outcome> {
    let fail = |m: String| Some(Outcome::Fail(m));
    match id {
        // /LF110/, /LF111/: every line of the character can be translated into the UI language,
        // each use counts as help, and the page offers again, slower and translate on every
        // line. That the line is spoken needs the audio mode.
        "mara_listen_tools" => {
            for (line, t) in &run.translations {
                match t {
                    Ok(t) if !t.is_empty() && !t.chars().any(is_japanese) => {}
                    Ok(t) => return fail(format!("{line}: translation '{t}' is not German")),
                    Err(e) => return fail(format!("{line}: no translation ({e})")),
                }
            }
            if run.aids as usize != run.translations.len() {
                return fail(format!(
                    "{} translations counted as {} aids",
                    run.translations.len(),
                    run.aids
                ));
            }
            let page = std::fs::read_to_string(run.root.join("app/qml/ConversationPage.qml"))
                .unwrap_or_default();
            for aid in 0..3 {
                if !page.contains(&format!("conv.listen(msg.index, {aid})")) {
                    return fail(format!("the conversation page has no listening aid {aid}"));
                }
            }
            Some(Outcome::Pass)
        }
        // /LF301/: every correction shown is a diff that spells out both forms, its
        // explanation is German, and the page can play the better form. That it sounds needs
        // the audio mode.
        "mara_diff_audio" => {
            use kaiwa_core::diff::PartKind;
            let mut seen = 0;
            for e in &run.events {
                let Event::Turn { outcome, .. } = e else {
                    continue;
                };
                for s in &outcome.shown {
                    seen += 1;
                    let c = &s.correction;
                    let side = |skip: PartKind| -> String {
                        s.parts
                            .iter()
                            .filter(|p| p.kind != skip)
                            .map(|p| p.text.as_str())
                            .collect()
                    };
                    if side(PartKind::Added) != c.original || side(PartKind::Removed) != c.better {
                        return fail(format!(
                            "the diff of '{}' does not give both forms",
                            c.better
                        ));
                    }
                    if !crate::ui::german(&c.explanation) {
                        return fail(format!("explanation not German: '{}'", c.explanation));
                    }
                }
            }
            if seen == 0 {
                return fail("no correction shown in the journey".into());
            }
            let page = std::fs::read_to_string(run.root.join("app/qml/ConversationPage.qml"))
                .unwrap_or_default();
            if !page.contains("conv.sayText(modelData.better)") {
                return fail("the hint card cannot play the better form".into());
            }
            Some(Outcome::Pass)
        }
        // /LF114/: help after 4 s of silence, three steps, the last a full sentence.
        "mara_hint_offered" => {
            let offered = run
                .events
                .iter()
                .any(|e| matches!(e, Event::Silence { secs, help_offered: true } if *secs >= 4.0));
            let levels: Vec<u8> = run
                .events
                .iter()
                .filter_map(|e| {
                    if let Event::Help(h) = e {
                        Some(h.level)
                    } else {
                        None
                    }
                })
                .collect();
            let full = run.events.iter().find_map(|e| match e {
                Event::Help(h) if h.level == 3 => Some(h.clone()),
                _ => None,
            });
            if !offered {
                return fail("no help offered after 4 s of silence".into());
            }
            if levels != [1, 2, 3] {
                return fail(format!("help levels {levels:?}, expected [1, 2, 3]"));
            }
            match full {
                Some(h)
                    if run
                        .scenario
                        .goal(&h.goal)
                        .is_some_and(|g| g.help.example == h.text) =>
                {
                    Some(Outcome::Pass)
                }
                _ => fail("level 3 is not the goal's full example".into()),
            }
        }
        // /LF304/, /LF306/: little feedback hides style and unnatural; only blocking mistakes interrupt.
        "mara_little_feedback" => {
            if run.amount != Amount::Little {
                return fail(format!("feedback amount {:?}, expected little", run.amount));
            }
            for (_, o) in turns(run) {
                if let Some(s) = o
                    .shown
                    .iter()
                    .find(|s| s.correction.severity < Severity::Wrong)
                {
                    return fail(format!(
                        "shown with little feedback: {:?} {}",
                        s.correction.severity, s.correction.better
                    ));
                }
                let blocks = o
                    .shown
                    .iter()
                    .any(|s| s.correction.severity == Severity::BlocksUnderstanding);
                if o.interrupt.is_some() != blocks {
                    return fail(
                        "an interruption without a blocking mistake, or the other way round".into(),
                    );
                }
            }
            Some(Outcome::Pass)
        }
        // /LL111/: planted errors of severity wrong or above reach the learner.
        "mara_recall" => {
            for (plants, o) in turns(run) {
                for p in plants {
                    let Some((sev, forms)) = right_forms(&run.persona, p) else {
                        continue;
                    };
                    if sev < Severity::Wrong {
                        continue;
                    }
                    if !o
                        .shown
                        .iter()
                        .any(|s| forms.contains(&norm(&s.correction.better)))
                    {
                        return fail(format!("planted error {p} was not shown"));
                    }
                }
            }
            Some(Outcome::Pass)
        }
        // /LF310/: at most three corrections in the summary, all of them go to the review.
        "mara_summary" => {
            if run.summary.top.len() > 3 {
                return fail(format!(
                    "{} corrections in the summary",
                    run.summary.top.len()
                ));
            }
            match run
                .summary
                .top
                .iter()
                .find(|s| !run.summary.review.contains(&s.correction.better))
            {
                Some(s) => fail(format!("{} is not in the review", s.correction.better)),
                None if run.summary.top.is_empty() => fail("no corrections in the summary".into()),
                None => Some(Outcome::Pass),
            }
        }
        // /LF300/, /LF302/, /LF117/: every planted error reaches the learner with the severity
        // its persona expects (register slips unnatural, おまえ wrong), with an explanation.
        "selin_register_flag" => {
            for (plants, o) in turns(run) {
                for p in plants {
                    let Some((sev, _)) = right_forms(&run.persona, p) else {
                        continue;
                    };
                    match o.shown.iter().find(|s| s.correction.severity == sev) {
                        Some(s) if !s.correction.explanation.trim().is_empty() => {}
                        Some(_) => return fail(format!("{p}: shown without an explanation")),
                        None => return fail(format!("{p}: no correction of severity {sev:?}")),
                    }
                }
            }
            Some(Outcome::Pass)
        }
        // /LF305/: after a register slip the character answers politely and goes on.
        "selin_recast" => {
            for (plants, o) in turns(run) {
                if plants.is_empty() {
                    continue;
                }
                let polite = ["です", "ます", "ございます"]
                    .iter()
                    .any(|m| kaiwa_core::plain(&o.reply).contains(m));
                if !polite || o.interrupt.is_some() {
                    return fail(format!(
                        "reply after {plants:?} is not a polite recast: {}",
                        o.reply
                    ));
                }
            }
            Some(Outcome::Pass)
        }
        _ => None,
    }
}

/// Plays every journey of `persona_id` that has a script; journeys without one are reported
/// with all their checks as not automated.
pub fn run_persona(root: &Path, persona_id: &str, live: bool) -> Result<Vec<JourneyReport>> {
    // With --live the tutor's settings are checked once, before anything is played.
    let tutor = if live {
        Some(
            kaiwa_ai::provider_from_env()
                .map_err(anyhow::Error::msg)?
                .context("--live needs KAIWA_PROVIDER (see app/README.md)")?,
        )
    } else {
        None
    };
    run_persona_with(root, persona_id, tutor)
}

/// [`run_persona`] with the tutor given: `Some` plays every script against it (`--live`).
pub fn run_persona_with(
    root: &Path,
    persona_id: &str,
    tutor: Option<Arc<dyn kaiwa_ai::ChatProvider>>,
) -> Result<Vec<JourneyReport>> {
    let path = root.join(format!("qa/personas/{persona_id}.json"));
    let persona: Value = serde_json::from_str(
        &std::fs::read_to_string(&path).with_context(|| path.display().to_string())?,
    )?;
    let mut reports = Vec::new();
    for journey in persona["journeys"].as_array().into_iter().flatten() {
        let jid = journey["id"].as_str().unwrap_or_default().to_string();
        if let Some(days) = crate::days::script_for(root, &jid)? {
            let run = crate::days::simulate(root, &persona, &days)?;
            let mut checks = BTreeMap::new();
            for c in journey["checks"].as_array().into_iter().flatten() {
                let cid = c["id"].as_str().unwrap_or_default();
                let outcome = crate::days::check_seeds(cid, root, &persona, &days)?
                    .or_else(|| crate::days::check(cid, &run))
                    .or_else(|| crate::engine::check(cid, root, &persona))
                    .or_else(|| crate::profile::check(cid, root))
                    .or_else(|| crate::ui::check(cid, root))
                    .unwrap_or(Outcome::NotAutomated);
                checks.insert(cid.to_string(), outcome);
            }
            let curve = run
                .sessions
                .iter()
                .map(|s| CurvePoint {
                    day: s.sim_day,
                    success: s.success,
                    level: s.level,
                })
                .collect();
            reports.push(JourneyReport {
                persona: persona_id.into(),
                journey: jid,
                checks,
                requirements: requirements_of(journey),
                curve,
            });
            continue;
        }
        let script_path = root.join(format!("qa/scripts/{jid}.json"));
        let run = if script_path.exists() {
            let script: Script = serde_json::from_str(&std::fs::read_to_string(&script_path)?)?;
            if script.journey != jid {
                bail!("{} names journey {}", script_path.display(), script.journey);
            }
            match &tutor {
                Some(t) => Some(play_with(root, &persona, &script, t.clone(), true)?),
                None if !script.live_only => Some(play(root, &persona, &script, false)?),
                None => None,
            }
        } else {
            None
        };
        let mut checks = BTreeMap::new();
        for c in journey["checks"].as_array().into_iter().flatten() {
            let cid = c["id"].as_str().unwrap_or_default();
            // With a real tutor one failed call (a limit, the network) stops only this journey:
            // the checks that need its run fail with the reason, the others run as always, and
            // the report still gets written.
            if let Some(why) = run.as_ref().and_then(|r| r.tutor_error.as_ref()) {
                let r = run.as_ref().expect("a run with an error");
                if LIVE.contains(&cid) || check(cid, r).is_some() {
                    checks.insert(cid.to_string(), Outcome::Fail(format!("tutor: {why}")));
                    continue;
                }
            }
            if LIVE.contains(&cid) {
                let outcome = match run.as_ref().filter(|r| r.live) {
                    Some(r) => live_check(cid, r).unwrap_or(Outcome::NotAutomated),
                    None => Outcome::LiveOnly,
                };
                checks.insert(cid.to_string(), outcome);
                continue;
            }
            let outcome = run
                .as_ref()
                .and_then(|r| check(cid, r))
                .or_else(|| crate::engine::check(cid, root, &persona))
                .or_else(|| crate::profile::check(cid, root))
                .or_else(|| crate::ui::check(cid, root))
                .unwrap_or(Outcome::NotAutomated);
            checks.insert(cid.to_string(), outcome);
        }
        reports.push(JourneyReport {
            persona: persona_id.into(),
            journey: jid,
            checks,
            requirements: requirements_of(journey),
            curve: Vec::new(),
        });
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn persona(id: &str) -> Value {
        serde_json::from_str(
            &std::fs::read_to_string(root().join(format!("qa/personas/{id}.json"))).unwrap(),
        )
        .unwrap()
    }

    fn script(journey: &str) -> Script {
        serde_json::from_str(
            &std::fs::read_to_string(root().join(format!("qa/scripts/{journey}.json"))).unwrap(),
        )
        .unwrap()
    }

    /// A stand-in tutor for the live checks: for each sentence of the script, the reply and the
    /// corrections (severity, explanation) `judge` gives it.
    fn tutor(
        script: &Script,
        judge: impl Fn(usize, &str, &[String]) -> (String, Vec<(&'static str, &'static str)>),
    ) -> Arc<dyn kaiwa_ai::ChatProvider> {
        let fixtures: Vec<Value> = script
            .steps
            .iter()
            .filter_map(|s| match s {
                Step::Say { say, plants, .. } => Some((say, plants)),
                _ => None,
            })
            .enumerate()
            .map(|(n, (say, plants))| {
                let (reply, corrections) = judge(n, say, plants);
                serde_json::json!({
                    "learner": say,
                    "reply": reply,
                    "analysis": {
                        "understood": true, "goals_met": [], "new_words": [],
                        "corrections": corrections.iter().map(|(sev, why)| serde_json::json!({
                            "severity": sev, "kind": "register", "original": say,
                            "better": "より良い形", "explanation": why, "also_possible": false,
                        })).collect::<Vec<_>>(),
                    },
                })
            })
            .collect();
        Arc::new(MockProvider::from_json(&Value::Array(fixtures).to_string()).unwrap())
    }

    fn live(
        persona_id: &str,
        journey: &str,
        id: &str,
        t: &dyn Fn(&Script) -> Arc<dyn kaiwa_ai::ChatProvider>,
    ) -> Outcome {
        let s = script(journey);
        let run = play_with(&root(), &persona(persona_id), &s, t(&s), true).unwrap();
        live_check(id, &run).unwrap()
    }

    #[test]
    fn mara_gets_the_tutors_readings_for_her_own_lines() {
        // Every kanji run read as か: kana, and without the markup the line itself.
        let read = |say: &str| {
            let mut out = String::new();
            let mut run = String::new();
            for c in say.chars().chain(std::iter::once('。')) {
                if ('\u{4e00}'..='\u{9fff}').contains(&c) {
                    run.push(c);
                    continue;
                }
                if !run.is_empty() {
                    out.push_str(&format!("{{{run}|か}}"));
                    run.clear();
                }
                out.push(c);
            }
            out.pop();
            out
        };
        let with_readings = |script: &Script| -> Arc<dyn kaiwa_ai::ChatProvider> {
            let fixtures: Vec<Value> = script
                .steps
                .iter()
                .filter_map(|s| match s {
                    Step::Say { say, .. } => Some(serde_json::json!({
                        "learner": say, "reply": "そうですか。",
                        "analysis": { "understood": true, "goals_met": [], "new_words": [],
                                      "corrections": [], "reading": read(say) },
                    })),
                    _ => None,
                })
                .collect();
            Arc::new(MockProvider::from_json(&Value::Array(fixtures).to_string()).unwrap())
        };
        assert_eq!(
            live(
                "mara",
                "mara_first_talk",
                "mara_own_furigana",
                &with_readings
            ),
            Outcome::Pass
        );
        fn plain_tutor(script: &Script) -> Arc<dyn kaiwa_ai::ChatProvider> {
            tutor(script, |_: usize, _: &str, _: &[String]| {
                ("そうですか。".into(), vec![])
            })
        }
        let without = plain_tutor;
        assert!(matches!(
            live("mara", "mara_first_talk", "mara_own_furigana", &without),
            Outcome::Fail(_)
        ));
    }

    #[test]
    fn theo_counts_false_alarms_and_hits() {
        let de = "Hier braucht es が: コーヒーが好きです。";
        // One correct sentence flagged, every planted error found, the door as "wrong".
        let good = |s: &Script| {
            tutor(s, |n, _, p| {
                let c = match (p.first().map(String::as_str), n) {
                    (Some("theo_transitive"), _) => vec![("wrong", de)],
                    (Some(_), _) => vec![("unnatural", de)],
                    (None, 3) => vec![("wrong", de)],
                    (None, _) => vec![],
                };
                ("そうですか。".into(), c)
            })
        };
        assert_eq!(
            live("theo", "theo_false_alarms", "theo_fp", &good),
            Outcome::Pass
        );
        assert_eq!(
            live("theo", "theo_false_alarms", "theo_tp", &good),
            Outcome::Pass
        );
        // Three correct sentences flagged, two planted errors missed.
        let bad = |s: &Script| {
            tutor(s, |n, _, p| {
                let c = match (p.first().map(String::as_str), n) {
                    (Some("theo_transitive" | "theo_anata"), _) => vec![],
                    (Some(_), _) => vec![("wrong", de)],
                    (None, 1..=3) => vec![("wrong", de)],
                    (None, _) => vec![],
                };
                ("そうですか。".into(), c)
            })
        };
        assert!(matches!(
            live("theo", "theo_false_alarms", "theo_fp", &bad),
            Outcome::Fail(_)
        ));
        assert!(matches!(
            live("theo", "theo_false_alarms", "theo_tp", &bad),
            Outcome::Fail(_)
        ));
    }

    #[test]
    fn emi_needs_every_keigo_mistake_at_its_severity() {
        let why = "Gegenüber Vorgesetzten sagt man お疲れ様です。";
        let judge = |double: &'static str| {
            move |s: &Script| {
                tutor(s, move |_, _, p| {
                    let c = match p.first().map(String::as_str) {
                        Some("emi_double_keigo") => vec![("style", why), (double, why)],
                        Some(_) => vec![("wrong", why)],
                        None => vec![],
                    };
                    ("かしこまりました。".into(), c)
                })
            }
        };
        assert_eq!(
            live(
                "emi",
                "emi_interview",
                "emi_keigo_detect",
                &judge("unnatural")
            ),
            Outcome::Pass
        );
        assert!(matches!(
            live("emi", "emi_interview", "emi_keigo_detect", &judge("wrong")),
            Outcome::Fail(_)
        ));
    }

    #[test]
    fn mara_asks_in_german_and_reads_german_explanations() {
        let with = |reply: &'static str, why: &'static str| {
            move |s: &Script| {
                tutor(s, move |_, say, p| {
                    if p.iter().any(|x| x == "mara_codeswitch") {
                        return (reply.into(), vec![]);
                    }
                    let c = if say.contains("を好き") {
                        vec![("wrong", why)]
                    } else {
                        vec![]
                    };
                    ("そうですか。".into(), c)
                })
            }
        };
        let de = "好き verlangt が: コーヒーが好きです。";
        let ok = with("プロデューサーです。言ってみてください。", de);
        let mute = with("「プロデューサー」と言います。", de);
        assert!(matches!(
            live("mara", "mara_first_talk", "mara_codeswitch", &mute),
            Outcome::Fail(_)
        ));
        let greets = with("プロデューサーです。どうぞよろしくお願いします。", de);
        assert!(matches!(
            live("mara", "mara_first_talk", "mara_codeswitch", &greets),
            Outcome::Fail(_)
        ));
        for greeting in [
            "プロデューサーです。どうぞ宜しく。",
            "プロデューサーです。どうぞ、よろしく。",
            "プロデューサー。はい、どうぞ。",
        ] {
            let r = with(greeting, de);
            assert!(matches!(
                live("mara", "mara_first_talk", "mara_codeswitch", &r),
                Outcome::Fail(_)
            ));
        }
        assert_eq!(
            live("mara", "mara_first_talk", "mara_codeswitch", &ok),
            Outcome::Pass
        );
        assert_eq!(
            live("mara", "mara_first_talk", "mara_explain_de", &ok),
            Outcome::Pass
        );
        let off = with(
            "すみません、わかりません。",
            "Use が with 好き: コーヒーが好きです。",
        );
        assert!(matches!(
            live("mara", "mara_first_talk", "mara_codeswitch", &off),
            Outcome::Fail(_)
        ));
        assert!(matches!(
            live("mara", "mara_first_talk", "mara_explain_de", &off),
            Outcome::Fail(_)
        ));
        // Without --live the tutor checks are listed, not run.
        let reports = run_persona(&root(), "theo", false).unwrap();
        let j = reports
            .iter()
            .find(|r| r.journey == "theo_false_alarms")
            .unwrap();
        assert_eq!(j.checks["theo_fp"], Outcome::LiveOnly);
    }

    /// A tutor that answers the first `n` calls from the recordings, then fails like a limit.
    struct Flaky(MockProvider, std::sync::atomic::AtomicUsize, usize);

    impl kaiwa_ai::ChatProvider for Flaky {
        fn reply(&self, r: &kaiwa_ai::TurnRequest) -> Result<String, kaiwa_ai::AiError> {
            if self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= self.2 {
                return Err(kaiwa_ai::AiError::NoFixture);
            }
            self.0.reply(r)
        }
        fn analyse(
            &self,
            r: &kaiwa_ai::TurnRequest,
        ) -> Result<kaiwa_ai::Analysis, kaiwa_ai::AiError> {
            self.0.analyse(r)
        }
    }

    #[test]
    fn a_tutor_error_stops_only_the_checks_that_need_the_run() {
        let rec = MockProvider::from_json(
            &std::fs::read_to_string(root().join("qa/fixtures/n5-02-jikoshoukai.json")).unwrap(),
        )
        .unwrap()
        .with_fallback("そうですか。");
        let tutor: Arc<dyn kaiwa_ai::ChatProvider> =
            Arc::new(Flaky(rec, std::sync::atomic::AtomicUsize::new(0), 2));
        let reports = run_persona_with(&root(), "mara", Some(tutor)).unwrap();
        let first = reports
            .iter()
            .find(|r| r.journey == "mara_first_talk")
            .unwrap();
        // Needs the run (and the tutor): fails with the reason.
        for id in ["mara_codeswitch", "mara_recall"] {
            assert!(
                matches!(&first.checks[id], Outcome::Fail(why) if why.starts_with("tutor:")),
                "{id}: {:?}",
                first.checks[id]
            );
        }
        // A check by hand stays one; the long journey without a tutor still runs.
        assert_eq!(first.checks["mara_mora_hint"], Outcome::NotAutomated);
        assert_eq!(
            first.checks["mara_ptt"],
            Outcome::Pass,
            "ui checks run as always"
        );
        let days = reports
            .iter()
            .find(|r| r.journey == "mara_30_days")
            .unwrap();
        assert_eq!(days.checks["mara_success_band"], Outcome::Pass);
    }

    #[test]
    fn mara_first_talk_passes_its_automated_checks() {
        let reports = run_persona(&root(), "mara", false).unwrap();
        let first = reports
            .iter()
            .find(|r| r.journey == "mara_first_talk")
            .unwrap();
        for id in [
            "mara_hint_offered",
            "mara_little_feedback",
            "mara_recall",
            "mara_summary",
        ] {
            assert_eq!(first.checks[id], Outcome::Pass, "{id}");
        }
        assert_eq!(first.checks["mara_listen_tools"], Outcome::Pass);
        assert_eq!(
            first.requirements["mara_recall"],
            vec!["/LL111/".to_string()]
        );
        let days = reports
            .iter()
            .find(|r| r.journey == "mara_30_days")
            .unwrap();
        assert!(!days.curve.is_empty(), "the long journey has its curve");
        assert_eq!(first.checks["mara_codeswitch"], Outcome::LiveOnly);
    }

    #[test]
    fn selin_register_passes_its_automated_checks() {
        let reports = run_persona(&root(), "selin", false).unwrap();
        let j = reports
            .iter()
            .find(|r| r.journey == "selin_register")
            .unwrap();
        assert_eq!(j.checks["selin_register_flag"], Outcome::Pass);
        assert_eq!(j.checks["selin_recast"], Outcome::Pass);
        assert_eq!(j.checks["selin_casual_ok"], Outcome::Pass);
    }

    #[test]
    fn a_hidden_planted_error_fails_recall() {
        let persona: Value = serde_json::from_str(
            &std::fs::read_to_string(root().join("qa/personas/mara.json")).unwrap(),
        )
        .unwrap();
        let script: Script = serde_json::from_str(
            &std::fs::read_to_string(root().join("qa/scripts/mara_first_talk.json")).unwrap(),
        )
        .unwrap();
        let mut run = play(&root(), &persona, &script, false).unwrap();
        for e in &mut run.events {
            if let Event::Turn { outcome, .. } = e {
                outcome.shown.clear();
            }
        }
        assert!(matches!(check("mara_recall", &run), Some(Outcome::Fail(_))));
    }
}
