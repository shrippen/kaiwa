//! The conversation as a QObject for QML. State goes to QML as JSON strings (turns, goals,
//! summary), so the page renders it with Kante components without a C++ model class.
//! The session lives in the profile: saved after every turn (to continue later, /LF121/) and
//! recorded at the end (learner model, difficulty, path, review, day).
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
        /// The character's voice when one is loaded ("" without), e.g. "Übergangsstimme".
        #[qproperty(QString, voice)]
        /// Whether the character speaks its replies (the learner can mute it).
        #[qproperty(bool, voice_on, cxx_name = "voiceOn")]
        /// Why there is no voice, with what to do about it.
        #[qproperty(QString, voice_note, cxx_name = "voiceNote")]
        /// Both speech models are loaded or known to be missing.
        #[qproperty(bool, speech_ready, cxx_name = "speechReady")]
        /// The scenario being played.
        #[qproperty(QString, scenario)]
        /// The character's name, e.g. "田中 · Tanaka".
        #[qproperty(QString, character)]
        /// What the end of the session did: XP and why, the difficulty, new review items.
        #[qproperty(QString, recorded)]
        type Conversation = super::ConversationRust;

        /// Start a scenario (the character's opening line); "" plays the next one on the path.
        #[qinvokable]
        fn start(self: Pin<&mut Conversation>, scenario: &QString);

        /// Continue the session that was interrupted (/LF121/).
        #[qinvokable]
        fn resume(self: Pin<&mut Conversation>);

        /// The learner's turn at `log_index` was misrecognised (/LF113/): no learning mistakes.
        #[qinvokable]
        #[cxx_name = "markMisrecognized"]
        fn mark_misrecognized(self: Pin<&mut Conversation>, log_index: i32);

        /// The learner disputes the correction `better` of the turn at `log_index` (/LF307/).
        #[qinvokable]
        fn dispute(self: Pin<&mut Conversation>, log_index: i32, better: &QString);

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

        /// Stop the character speaking (talk button tapped while it speaks).
        #[qinvokable]
        fn interrupt(self: Pin<&mut Conversation>);
    }

    impl cxx_qt::Threading for Conversation {}
}

use core::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::QString;
use kaiwa_ai::ChatProvider;
use kaiwa_ai::mock::MockProvider;
use kaiwa_core::feedback::{Shown, kind_word, severity_word};
use kaiwa_core::session::SessionState;
use kaiwa_core::{Input, Scenario, Session, Summary, TurnOutcome};
use kaiwa_speech::SpeechError;
use kaiwa_speech::capture::{Capture, Recording};
use kaiwa_speech::models::{KOKORO_URL, REAZONSPEECH_URL, kokoro_dir, reazonspeech_dir};
use kaiwa_speech::playback::Player;
use kaiwa_speech::stt::Recognizer;
use kaiwa_speech::tts::{self, Voice};
use serde_json::{Value, json};

use crate::state::{now, state};

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
    voice: QString,
    voice_on: bool,
    voice_note: QString,
    speech_ready: bool,
    scenario: QString,
    character: QString,
    recorded: QString,
    lang: String,
    session_id: Option<i64>,
    started: Option<kaiwa_core::Moment>,
    session: Option<Arc<Mutex<Session>>>,
    log: Vec<Value>,
    recognizer: Option<Arc<Recognizer>>,
    capture: Option<Capture>,
    speaker: Option<Speaker>,
    /// Counts what the character said; speaking stops when it moves on.
    utterance: Arc<AtomicU64>,
}

/// The interim voice and the speaker it plays through.
#[derive(Clone)]
struct Speaker {
    voice: Arc<Mutex<Voice>>,
    player: Arc<Player>,
}

const READY: i32 = 0;
const LISTENING: i32 = 1;
const THINKING: i32 = 2;
const SPEAKING: i32 = 3;
const ERROR: i32 = 4;

/// Tanaka speaks slowly (the scenario says so), so a little below Kokoro's normal pace.
const SPEED: f32 = 0.9;

/// The tutor from the environment (`kaiwa_ai::provider_from_env`, for the owner's local
/// test); recorded answers otherwise (demo mode).
fn provider(scenario: &str) -> (Arc<dyn ChatProvider>, bool) {
    match kaiwa_ai::provider_from_env() {
        Ok(Some(p)) => return (p, false),
        Ok(None) => {}
        Err(e) => eprintln!("kaiwa: {e}; using recorded answers"),
    }
    let mock = crate::state::fixtures(scenario)
        .and_then(|f| MockProvider::from_json(f).ok())
        .unwrap_or_default()
        .with_fallback("すみません、もう一度お願いします。");
    (Arc::new(mock), true)
}

fn shown_json(s: &Shown, lang: &str) -> Value {
    json!({
        "tier": s.tier,
        "tierText": if s.correction.also_possible { if lang == "de" { "Auch möglich" } else { "Also possible" } } else { severity_word(s.correction.severity, lang) },
        "meta": kind_word(s.correction.kind, lang),
        "parts": s.parts,
        "better": s.correction.better,
        "explanation": s.correction.explanation,
        "interrupts": s.interrupts,
    })
}

fn summary_json(s: &Summary, lang: &str) -> Value {
    json!({
        "goalsMet": s.goals_met.len(),
        "goalsTotal": s.goals_total,
        "top": s.top.iter().map(|x| shown_json(x, lang)).collect::<Vec<_>>(),
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
        let lang = self.lang.clone();
        let session_id = self.session_id;
        let started = self.started;
        let guard = session.lock().expect("session lock");
        let met = guard.summary().goals_met;
        let goals: Vec<Value> = guard
            .goals()
            .iter()
            .map(|g| json!({ "id": g.id, "text": g.text.get(&lang), "done": met.contains(&g.id) }))
            .collect();
        let done = guard.done();
        let summary = summary_json(&guard.summary(), &lang).to_string();
        // Keep the state after every turn; record the session once it is done.
        let recorded = {
            let st = state();
            match session_id {
                Some(id) if done && !*self.finished() => {
                    let end = now();
                    match st.store.record(
                        id,
                        &guard,
                        started.unwrap_or(end),
                        end,
                        Some(&st.curriculum),
                    ) {
                        Ok(r) => Some(recorded_json(&r, &guard, st.store.profile().ok(), &lang)),
                        Err(e) => {
                            eprintln!("kaiwa: {e}");
                            None
                        }
                    }
                }
                Some(id) if !done => {
                    if let Err(e) = st.store.save_session(id, &guard.snapshot()) {
                        eprintln!("kaiwa: {e}");
                    }
                    None
                }
                _ => None,
            }
        };
        drop(guard);
        self.as_mut()
            .set_goals(QString::from(&Value::Array(goals).to_string()));
        if let Some(r) = recorded {
            self.as_mut().set_recorded(QString::from(&r.to_string()));
        }
        if done {
            self.as_mut().set_summary(QString::from(&summary));
            self.as_mut().set_finished(true);
        }
    }

    /// Shows a session (new or restored) and loads speech once.
    fn begin(
        mut self: Pin<&mut Self>,
        session: Session,
        id: Option<i64>,
        demo: bool,
        log: Vec<Value>,
        lang: String,
    ) {
        let title = session.scenario.title.get(&lang).to_string();
        let scenario = session.scenario.id.clone();
        let character = session.scenario.name.clone().unwrap_or_else(|| {
            if lang == "de" {
                "Figur".into()
            } else {
                "Character".into()
            }
        });
        let voice = session
            .scenario
            .voice
            .clone()
            .unwrap_or_else(|| "jf_tebukuro".into());
        let opening = (log.len() == 1).then(|| session.opening().to_string());
        {
            let mut rust = self.as_mut().rust_mut();
            rust.session = Some(Arc::new(Mutex::new(session)));
            rust.log = log;
            rust.lang = lang;
            rust.session_id = id;
            rust.started = Some(now());
        }
        self.as_mut().set_title(QString::from(&title));
        self.as_mut().set_scenario(QString::from(&scenario));
        self.as_mut().set_character(QString::from(&character));
        self.as_mut().set_recorded(QString::default());
        self.as_mut().set_summary(QString::default());
        self.as_mut().set_demo(demo);
        self.as_mut().set_finished(false);
        self.as_mut().set_error(QString::default());
        self.as_mut().set_help_text(QString::default());
        self.as_mut().set_help_level(0);
        self.as_mut().publish_turns();
        self.as_mut().publish_goals();
        if *self.speech_ready() {
            // Speech is loaded already: speak the opening of the new scenario.
            if let Some(o) = opening {
                self.as_mut().speak(&o);
            }
            return;
        }
        self.as_mut().set_voice_on(true);
        self.as_mut().set_talk_state(THINKING);
        let thread = self.qt_thread();
        let lang = self.lang.clone();
        // One after the other: both models share ONNX Runtime, which the first sets up.
        std::thread::spawn(move || {
            load_recognizer(&thread, &lang);
            load_voice(&thread, &voice, &lang);
        });
    }

    /// The session's turn index of a log entry (learner entries before it).
    fn turn_of(&self, log_index: i32) -> Option<usize> {
        let i = usize::try_from(log_index).ok()?;
        (self.log.get(i)?["who"] == "learner").then(|| {
            self.log[..i]
                .iter()
                .filter(|e| e["who"] == "learner")
                .count()
        })
    }

    fn save(&self) {
        if let (Some(session), Some(id)) = (&self.session, self.session_id)
            && let Ok(s) = session.lock()
            && let Err(e) = state().store.save_session(id, &s.snapshot())
        {
            eprintln!("kaiwa: {e}");
        }
    }

    fn mark_misrecognized(mut self: Pin<&mut Self>, log_index: i32) {
        let Some(turn) = self.turn_of(log_index) else {
            return;
        };
        if let Some(session) = &self.session
            && let Ok(mut s) = session.lock()
        {
            s.mark_misrecognized(turn);
        }
        if let Some(e) = self.as_mut().rust_mut().log.get_mut(log_index as usize) {
            e["corrections"] = json!([]);
            e["misrecognized"] = json!(true);
        }
        self.save();
        self.as_mut().publish_turns();
    }

    fn dispute(mut self: Pin<&mut Self>, log_index: i32, better: &QString) {
        let better = better.to_string();
        let Some(turn) = self.turn_of(log_index) else {
            return;
        };
        let found = self.session.as_ref().and_then(|session| {
            let mut s = session.lock().ok()?;
            let learner = s.turns().get(turn)?.learner.clone();
            let c = s.dispute(turn, &better)?;
            Some((s.scenario.id.clone(), learner, c))
        });
        let Some((scenario, learner, c)) = found else {
            return;
        };
        if let Err(e) = state().store.add_dispute(&kaiwa_store::Dispute {
            at: now(),
            scenario,
            learner,
            original: c.original,
            better: c.better.clone(),
            explanation: c.explanation,
        }) {
            eprintln!("kaiwa: {e}");
        }
        if let Some(e) = self.as_mut().rust_mut().log.get_mut(log_index as usize)
            && let Some(list) = e["corrections"].as_array_mut()
        {
            for x in list.iter_mut().filter(|x| x["better"] == c.better.as_str()) {
                x["disputed"] = json!(true);
            }
        }
        self.save();
        self.as_mut().publish_turns();
    }

    fn resume(self: Pin<&mut Self>) {
        let found = {
            let st = state();
            let profile = st.store.profile().unwrap_or_default();
            st.store.open_session().ok().flatten().and_then(|row| {
                let state = st.store.session_state(row.id).ok()?;
                let scenario = st.scenarios.get(&state.scenario)?.clone();
                Some((row.id, state, scenario, profile.ui_lang))
            })
        };
        let Some((id, state, scenario, lang)) = found else {
            return;
        };
        let log = log_of(&scenario, &state, &lang);
        let (provider, demo) = provider(&scenario.id);
        let session = Session::restore(scenario, provider, state);
        self.begin(session, Some(id), demo, log, lang);
    }

    fn start(self: Pin<&mut Self>, scenario: &QString) {
        let wanted = scenario.to_string();
        let prepared = {
            let st = state();
            let profile = st.store.profile().unwrap_or_default();
            let learning = st.store.learning().unwrap_or_default();
            let id = if wanted.is_empty() {
                st.curriculum
                    .next_scenario(&learning.course)
                    .unwrap_or_else(|| "n5-02-jikoshoukai".into())
            } else {
                wanted
            };
            st.scenarios
                .get(&id)
                .cloned()
                .map(|s| (s, profile, learning))
        };
        let Some((scenario, profile, learning)) = prepared else {
            return;
        };
        let (provider, demo) = provider(&scenario.id);
        let opening = scenario.opening.clone();
        let session = Session::new(scenario, provider, profile.feedback, &profile.ui_lang)
            .with_difficulty(learning.regulator.difficulty.clone());
        let id = state().store.begin_session(&session.snapshot(), now()).ok();
        let log = vec![json!({ "who": "character", "markup": opening })];
        self.begin(session, id, demo, log, profile.ui_lang);
    }

    /// Says a line of the character (furigana markup) sentence by sentence; the first plays
    /// while the next is made. The talk button shows "speaks" until the speaker is quiet.
    fn speak(mut self: Pin<&mut Self>, markup: &str) {
        let Some(speaker) = self.speaker.clone() else {
            return;
        };
        if !*self.voice_on() {
            return;
        }
        let id = self.utterance.fetch_add(1, Ordering::SeqCst) + 1;
        let current = self.utterance.clone();
        if *self.talk_state() == READY {
            self.as_mut().set_talk_state(SPEAKING);
        }
        let text = kaiwa_core::spoken(markup);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let still = || current.load(Ordering::SeqCst) == id;
            for sentence in tts::sentences(&text) {
                if !still() {
                    return;
                }
                let audio = speaker.voice.lock().map(|mut v| v.speak(&sentence, SPEED));
                match audio {
                    Ok(Ok(samples)) if still() => speaker.player.play(&samples, tts::SAMPLE_RATE),
                    Ok(Err(e)) => eprintln!("kaiwa: {e}"),
                    _ => {}
                }
            }
            while still() && speaker.player.playing() {
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = thread.queue(move |mut q| {
                if q.utterance.load(Ordering::SeqCst) == id && *q.talk_state() == SPEAKING {
                    q.as_mut().set_talk_state(READY);
                }
            });
        });
    }

    fn interrupt(mut self: Pin<&mut Self>) {
        self.utterance.fetch_add(1, Ordering::SeqCst);
        if let Some(speaker) = &self.speaker {
            speaker.player.silence();
        }
        if *self.talk_state() == SPEAKING {
            self.as_mut().set_talk_state(READY);
        }
    }

    fn start_talk(mut self: Pin<&mut Self>) {
        if *self.talk_state() == SPEAKING {
            self.as_mut().interrupt();
        }
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
                let text = speech_text(&e, &self.lang);
                self.as_mut().set_speech_error(QString::from(&text));
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
                let text = speech_text(&e, &self.lang);
                self.as_mut().set_speech_error(QString::from(&text));
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
                    let msg = if q.lang == "en" {
                        "Nothing understood. Once more, a little louder or closer to the microphone."
                    } else {
                        "Nichts verstanden. Noch einmal, etwas lauter oder näher am Mikrofon."
                    };
                    q.as_mut().set_error(QString::from(msg));
                } else {
                    q.as_mut().say(&QString::from(&text), false);
                }
            });
        });
    }

    fn say(mut self: Pin<&mut Self>, text: &QString, typed: bool) {
        let text = text.to_string().trim().to_string();
        // Typed rōmaji becomes kana, shown as such (/LF120/).
        let text = if typed && kaiwa_core::romaji::has_romaji(&text) {
            kaiwa_core::romaji::to_kana(&text)
        } else {
            text
        };
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
                            let lang = rust.lang.clone();
                            if let Some(turn) = rust.log.get_mut(index) {
                                turn["corrections"] = Value::Array(
                                    outcome.shown.iter().map(|x| shown_json(x, &lang)).collect(),
                                );
                                turn["pending"] = json!(false);
                            }
                            rust.log.push(
                                json!({ "who": "character", "markup": outcome.reply.clone() }),
                            );
                        }
                        q.as_mut().set_error(QString::default());
                        q.as_mut().speak(&outcome.reply);
                    }
                    Err(e) => {
                        if let Some(turn) = q.as_mut().rust_mut().log.get_mut(index) {
                            turn["pending"] = json!(false);
                        }
                        let msg = if q.lang == "en" {
                            format!("The tutor did not answer: {e}")
                        } else {
                            format!("Der Tutor hat nicht geantwortet: {e}")
                        };
                        q.as_mut().set_error(QString::from(&msg));
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

/// Loads ReazonSpeech; without the model the talk button shows why.
fn load_recognizer(thread: &CxxQtThread<qobject::Conversation>, lang: &str) {
    let lang = lang.to_string();
    let dir = reazonspeech_dir();
    let loaded = Recognizer::from_dir(&dir, 2);
    let _ = thread.queue(move |mut q| match loaded {
        Ok(rec) => {
            q.as_mut().rust_mut().recognizer = Some(Arc::new(rec));
            q.as_mut().set_speech_error(QString::default());
            if *q.talk_state() != SPEAKING {
                q.as_mut().set_talk_state(READY);
            }
        }
        Err(e) => {
            let text = match e {
                SpeechError::ModelMissing(_) if lang == "de" => format!(
                    "Das Modell für die Spracherkennung fehlt. ReazonSpeech (Apache-2.0, etwa 150 MB) von {REAZONSPEECH_URL} laden und nach {} entpacken. Bis dahin geht Tippen.",
                    dir.display()
                ),
                SpeechError::ModelMissing(_) => format!(
                    "The speech recognition model is missing. Download ReazonSpeech (Apache-2.0, about 150 MB) from {REAZONSPEECH_URL} and unpack it to {}. You can type meanwhile.",
                    dir.display()
                ),
                e => speech_text(&e, &lang),
            };
            q.as_mut().set_speech_error(QString::from(&text));
            q.as_mut().set_talk_state(ERROR);
        }
    });
}

/// Loads Kokoro and opens the speaker, then says the opening line.
fn load_voice(thread: &CxxQtThread<qobject::Conversation>, name: &str, lang: &str) {
    let lang = lang.to_string();
    let dir = kokoro_dir();
    let loaded = Voice::from_dir(&dir, name, 2).and_then(|voice| {
        Ok(Speaker {
            voice: Arc::new(Mutex::new(voice)),
            player: Arc::new(Player::start()?),
        })
    });
    let _ = thread.queue(move |mut q| {
        q.as_mut().set_speech_ready(true);
        match loaded {
        Ok(speaker) => {
            q.as_mut().rust_mut().speaker = Some(speaker);
            q.as_mut().set_voice(QString::from("Übergangsstimme"));
            q.as_mut().set_voice_note(QString::default());
            if q.log.len() == 1 && !*q.busy() {
                let opening = q.log[0]["markup"].as_str().unwrap_or_default().to_string();
                q.speak(&opening);
            }
        }
        Err(e) => {
            let text = match (e, lang == "de") {
                (SpeechError::ModelMissing(_), true) => format!(
                    "Die Figur antwortet ohne Stimme. Für die Übergangsstimme Kokoro (Apache-2.0) kokoro-v1.0.int8.onnx (88 MB) und voices-v1.0.bin (27 MB) von {KOKORO_URL} nach {} laden.",
                    dir.display()
                ),
                (SpeechError::ModelMissing(_), false) => format!(
                    "The character answers without a voice. For the interim voice, download Kokoro (Apache-2.0) kokoro-v1.0.int8.onnx (88 MB) and voices-v1.0.bin (27 MB) from {KOKORO_URL} to {}.",
                    dir.display()
                ),
                (e, true) => format!("Die Figur antwortet ohne Stimme: {}", speech_text(&e, "de")),
                (e, false) => format!("The character answers without a voice: {}", speech_text(&e, "en")),
            };
            q.as_mut().set_voice_note(QString::from(&text));
        }
        }
    });
}

/// What the learner reads when speaking does not work, in the UI language.
fn speech_text(e: &SpeechError, lang: &str) -> String {
    let de = lang == "de";
    match (e, de) {
        (SpeechError::NoMicrophone, true) => {
            "Kein Mikrofon gefunden. Bis dahin geht Tippen.".into()
        }
        (SpeechError::NoMicrophone, false) => "No microphone found. You can type meanwhile.".into(),
        (SpeechError::Microphone(why), true) => {
            format!("Das Mikrofon lässt sich nicht öffnen ({why}). Bis dahin geht Tippen.")
        }
        (SpeechError::Microphone(why), false) => {
            format!("The microphone cannot be opened ({why}). You can type meanwhile.")
        }
        (SpeechError::ModelMissing(dir), true) => {
            format!("Das Modell für die Spracherkennung fehlt in {dir}.")
        }
        (SpeechError::ModelMissing(dir), false) => format!("The speech model is missing in {dir}."),
        (SpeechError::ModelLoad(dir), true) => format!(
            "Das Modell in {dir} lässt sich nicht laden. Neu entpacken; bis dahin geht Tippen."
        ),
        (SpeechError::ModelLoad(dir), false) => {
            format!("The model in {dir} cannot be loaded. Unpack it again; you can type meanwhile.")
        }
        (SpeechError::NoSpeaker, true) => "Kein Lautsprecher gefunden.".into(),
        (SpeechError::NoSpeaker, false) => "No speaker found.".into(),
        (SpeechError::Speaker(why), true) => {
            format!("Der Lautsprecher lässt sich nicht öffnen ({why}).")
        }
        (SpeechError::Speaker(why), false) => format!("The speaker cannot be opened ({why})."),
        (SpeechError::Voice(why), true) => format!("Die Stimme ist ausgefallen ({why})."),
        (SpeechError::Voice(why), false) => format!("The voice failed ({why})."),
    }
}

/// The page's log of a restored session: the opening, then each turn with its corrections and
/// the character's reply.
fn log_of(scenario: &Scenario, state: &SessionState, lang: &str) -> Vec<Value> {
    let mut log = vec![json!({ "who": "character", "markup": scenario.opening })];
    for t in &state.turns {
        let shown = kaiwa_core::feedback::visible(
            if t.misrecognized { &[] } else { &t.corrections },
            state.amount,
        );
        let corrections: Vec<Value> = shown
            .iter()
            .map(|x| {
                let mut v = shown_json(x, lang);
                v["disputed"] = json!(t.disputed.contains(&x.correction.better));
                v
            })
            .collect();
        log.push(json!({
            "who": "learner",
            "text": t.learner,
            "typed": t.input == Input::Typed,
            "corrections": corrections,
            "pending": false,
            "misrecognized": t.misrecognized,
        }));
        log.push(json!({ "who": "character", "markup": t.reply }));
    }
    log
}

/// What the end of the session did, in words (/LF310/, /LF603/, /LF605/).
fn recorded_json(
    r: &kaiwa_store::Recorded,
    s: &Session,
    profile: Option<kaiwa_store::Profile>,
    lang: &str,
) -> Value {
    let gamification = profile.map(|p| p.gamification).unwrap_or(true);
    let change = r.change.as_ref().map(|c| c.words(lang));
    json!({
        "xp": if gamification { Some(r.xp) } else { None },
        "xpWhy": if gamification { s.summary().xp.explain(lang) } else { Vec::new() },
        "change": change,
        "newItems": r.new_items,
        "scenarioDone": r.scenario_done,
        "minutes": r.day.speaking_secs / 60,
    })
}
