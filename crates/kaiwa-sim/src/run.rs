//! `kaiwa-sim run`: plays a persona's journey in text mode against the engine.
//!
//! A journey script (`qa/scripts/<journey>.json`) lists what the learner does: silence, asking
//! for help, saying or typing a sentence (with the ids of the errors it plants). The engine
//! is the real `kaiwa_core::Session`; the tutor answers from recorded fixtures
//! (`qa/fixtures/<scenario>.json`, ADR-004: live providers are tried in the owner's local test).
//! Afterwards every check of the journey runs: automated ones pass or fail, the others are
//! listed as not automated yet (GUI, audio or a live tutor needed).

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
    pub fixtures: String,
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
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "result", content = "detail", rename_all = "snake_case")]
pub enum Outcome {
    Pass,
    Fail(String),
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
    let scenario_path = root.join(format!("data/scenarios/{}.json", script.scenario));
    let scenario = Scenario::from_json(
        &std::fs::read_to_string(&scenario_path)
            .with_context(|| scenario_path.display().to_string())?,
    )?;
    let fixtures_path = root.join(format!("qa/fixtures/{}.json", script.fixtures));
    let fixtures = std::fs::read_to_string(&fixtures_path)
        .with_context(|| fixtures_path.display().to_string())?;
    let provider: Arc<dyn kaiwa_ai::ChatProvider> = if live {
        kaiwa_ai::provider_from_env()
            .map_err(anyhow::Error::msg)?
            .context("--live needs KAIWA_PROVIDER (see app/README.md)")?
    } else {
        Arc::new(
            MockProvider::from_json(&fixtures)?.with_fallback("すみません、もう一度お願いします。"),
        )
    };
    let amount = amount_of(persona);
    let mut session = Session::new(
        scenario.clone(),
        provider,
        amount,
        persona["profile"]["ui_language"].as_str().unwrap_or("de"),
    );
    let mut events = Vec::new();
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
                let outcome = session.turn(say, input)?;
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
    })
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
                    .any(|m| o.reply.contains(m));
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
            Some(play(root, &persona, &script, live)?)
        } else {
            None
        };
        let mut checks = BTreeMap::new();
        for c in journey["checks"].as_array().into_iter().flatten() {
            let cid = c["id"].as_str().unwrap_or_default();
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
        assert_eq!(first.checks["mara_codeswitch"], Outcome::NotAutomated);
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
