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
        /// Easier listening (/LL204/), from the profile.
        #[qproperty(bool, louder)]
        #[qproperty(bool, clear)]
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
        /// A placement is running (/LF200/): stages at rising levels, nothing recorded.
        #[qproperty(bool, placing)]
        /// The stage, e.g. "2/4".
        #[qproperty(QString, stage)]
        /// The stages are over; the two reading questions are open.
        #[qproperty(bool, placement_ask, cxx_name = "placementAsk")]
        type Conversation = super::ConversationRust;

        /// Start a scenario (the character's opening line); "" plays the next one on the path.
        #[qinvokable]
        fn start(self: Pin<&mut Conversation>, scenario: &QString);

        /// Start the placement by conversation (/LF200/).
        #[qinvokable]
        #[cxx_name = "startPlacement"]
        fn start_placement(self: Pin<&mut Conversation>);

        /// End the placement with what the learner says about reading (0 none, 1 some,
        /// 2 fluently) and keep the result as the learner model.
        #[qinvokable]
        #[cxx_name = "finishPlacement"]
        fn finish_placement(self: Pin<&mut Conversation>, kana: i32, kanji: i32);

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

        /// Louder and clearer voice (/LL204/), kept in the profile.
        #[qinvokable]
        #[cxx_name = "setSound"]
        fn set_sound(self: Pin<&mut Conversation>, louder: bool, clear: bool);

        /// A listening aid on the character's line at `log_index` (/LF111/): 0 says it again,
        /// 1 says it at 0.7×, 2 translates it into the UI language. Each counts as help.
        #[qinvokable]
        fn listen(self: Pin<&mut Conversation>, log_index: i32, aid: i32);
    }

    impl cxx_qt::Threading for Conversation {}
}

use core::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::QString;
use kaiwa_core::feedback::{Shown, kind_word, severity_word};
use kaiwa_core::learner::Skill;
use kaiwa_core::placement::{self, Placement, Reading, STAGES, StageResult, TURNS_PER_STAGE};
use kaiwa_core::session::{Aid, SessionState};
use kaiwa_core::{Input, Scenario, Session, Summary, TurnOutcome};
use kaiwa_speech::SpeechError;
use kaiwa_speech::capture::{Capture, Recording};
use kaiwa_speech::g2p::G2p;
use kaiwa_speech::models::{KANA_CTC_URL, kokoro_dir, models_dir, reazonspeech_dir};
use kaiwa_speech::mora::{self, Checker, Hint, Kind};
use kaiwa_speech::playback::{self, Player};
use kaiwa_speech::stt::Recognizer;
use kaiwa_speech::tts::{self, Voice};
use serde_json::{Value, json};

use crate::state::{now, provider, state};

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
    louder: bool,
    clear: bool,
    voice_note: QString,
    speech_ready: bool,
    scenario: QString,
    character: QString,
    recorded: QString,
    placing: bool,
    stage: QString,
    placement_ask: bool,
    placement: Option<Placement>,
    lang: String,
    session_id: Option<i64>,
    started: Option<kaiwa_core::Moment>,
    session: Option<Arc<Mutex<Session>>>,
    log: Vec<Value>,
    recognizer: Option<Arc<Recognizer>>,
    capture: Option<Capture>,
    speaker: Option<Speaker>,
    /// The mora check (/LF320/); `None` without its model.
    mora: Option<Arc<Mutex<MoraCheck>>>,
    /// Counts what the character said; speaking stops when it moves on.
    utterance: Arc<AtomicU64>,
}

/// The mora check's model and the reading of what the learner meant.
struct MoraCheck {
    checker: Checker,
    g2p: G2p,
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

/// The voice's pace without a session: a little below Kokoro's normal one.
const SPEED: f32 = 0.9;

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
        if done && self.placement.is_none() {
            self.as_mut().set_summary(QString::from(&summary));
            self.as_mut().set_finished(true);
        }
    }

    fn start_placement(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().placement = Some(Placement::default());
        self.as_mut().set_placement_ask(false);
        self.begin_stage(0);
    }

    /// Plays stage `i` of the placement: its scenario at the stage's level, help only on
    /// request, nothing stored.
    fn begin_stage(mut self: Pin<&mut Self>, i: usize) {
        let (id, level) = STAGES[i];
        let prepared = {
            let st = state();
            let profile = st.store.profile().unwrap_or_default();
            st.scenarios.get(id).cloned().map(|s| (s, profile))
        };
        let Some((scenario, profile)) = prepared else {
            return;
        };
        let (provider, demo) = provider(&scenario.id);
        let log = vec![json!({ "who": "character", "markup": scenario.opening.clone() })];
        let session = Session::new(scenario, provider, profile.feedback, &profile.ui_lang)
            .with_difficulty(placement::difficulty_at(level));
        let lang = profile.ui_lang;
        self.as_mut().begin(session, None, demo, log, lang.clone());
        let word = if lang == "en" {
            "Placement"
        } else {
            "Einstufung"
        };
        let title = format!("{word} · {}", self.title());
        self.as_mut().set_title(QString::from(&title));
        self.as_mut()
            .set_stage(QString::from(&format!("{}/{}", i + 1, STAGES.len())));
        self.as_mut().set_placing(true);
    }

    /// After a turn of the placement: the stage ends after its turns (or its goals), then the
    /// next stage follows while the learner keeps up.
    fn placement_step(mut self: Pin<&mut Self>) {
        if self.placement.is_none() {
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };
        let (turns, done, stats) = {
            let s = session.lock().expect("session lock");
            (s.turns().len() as u32, s.done(), s.stats())
        };
        if turns < TURNS_PER_STAGE && !done {
            return;
        }
        let next = {
            let mut rust = self.as_mut().rust_mut();
            let Some(p) = rust.placement.as_mut() else {
                return;
            };
            let level = STAGES[p.results.len()].1;
            p.add(StageResult::from_stats(level, &stats));
            p.next_stage()
        };
        match next {
            Some(i) => self.begin_stage(i),
            None => {
                self.as_mut().set_placement_ask(true);
                self.as_mut().set_finished(true);
            }
        }
    }

    fn finish_placement(mut self: Pin<&mut Self>, kana: i32, kanji: i32) {
        let Some(mut p) = self.as_mut().rust_mut().placement.take() else {
            return;
        };
        let reading = |i: i32| match i {
            2 => Reading::Fluent,
            1 => Reading::Some,
            _ => Reading::None,
        };
        p.kana = reading(kana);
        p.kanji = reading(kanji);
        let mut model = p.result();
        // A talk does not show pronunciation (typed turns count too): it stays as it was.
        let before = state()
            .store
            .learning()
            .map(|l| l.model)
            .unwrap_or_default();
        model.set(Skill::Pronunciation, before.level(Skill::Pronunciation));
        let lang = self.lang.clone();
        let words: Vec<String> = model
            .describe(&lang)
            .into_iter()
            .filter(|(s, _)| *s != Skill::Pronunciation)
            .map(|(_, w)| w)
            .collect();
        if let Err(e) = state().store.set_placement(model) {
            eprintln!("kaiwa: {e}");
        }
        let minutes = (p.results.len() as u32 * TURNS_PER_STAGE).div_ceil(2);
        let r = json!({ "placement": words, "stages": p.results.len(), "minutes": minutes });
        self.as_mut().set_recorded(QString::from(&r.to_string()));
        self.as_mut().set_placement_ask(false);
        self.as_mut().set_placing(false);
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
        let sound = state()
            .store
            .profile()
            .map(|p| (p.louder, p.clear))
            .unwrap_or_default();
        self.as_mut().set_louder(sound.0);
        self.as_mut().set_clear(sound.1);
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
            // Load what was missing and may have been downloaded since (settings, F-08).
            let missing = (
                self.recognizer.is_none(),
                self.speaker.is_none(),
                self.mora.is_none(),
            );
            if missing != (false, false, false) {
                let thread = self.qt_thread();
                let lang = self.lang.clone();
                std::thread::spawn(move || {
                    if missing.0 {
                        load_recognizer(&thread, &lang);
                    }
                    if missing.1 {
                        load_voice(&thread, &voice, &lang);
                    }
                    if missing.2 {
                        load_mora(&thread);
                    }
                });
            }
            // Speak the opening of the new scenario.
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
            load_mora(&thread);
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

    fn resume(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().placement = None;
        self.as_mut().set_placing(false);
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

    fn start(mut self: Pin<&mut Self>, scenario: &QString) {
        self.as_mut().rust_mut().placement = None;
        self.as_mut().set_placing(false);
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
        let _ = state().arm_brake(now());
        let opening = scenario.opening.clone();
        let session = Session::new(scenario, provider, profile.feedback, &profile.ui_lang)
            .with_difficulty(learning.regulator.difficulty.clone());
        let id = state().store.begin_session(&session.snapshot(), now()).ok();
        let log = vec![json!({ "who": "character", "markup": opening })];
        self.begin(session, id, demo, log, profile.ui_lang);
    }

    /// Says a line of the character (furigana markup) sentence by sentence; the first plays
    /// while the next is made. The talk button shows "speaks" until the speaker is quiet.
    fn speak(self: Pin<&mut Self>, markup: &str) {
        let speed = self.speed();
        self.speak_at(markup, speed);
    }

    /// The pace the difficulty sets, 0.7 to 1.1 (/LF211/); eased within the session after
    /// two misses.
    fn speed(&self) -> f32 {
        self.session
            .as_ref()
            .and_then(|s| s.lock().ok().map(|s| s.difficulty.speed() as f32))
            .unwrap_or(SPEED)
    }

    fn speak_at(mut self: Pin<&mut Self>, markup: &str, speed: f32) {
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
        let (louder, clear) = (*self.louder(), *self.clear());
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let still = || current.load(Ordering::SeqCst) == id;
            for sentence in tts::sentences(&text) {
                if !still() {
                    return;
                }
                let audio = speaker.voice.lock().map(|mut v| v.speak(&sentence, speed));
                match audio {
                    Ok(Ok(mut samples)) if still() => {
                        playback::enhance(&mut samples, tts::SAMPLE_RATE, louder, clear);
                        speaker.player.play(&samples, tts::SAMPLE_RATE)
                    }
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

    fn set_sound(mut self: Pin<&mut Self>, louder: bool, clear: bool) {
        {
            let st = state();
            if let Ok(mut p) = st.store.profile() {
                p.louder = louder;
                p.clear = clear;
                if let Err(e) = st.store.set_profile(&p) {
                    eprintln!("kaiwa: {e}");
                }
            }
        }
        self.as_mut().set_louder(louder);
        self.as_mut().set_clear(clear);
    }

    fn listen(mut self: Pin<&mut Self>, log_index: i32, aid: i32) {
        let Some(i) = usize::try_from(log_index).ok() else {
            return;
        };
        let Some(entry) = self.log.get(i).filter(|e| e["who"] == "character") else {
            return;
        };
        let markup = entry["markup"].as_str().unwrap_or_default().to_string();
        let translated = entry.get("translation").is_some();
        let Some(session) = self.session.clone() else {
            return;
        };
        match aid {
            0 | 1 => {
                if let Ok(mut s) = session.lock() {
                    s.aid(if aid == 0 { Aid::Repeat } else { Aid::Slow });
                }
                self.as_mut().interrupt();
                let speed = self.speed() * if aid == 0 { 1.0 } else { 0.7 };
                self.speak_at(&markup, speed);
            }
            2 if !translated => {
                let lang = self.lang.clone();
                let thread = self.qt_thread();
                std::thread::spawn(move || {
                    let line = kaiwa_core::plain(&markup);
                    let result = session.lock().ok().map(|mut s| s.translate(&line));
                    let _ = thread.queue(move |mut q| {
                        match result {
                            Some(Ok(text)) => {
                                if let Some(e) = q.as_mut().rust_mut().log.get_mut(i) {
                                    e["translation"] = json!(text);
                                }
                                q.as_mut().publish_turns();
                            }
                            Some(Err(kaiwa_ai::AiError::NoFixture)) => {
                                let msg = if lang == "en" {
                                    "The recorded answers have no translation for this line."
                                } else {
                                    "Für diese Zeile haben die aufgezeichneten Antworten keine Übersetzung."
                                };
                                q.as_mut().set_error(QString::from(msg));
                            }
                            Some(Err(e)) => {
                                let msg = if lang == "en" {
                                    format!("No translation right now: {e}")
                                } else {
                                    format!("Gerade keine Übersetzung: {e}")
                                };
                                q.as_mut().set_error(QString::from(&msg));
                            }
                            None => {}
                        }
                    });
                });
            }
            _ => {}
        }
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
        // Speaking over the character silences it and starts the new turn (/LF103/).
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
        // Kept only with the opt-in, for the chosen days (/LF712/).
        if let Some(id) = self.session_id {
            let turn = self.log.iter().filter(|e| e["who"] == "learner").count();
            if let Err(e) = state().keep_recording(id, turn, &rec.samples, rec.sample_rate, now()) {
                eprintln!("kaiwa: {e:#}");
            }
        }
        self.as_mut().set_talk_state(THINKING);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let text = recognizer.transcribe(&rec.samples, rec.sample_rate);
            let _ = thread.queue(move |mut q| {
                q.as_mut().set_talk_state(READY);
                if !text.is_empty() {
                    if let Some(index) = q.as_mut().submit(&text, false) {
                        q.check_moras(index, text, rec);
                    }
                    return;
                }
                let msg = if q.lang == "en" {
                    "Nothing understood. Once more, a little louder or closer to the microphone."
                } else {
                    "Nichts verstanden. Noch einmal, etwas lauter oder näher am Mikrofon."
                };
                q.as_mut().set_error(QString::from(msg));
            });
        });
    }

    /// Checks the mora lengths of a spoken turn beside the tutor's answer and adds the hint to
    /// the turn when it comes (/LF320/: only shortenings, only in sentences, at most one, never
    /// a mistake; K-01, K-02, J-05). The recording stays in memory (/LF712/).
    fn check_moras(&self, index: usize, text: String, rec: Recording) {
        let Some(check) = self.mora.clone() else {
            return;
        };
        if !state().store.profile().is_ok_and(|p| p.mora_hints) {
            return;
        }
        let lang = self.lang.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let hints = {
                let Ok(mut c) = check.lock() else {
                    return;
                };
                let Some(words) = c.g2p.pronounced_words(&text) else {
                    return;
                };
                match c.checker.hints(&rec.samples, rec.sample_rate, &words) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("kaiwa: {e}");
                        return;
                    }
                }
            };
            if hints.is_empty() {
                return;
            }
            let _ = thread.queue(move |mut q| {
                if let Some(turn) = q.as_mut().rust_mut().log.get_mut(index)
                    && turn["text"] == text.trim()
                {
                    turn["mora"] =
                        Value::Array(hints.iter().map(|h| hint_json(h, &lang)).collect());
                }
                q.as_mut().publish_turns();
            });
        });
    }

    fn say(self: Pin<&mut Self>, text: &QString, typed: bool) {
        self.submit(&text.to_string(), typed);
    }

    /// Sends a turn to the tutor; returns its index in the log.
    fn submit(mut self: Pin<&mut Self>, text: &str, typed: bool) -> Option<usize> {
        let text = text.trim().to_string();
        // Typed rōmaji becomes kana, shown as such (/LF120/).
        let text = if typed && kaiwa_core::romaji::has_romaji(&text) {
            kaiwa_core::romaji::to_kana(&text)
        } else {
            text
        };
        if text.is_empty() || *self.busy() || *self.finished() {
            return None;
        }
        let session = self.session.clone()?;
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
                        let budget = e == kaiwa_ai::AiError::Budget.to_string();
                        let msg = match (budget, q.lang == "en") {
                            (true, true) => "The cost limit for this month is reached. Raise it in the settings or switch to a local tutor.".to_string(),
                            (true, false) => "Das Kostenlimit für diesen Monat ist erreicht. In den Einstellungen anheben oder auf einen lokalen Tutor wechseln.".to_string(),
                            (false, true) => format!("The tutor did not answer: {e}"),
                            (false, false) => format!("Der Tutor hat nicht geantwortet: {e}"),
                        };
                        q.as_mut().set_error(QString::from(&msg));
                    }
                }
                q.as_mut().set_busy(false);
                q.as_mut().publish_turns();
                q.as_mut().publish_goals();
                // The cost brake: book what the turn cost (/LF714/).
                if let Err(e) = state().book_costs(now(), *q.finished()) {
                    eprintln!("kaiwa: {e}");
                }
                q.placement_step();
            });
        });
        Some(index)
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
                SpeechError::ModelMissing(_) if lang == "de" => "Das Modell für die Spracherkennung fehlt. ReazonSpeech lässt sich in den Einstellungen unter „Modelle“ laden. Bis dahin geht Tippen.".to_string(),
                SpeechError::ModelMissing(_) => "The speech recognition model is missing. Download ReazonSpeech in the settings under “Models”. You can type meanwhile.".to_string(),
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
                (SpeechError::ModelMissing(_), true) => "Die Figur antwortet ohne Stimme. Die Übergangsstimme Kokoro lässt sich in den Einstellungen unter „Modelle“ laden.".to_string(),
                (SpeechError::ModelMissing(_), false) => "The character answers without a voice. Download the interim voice Kokoro in the settings under “Models”.".to_string(),
                (e, true) => format!("Die Figur antwortet ohne Stimme: {}", speech_text(&e, "de")),
                (e, false) => format!("The character answers without a voice: {}", speech_text(&e, "en")),
            };
            q.as_mut().set_voice_note(QString::from(&text));
        }
        }
    });
}

/// Loads the mora check (/LF320/). Without its model there are no hints; the settings say where
/// it comes from (`mora_model_json`).
fn load_mora(thread: &CxxQtThread<qobject::Conversation>) {
    let loaded = Checker::from_dir(&models_dir().join(mora::MODEL), 2)
        .and_then(|checker| Ok((checker, G2p::new()?)));
    match loaded {
        Ok((checker, g2p)) => {
            let check = Arc::new(Mutex::new(MoraCheck { checker, g2p }));
            let _ = thread.queue(move |mut q| q.as_mut().rust_mut().mora = Some(check));
        }
        Err(SpeechError::ModelMissing(_)) => {}
        Err(e) => eprintln!("kaiwa: mora check: {e}"),
    }
}

/// The mora check's model for the settings: whether it is there, where it goes and where it
/// comes from (F-08: source, size and licence before any download).
pub fn mora_model_json() -> Value {
    let dir = models_dir().join(mora::MODEL);
    json!({
        "present": dir.join("model.int8.onnx").exists() || dir.join("model.onnx").exists(),
        "dir": dir.display().to_string(),
        "url": KANA_CTC_URL,
        "size": "356 MB",
        "licence": "Apache-2.0",
    })
}

/// A mora hint as the page shows it: cautious, in the UI language, no severity (K-02).
fn hint_json(h: &Hint, lang: &str) -> Value {
    let de = lang == "de";
    let word = if h.surface == h.meant {
        h.meant.clone()
    } else {
        format!("{} ({})", h.surface, h.meant)
    };
    let text = match (h.kind, de) {
        (Kind::GeminateLeftOut, true) => format!(
            "Das っ in {word} war kaum zu hören, es klang eher wie {}. Vor dem nächsten Laut eine kurze Pause halten, so lang wie eine Silbe. Die Prüfung irrt sich manchmal; das zählt nicht als Fehler.",
            h.heard
        ),
        (Kind::GeminateLeftOut, false) => format!(
            "The っ in {word} was hard to hear; it sounded more like {}. Pause briefly before the next sound, as long as one syllable. The check is sometimes wrong; this does not count as a mistake.",
            h.heard
        ),
        (_, true) => format!(
            "Der lange Vokal in {word} klang eher kurz, wie {}. Ihn etwa doppelt so lang halten. Die Prüfung irrt sich manchmal; das zählt nicht als Fehler.",
            h.heard
        ),
        (_, false) => format!(
            "The long vowel in {word} sounded short, like {}. Hold it about twice as long. The check is sometimes wrong; this does not count as a mistake.",
            h.heard
        ),
    };
    json!({
        "title": format!("{} → {}?", h.meant, h.heard),
        "tierText": if de { "Aussprache · Hinweis" } else { "Pronunciation · hint" },
        "meta": if de { "Moren-Länge" } else { "Mora length" },
        "text": text,
    })
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
