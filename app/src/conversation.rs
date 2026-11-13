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
        /// Hands-free (/LF100/): no button to hold; the pause ends the turn.
        #[qproperty(bool, hands_free, cxx_name = "handsFree")]
        /// Two turns sounded cut off by the pause: offer a longer one (/LF101/), in seconds.
        #[qproperty(f64, longer_pause, cxx_name = "longerPause")]
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

        /// Take the skip the placement result offers, up to the unit `last` (/LF215/).
        #[qinvokable]
        #[cxx_name = "skipPlacement"]
        fn skip_placement(self: Pin<&mut Conversation>, last: &QString) -> bool;

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

        /// Stop the character speaking (the page closes).
        #[qinvokable]
        fn interrupt(self: Pin<&mut Conversation>);

        /// The learner cuts the character short (talk button while it speaks, /LF103/): it
        /// falls silent, and hands-free the learner's turn begins.
        #[qinvokable]
        #[cxx_name = "bargeIn"]
        fn barge_in(self: Pin<&mut Conversation>);

        /// After a microphone error: the talk button works again, the next press tries the
        /// microphone anew. Without a recogniser there is nothing to try.
        #[qinvokable]
        #[cxx_name = "retryTalk"]
        fn retry_talk(self: Pin<&mut Conversation>);

        /// Called every minute: unloads the speech models after the idle time of the profile
        /// (/LL012/); they load again when needed.
        #[qinvokable]
        fn idle(self: Pin<&mut Conversation>);

        /// Takes the offered longer pause into the profile.
        #[qinvokable]
        #[cxx_name = "takeLongerPause"]
        fn take_longer_pause(self: Pin<&mut Conversation>);
        /// "So lassen" (T-02): the offer goes, the pause stays.
        #[qinvokable]
        #[cxx_name = "declineLongerPause"]
        fn decline_longer_pause(self: Pin<&mut Conversation>);

        /// Louder and clearer voice (/LL204/), kept in the profile.
        #[qinvokable]
        #[cxx_name = "setSound"]
        fn set_sound(self: Pin<&mut Conversation>, louder: bool, clear: bool);

        /// A listening aid on the character's line at `log_index` (/LF111/): 0 says it again,
        /// 1 says it at 0.7×, 2 translates it into the UI language. Each counts as help.
        #[qinvokable]
        fn listen(self: Pin<&mut Conversation>, log_index: i32, aid: i32);

        /// The word lookup (/LF112/): what a click on word `word` of the character's line at
        /// `log_index` shows, as JSON (reading, meanings, part of speech, target accent), or
        /// `{"missing": true}` without the dictionary.
        #[qinvokable]
        fn lookup(self: Pin<&mut Conversation>, log_index: i32, word: i32) -> QString;

        /// Takes the looked-up word into the review (/LF230/); false when it is there already.
        #[qinvokable]
        fn remember(self: Pin<&mut Conversation>, log_index: i32, word: i32) -> bool;

        /// Says the looked-up word with the character's voice.
        #[qinvokable]
        #[cxx_name = "sayWord"]
        fn say_word(self: Pin<&mut Conversation>, log_index: i32, word: i32);

        /// Says a Japanese text with the character's voice, e.g. the better form of a
        /// correction (/LF301/).
        #[qinvokable]
        #[cxx_name = "sayText"]
        fn say_text(self: Pin<&mut Conversation>, text: &QString);

        /// The grammar of this conversation's unit (/LF220/, /LL112/): JSON list of
        /// {label, explain, examples, level}; empty for free talk and the placement.
        #[qinvokable]
        fn grammar(self: &Conversation) -> QString;
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
use kaiwa_core::placement::{self, Placement, Reading, STAGES, Step, TURNS_PER_STAGE};
use kaiwa_core::session::{Aid, SessionState};
use kaiwa_core::{Input, Scenario, Session, Summary, TurnOutcome};
use kaiwa_speech::SpeechError;
use kaiwa_speech::capture::{Capture, Recording};
use kaiwa_speech::g2p::G2p;
use kaiwa_speech::models::{KANA_CTC_URL, kokoro_dir, models_dir, reazonspeech_dir};
use kaiwa_speech::mora::{self, Checker, Hint, Kind};
use kaiwa_speech::playback::{self, Player};
use kaiwa_speech::stt::Recognizer;
use kaiwa_speech::tts::{self, SpeechCache, Voice};
use kaiwa_speech::{Synthesizer, Transcriber};
use serde_json::{Value, json};

use crate::state::{now, state, views};

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
    hands_free: bool,
    longer_pause: f64,
    /// The pause ended the last recording, and the turns that sounded cut off by it.
    ended_by_pause: bool,
    /// Hands-free: turns in a row the pause ended that gave nothing to send.
    empty_turns: kaiwa_core::turn_end::EmptyTurns,
    cut_offs: kaiwa_core::turn_end::CutOffs,
    /// When the learner stopped speaking, for the response time (/LL010/); handed on to the
    /// reply, which books it at its first sound.
    speech_end: Option<std::time::Instant>,
    /// While listening hands-free: when the turn ends, and when the level was last read.
    turn_end: Option<(kaiwa_core::turn_end::EndOfTurn, std::time::Instant)>,
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
    /// Since when the time in this talk has not been added to the session.
    sitting: Option<std::time::Instant>,
    /// When this sitting began: its day gets the practice.
    began: Option<kaiwa_core::Moment>,
    session: Option<Arc<Mutex<Session>>>,
    log: Vec<Value>,
    /// Any engine behind the traits (/LL504/); ReazonSpeech and Kokoro today.
    recognizer: Option<Arc<dyn Transcriber>>,
    capture: Option<Capture>,
    speaker: Option<Speaker>,
    /// The mora check (/LF320/); `None` without its model.
    mora: Option<Arc<Mutex<MoraCheck>>>,
    /// The last time speech was used; the models were unloaded since when `unloaded`.
    last_use: Option<std::time::Instant>,
    unloaded: bool,
    /// Counts what the character said; speaking stops when it moves on.
    utterance: Arc<AtomicU64>,
    /// The utterance up to which the character is done speaking; it speaks while `utterance`
    /// is ahead of this (the talk state does not say it: it can speak while the learner's
    /// turn is still being recognised, or after a microphone error).
    quiet: Arc<AtomicU64>,
    /// What this conversation already said, so "again" plays at once (/LF111/).
    said: Arc<Mutex<SpeechCache>>,
    /// The voice of this conversation's character (/LF117/); a loaded voice switches to it.
    voice_name: String,
}

/// The mora check's model and the reading of what the learner meant.
struct MoraCheck {
    checker: Checker,
    g2p: G2p,
}

/// The interim voice and the speaker it plays through.
#[derive(Clone)]
struct Speaker {
    voice: Arc<Mutex<dyn Synthesizer>>,
    player: Arc<Player>,
}

const READY: i32 = 0;
const LISTENING: i32 = 1;
const THINKING: i32 = 2;
const SPEAKING: i32 = 3;
const ERROR: i32 = 4;

/// Hands-free: listening stops after this long without any speech.
const HANDS_FREE_WAIT_SECS: f64 = 30.0;
/// Shorter recordings (a tap) are not checked for a voice.
const MIN_CHECKED_SECS: f32 = 0.5;

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
        // Every line as words to look up (/LF112/, Kante 1.33), once per line: the
        // character's with the tutor's furigana, the learner's own with the dictionary's until the
        // tutor's reading comes with its answer (/LF110/, ADR-004).
        for entry in self.as_mut().rust_mut().log.iter_mut() {
            if entry.get("segments").is_some() {
                continue;
            }
            let line = match entry["who"].as_str() {
                Some("character") => {
                    kaiwa_backend::lookup::line(entry["markup"].as_str().unwrap_or_default())
                }
                // With the tutor's reading once its answer is there, before with the
                // dictionary's (ADR-004).
                Some("learner") => match entry["markup"].as_str() {
                    Some(markup) => kaiwa_backend::lookup::line(markup),
                    None => {
                        kaiwa_backend::lookup::own_line(entry["text"].as_str().unwrap_or_default())
                    }
                },
                _ => continue,
            };
            entry["segments"] = line["segments"].clone();
            entry["words"] = line["words"].clone();
        }
        let text = Value::Array(self.log.clone()).to_string();
        self.as_mut().set_turns(QString::from(&text));
    }

    fn publish_goals(mut self: Pin<&mut Self>) {
        let Some(session) = self.session.clone() else {
            return;
        };
        let lang = self.lang.clone();
        let session_id = self.session_id;
        let mut guard = session.lock().expect("session lock");
        // The time since the talk began here, or since it was last kept, counts as spent in
        // it; a talk continued later keeps what it had (/LF121/).
        if let Some(since) = self.sitting {
            // Whole seconds; the rest stays for the next time.
            let secs = since.elapsed().as_secs();
            self.as_mut().rust_mut().sitting = Some(since + Duration::from_secs(secs));
            guard.spend(secs.min(3600) as u32);
        }
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
                    // Practice time is the time spent in the talk, over every sitting; it counts
                    // for the day this sitting began.
                    let begun = end.plus_secs(-i64::from(guard.spent_secs()));
                    let day = self.began.unwrap_or(end).day();
                    match st
                        .store
                        .record_on(id, &guard, begun, end, day, Some(&st.curriculum))
                    {
                        Ok(r) => {
                            guard.close();
                            if let Err(e) = st.track_grammar(&guard, end) {
                                eprintln!("kaiwa: {e:#}");
                            }
                            Some(recorded_json(&r, &guard, st.store.profile().ok(), &lang))
                        }
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

    /// Plays stage `i` of the placement ([`placement::stage_session`]), nothing stored.
    fn begin_stage(mut self: Pin<&mut Self>, i: usize) {
        let id = STAGES[i].0;
        let prepared = {
            let st = state();
            let profile = st.store.profile().unwrap_or_default();
            st.scenarios.get(id).cloned().map(|s| (s, profile))
        };
        let Some((scenario, profile)) = prepared else {
            return;
        };
        let (provider, demo) = state().provider(&scenario.id);
        let log = vec![json!({ "who": "character", "markup": scenario.opening.clone() })];
        let session =
            placement::stage_session(scenario, provider, profile.feedback, &profile.ui_lang, i);
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
        let step = {
            let mut rust = self.as_mut().rust_mut();
            let Some(p) = rust.placement.as_mut() else {
                return;
            };
            p.after_turn(&session.lock().expect("session lock"))
        };
        match step {
            Step::Go => {}
            Step::Next(i) => self.begin_stage(i),
            Step::Over => {
                self.as_mut().set_placement_ask(true);
                self.as_mut().set_finished(true);
            }
            // T-11: too many turns the tutor analysed with gaps. Nothing is asked and nothing
            // stored: the level stays what it was, and the page says why.
            Step::Unjudged => {
                self.as_mut().rust_mut().placement = None;
                let r = json!({ "placement": [], "unjudged": true });
                self.as_mut().set_recorded(QString::from(&r.to_string()));
                self.as_mut().set_placing(false);
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
        // The store keeps what a talk does not show (pronunciation) and the history.
        let model = p.result();
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
        // The skip the result offers (/LF215/); none for a beginner.
        let offer = views::placement_offer(&state()).unwrap_or_default();
        let skip = (!offer.units.is_empty())
            .then(|| json!({ "last": offer.units.last(), "title": offer.title }));
        let r = json!({
            "placement": words,
            "stages": p.results.len(),
            "minutes": minutes,
            "skip": skip,
        });
        self.as_mut().set_recorded(QString::from(&r.to_string()));
        self.as_mut().set_placement_ask(false);
        self.as_mut().set_placing(false);
    }

    fn skip_placement(self: Pin<&mut Self>, last: &QString) -> bool {
        // Only the offer that ends at `last`, once (a second click changes nothing).
        views::take_placement_skip(&state(), &last.to_string()).unwrap_or_else(|e| {
            eprintln!("kaiwa: {e}");
            false
        })
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
        // The cost limit holds from the first call of every talk: a new one, one continued
        // and the placement (/LF714/).
        if let Err(e) = state().arm_brake(now()) {
            eprintln!("kaiwa: {e}");
        }
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
            .map(|p| (p.louder, p.clear, p.hands_free))
            .unwrap_or_default();
        self.as_mut().set_louder(sound.0);
        self.as_mut().set_clear(sound.1);
        self.as_mut().set_hands_free(sound.2);
        {
            let mut rust = self.as_mut().rust_mut();
            rust.session = Some(Arc::new(Mutex::new(session)));
            rust.voice_name = voice.clone();
            rust.log = log;
            rust.lang = lang;
            rust.session_id = id;
            rust.sitting = Some(std::time::Instant::now());
            rust.began = Some(now());
            rust.last_use = Some(std::time::Instant::now());
            // Another conversation: nothing said before counts.
            if let Ok(mut c) = rust.said.lock() {
                c.clear();
            }
            // A new conversation loads what is missing itself (below).
            rust.unloaded = false;
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

    /// The session's turn index of a log entry; a line the tutor never answered has none.
    fn turn_of(&self, log_index: i32) -> Option<usize> {
        kaiwa_backend::talk::turn_of(&self.log, usize::try_from(log_index).ok()?)
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
            && !s.mark_misrecognized(turn)
        {
            return;
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
        let continued = crate::state::state().continued_talk(scenario, state);
        match continued {
            Ok((session, demo)) => self.begin(session, Some(id), demo, log, lang),
            Err(e) => self.cannot_open(&e, &lang),
        }
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
            st.scenarios.get(&id).cloned().map(|s| (s, profile))
        };
        let Some((scenario, profile)) = prepared else {
            return;
        };
        let opening = scenario.opening.clone();
        let talk = state().new_talk(scenario);
        let (session, demo) = match talk {
            Ok(t) => t,
            Err(e) => {
                self.cannot_open(&e, &profile.ui_lang);
                return;
            }
        };
        let id = state().store.begin_session(&session.snapshot(), now()).ok();
        let log = vec![json!({ "who": "character", "markup": opening })];
        self.begin(session, id, demo, log, profile.ui_lang);
    }

    /// The profile could not be read for a talk: the page says so instead of staying empty.
    fn cannot_open(mut self: Pin<&mut Self>, e: &anyhow::Error, lang: &str) {
        eprintln!("kaiwa: {e:#}");
        let msg = if lang == "en" {
            format!("The talk cannot be opened: {e}")
        } else {
            format!("Das Gespräch lässt sich nicht öffnen: {e}")
        };
        self.as_mut().set_error(QString::from(&msg));
    }

    /// Says a line of the character (furigana markup) sentence by sentence; the first plays
    /// while the next is made. The talk button shows "speaks" until the speaker is quiet.
    fn speak(self: Pin<&mut Self>, markup: &str) {
        let speed = self.speed();
        self.speak_at(markup, speed, false);
    }

    /// The reply to a spoken turn: its first sound books the response time (/LL010/).
    fn speak_reply(self: Pin<&mut Self>, markup: &str) {
        let speed = self.speed();
        self.speak_at(markup, speed, true);
    }

    /// The pace the difficulty sets, 0.7 to 1.1 (/LF211/); eased within the session after
    /// two misses.
    fn speed(&self) -> f32 {
        self.session
            .as_ref()
            .and_then(|s| s.lock().ok().map(|s| s.difficulty.speed() as f32))
            .unwrap_or(SPEED)
    }

    fn speak_at(mut self: Pin<&mut Self>, markup: &str, speed: f32, reply: bool) {
        let Some(speaker) = self.speaker.clone() else {
            return;
        };
        if !*self.voice_on() {
            return;
        }
        // Hands-free the microphone is open between turns; it must not hear the character
        // (speakers instead of a headset) and send that as the learner's turn. It opens again
        // when the character is done.
        if *self.hands_free() {
            self.as_mut().close_mic();
        }
        let id = self.utterance.fetch_add(1, Ordering::SeqCst) + 1;
        let current = self.utterance.clone();
        if *self.talk_state() == READY {
            self.as_mut().set_talk_state(SPEAKING);
        }
        let text = kaiwa_core::spoken(markup);
        let (louder, clear) = (*self.louder(), *self.clear());
        let mut speech_end = if reply {
            self.as_mut().rust_mut().speech_end.take()
        } else {
            None
        };
        let said = self.said.clone();
        let name = self.voice_name.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let still = || current.load(Ordering::SeqCst) == id;
            for sentence in tts::sentences(&text) {
                if !still() {
                    return;
                }
                // In the character's own voice (/LF117/); what this conversation said before
                // plays at once (/LF111/).
                let audio = tts::sentence(&*speaker.voice, &said, &name, &sentence, speed);
                match audio {
                    Some(Ok((samples, rate))) if still() => {
                        let mut samples = samples.to_vec();
                        playback::enhance(&mut samples, rate, louder, clear);
                        speaker.player.play(&samples, rate);
                        // The first sound of the reply: the response time (/LL010/).
                        if let Some(t) = speech_end.take() {
                            book_latency(t);
                        }
                    }
                    Some(Err(e)) => eprintln!("kaiwa: {e}"),
                    _ => {}
                }
            }
            while still() && speaker.player.playing() {
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = thread.queue(move |mut q| {
                if q.utterance.load(Ordering::SeqCst) == id {
                    q.quiet.store(id, Ordering::SeqCst);
                    if *q.talk_state() == SPEAKING {
                        q.as_mut().set_talk_state(READY);
                    }
                    q.as_mut().listen_again();
                }
            });
        });
    }

    fn idle(mut self: Pin<&mut Self>) {
        let Some(minutes) = state()
            .store
            .profile()
            .ok()
            .and_then(|p| p.unload_after_min)
        else {
            return;
        };
        let quiet = *self.talk_state() == READY && !*self.busy() && self.capture.is_none();
        let long = self
            .last_use
            .is_some_and(|t| t.elapsed() >= Duration::from_secs(u64::from(minutes) * 60));
        let loaded = self.recognizer.is_some() || self.speaker.is_some() || self.mora.is_some();
        if quiet && long && loaded {
            let mut rust = self.as_mut().rust_mut();
            rust.recognizer = None;
            rust.speaker = None;
            rust.mora = None;
            rust.unloaded = true;
            if let Ok(mut c) = rust.said.lock() {
                c.clear();
            }
            // The readings (UniDic) load again with the next line that needs them.
            kaiwa_lang::unidic::unload();
        }
    }

    /// Notes a use of speech and loads the models again after `idle` unloaded them.
    fn wake(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().last_use = Some(std::time::Instant::now());
        if !std::mem::take(&mut self.as_mut().rust_mut().unloaded) {
            return;
        }
        let voice = self.voice_name.clone();
        let thread = self.qt_thread();
        let lang = self.lang.clone();
        self.as_mut().set_talk_state(THINKING);
        std::thread::spawn(move || {
            load_recognizer(&thread, &lang);
            load_voice(&thread, &voice, &lang);
            load_mora(&thread);
        });
    }

    /// A recognised turn: the first that the pause seems to have cut off offers a longer
    /// pause (/LF101/, T-01).
    fn after_turn(mut self: Pin<&mut Self>, by_pause: bool, text: &str) {
        let now = state()
            .store
            .profile()
            .map(|p| p.end_pause_secs)
            .unwrap_or(1.2);
        if let Some(next) = self
            .as_mut()
            .rust_mut()
            .cut_offs
            .after_turn(by_pause, text, now)
        {
            self.as_mut().set_longer_pause(f64::from(next));
        } else if *self.longer_pause() > 0.0 && f64::from(now) >= *self.longer_pause() {
            // The pause was set at least that long in the meantime: the offer is stale.
            self.as_mut().set_longer_pause(0.0);
        }
    }

    /// Takes the offered pause; it only ever makes the pause longer.
    fn take_longer_pause(mut self: Pin<&mut Self>) {
        let secs = *self.longer_pause() as f32;
        if secs > 0.0 {
            let st = state();
            if let Ok(mut p) = st.store.profile()
                && secs > p.end_pause_secs
            {
                p.end_pause_secs = secs;
                if let Err(e) = st.store.set_profile(&p) {
                    eprintln!("kaiwa: {e}");
                }
            }
        }
        self.as_mut().rust_mut().cut_offs.reset();
        self.as_mut().set_longer_pause(0.0);
    }

    /// Declines the offered pause: it comes back only after two more turns that sound cut off.
    fn decline_longer_pause(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().cut_offs.decline();
        self.as_mut().set_longer_pause(0.0);
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
        // The aid belongs to the clicked line: the turn that answers it (a line already
        // answered gives a past turn, and the aid counts for nothing).
        let line = kaiwa_backend::talk::turns(&self.log[..i]);
        // Already there, or on its way (a second click asks the tutor no second time).
        let translated = entry.get("translation").is_some() || entry["translating"] == true;
        let Some(session) = self.session.clone() else {
            return;
        };
        // Not while the tutor thinks about a turn already sent: the aid would count for it,
        // though the answer was given without (/LF212/).
        if *self.busy() {
            return;
        }
        self.as_mut().wake();
        match aid {
            0 | 1 => {
                // The voice is still loading again after the idle time (`wake` above): the
                // learner hears nothing, so nothing counts; the next click plays (/LF212/).
                if self.speaker.is_none() {
                    return;
                }
                if let Ok(mut s) = session.lock() {
                    let kind = if aid == 0 { Aid::Repeat } else { Aid::Slow };
                    s.aid(kind, line);
                }
                self.save();
                self.as_mut().interrupt();
                let speed = self.speed() * if aid == 0 { 1.0 } else { 0.7 };
                self.speak_at(&markup, speed, false);
            }
            2 if !translated => {
                let lang = self.lang.clone();
                let thread = self.qt_thread();
                if let Some(e) = self.as_mut().rust_mut().log.get_mut(i) {
                    e["translating"] = json!(true);
                }
                self.as_mut().publish_turns();
                // A turn sent meanwhile leaves the clicked line behind: the aid counts for nothing.
                let asked_at = line;
                std::thread::spawn(move || {
                    let line = kaiwa_core::plain(&markup);
                    // Asked without holding the session (the window needs it meanwhile).
                    let asking = session.lock().ok().map(|s| s.translating(&line));
                    let result = asking.map(|a| a.run());
                    let _ = thread.queue(move |mut q| {
                        if let Some(e) = q.as_mut().rust_mut().log.get_mut(i)
                            && let Some(e) = e.as_object_mut()
                        {
                            e.remove("translating");
                        }
                        // What the translation cost (/LF714/), not only with the next turn.
                        if let Err(e) = state().book_costs(now(), false) {
                            eprintln!("kaiwa: {e}");
                        }
                        match result {
                            Some(Ok(text)) => {
                                // It counts as help once the learner has it (/LF212/).
                                if let Ok(mut s) = session.lock() {
                                    s.aid(Aid::Translate, asked_at);
                                }
                                if let Some(e) = q.as_mut().rust_mut().log.get_mut(i) {
                                    e["translation"] = json!(text);
                                }
                                q.save();
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
                        q.as_mut().publish_turns();
                    });
                });
            }
            _ => {}
        }
    }

    /// The word `word` of the line at `log_index` (the character's or the learner's own).
    fn line_word(&self, log_index: i32, word: i32) -> Option<kaiwa_lang::LineWord> {
        let entry = self.log.get(usize::try_from(log_index).ok()?)?;
        serde_json::from_value(entry["words"].get(usize::try_from(word).ok()?)?.clone()).ok()
    }

    fn lookup(self: Pin<&mut Self>, log_index: i32, word: i32) -> QString {
        let Some(w) = self.line_word(log_index, word) else {
            return QString::from("null");
        };
        let lang = self.lang.clone();
        let st = state();
        let mut v = kaiwa_backend::lookup::lookup(&st, &w, &lang);
        if v["missing"] == true {
            // What the download would bring (M-03): offered in the popup, only after a click.
            if let Some(m) = kaiwa_speech::download::MODELS
                .iter()
                .find(|m| m.dir == kaiwa_lang::PACKAGE)
            {
                v["download"] = json!({
                    "dir": m.dir,
                    "name": m.name,
                    "licence": m.licence,
                    "origin": m.origin,
                    "downloadMB": (m.download_bytes() as f64 / 1e6).round(),
                    "diskMB": (m.disk_bytes as f64 / 1e6).round(),
                });
            }
        }
        QString::from(&v.to_string())
    }

    fn grammar(&self) -> QString {
        let st = state();
        let v = kaiwa_backend::views::grammar(&st, &self.scenario().to_string(), &self.lang);
        QString::from(&v.to_string())
    }

    fn remember(self: Pin<&mut Self>, log_index: i32, word: i32) -> bool {
        let Some(w) = self.line_word(log_index, word) else {
            return false;
        };
        let lang = self.lang.clone();
        let st = state();
        let found = kaiwa_backend::lookup::lookup(&st, &w, &lang);
        let text = found["word"].as_str().unwrap_or(&w.key).to_string();
        // The unit as the review counts it (the curriculum's id, as for words from mistakes).
        let unit = st
            .curriculum
            .unit_of(&self.scenario().to_string())
            .map(|u| u.id.clone());
        match kaiwa_backend::lookup::remember(&st, &text, unit.as_deref(), now()) {
            Ok(added) => added,
            Err(e) => {
                eprintln!("kaiwa: {e:#}");
                false
            }
        }
    }

    fn say_word(mut self: Pin<&mut Self>, log_index: i32, word: i32) {
        let Some(w) = self.line_word(log_index, word) else {
            return;
        };
        // The voice says the word as the popup shows it: the entry's spelling and reading
        // (IPADIC reads 四人 よんにん, the dictionary よにん); without the dictionary as written.
        let lang = self.lang.clone();
        let entry = kaiwa_backend::lookup::lookup(&state(), &w, &lang);
        let markup = match (entry["word"].as_str(), entry["reading"].as_str()) {
            (Some(word), Some(reading)) if word != reading => format!("{{{word}|{reading}}}"),
            (Some(word), _) => word.to_string(),
            _ => w.text.clone(),
        };
        self.as_mut().wake();
        self.as_mut().interrupt();
        let speed = self.speed();
        self.speak_at(&markup, speed, false);
    }

    fn say_text(mut self: Pin<&mut Self>, text: &QString) {
        let text = text.to_string();
        self.as_mut().wake();
        self.as_mut().interrupt();
        let speed = self.speed();
        self.speak_at(&text, speed, false);
    }

    fn interrupt(mut self: Pin<&mut Self>) {
        let id = self.utterance.fetch_add(1, Ordering::SeqCst) + 1;
        self.quiet.store(id, Ordering::SeqCst);
        if let Some(speaker) = &self.speaker {
            speaker.player.silence();
        }
        if *self.talk_state() == SPEAKING {
            self.as_mut().set_talk_state(READY);
        }
    }

    /// Whether the character is speaking or about to (a sentence being made).
    fn speaking(&self) -> bool {
        self.utterance.load(Ordering::SeqCst) != self.quiet.load(Ordering::SeqCst)
    }

    fn barge_in(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().empty_turns.reset();
        self.as_mut().interrupt();
        self.listen_again();
    }

    fn retry_talk(mut self: Pin<&mut Self>) {
        if *self.talk_state() == ERROR && self.recognizer.is_some() {
            self.as_mut().set_talk_state(READY);
        }
    }

    /// Hands-free after a turn that gave nothing to send: listens again, but not for ever.
    fn listen_after_nothing(mut self: Pin<&mut Self>) {
        // After the third in a row the message stays and the microphone is closed.
        if self.as_mut().rust_mut().empty_turns.after_empty() {
            self.listen_again();
        }
    }

    /// Stops listening without sending anything (hands-free: the character speaks, or the
    /// learner typed instead).
    fn close_mic(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().turn_end = None;
        if let Some(c) = self.as_mut().rust_mut().capture.take() {
            drop(c.stop());
            self.as_mut().set_level(0.0);
            if *self.talk_state() == LISTENING {
                self.as_mut().set_talk_state(READY);
            }
        }
    }

    /// The talk button: the learner starts a turn.
    fn start_talk(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().empty_turns.reset();
        self.open_mic();
    }

    /// Opens the microphone for a turn (the button, or hands-free by itself).
    fn open_mic(mut self: Pin<&mut Self>) {
        if self.unloaded {
            // The models load again; the next press talks.
            self.as_mut().wake();
            return;
        }
        self.as_mut().wake();
        // Speaking over the character silences it and starts the new turn (/LF103/).
        if *self.talk_state() == SPEAKING || self.speaking() {
            self.as_mut().interrupt();
        }
        if *self.talk_state() != READY || *self.busy() || *self.finished() {
            return;
        }
        let microphone = state().store.profile().ok().and_then(|p| p.microphone);
        match Capture::start_on(microphone.as_deref()) {
            Ok(c) => {
                let turn_end = self.hands_free.then(|| {
                    let pause = state()
                        .store
                        .profile()
                        .map(|p| p.end_pause_secs)
                        .unwrap_or(1.2);
                    (
                        kaiwa_core::turn_end::EndOfTurn::new(pause),
                        std::time::Instant::now(),
                    )
                });
                self.as_mut().rust_mut().turn_end = turn_end;
                self.as_mut().rust_mut().capture = Some(c);
                // The microphone is back (after an error and `retry_talk`).
                self.as_mut().set_speech_error(QString::default());
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
        // Hands-free: nothing said for a while stops listening, without sending anything, so
        // the microphone does not record an empty room for hours. The next click or reply
        // starts it again.
        let silent = self
            .turn_end
            .as_ref()
            .is_some_and(|(e, _)| !e.heard() && e.listened_secs() >= HANDS_FREE_WAIT_SECS);
        if silent {
            self.as_mut().rust_mut().empty_turns.reset();
            self.as_mut().rust_mut().turn_end = None;
            if let Some(c) = self.as_mut().rust_mut().capture.take() {
                drop(c.stop());
            }
            self.as_mut().set_level(0.0);
            self.as_mut().set_talk_state(READY);
            return;
        }
        // Hands-free: the set pause after speaking ends the turn (/LF101/).
        let ended = self
            .as_mut()
            .rust_mut()
            .turn_end
            .as_mut()
            .is_some_and(|(e, last)| {
                let dt = last.elapsed().as_secs_f64();
                *last = std::time::Instant::now();
                e.update(level as f32, dt)
            });
        if ended {
            // The speech ended a pause ago, not now.
            let quiet = self.turn_end.as_ref().map_or(0.0, |(e, _)| e.quiet_secs());
            self.as_mut().rust_mut().ended_by_pause = true;
            self.as_mut().stop_talk();
            let end = std::time::Instant::now()
                .checked_sub(Duration::from_secs_f64(quiet))
                .unwrap_or_else(std::time::Instant::now);
            self.as_mut().rust_mut().speech_end = Some(end);
        }
    }

    /// Hands-free: listen again once the character is done (/LF100/).
    fn listen_again(mut self: Pin<&mut Self>) {
        let ready = *self.talk_state() == READY && !*self.busy() && !*self.finished();
        // Not in the placement: its stages change the scene between turns. And not into the
        // character's voice: it listens again when the character is done.
        let talk =
            ready && self.recognizer.is_some() && self.placement.is_none() && !self.speaking();
        if *self.hands_free() && talk {
            self.as_mut().open_mic();
        }
    }

    fn stop_talk(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().turn_end = None;
        let Some(capture) = self.as_mut().rust_mut().capture.take() else {
            return;
        };
        self.as_mut().rust_mut().speech_end = Some(std::time::Instant::now());
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
        // No voice in it (no microphone, muted, wrong input): say so instead of letting the
        // recognition invent sounds from the silence (/LF104/). A tap too short to measure
        // goes on as before.
        let level = kaiwa_speech::capture::check(&rec);
        if level.seconds >= MIN_CHECKED_SECS && !level.voice() {
            let by_pause = std::mem::take(&mut self.as_mut().rust_mut().ended_by_pause);
            self.as_mut().set_talk_state(READY);
            let msg = if self.lang == "en" {
                "No sound came in. Is a microphone plugged in, switched on and chosen? Settings › Microphone tests it."
            } else {
                "Es kam kein Ton an. Ist ein Mikrofon angeschlossen, eingeschaltet und gewählt? Einstellungen › Mikrofon testet es."
            };
            self.as_mut().set_error(QString::from(msg));
            // Hands-free goes on listening when the pause ended the turn; a click on the
            // button closed the microphone, and it stays closed.
            if by_pause {
                self.listen_after_nothing();
            }
            return;
        }
        // Kept only with the opt-in, for the chosen days (/LF712/).
        if let Some(id) = self.session_id {
            let turn = kaiwa_backend::talk::turns(&self.log);
            if let Err(e) = state().keep_recording(id, turn, &rec.samples, rec.sample_rate, now()) {
                eprintln!("kaiwa: {e:#}");
            }
        }
        self.as_mut().set_talk_state(THINKING);
        // A spoken turn in flight is busy like a typed one: no typed send, help or listening
        // aid meanwhile, or the recognised turn would be dropped or get the other one's index.
        self.as_mut().set_busy(true);
        let by_pause = std::mem::take(&mut self.as_mut().rust_mut().ended_by_pause);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let text = recognizer.transcribe(&rec.samples, rec.sample_rate);
            let _ = thread.queue(move |mut q| {
                q.as_mut().set_busy(false);
                q.as_mut().set_talk_state(READY);
                q.as_mut().after_turn(by_pause, &text);
                if !text.is_empty() {
                    q.as_mut().rust_mut().empty_turns.reset();
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
                if by_pause {
                    q.listen_after_nothing();
                }
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
        if typed {
            // Only spoken turns have a response time.
            self.as_mut().rust_mut().speech_end = None;
            self.as_mut().rust_mut().empty_turns.reset();
            // Typed instead of spoken: hands-free stops listening until the reply is through.
            if *self.hands_free() {
                self.as_mut().close_mic();
            }
        }
        self.as_mut().wake();
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
            // The session is held only to ask and to take the answer, not while the tutor
            // thinks: the window locks it for help and the listening aids.
            let asking = session.lock().map(|s| s.ask(&text));
            let result: Result<TurnOutcome, String> = asking
                .map_err(|e| e.to_string())
                .and_then(|a| a.run().map_err(|e| e.to_string()))
                .and_then(|answer| {
                    session
                        .lock()
                        .map(|mut s| s.take(&text, input, answer))
                        .map_err(|e| e.to_string())
                });
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
                                turn["incomplete"] = json!(outcome.incomplete);
                                // The tutor's furigana replace the dictionary's (ADR-004).
                                if !outcome.own_markup.is_empty() {
                                    turn["markup"] = outcome.own_markup.clone().into();
                                    if let Some(t) = turn.as_object_mut() {
                                        t.remove("segments");
                                        t.remove("words");
                                    }
                                }
                            }
                            rust.log.push(
                                json!({ "who": "character", "markup": outcome.reply.clone() }),
                            );
                        }
                        q.as_mut().set_error(QString::default());
                        q.as_mut().speak_reply(&outcome.reply);
                        // Without a voice the reply's text is the answer.
                        if let Some(t) = q.as_mut().rust_mut().speech_end.take() {
                            book_latency(t);
                        }
                    }
                    Err(e) => {
                        // The line stays to be read, but it is no turn of the session.
                        if let Some(turn) = q.as_mut().rust_mut().log.get_mut(index) {
                            turn["pending"] = json!(false);
                            turn["failed"] = json!(true);
                        }
                        let budget = e == kaiwa_ai::AiError::Budget.to_string();
                        let no_tutor = e == kaiwa_ai::AiError::NoTutor.to_string();
                        let msg = match (no_tutor, q.lang == "en") {
                            (true, true) => "No tutor is set up. Choose one in the settings under Tutor.".to_string(),
                            (true, false) => "Es ist kein Tutor eingerichtet. In den Einstellungen unter „Tutor“ einen wählen.".to_string(),
                            _ => match (budget, q.lang == "en") {
                            (true, true) => "The cost limit for this month is reached. Raise it in the settings or switch to a local tutor.".to_string(),
                            (true, false) => "Das Kostenlimit für diesen Monat ist erreicht. In den Einstellungen anheben oder auf einen lokalen Tutor wechseln.".to_string(),
                            (false, true) => format!("The tutor did not answer: {e}"),
                            (false, false) => format!("Der Tutor hat nicht geantwortet: {e}"),
                            },
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
                // Without a voice nothing is spoken: listen again right away.
                q.as_mut().listen_again();
                q.placement_step();
            });
        });
        Some(index)
    }

    fn help(mut self: Pin<&mut Self>) {
        // Not while the tutor thinks about a turn already sent: the help would count for it.
        if *self.busy() {
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };
        let step = session.lock().expect("session lock").help();
        // Kept at once: help used after the last turn is there when the talk is continued.
        self.save();
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
            } else {
                // A talk continued, or the models back after the idle time: hands-free
                // listens without waiting for a click.
                q.listen_again();
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
            // Without a voice nothing is spoken: hands-free listens right away, in a new
            // talk, one continued and when the models are back after the idle time.
            q.as_mut().listen_again();
        }
        }
    });
}

/// Keeps a response time on the device (/LL010/).
fn book_latency(since: std::time::Instant) {
    let ms = u32::try_from(since.elapsed().as_millis()).unwrap_or(u32::MAX);
    if let Err(e) = state().add_latency(ms) {
        eprintln!("kaiwa: {e:#}");
    }
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
pub(crate) fn speech_text(e: &SpeechError, lang: &str) -> String {
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
            "incomplete": t.incomplete,
        }));
        if !t.markup.is_empty() {
            log.last_mut().expect("just pushed")["markup"] = t.markup.clone().into();
        }
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
