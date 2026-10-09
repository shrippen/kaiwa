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
        /// KanteTalkButton.State: 0 ready, 1 listening, 2 thinking, 3 speaking, 4 error.
        #[qproperty(i32, talk_state, cxx_name = "talkState")]
        #[qproperty(f64, level)]
        #[qproperty(QString, speech_error, cxx_name = "speechError")]
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

        /// Start recording (talk button pressed).
        #[qinvokable]
        #[cxx_name = "startTalk"]
        fn start_talk(self: Pin<&mut Conversation>);

        /// Stop recording, recognise and send the turn (talk button released).
        #[qinvokable]
        #[cxx_name = "stopTalk"]
        fn stop_talk(self: Pin<&mut Conversation>);

        /// Refresh `level` while listening (QML polls it).
        #[qinvokable]
        #[cxx_name = "pollLevel"]
        fn poll_level(self: Pin<&mut Conversation>);

        /// Recognise a WAV file as a spoken turn (screenshot mode and tests).
        #[qinvokable]
        #[cxx_name = "sayWav"]
        fn say_wav(self: Pin<&mut Conversation>, path: &QString);
    }

    impl cxx_qt::Threading for Conversation {}
}

use core::pin::Pin;
use std::sync::{Arc, Mutex};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use kaiwa_ai::ChatProvider;
use kaiwa_ai::mock::MockProvider;
use kaiwa_core::feedback::{Shown, kind_word, severity_word};
use kaiwa_core::{Amount, Input, Scenario, Session, Summary, TurnOutcome};
use kaiwa_speech::SpeechError;
use kaiwa_speech::capture::{Capture, Recording};
use kaiwa_speech::models::{REAZONSPEECH_URL, reazonspeech_dir};
use kaiwa_speech::stt::Recognizer;
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
    talk_state: i32,
    level: f64,
    speech_error: QString,
    session: Option<Arc<Mutex<Session>>>,
    log: Vec<Value>,
    recognizer: Option<Arc<Recognizer>>,
    capture: Option<Capture>,
}

const READY: i32 = 0;
const LISTENING: i32 = 1;
const THINKING: i32 = 2;
const ERROR: i32 = 4;

/// The tutor from the environment (`kaiwa_ai::provider_from_env`, for the owner's local
/// test); recorded answers otherwise (demo mode).
fn provider() -> (Arc<dyn ChatProvider>, bool) {
    match kaiwa_ai::provider_from_env() {
        Ok(Some(p)) => return (p, false),
        Ok(None) => {}
        Err(e) => eprintln!("kaiwa: {e}; using recorded answers"),
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
        self.as_mut().publish_goals();
        self.load_recognizer();
    }

    /// Loads ReazonSpeech on a worker thread; without the model the talk button shows why.
    fn load_recognizer(mut self: Pin<&mut Self>) {
        self.as_mut().set_talk_state(THINKING);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let dir = reazonspeech_dir();
            let loaded = Recognizer::from_dir(&dir, 2);
            let _ = thread.queue(move |mut q| match loaded {
                Ok(rec) => {
                    q.as_mut().rust_mut().recognizer = Some(Arc::new(rec));
                    q.as_mut().set_speech_error(QString::default());
                    q.as_mut().set_talk_state(READY);
                }
                Err(e) => {
                    let text = match e {
                        SpeechError::ModelMissing(_) => format!(
                            "Das Modell für die Spracherkennung fehlt. ReazonSpeech (Apache-2.0, etwa 150 MB) von {REAZONSPEECH_URL} laden und nach {} entpacken. Bis dahin geht Tippen.",
                            dir.display()
                        ),
                        e => german(&e),
                    };
                    q.as_mut().set_speech_error(QString::from(&text));
                    q.as_mut().set_talk_state(ERROR);
                }
            });
        });
    }

    fn start_talk(mut self: Pin<&mut Self>) {
        if *self.talk_state() != READY || *self.busy() || *self.finished() {
            return;
        }
        match Capture::start() {
            Ok(c) => {
                self.as_mut().rust_mut().capture = Some(c);
                self.as_mut().set_level(0.0);
                self.as_mut().set_talk_state(LISTENING);
            }
            Err(e) => {
                self.as_mut().set_speech_error(QString::from(&german(&e)));
                self.as_mut().set_talk_state(ERROR);
            }
        }
    }

    fn poll_level(mut self: Pin<&mut Self>) {
        let level = self
            .capture
            .as_ref()
            .map(|c| f64::from(c.level()))
            .unwrap_or(0.0);
        self.as_mut().set_level(level);
    }

    fn stop_talk(mut self: Pin<&mut Self>) {
        let Some(capture) = self.as_mut().rust_mut().capture.take() else {
            return;
        };
        self.as_mut().set_level(0.0);
        match capture.stop() {
            Ok(rec) => self.recognize(rec),
            Err(e) => {
                self.as_mut().set_speech_error(QString::from(&german(&e)));
                self.as_mut().set_talk_state(ERROR);
            }
        }
    }

    fn say_wav(self: Pin<&mut Self>, path: &QString) {
        let path = path.to_string();
        let rec = hound::WavReader::open(&path).ok().map(|mut r| {
            let spec = r.spec();
            let samples: Vec<f32> = match spec.sample_format {
                hound::SampleFormat::Float => r.samples::<f32>().filter_map(Result::ok).collect(),
                hound::SampleFormat::Int => r
                    .samples::<i32>()
                    .filter_map(Result::ok)
                    .map(|s| s as f32 / (1u64 << (spec.bits_per_sample - 1)) as f32)
                    .collect(),
            };
            let channels = usize::from(spec.channels);
            Recording {
                samples: kaiwa_speech::capture::to_mono(&samples, channels).collect(),
                sample_rate: spec.sample_rate,
            }
        });
        if let Some(rec) = rec {
            self.recognize(rec);
        }
    }

    /// Recognises a recording on a worker thread and sends it as a spoken turn.
    fn recognize(mut self: Pin<&mut Self>, rec: Recording) {
        let Some(recognizer) = self.recognizer.clone() else {
            self.as_mut().set_talk_state(ERROR);
            return;
        };
        self.as_mut().set_talk_state(THINKING);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let text = recognizer.transcribe(&rec.samples, rec.sample_rate);
            let _ = thread.queue(move |mut q| {
                q.as_mut().set_talk_state(READY);
                if text.is_empty() {
                    q.as_mut().set_error(QString::from(
                        "Nichts verstanden. Noch einmal, etwas lauter oder näher am Mikrofon.",
                    ));
                } else {
                    q.as_mut().say(&QString::from(&text), false);
                }
            });
        });
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

/// What the learner reads when speaking does not work (the GUI is German for now).
fn german(e: &SpeechError) -> String {
    match e {
        SpeechError::NoMicrophone => "Kein Mikrofon gefunden. Bis dahin geht Tippen.".into(),
        SpeechError::Microphone(why) => {
            format!("Das Mikrofon lässt sich nicht öffnen ({why}). Bis dahin geht Tippen.")
        }
        SpeechError::ModelMissing(dir) => {
            format!("Das Modell für die Spracherkennung fehlt in {dir}.")
        }
        SpeechError::ModelLoad(dir) => format!(
            "Das Modell für die Spracherkennung in {dir} lässt sich nicht laden. Neu entpacken; bis dahin geht Tippen."
        ),
    }
}
