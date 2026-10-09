//! The conversation as a QObject for QML. State goes to QML as JSON strings (turns, goals,
//! summary), so the page renders it with Kante components without a C++ model class.
//! Provider calls run on a worker thread and come back through `qt_thread().queue`, the
//! path spike 0.2 tried (ADR-001).

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, title)]
        #[qproperty(bool, demo)]
        #[qproperty(bool, busy)]
        #[qproperty(QString, turns)]
        #[qproperty(QString, goals)]
        #[qproperty(QString, help_text, cxx_name = "helpText")]
        #[qproperty(i32, help_level, cxx_name = "helpLevel")]
        #[qproperty(bool, finished)]
        #[qproperty(QString, summary)]
        #[qproperty(QString, error)]
        type Conversation = super::ConversationRust;

        /// Start the scenario (the character's opening line).
        #[qinvokable]
        fn start(self: Pin<&mut Conversation>);

        /// The learner's turn, spoken (`typed` false) or typed.
        #[qinvokable]
        fn say(self: Pin<&mut Conversation>, text: &QString, typed: bool);

        /// The next step of help for the current goal.
        #[qinvokable]
        fn help(self: Pin<&mut Conversation>);
    }

    impl cxx_qt::Threading for Conversation {}
}

use core::pin::Pin;
use std::sync::{Arc, Mutex};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use kaiwa_ai::ChatProvider;
use kaiwa_ai::anthropic::AnthropicProvider;
use kaiwa_ai::mock::MockProvider;
use kaiwa_ai::openai::OpenAiCompatible;
use kaiwa_core::feedback::{Shown, kind_word, severity_word};
use kaiwa_core::{Amount, Input, Scenario, Session, Summary, TurnOutcome};
use serde_json::{Value, json};

const SCENARIO: &str = include_str!("../../data/scenarios/n5-02-jikoshoukai.json");
const FIXTURES: &str = include_str!("../../qa/fixtures/n5-02-jikoshoukai.json");
const LANG: &str = "de";

#[derive(Default)]
pub struct ConversationRust {
    title: QString,
    demo: bool,
    busy: bool,
    turns: QString,
    goals: QString,
    help_text: QString,
    help_level: i32,
    finished: bool,
    summary: QString,
    error: QString,
    session: Option<Arc<Mutex<Session>>>,
    log: Vec<Value>,
}

/// The tutor from the environment (for the owner's local test, ADR-004); recorded answers
/// otherwise (demo mode).
///
///   KAIWA_PROVIDER=anthropic  ANTHROPIC_API_KEY=…
///   KAIWA_PROVIDER=openai     KAIWA_OPENAI_BASE=http://localhost:11434/v1  KAIWA_MODEL=…  [KAIWA_API_KEY=…]
fn provider() -> (Arc<dyn ChatProvider>, bool) {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    match env("KAIWA_PROVIDER").as_deref() {
        Some("anthropic") => {
            if let Some(key) = env("ANTHROPIC_API_KEY") {
                return (Arc::new(AnthropicProvider::preset(key)), false);
            }
            eprintln!(
                "kaiwa: KAIWA_PROVIDER=anthropic needs ANTHROPIC_API_KEY; using recorded answers"
            );
        }
        Some("openai") => {
            if let Some(model) = env("KAIWA_MODEL") {
                let base =
                    env("KAIWA_OPENAI_BASE").unwrap_or_else(|| "http://localhost:11434/v1".into());
                let mut p = OpenAiCompatible::new(base, model);
                p.api_key = env("KAIWA_API_KEY").unwrap_or_default();
                return (Arc::new(p), false);
            }
            eprintln!("kaiwa: KAIWA_PROVIDER=openai needs KAIWA_MODEL; using recorded answers");
        }
        _ => {}
    }
    let mock = MockProvider::from_json(FIXTURES)
        .expect("bundled fixtures are valid")
        .with_fallback("すみません、もう一度お願いします。");
    (Arc::new(mock), true)
}

fn shown_json(s: &Shown) -> Value {
    json!({
        "tier": s.tier,
        "tierText": if s.correction.also_possible { if LANG == "de" { "Auch möglich" } else { "Also possible" } } else { severity_word(s.correction.severity, LANG) },
        "meta": kind_word(s.correction.kind, LANG),
        "parts": s.parts,
        "better": s.correction.better,
        "explanation": s.correction.explanation,
        "interrupts": s.interrupts,
    })
}

fn summary_json(s: &Summary) -> Value {
    json!({
        "goalsMet": s.goals_met.len(),
        "goalsTotal": s.goals_total,
        "top": s.top.iter().map(shown_json).collect::<Vec<_>>(),
        "review": s.review,
        "spoken": s.spoken_turns,
        "typed": s.typed_turns,
        "helps": s.helps_used,
    })
}

impl qobject::Conversation {
    fn publish_turns(mut self: Pin<&mut Self>) {
        let text = Value::Array(self.log.clone()).to_string();
        self.as_mut().set_turns(QString::from(&text));
    }

    fn publish_goals(mut self: Pin<&mut Self>) {
        let Some(session) = self.session.clone() else {
            return;
        };
        let session = session.lock().expect("session lock");
        let met = session.summary().goals_met;
        let goals: Vec<Value> = session
            .scenario
            .goals
            .iter()
            .map(|g| json!({ "id": g.id, "text": g.text.get(LANG), "done": met.contains(&g.id) }))
            .collect();
        let done = session.done();
        let summary = summary_json(&session.summary()).to_string();
        drop(session);
        self.as_mut()
            .set_goals(QString::from(&Value::Array(goals).to_string()));
        if done {
            self.as_mut().set_summary(QString::from(&summary));
            self.as_mut().set_finished(true);
        }
    }

    fn start(mut self: Pin<&mut Self>) {
        let scenario = Scenario::from_json(SCENARIO).expect("bundled scenario is valid");
        let (provider, demo) = provider();
        let title = scenario.title.get(LANG).to_string();
        let opening = scenario.opening.clone();
        let session = Session::new(scenario, provider, Amount::Little, LANG);
        {
            let mut rust = self.as_mut().rust_mut();
            rust.session = Some(Arc::new(Mutex::new(session)));
            rust.log = vec![json!({ "who": "character", "markup": opening })];
        }
        self.as_mut().set_title(QString::from(&title));
        self.as_mut().set_demo(demo);
        self.as_mut().set_finished(false);
        self.as_mut().set_error(QString::default());
        self.as_mut().publish_turns();
        self.publish_goals();
    }

    fn say(mut self: Pin<&mut Self>, text: &QString, typed: bool) {
        let text = text.to_string().trim().to_string();
        if text.is_empty() || *self.busy() || *self.finished() {
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };
        let index = self.log.len();
        self.as_mut().rust_mut().log.push(json!({ "who": "learner", "text": text, "typed": typed, "corrections": [], "pending": true }));
        self.as_mut().set_busy(true);
        self.as_mut().set_help_text(QString::default());
        self.as_mut().set_help_level(0);
        self.as_mut().publish_turns();

        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let input = if typed { Input::Typed } else { Input::Spoken };
            let result: Result<TurnOutcome, String> = session
                .lock()
                .map_err(|e| e.to_string())
                .and_then(|mut s| s.turn(&text, input).map_err(|e| e.to_string()));
            let _ = thread.queue(move |mut q| {
                match result {
                    Ok(outcome) => {
                        {
                            let mut rust = q.as_mut().rust_mut();
                            if let Some(turn) = rust.log.get_mut(index) {
                                turn["corrections"] =
                                    Value::Array(outcome.shown.iter().map(shown_json).collect());
                                turn["pending"] = json!(false);
                            }
                            rust.log
                                .push(json!({ "who": "character", "markup": outcome.reply }));
                        }
                        q.as_mut().set_error(QString::default());
                    }
                    Err(e) => {
                        if let Some(turn) = q.as_mut().rust_mut().log.get_mut(index) {
                            turn["pending"] = json!(false);
                        }
                        q.as_mut().set_error(QString::from(&format!(
                            "Der Tutor hat nicht geantwortet: {e}"
                        )));
                    }
                }
                q.as_mut().set_busy(false);
                q.as_mut().publish_turns();
                q.publish_goals();
            });
        });
    }

    fn help(mut self: Pin<&mut Self>) {
        let Some(session) = self.session.clone() else {
            return;
        };
        let step = session.lock().expect("session lock").help();
        if let Some(step) = step {
            self.as_mut().set_help_text(QString::from(&step.text));
            self.as_mut().set_help_level(i32::from(step.level));
        }
    }
}
