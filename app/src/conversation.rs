//! The conversation as a QObject for QML. State goes to QML as JSON strings (turns, goals,
//! summary), so the page renders it with Kante components without a C++ model class.
//! The talk (`kaiwa_backend::flow::Talk`) lives in the profile: the backend saves it after
//! every turn (to continue later, /LF121/) and records it at the end; this object shows it.
//! Lock order: the talk first, then `state()`; nothing holds a `state()` guard while it
//! takes the talk.
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
        /// A save or load that failed, for the page to show (like `KaiwaApp.notice`).
        #[qproperty(QString, notice)]
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
use kaiwa_backend::flow::{self, Length, Talk};
use kaiwa_core::feedback::{Shown, kind_word, severity_word};
use kaiwa_core::learner::Skill;
use kaiwa_core::placement::{self, Placement, Reading, STAGES, Step, TURNS_PER_STAGE};
use kaiwa_core::session::{Aid, SessionState};
use kaiwa_core::{Input, Scenario, Session, Summary};
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
    notice: QString,
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
    /// Since when the time in this talk has not been added to the session.
    sitting: Option<std::time::Instant>,
    talk: Option<Arc<Mutex<Talk>>>,
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
    /// What the voice owes the talk while it loads: an early reply, the opening once.
    debt: kaiwa_backend::talk::Debt,
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

    /// Shows the goals and, once the goals are reached, the summary. Keeping the talk is the
    /// backend's (`Talk::take`).
    fn publish_goals(mut self: Pin<&mut Self>) {
        let Some(talk) = self.talk.clone() else {
            return;
        };
        let lang = self.lang.clone();
        let (goals, done, summary) = {
            let talk = talk.lock().expect("talk lock");
            let session = talk.session();
            let met = session.summary().goals_met;
            let goals: Vec<Value> = session
                .goals()
                .iter()
                .map(|g| json!({ "id": g.id, "text": g.text.get(&lang), "done": met.contains(&g.id) }))
                .collect();
            let summary = summary_json(&session.summary(), &lang).to_string();
            (goals, session.done(), summary)
        };
        self.as_mut()
            .set_goals(QString::from(&Value::Array(goals).to_string()));
        if done && self.placement.is_none() {
            self.as_mut().set_summary(QString::from(&summary));
            self.as_mut().set_finished(true);
        }
    }

    /// Adds the time since the talk began here, or since it was last counted, to the talk
    /// (a talk continued later keeps what it had, /LF121/). Before `Talk::take`: the record
    /// counts it.
    fn spend_sitting(mut self: Pin<&mut Self>, talk: &mut Talk) {
        let Some(since) = self.sitting else {
            return;
        };
        // Whole seconds; the rest stays for the next time.
        let secs = since.elapsed().as_secs();
        self.as_mut().rust_mut().sitting = Some(since + Duration::from_secs(secs));
        talk.spend(u32::try_from(secs).unwrap_or(u32::MAX));
    }

    fn start_placement(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().placement = Some(Placement::default());
        self.as_mut().set_placement_ask(false);
        // Cleared here, not per stage: a failure booked at the end of a stage stays shown.
        self.as_mut().set_notice(QString::default());
        self.begin_stage(0);
    }

    /// Plays stage `i` of the placement ([`placement::stage_session`]), nothing stored.
    fn begin_stage(mut self: Pin<&mut Self>, i: usize) {
        let id = STAGES[i].0;
        let mut failed = Vec::new();
        let prepared = {
            let st = state();
            let profile = or_default(st.store.profile(), &mut failed);
            st.scenarios.get(id).cloned().map(|s| (s, profile))
        };
        let Some((scenario, profile)) = prepared else {
            self.as_mut().say_all_unsaved(&failed);
            return;
        };
        self.as_mut().rust_mut().lang = profile.ui_lang.clone();
        self.as_mut().say_all_unsaved(&failed);
        let (provider, demo) = state().provider(&scenario.id);
        let log = vec![json!({ "who": "character", "markup": scenario.opening.clone() })];
        let session =
            placement::stage_session(scenario, provider, profile.feedback, &profile.ui_lang, i);
        let lang = profile.ui_lang;
        let opened = Talk::unstored(&state(), session, demo, now());
        self.as_mut().say_all_failed(&opened.failed);
        self.as_mut().begin(opened.talk, log, lang.clone());
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
        let Some(talk) = self.talk.clone() else {
            return;
        };
        let step = {
            let mut rust = self.as_mut().rust_mut();
            let Some(p) = rust.placement.as_mut() else {
                return;
            };
            p.after_turn(talk.lock().expect("talk lock").session())
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
        let saved = state().store.set_placement(model);
        if let Err(e) = saved {
            self.as_mut().say_unsaved(&e.to_string());
        }
        let minutes = (p.results.len() as u32 * TURNS_PER_STAGE).div_ceil(2);
        // The skip the result offers (/LF215/); none for a beginner.
        let offered = views::placement_offer(&state());
        let offer = offered.unwrap_or_else(|e| {
            self.as_mut().say_unsaved(&e.to_string());
            Default::default()
        });
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

    fn skip_placement(mut self: Pin<&mut Self>, last: &QString) -> bool {
        // Only the offer that ends at `last`, once (a second click changes nothing).
        let taken = views::take_placement_skip(&state(), &last.to_string());
        taken.unwrap_or_else(|e| {
            self.as_mut().say_unsaved(&e.to_string());
            false
        })
    }

    /// Shows a session (new or restored) and loads speech once.
    fn begin(mut self: Pin<&mut Self>, talk: Talk, log: Vec<Value>, lang: String) {
        let demo = talk.demo();
        let session = talk.session();
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
        let mut failed = Vec::new();
        let read = state().store.profile();
        let sound = or_default(read.map(|p| (p.louder, p.clear, p.hands_free)), &mut failed);
        self.as_mut().say_all_unsaved(&failed);
        self.as_mut().set_louder(sound.0);
        self.as_mut().set_clear(sound.1);
        self.as_mut().set_hands_free(sound.2);
        {
            let mut rust = self.as_mut().rust_mut();
            rust.talk = Some(Arc::new(Mutex::new(talk)));
            rust.voice_name = voice.clone();
            rust.log = log;
            rust.lang = lang;
            rust.sitting = Some(std::time::Instant::now());
            rust.last_use = Some(std::time::Instant::now());
            // Another conversation: nothing said before counts.
            if let Ok(mut c) = rust.said.lock() {
                c.clear();
            }
            // A new conversation loads what is missing itself (below).
            rust.unloaded = false;
            rust.debt.reset();
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
                let voice = if self.speaker.is_some() && *self.voice_on() {
                    kaiwa_backend::talk::Voice::Ready
                } else {
                    kaiwa_backend::talk::Voice::Off
                };
                self.as_mut().rust_mut().debt.opening_said(voice);
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

    /// Tells the learner a save or load failed (it was only logged before). The page shows
    /// `notice` as the settings page does; it is cleared when the next talk begins.
    fn say_unsaved(mut self: Pin<&mut Self>, e: &str) {
        let msg = match self.lang == "de" {
            true => format!("Das Profil ließ sich nicht lesen oder schreiben: {e}"),
            false => format!("The profile could not be read or written: {e}"),
        };
        self.as_mut().set_notice(QString::from(&msg));
    }

    fn say_all_unsaved(mut self: Pin<&mut Self>, failed: &[String]) {
        for e in failed {
            self.as_mut().say_unsaved(e);
        }
    }

    /// What failed beside what worked: each one to the notice.
    fn say_all_failed(mut self: Pin<&mut Self>, failed: &[anyhow::Error]) {
        for e in failed {
            self.as_mut().say_unsaved(&format!("{e:#}"));
        }
    }

    /// Keeps the state of the talk after a change outside a turn (help, a listening aid, a
    /// translation).
    fn save(mut self: Pin<&mut Self>) {
        let Some(talk) = self.talk.clone() else {
            return;
        };
        let saved = talk.lock().ok().map(|t| t.save(&state()));
        if let Some(Err(e)) = saved {
            self.as_mut().say_unsaved(&format!("{e:#}"));
        }
    }

    fn mark_misrecognized(mut self: Pin<&mut Self>, log_index: i32) {
        let Some(turn) = self.turn_of(log_index) else {
            return;
        };
        let Some(talk) = self.talk.clone() else {
            return;
        };
        let marked = talk
            .lock()
            .expect("talk lock")
            .mark_misrecognized(&state(), turn);
        let Some(marked) = marked else {
            return;
        };
        if let Some(e) = self.as_mut().rust_mut().log.get_mut(log_index as usize) {
            e["corrections"] = json!([]);
            e["misrecognized"] = json!(true);
        }
        self.as_mut().show_marked(&talk, marked);
        self.as_mut().publish_turns();
    }

    fn dispute(mut self: Pin<&mut Self>, log_index: i32, better: &QString) {
        let better = better.to_string();
        let Some(turn) = self.turn_of(log_index) else {
            return;
        };
        let Some(talk) = self.talk.clone() else {
            return;
        };
        let marked = talk
            .lock()
            .expect("talk lock")
            .dispute(&state(), turn, &better);
        let Some(marked) = marked else {
            return;
        };
        if let Some(c) = &marked.correction
            && let Some(e) = self.as_mut().rust_mut().log.get_mut(log_index as usize)
            && let Some(list) = e["corrections"].as_array_mut()
        {
            for x in list.iter_mut().filter(|x| x["better"] == c.better.as_str()) {
                x["disputed"] = json!(true);
            }
        }
        self.as_mut().show_marked(&talk, marked);
        self.as_mut().publish_turns();
    }

    /// Shows a recorded talk's summary, and that the profile could not be read if so.
    fn show_recorded(mut self: Pin<&mut Self>, (recorded, unread): (Value, Option<String>)) {
        if let Some(e) = unread {
            self.as_mut().say_unsaved(&e);
        }
        self.as_mut()
            .set_recorded(QString::from(&recorded.to_string()));
    }

    /// Shows a kept mark. On the summary (V-05) the mark corrected the recorded talk: its
    /// record, the learner model and the review, and the summary shows the corrected result.
    fn show_marked(mut self: Pin<&mut Self>, talk: &Arc<Mutex<Talk>>, marked: flow::Marked) {
        self.as_mut().say_all_failed(&marked.failed);
        let lang = self.lang.clone();
        let (finished, shown) = {
            let talk = talk.lock().expect("talk lock");
            let shown = marked.recorded.as_ref().map(|r| {
                let summary = summary_json(&talk.session().summary(), &lang);
                (recorded_view(r, talk.session(), &lang), summary)
            });
            (talk.finished(), shown)
        };
        let Some((recorded, summary)) = shown else {
            // The record stays (a later talk was recorded, or this one has no record to
            // redo): the page says so instead of promising a correction. Without a record
            // there is nothing to say.
            if finished && let Some(kept) = keep_mark(&self.recorded().to_string()) {
                self.as_mut().set_recorded(QString::from(&kept));
            }
            return;
        };
        self.as_mut().show_recorded(recorded);
        self.as_mut()
            .set_summary(QString::from(&summary.to_string()));
    }

    fn resume(mut self: Pin<&mut Self>) {
        self.as_mut().set_notice(QString::default());
        self.as_mut().rust_mut().placement = None;
        self.as_mut().set_placing(false);
        let mut failed = Vec::new();
        let (opened, lang) = {
            let st = state();
            let lang = or_default(st.store.profile(), &mut failed).ui_lang;
            (Talk::resume(&st, now()), lang)
        };
        self.as_mut().rust_mut().lang = lang.clone();
        self.as_mut().say_all_unsaved(&failed);
        let opened = match opened {
            Ok(Some(opened)) => opened,
            Ok(None) => return,
            Err(e) => return self.cannot_open(&e, &lang),
        };
        self.as_mut().say_all_failed(&opened.failed);
        let session = opened.talk.session();
        let log = log_of(&session.scenario, &session.snapshot(), &lang);
        self.begin(opened.talk, log, lang);
    }

    fn start(mut self: Pin<&mut Self>, scenario: &QString) {
        self.as_mut().set_notice(QString::default());
        self.as_mut().rust_mut().placement = None;
        self.as_mut().set_placing(false);
        let wanted = scenario.to_string();
        let mut failed = Vec::new();
        let (opened, lang) = {
            let st = state();
            let lang = or_default(st.store.profile(), &mut failed).ui_lang;
            let id = match wanted.is_empty() {
                true => {
                    let (id, next_failed) = Talk::next_scenario(&st);
                    failed.extend(next_failed.iter().map(|e| format!("{e:#}")));
                    id
                }
                false => wanted,
            };
            // Nothing opens for a scenario that does not exist.
            let opened = st
                .scenarios
                .contains_key(&id)
                .then(|| Talk::begin(&st, &id, now(), Length::Full));
            (opened, lang)
        };
        self.as_mut().rust_mut().lang = lang.clone();
        self.as_mut().say_all_unsaved(&failed);
        let opened = match opened {
            Some(Ok(opened)) => opened,
            Some(Err(e)) => return self.cannot_open(&e, &lang),
            None => return,
        };
        self.as_mut().say_all_failed(&opened.failed);
        let log = vec![json!({ "who": "character", "markup": opened.talk.session().opening() })];
        self.begin(opened.talk, log, lang);
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
        self.talk
            .as_ref()
            .and_then(|t| t.lock().ok().map(|t| t.session().difficulty.speed() as f32))
            .unwrap_or(SPEED)
    }

    fn speak_at(mut self: Pin<&mut Self>, markup: &str, speed: f32, reply: bool) {
        let Some(speaker) = self.speaker.clone() else {
            // The voice is still loading (after the idle time): say the reply when it is ready.
            if reply && *self.voice_on() {
                self.as_mut().rust_mut().debt.reply_early(markup);
            }
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
                        if let Some(e) = speech_end.take().and_then(book_latency) {
                            let _ = thread.queue(move |mut q| q.as_mut().say_unsaved(&e));
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
            .unwrap_or(kaiwa_core::turn_end::DEFAULT_PAUSE);
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
        let mut failed = None;
        if secs > 0.0 {
            let st = state();
            match st.store.profile() {
                Ok(mut p) if secs > p.end_pause_secs => {
                    p.end_pause_secs = secs;
                    failed = st.store.set_profile(&p).err().map(|e| e.to_string());
                }
                Ok(_) => {}
                Err(e) => failed = Some(e.to_string()),
            }
        }
        if let Some(e) = failed {
            self.as_mut().say_unsaved(&e);
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
        let saved = {
            let st = state();
            st.store.profile().and_then(|mut p| {
                p.louder = louder;
                p.clear = clear;
                st.store.set_profile(&p)
            })
        };
        if let Err(e) = saved {
            self.as_mut().say_unsaved(&e.to_string());
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
        let Some(talk) = self.talk.clone() else {
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
                if let Ok(mut t) = talk.lock() {
                    let kind = if aid == 0 { Aid::Repeat } else { Aid::Slow };
                    t.session_mut().aid(kind, line);
                }
                self.as_mut().save();
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
                    let asking = talk.lock().ok().map(|t| t.session().translating(&line));
                    let result = asking.map(|a| a.run());
                    let _ = thread.queue(move |mut q| {
                        if let Some(e) = q.as_mut().rust_mut().log.get_mut(i)
                            && let Some(e) = e.as_object_mut()
                        {
                            e.remove("translating");
                        }
                        // What the translation cost (/LF714/), not only with the next turn.
                        let booked = state().book_costs(now(), false);
                        if let Err(e) = booked {
                            q.as_mut().say_unsaved(&e.to_string());
                        }
                        match result {
                            Some(Ok(text)) => {
                                // It counts as help once the learner has it (/LF212/).
                                if let Ok(mut t) = talk.lock() {
                                    t.session_mut().aid(Aid::Translate, asked_at);
                                }
                                if let Some(e) = q.as_mut().rust_mut().log.get_mut(i) {
                                    e["translation"] = json!(text);
                                }
                                q.as_mut().save();
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

    fn remember(mut self: Pin<&mut Self>, log_index: i32, word: i32) -> bool {
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
                self.as_mut().say_unsaved(&format!("{e:#}"));
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
        let read = state().store.profile();
        let read = read.inspect_err(|e| self.as_mut().say_unsaved(&e.to_string()));
        let microphone = read.ok().and_then(|p| p.microphone);
        match Capture::start_on(microphone.as_deref()) {
            Ok(c) => {
                let turn_end = self.hands_free.then(|| {
                    let pause = state()
                        .store
                        .profile()
                        .map(|p| p.end_pause_secs)
                        .unwrap_or(kaiwa_core::turn_end::DEFAULT_PAUSE);
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
        if !kaiwa_backend::talk::may_recognize(*self.busy()) {
            return;
        }
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
        if let Some(id) = self.talk.as_ref().and_then(|t| t.lock().ok()?.id()) {
            let turn = kaiwa_backend::talk::turns(&self.log);
            let kept = state().keep_recording(id, turn, &rec.samples, rec.sample_rate, now());
            if let Err(e) = kept {
                self.as_mut().say_unsaved(&format!("{e:#}"));
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
        let input = if typed { Input::Typed } else { Input::Spoken };
        // Typed rōmaji becomes kana, shown as such (/LF120/).
        let text = flow::prepare(text, input);
        let talk = self.talk.clone()?;
        if *self.busy() || *self.finished() || !talk.lock().ok()?.may_say(&text) {
            return None;
        }
        if typed {
            // Only spoken turns have a response time.
            self.as_mut().rust_mut().speech_end = None;
            self.as_mut().rust_mut().empty_turns.reset();
            // Typed instead of spoken: the microphone closes, hands-free or not; a recording
            // finishing during this turn would run a second one beside it.
            self.as_mut().close_mic();
        }
        self.as_mut().wake();
        let index = self.log.len();
        self.as_mut().rust_mut().log.push(json!({ "who": "learner", "text": text, "typed": typed, "corrections": [], "pending": true }));
        self.as_mut().set_busy(true);
        self.as_mut().set_help_text(QString::default());
        self.as_mut().set_help_level(0);
        self.as_mut().publish_turns();

        let thread = self.qt_thread();
        std::thread::spawn(move || {
            // The talk is held only to ask and to take the answer, not while the tutor
            // thinks: the window locks it for help and the listening aids.
            let asking = talk.lock().map(|t| t.ask(&text));
            let answered: Result<_, String> = asking
                .map_err(|e| e.to_string())
                .and_then(|a| a.run().map_err(|e| e.to_string()));
            let _ = thread.queue(move |mut q| {
                // Lock order: the talk, then the state. The backend keeps the talk, records
                // the end and books the cost; what it could not do is shown.
                let result = {
                    let mut t = talk.lock().expect("talk lock");
                    match answered {
                        Ok(answer) => {
                            q.as_mut().spend_sitting(&mut t);
                            let turned = t.take(&state(), &text, input, answer, now());
                            let shown = turned
                                .recorded
                                .as_ref()
                                .map(|r| recorded_view(r, t.session(), &q.lang));
                            Ok((turned, shown))
                        }
                        Err(e) => Err((e, t.failed_call(&state(), now()))),
                    }
                };
                match result {
                    Ok((turned, shown)) => {
                        let outcome = turned.outcome;
                        {
                            let mut rust = q.as_mut().rust_mut();
                            let lang = rust.lang.clone();
                            if let Some(turn) = rust.log.get_mut(index) {
                                show_outcome(turn, &outcome, &lang);
                            }
                            rust.log.push(
                                json!({ "who": "character", "markup": outcome.reply.clone() }),
                            );
                        }
                        q.as_mut().set_error(QString::default());
                        q.as_mut().say_all_failed(&turned.failed);
                        if let Some(shown) = shown {
                            q.as_mut().show_recorded(shown);
                        }
                        q.as_mut().speak_reply(&outcome.reply);
                        // Without a voice the reply's text is the answer.
                        if let Some(e) = q
                            .as_mut()
                            .rust_mut()
                            .speech_end
                            .take()
                            .and_then(book_latency)
                        {
                            q.as_mut().say_unsaved(&e);
                        }
                    }
                    Err((e, failed)) => {
                        q.as_mut().say_all_failed(&failed);
                        // The line stays to be read, but it is no turn of the session.
                        if let Some(turn) = q.as_mut().rust_mut().log.get_mut(index) {
                            turn["pending"] = json!(false);
                            turn["failed"] = json!(true);
                        }
                        let msg = tutor_error_text(&e, &q.lang);
                        q.as_mut().set_error(QString::from(&msg));
                    }
                }
                q.as_mut().set_busy(false);
                q.as_mut().publish_turns();
                q.as_mut().publish_goals();
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
        let Some(talk) = self.talk.clone() else {
            return;
        };
        let step = talk.lock().expect("talk lock").session_mut().help();
        // Kept at once: help used after the last turn is there when the talk is continued.
        self.as_mut().save();
        if let Some(step) = step {
            self.as_mut().set_help_text(QString::from(&step.text));
            self.as_mut().set_help_level(i32::from(step.level));
        }
    }
}

/// The learner's line of the log with what the tutor made of it.
fn show_outcome(turn: &mut Value, outcome: &kaiwa_core::TurnOutcome, lang: &str) {
    turn["corrections"] = Value::Array(outcome.shown.iter().map(|x| shown_json(x, lang)).collect());
    turn["pending"] = json!(false);
    turn["incomplete"] = json!(outcome.incomplete);
    // The tutor's furigana replace the dictionary's (ADR-004).
    if outcome.own_markup.is_empty() {
        return;
    }
    turn["markup"] = outcome.own_markup.clone().into();
    if let Some(t) = turn.as_object_mut() {
        t.remove("segments");
        t.remove("words");
    }
}

/// What the page says when the tutor did not answer.
fn tutor_error_text(e: &str, lang: &str) -> String {
    let budget = e == kaiwa_ai::AiError::Budget.to_string();
    let no_tutor = e == kaiwa_ai::AiError::NoTutor.to_string();
    match (no_tutor, lang == "en") {
        (true, true) => "No tutor is set up. Choose one in the settings under Tutor.".to_string(),
        (true, false) => "Es ist kein Tutor eingerichtet. In den Einstellungen unter „Tutor“ einen wählen.".to_string(),
        _ => match (budget, lang == "en") {
        (true, true) => "The cost limit for this month is reached. Raise it in the settings or switch to a local tutor.".to_string(),
        (true, false) => "Das Kostenlimit für diesen Monat ist erreicht. In den Einstellungen anheben oder auf einen lokalen Tutor wechseln.".to_string(),
        (false, true) => format!("The tutor did not answer: {e}"),
        (false, false) => format!("Der Tutor hat nicht geantwortet: {e}"),
        },
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
            let (lines, busy) = (q.log.len(), *q.busy());
            let voice = if *q.voice_on() {
                kaiwa_backend::talk::Voice::Ready
            } else {
                kaiwa_backend::talk::Voice::Off
            };
            match q.as_mut().rust_mut().debt.voice_ready(lines, busy, voice) {
                // The reply came before the voice did; no listening aid, no response time.
                kaiwa_backend::talk::Owed::Reply(markup) => q.speak(&markup),
                kaiwa_backend::talk::Owed::Opening => {
                    let opening = q.log[0]["markup"].as_str().unwrap_or_default().to_string();
                    q.speak(&opening);
                }
                // A talk continued, or the models back after the idle time: hands-free
                // listens without waiting for a click.
                kaiwa_backend::talk::Owed::Nothing => q.listen_again(),
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
            q.as_mut().rust_mut().debt.forgive();
            // Without a voice nothing is spoken: hands-free listens right away, in a new
            // talk, one continued and when the models are back after the idle time.
            q.as_mut().listen_again();
        }
        }
    });
}

/// Keeps a response time on the device (/LL010/); the failure, if any, as text.
fn book_latency(since: std::time::Instant) -> Option<String> {
    let ms = u32::try_from(since.elapsed().as_millis()).unwrap_or(u32::MAX);
    state().add_latency(ms).err().map(|e| format!("{e:#}"))
}

/// The value of a store read; a failure is noted in `failed` (for `say_unsaved`) and the
/// default stands in.
fn or_default<T: Default, E: std::fmt::Display>(r: Result<T, E>, failed: &mut Vec<String>) -> T {
    r.unwrap_or_else(|e| {
        failed.push(e.to_string());
        T::default()
    })
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

/// The record of the summary with the kept mark in it. `None` without a record (recording the
/// talk failed): the mark has nothing to stay with.
fn keep_mark(recorded: &str) -> Option<String> {
    let mut recorded: Value = serde_json::from_str(recorded).ok()?;
    if !recorded.is_object() {
        return None;
    }
    recorded["markKept"] = json!(true);
    Some(recorded.to_string())
}

/// The summary of a recorded talk, and why the profile could not be read if it could not:
/// the page then shows the summary with the defaults and says so.
fn recorded_view(r: &kaiwa_store::Recorded, s: &Session, lang: &str) -> (Value, Option<String>) {
    let profile = state().store.profile();
    let unread = profile.as_ref().err().map(|e| e.to_string());
    (recorded_json(r, s, profile.ok(), lang), unread)
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

#[cfg(test)]
mod tests {
    use super::keep_mark;

    #[test]
    fn a_kept_mark_needs_a_record() {
        // Recording failed: `recorded` is empty, the page must not get a record from it.
        assert_eq!(keep_mark(""), None);
        assert_eq!(keep_mark("null"), None);

        let kept = keep_mark(r#"{"minutes":3,"xp":null}"#).unwrap();
        let v: serde_json::Value = serde_json::from_str(&kept).unwrap();
        assert_eq!(v["markKept"], true);
        assert_eq!(v["minutes"], 3);
    }
}
