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
        #[qproperty(bool, speech_missing, cxx_name = "speechMissing")]
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
        #[cxx_name = "takePlacementOffer"]
        fn take_placement_offer(self: Pin<&mut Conversation>, last: &QString) -> bool;

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
use kaiwa_ai::AiError;
use kaiwa_backend::ears::{
    self, Clip, Ears, Hands, Heard, Mic, MoraFailure, Moras, Offer, Patience,
};
use kaiwa_backend::flow::{self, Hear, Length, Talk, VoiceState};
use kaiwa_backend::log::Log;
use kaiwa_backend::placing::{self, Next, Placing, Stage};
use kaiwa_backend::voice::{self, Debt, Enhance, Line, Owed, Pace, Said, Speaker};
use kaiwa_backend::{Lang, words};
use kaiwa_core::Input;
use serde_json::Value;

use crate::state::{now, state};
use crate::util::{append_notice, book_latency, keep_mark, or_default};

mod recognizer;
use recognizer::load_recognizer;

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
    speech_missing: bool,
    voice: QString,
    voice_on: bool,
    louder: bool,
    clear: bool,
    hands_free: bool,
    longer_pause: f64,
    /// The pause ended the last recording, and the turns that sounded cut off by it.
    ended_by_pause: bool,
    /// Hands-free: turns in a row the pause ended that gave nothing to send, and the turns
    /// it cut off.
    patience: Patience,
    /// When the learner stopped speaking, for the response time (/LL010/); handed on to the
    /// reply, which books it at its first sound.
    speech_end: Option<std::time::Instant>,
    voice_note: QString,
    speech_ready: bool,
    scenario: QString,
    character: QString,
    recorded: QString,
    placing: bool,
    stage: QString,
    placement_ask: bool,
    placement: Option<Placing>,
    lang: Lang,
    /// Since when the time in this talk has not been added to the session.
    sitting: Option<std::time::Instant>,
    talk: Option<Arc<Mutex<Talk>>>,
    log: Log,
    /// The recognition (/LL504/) and the mora check (/LF320/); each missing without its model.
    ears: Ears,
    /// Open while listening.
    mic: Option<Mic>,
    speaker: Option<Speaker>,
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
    said: Said,
    /// The voice of this conversation's character (/LF117/); a loaded voice switches to it.
    voice_name: String,
    /// What the voice owes the talk while it loads: an early reply, the opening once.
    debt: Debt,
}

const READY: i32 = 0;
const LISTENING: i32 = 1;
const THINKING: i32 = 2;
const SPEAKING: i32 = 3;
const ERROR: i32 = 4;

impl qobject::Conversation {
    fn publish_turns(mut self: Pin<&mut Self>) {
        let text = self.as_mut().rust_mut().log.view();
        self.as_mut().set_turns(QString::from(&text));
    }

    /// Shows the goals and, once the goals are reached, the summary. Keeping the talk is the
    /// backend's (`Talk::take`).
    fn publish_goals(mut self: Pin<&mut Self>) {
        let Some(talk) = self.talk.clone() else {
            return;
        };
        let lang = self.lang;
        let (goals, done, summary) = {
            let talk = talk.lock().expect("talk lock");
            let session = talk.session();
            let goals = kaiwa_backend::summary::goals_view(session, lang);
            let summary = kaiwa_backend::summary::summary_view(session, lang).to_string();
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
        self.as_mut().set_placement_ask(false);
        // Cleared here, not per stage: a failure booked at the end of a stage stays shown.
        self.as_mut().set_notice(QString::default());
        let started = Placing::start(&state(), now());
        match started {
            Ok((placing, stage)) => {
                self.as_mut().rust_mut().placement = Some(placing);
                self.begin_stage(stage);
            }
            Err(e) => {
                self.as_mut().rust_mut().placement = None;
                self.as_mut().say_unsaved(&format!("{e:#}"));
            }
        }
    }

    /// Shows the stage the placement opened.
    fn begin_stage(mut self: Pin<&mut Self>, stage: Stage) {
        self.as_mut().rust_mut().lang = stage.lang;
        self.as_mut().say_all_failed(&stage.opened.failed);
        let log = Log::opened(&stage.opening);
        self.as_mut().begin(stage.opened.talk, log, stage.lang);
        let word = if stage.lang == Lang::En {
            "Placement"
        } else {
            "Einstufung"
        };
        let title = format!("{word} · {}", self.title());
        self.as_mut().set_title(QString::from(&title));
        self.as_mut()
            .set_stage(QString::from(&format!("{}/{}", stage.index + 1, stage.of)));
        self.as_mut().set_placing(true);
    }

    /// After a turn of the placement ([`Placing::after_turn`]).
    fn placement_step(mut self: Pin<&mut Self>) {
        let Some(talk) = self.talk.clone() else {
            return;
        };
        let next = {
            let mut rust = self.as_mut().rust_mut();
            let Some(p) = rust.placement.as_mut() else {
                return;
            };
            let talk = talk.lock().expect("talk lock");
            p.after_turn(&state(), talk.session(), now())
        };
        match next {
            Ok(Next::Go) => {}
            Ok(Next::Stage(stage)) => self.begin_stage(*stage),
            Ok(Next::Over) => {
                self.as_mut().set_placement_ask(true);
                self.as_mut().set_finished(true);
            }
            Ok(Next::Unjudged(result)) => {
                self.as_mut().rust_mut().placement = None;
                self.as_mut()
                    .set_recorded(QString::from(&result.to_string()));
                self.as_mut().set_placing(false);
                self.as_mut().set_finished(true);
            }
            Err(e) => self.as_mut().say_unsaved(&format!("{e:#}")),
        }
    }

    fn finish_placement(mut self: Pin<&mut Self>, kana: i32, kanji: i32) {
        let Some(placing) = self.as_mut().rust_mut().placement.take() else {
            return;
        };
        let done = placing.finish(&state(), placing::reading(kana), placing::reading(kanji));
        self.as_mut().say_all_failed(&done.failed);
        self.as_mut()
            .set_recorded(QString::from(&done.result.to_string()));
        self.as_mut().set_placement_ask(false);
        self.as_mut().set_placing(false);
    }

    fn take_placement_offer(mut self: Pin<&mut Self>, last: &QString) -> bool {
        // Only the offer that ends at `last`, once (a second click changes nothing).
        let taken = kaiwa_backend::skips::take_placement_skip(&state(), &last.to_string());
        taken.unwrap_or_else(|e| {
            self.as_mut().say_unsaved(&e.to_string());
            false
        })
    }

    /// Shows a session (new or restored) and loads speech once.
    fn begin(mut self: Pin<&mut Self>, talk: Talk, log: Log, lang: Lang) {
        let demo = talk.demo();
        let session = talk.session();
        let title = session.scenario.title.get(lang).to_string();
        let scenario = session.scenario.id.clone();
        let character = session
            .scenario
            .name
            .clone()
            .unwrap_or_else(|| lang.pick("Figur", "Character").into());
        let voice = session
            .scenario
            .voice
            .clone()
            .unwrap_or_else(|| "jf_tebukuro".into());
        let opening = (log.len() == 1).then(|| session.opening().to_string());
        let mut failed = Vec::new();
        let read = state().profile();
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
            rust.said.clear();
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
                self.ears.recognition().is_none(),
                self.speaker.is_none(),
                self.ears.moras().is_none(),
            );
            if missing != (false, false, false) {
                let thread = self.qt_thread();
                let lang = self.lang;
                std::thread::spawn(move || {
                    if missing.0 {
                        load_recognizer(&thread, lang);
                    }
                    if missing.1 {
                        load_voice(&thread, &voice, lang);
                    }
                    if missing.2 {
                        load_mora(&thread);
                    }
                });
            }
            // Speak the opening of the new scenario.
            if let Some(o) = opening {
                let voice = if self.speaker.is_some() && *self.voice_on() {
                    voice::Voice::Ready
                } else {
                    voice::Voice::Off
                };
                self.as_mut().rust_mut().debt.opening_said(voice);
                self.as_mut().speak(&o);
            }
            return;
        }
        self.as_mut().set_voice_on(true);
        self.as_mut().set_talk_state(THINKING);
        let thread = self.qt_thread();
        let lang = self.lang;
        // One after the other: both models share ONNX Runtime, which the first sets up.
        std::thread::spawn(move || {
            load_recognizer(&thread, lang);
            load_voice(&thread, &voice, lang);
            load_mora(&thread);
        });
    }

    /// The session's turn index of a log entry; a line the tutor never answered has none.
    fn turn_of(&self, log_index: i32) -> Option<usize> {
        self.log.turn_of(usize::try_from(log_index).ok()?)
    }

    /// Tells the learner a save or load failed (it was only logged before). The page shows
    /// `notice` as the settings page does; it is cleared when the next talk begins.
    fn say_unsaved(self: Pin<&mut Self>, e: &str) {
        let msg = words::unsaved(e, self.lang);
        self.add_notice(&msg);
    }

    /// Adds `msg` below what the notice already says, once: a second failure does not wipe
    /// the first.
    fn add_notice(mut self: Pin<&mut Self>, msg: &str) {
        let new = append_notice(&self.notice().to_string(), msg);
        self.as_mut().set_notice(QString::from(&new));
    }

    /// Tells the learner why there are no pronunciation hints; the talk goes on without.
    fn say_mora_failed(self: Pin<&mut Self>, e: &ears::SpeechError, when: MoraFailure) {
        eprintln!("kaiwa: mora check: {e}");
        let msg = ears::mora_text(when, self.lang);
        self.add_notice(msg);
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
        self.as_mut()
            .rust_mut()
            .log
            .misrecognized(log_index as usize);
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
        if let Some(c) = &marked.correction {
            self.as_mut()
                .rust_mut()
                .log
                .disputed(log_index as usize, &c.better);
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
        let lang = self.lang;
        let (finished, shown) = {
            let talk = talk.lock().expect("talk lock");
            let shown = marked.recorded.as_ref().map(|r| {
                let summary = kaiwa_backend::summary::summary_view(talk.session(), lang);
                (state().recorded_view(r, talk.session(), lang), summary)
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
            let lang = or_default(st.profile(), &mut failed).ui_lang;
            (Talk::resume(&st, now()), lang)
        };
        self.as_mut().rust_mut().lang = lang;
        self.as_mut().say_all_unsaved(&failed);
        let opened = match opened {
            Ok(Some(opened)) => opened,
            Ok(None) => return,
            Err(e) => return self.cannot_open(&e, lang),
        };
        self.as_mut().say_all_failed(&opened.failed);
        let session = opened.talk.session();
        let log = Log::restored(&session.scenario.opening, &session.snapshot(), lang);
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
            let lang = or_default(st.profile(), &mut failed).ui_lang;
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
                .has_scenario(&id)
                .then(|| Talk::begin(&st, &id, now(), Length::Full));
            (opened, lang)
        };
        self.as_mut().rust_mut().lang = lang;
        self.as_mut().say_all_unsaved(&failed);
        let opened = match opened {
            Some(Ok(opened)) => opened,
            Some(Err(e)) => return self.cannot_open(&e, lang),
            None => return,
        };
        self.as_mut().say_all_failed(&opened.failed);
        let log = Log::opened(opened.talk.session().opening());
        self.begin(opened.talk, log, lang);
    }

    /// The profile could not be read for a talk: the page says so instead of staying empty.
    fn cannot_open(mut self: Pin<&mut Self>, e: &anyhow::Error, lang: Lang) {
        eprintln!("kaiwa: {e:#}");
        let msg = words::talk_unopened(e, lang);
        self.as_mut().set_error(QString::from(&msg));
    }

    /// Says a line of the character (furigana markup) sentence by sentence; the first plays
    /// while the next is made. The talk button shows "speaks" until the speaker is quiet.
    fn speak(self: Pin<&mut Self>, markup: &str) {
        let speed = self.speed(Pace::Normal);
        self.speak_at(markup, speed, false);
    }

    /// The reply to a spoken turn: its first sound books the response time (/LL010/).
    fn speak_reply(self: Pin<&mut Self>, markup: &str) {
        let speed = self.speed(Pace::Normal);
        self.speak_at(markup, speed, true);
    }

    /// The pace of the voice for this talk (/LF211/).
    fn speed(&self, how: Pace) -> f32 {
        let talk = self.talk.as_ref().and_then(|t| t.lock().ok());
        voice::pace(talk.as_ref().map(|t| &t.session().difficulty), how)
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
        let line = Line {
            markup: markup.to_string(),
            voice: self.voice_name.clone(),
            speed,
            enhance: Enhance {
                louder: *self.louder(),
                clear: *self.clear(),
            },
        };
        let mut speech_end = if reply {
            self.as_mut().rust_mut().speech_end.take()
        } else {
            None
        };
        let said = self.said.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let still = || current.load(Ordering::SeqCst) == id;
            // The first sound of the reply: the response time (/LL010/).
            let mut on_sound = || {
                let Some(e) = speech_end.take().and_then(book_latency) else {
                    return;
                };
                let _ = thread.queue(move |mut q| q.as_mut().say_unsaved(&e));
            };
            let failed = speaker.say(&said, &line, &still, &mut on_sound);
            let _ = thread.queue(move |mut q| {
                // A failed sentence leaves a note until a line plays without one.
                let note = failed.map(|e| voice::voice_text(&e, q.lang));
                q.as_mut()
                    .set_voice_note(QString::from(&note.unwrap_or_default()));
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
        let profile = state().profile();
        let minutes = match profile {
            Ok(p) => p.unload_after_min,
            Err(e) => {
                self.as_mut().say_unsaved(&format!("{e:#}"));
                return;
            }
        };
        let Some(minutes) = minutes else {
            return;
        };
        let quiet = *self.talk_state() == READY && !*self.busy() && self.mic.is_none();
        let long = self
            .last_use
            .is_some_and(|t| t.elapsed() >= Duration::from_secs(u64::from(minutes) * 60));
        let loaded = self.ears.loaded() || self.speaker.is_some();
        if quiet && long && loaded {
            let mut rust = self.as_mut().rust_mut();
            rust.ears.unload();
            rust.speaker = None;
            rust.unloaded = true;
            rust.said.clear();
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
        let lang = self.lang;
        self.as_mut().set_talk_state(THINKING);
        std::thread::spawn(move || {
            load_recognizer(&thread, lang);
            load_voice(&thread, &voice, lang);
            load_mora(&thread);
        });
    }

    /// A recognised turn: the first that the pause seems to have cut off offers a longer
    /// pause (/LF101/, T-01).
    fn after_turn(mut self: Pin<&mut Self>, by_pause: bool, text: &str) {
        let offered = *self.longer_pause() as f32;
        let offer = self
            .as_mut()
            .rust_mut()
            .patience
            .after_turn(&state(), by_pause, text, offered);
        match offer {
            Offer::Make(next) => self.as_mut().set_longer_pause(f64::from(next)),
            Offer::Withdraw => self.as_mut().set_longer_pause(0.0),
            Offer::Keep => {}
        }
    }

    /// Takes the offered pause; it only ever makes the pause longer.
    fn take_longer_pause(mut self: Pin<&mut Self>) {
        let secs = *self.longer_pause() as f32;
        let mut failed = None;
        if secs > 0.0 {
            failed = state().take_longer_pause(secs, now()).err();
        }
        if let Some(e) = failed {
            self.as_mut().say_unsaved(&e.to_string());
        }
        self.as_mut().rust_mut().patience.take_offer();
        self.as_mut().set_longer_pause(0.0);
    }

    /// Declines the offered pause: it comes back only after two more turns that sound cut off.
    fn decline_longer_pause(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().patience.decline_offer();
        self.as_mut().set_longer_pause(0.0);
    }

    fn set_sound(mut self: Pin<&mut Self>, louder: bool, clear: bool) {
        state()
            .set_sound(louder, clear)
            .unwrap_or_else(|e| self.as_mut().say_unsaved(&e.to_string()));
        self.as_mut().set_louder(louder);
        self.as_mut().set_clear(clear);
    }

    fn listen(mut self: Pin<&mut Self>, log_index: i32, aid: i32) {
        let Some(i) = usize::try_from(log_index).ok() else {
            return;
        };
        let Some(markup) = self.log.character_line(i).map(str::to_string) else {
            return;
        };
        // The aid belongs to the clicked line: the turn that answers it (a line already
        // answered gives a past turn, and the aid counts for nothing).
        let line = self.log.turns_before(i);
        // Already there, or on its way (a second click asks the tutor no second time).
        let translated = self.log.is_translated(i);
        let Some(talk) = self.talk.clone() else {
            return;
        };
        // A recognition in flight is busy too (the tutor's own turn the talk knows).
        if *self.busy() {
            return;
        }
        self.as_mut().wake();
        match aid {
            0 | 1 => {
                let kind = if aid == 0 { Hear::Repeat } else { Hear::Slow };
                let voice = match self.speaker {
                    Some(_) => VoiceState::Ready,
                    None => VoiceState::Loading,
                };
                let heard = talk
                    .lock()
                    .ok()
                    .and_then(|mut t| t.hear(&state(), kind, line, voice));
                let Some(heard) = heard else {
                    return;
                };
                self.as_mut().say_all_failed(&heard.failed);
                self.as_mut().interrupt();
                let speed = self.speed(if aid == 0 { Pace::Normal } else { Pace::Slow });
                self.speak_at(&markup, speed, false);
            }
            2 if !translated => self.translate(i, &markup, line, talk),
            _ => {}
        }
    }

    /// Asks the tutor for the translation of the character's line at log index `i`.
    fn translate(
        mut self: Pin<&mut Self>,
        i: usize,
        markup: &str,
        line: usize,
        talk: Arc<Mutex<Talk>>,
    ) {
        // Not while the tutor thinks about a turn already sent.
        let asking = talk
            .lock()
            .ok()
            .and_then(|t| t.ask_translation(markup, line));
        let Some(asking) = asking else {
            return;
        };
        let lang = self.lang;
        let thread = self.qt_thread();
        self.as_mut().rust_mut().log.translating(i);
        self.as_mut().publish_turns();
        std::thread::spawn(move || {
            // Asked without holding the talk (the window needs it meanwhile).
            let fetched = asking.run();
            let _ = thread.queue(move |mut q| {
                q.as_mut().rust_mut().log.translated_not(i);
                let taken = talk
                    .lock()
                    .ok()
                    .map(|mut t| t.take_translation(&state(), now(), fetched));
                let Some(taken) = taken else {
                    q.as_mut().publish_turns();
                    return;
                };
                q.as_mut().say_all_failed(&taken.failed);
                match taken.result {
                    Ok(text) => q.as_mut().rust_mut().log.translation(i, &text),
                    Err(e) => {
                        let msg = words::translation_failed(&e, lang);
                        q.as_mut().set_error(QString::from(&msg));
                    }
                }
                q.as_mut().publish_turns();
            });
        });
    }

    /// The word `word` of the line at `log_index` (the character's or the learner's own).
    fn line_word(&self, log_index: i32, word: i32) -> Option<kaiwa_lang::LineWord> {
        self.log.word(
            usize::try_from(log_index).ok()?,
            usize::try_from(word).ok()?,
        )
    }

    fn lookup(self: Pin<&mut Self>, log_index: i32, word: i32) -> QString {
        let Some(w) = self.line_word(log_index, word) else {
            return QString::from("null");
        };
        let lang = self.lang;
        let st = state();
        let v = kaiwa_backend::lookup::lookup(&st, &w, lang);
        QString::from(&v.to_string())
    }

    fn grammar(&self) -> QString {
        let st = state();
        let v = kaiwa_backend::views::grammar(&st, &self.scenario().to_string(), self.lang);
        QString::from(&v.to_string())
    }

    fn remember(mut self: Pin<&mut Self>, log_index: i32, word: i32) -> bool {
        let Some(w) = self.line_word(log_index, word) else {
            return false;
        };
        let lang = self.lang;
        let st = state();
        let found = kaiwa_backend::lookup::lookup(&st, &w, lang);
        let text = found["word"].as_str().unwrap_or(&w.key).to_string();
        // The unit as the review counts it (the curriculum's id, as for words from mistakes).
        let unit = st.unit_id_of(&self.scenario().to_string());
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
        let lang = self.lang;
        let entry = kaiwa_backend::lookup::lookup(&state(), &w, lang);
        let markup = match (entry["word"].as_str(), entry["reading"].as_str()) {
            (Some(word), Some(reading)) if word != reading => format!("{{{word}|{reading}}}"),
            (Some(word), _) => word.to_string(),
            _ => w.text.clone(),
        };
        self.as_mut().wake();
        self.as_mut().interrupt();
        let speed = self.speed(Pace::Normal);
        self.speak_at(&markup, speed, false);
    }

    fn say_text(mut self: Pin<&mut Self>, text: &QString) {
        let text = text.to_string();
        self.as_mut().wake();
        self.as_mut().interrupt();
        let speed = self.speed(Pace::Normal);
        self.speak_at(&text, speed, false);
    }

    fn interrupt(mut self: Pin<&mut Self>) {
        let id = self.utterance.fetch_add(1, Ordering::SeqCst) + 1;
        self.quiet.store(id, Ordering::SeqCst);
        if let Some(speaker) = &self.speaker {
            speaker.silence();
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
        self.as_mut().rust_mut().patience.reset_empty();
        self.as_mut().interrupt();
        self.listen_again();
    }

    fn retry_talk(mut self: Pin<&mut Self>) {
        if *self.talk_state() == ERROR && self.ears.recognition().is_some() {
            self.as_mut().set_talk_state(READY);
        }
    }

    /// Hands-free after a turn that gave nothing to send: listens again, but not for ever.
    fn listen_after_nothing(mut self: Pin<&mut Self>) {
        // After the third in a row the message stays and the microphone is closed.
        if self.as_mut().rust_mut().patience.listen_after_empty() {
            self.listen_again();
        }
    }

    /// Stops listening without sending anything (hands-free: the character speaks, or the
    /// learner typed instead).
    fn close_mic(mut self: Pin<&mut Self>) {
        if let Some(m) = self.as_mut().rust_mut().mic.take() {
            drop(m.stop());
            self.as_mut().set_level(0.0);
            if *self.talk_state() == LISTENING {
                self.as_mut().set_talk_state(READY);
            }
        }
    }

    /// The talk button: the learner starts a turn.
    fn start_talk(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().patience.reset_empty();
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
        let hands = if self.hands_free {
            Hands::Free
        } else {
            Hands::Button
        };
        let opened = Mic::open(&state(), hands);
        if let Some(e) = opened.unread {
            self.as_mut().say_unsaved(&e);
        }
        match opened.mic {
            Ok(m) => {
                self.as_mut().rust_mut().mic = Some(m);
                // The microphone is back (after an error and `retry_talk`).
                self.as_mut().set_speech_error(QString::default());
                self.as_mut().set_level(0.0);
                self.as_mut().set_talk_state(LISTENING);
            }
            Err(e) => {
                let text = ears::speech_text(&e, self.lang);
                self.as_mut().set_speech_error(QString::from(&text));
                self.as_mut().set_talk_state(ERROR);
            }
        }
    }

    fn poll_level(mut self: Pin<&mut Self>) {
        let heard = self
            .as_mut()
            .rust_mut()
            .mic
            .as_mut()
            .map(|m| m.poll(std::time::Instant::now()));
        match heard {
            None => self.as_mut().set_level(0.0),
            Some(Heard::Listening(level)) => self.as_mut().set_level(f64::from(level)),
            // Nothing said for a while: stop listening without sending anything. The next
            // click or reply starts it again.
            Some(Heard::GaveUp) => {
                self.as_mut().rust_mut().patience.reset_empty();
                if let Some(m) = self.as_mut().rust_mut().mic.take() {
                    drop(m.stop());
                }
                self.as_mut().set_level(0.0);
                self.as_mut().set_talk_state(READY);
            }
            // The set pause after speaking ends the turn (/LF101/).
            Some(Heard::Ended { quiet_secs }) => {
                self.as_mut().rust_mut().ended_by_pause = true;
                self.as_mut().stop_talk();
                // The speech ended a pause ago, not now.
                let end = std::time::Instant::now()
                    .checked_sub(Duration::from_secs_f64(quiet_secs))
                    .unwrap_or_else(std::time::Instant::now);
                self.as_mut().rust_mut().speech_end = Some(end);
            }
        }
    }

    /// Hands-free: listen again once the character is done (/LF100/).
    fn listen_again(mut self: Pin<&mut Self>) {
        let ready = *self.talk_state() == READY && !*self.busy() && !*self.finished();
        // Not in the placement: its stages change the scene between turns. And not into the
        // character's voice: it listens again when the character is done.
        let talk = ready
            && self.ears.recognition().is_some()
            && self.placement.is_none()
            && !self.speaking();
        if *self.hands_free() && talk {
            self.as_mut().open_mic();
        }
    }

    fn stop_talk(mut self: Pin<&mut Self>) {
        let Some(mic) = self.as_mut().rust_mut().mic.take() else {
            return;
        };
        self.as_mut().rust_mut().speech_end = Some(std::time::Instant::now());
        self.as_mut().set_level(0.0);
        match mic.stop() {
            Ok(rec) => self.recognize(rec),
            Err(e) => {
                let text = ears::speech_text(&e, self.lang);
                self.as_mut().set_speech_error(QString::from(&text));
                self.as_mut().set_talk_state(ERROR);
            }
        }
    }

    fn say_wav(self: Pin<&mut Self>, path: &QString) {
        let rec = Clip::from_wav(std::path::Path::new(&path.to_string()));
        if let Some(rec) = rec {
            self.recognize(rec);
        }
    }

    /// Recognises a recording on a worker thread and sends it as a spoken turn.
    fn recognize(mut self: Pin<&mut Self>, rec: Clip) {
        let Some(recognizer) = self.ears.recognition() else {
            self.as_mut().set_talk_state(ERROR);
            return;
        };
        if *self.busy() {
            return;
        }
        // No voice in it (no microphone, muted, wrong input): say so instead of letting the
        // recognition invent sounds from the silence (/LF104/). A tap too short to measure
        // goes on as before.
        if rec.check().no_voice() {
            let by_pause = std::mem::take(&mut self.as_mut().rust_mut().ended_by_pause);
            self.as_mut().set_talk_state(READY);
            let msg = words::no_sound(self.lang);
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
            let turn = self.log.turns();
            let kept = state().keep_recording(&id, turn, rec.samples(), rec.sample_rate(), now());
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
            let text = recognizer.text(&rec);
            let _ = thread.queue(move |mut q| {
                q.as_mut().set_busy(false);
                q.as_mut().set_talk_state(READY);
                q.as_mut().after_turn(by_pause, &text);
                if !text.is_empty() {
                    q.as_mut().rust_mut().patience.reset_empty();
                    if let Some(index) = q.as_mut().submit(&text, false) {
                        q.check_moras(index, text, rec);
                    }
                    return;
                }
                let msg = words::not_understood(q.lang);
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
    fn check_moras(&self, index: usize, text: String, rec: Clip) {
        let Some(check) = self.ears.moras() else {
            return;
        };
        if !state().profile().is_ok_and(|p| p.mora_hints) {
            return;
        }
        let lang = self.lang;
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let notes = match check.check(&rec, &text) {
                Ok(notes) => notes,
                Err(e) => {
                    let _ = thread
                        .queue(move |mut q| q.as_mut().say_mora_failed(&e, MoraFailure::Check));
                    return;
                }
            };
            if notes.is_empty() {
                return;
            }
            let _ = thread.queue(move |mut q| {
                q.as_mut()
                    .rust_mut()
                    .log
                    .add_mora(index, &text, &notes, lang);
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
            self.as_mut().rust_mut().patience.reset_empty();
            // Typed instead of spoken: the microphone closes, hands-free or not; a recording
            // finishing during this turn would run a second one beside it.
            self.as_mut().close_mic();
        }
        self.as_mut().wake();
        let index = self.as_mut().rust_mut().log.add_learner(&text, input);
        self.as_mut().set_busy(true);
        self.as_mut().set_help_text(QString::default());
        self.as_mut().set_help_level(0);
        self.as_mut().publish_turns();

        let thread = self.qt_thread();
        std::thread::spawn(move || {
            // The talk is held only to ask and to take the answer, not while the tutor
            // thinks: the window locks it for help and the listening aids.
            let asking = talk.lock().map(|mut t| t.ask(&text));
            let answered: Result<_, AiError> = asking
                .map_err(|e| AiError::Format(e.to_string()))
                .and_then(|a| a.run());
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
                                .map(|r| state().recorded_view(r, t.session(), q.lang));
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
                            let lang = rust.lang;
                            rust.log.answer(index, &outcome, lang);
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
                        q.as_mut().rust_mut().log.fail(index);
                        let msg = words::tutor_failed(&e, q.lang);
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
        // A recognition in flight is busy too (the tutor's own turn the talk knows).
        if *self.busy() {
            return;
        }
        let Some(talk) = self.talk.clone() else {
            return;
        };
        let helped = talk.lock().expect("talk lock").help(&state());
        self.as_mut().say_all_failed(&helped.failed);
        if let Some(step) = helped.step {
            self.as_mut().set_help_text(QString::from(&step.text));
            self.as_mut().set_help_level(i32::from(step.level));
        }
    }
}

/// Loads Kokoro and opens the speaker, then says the opening line.
fn load_voice(thread: &CxxQtThread<qobject::Conversation>, name: &str, lang: Lang) {
    let loaded = Speaker::load(name);
    let _ = thread.queue(move |mut q| {
        q.as_mut().set_speech_ready(true);
        let speaker = match loaded {
            Ok(speaker) => speaker,
            Err(e) => {
                let text = voice::voice_text(&e, lang);
                q.as_mut().set_voice_note(QString::from(&text));
                q.as_mut().rust_mut().debt.forgive();
                // Without a voice nothing is spoken: hands-free listens right away, in a new
                // talk, one continued and when the models are back after the idle time.
                q.as_mut().listen_again();
                return;
            }
        };
        q.as_mut().rust_mut().speaker = Some(speaker);
        q.as_mut().set_voice(QString::from("Übergangsstimme"));
        q.as_mut().set_voice_note(QString::default());
        let (lines, busy) = (q.log.len(), *q.busy());
        let on = if *q.voice_on() {
            voice::Voice::Ready
        } else {
            voice::Voice::Off
        };
        match q.as_mut().rust_mut().debt.voice_ready(lines, busy, on) {
            // The reply came before the voice did; no listening aid, no response time.
            Owed::Reply(markup) => q.speak(&markup),
            Owed::Opening => {
                let opening = q.log.opening().to_string();
                q.speak(&opening);
            }
            // A talk continued, or the models back after the idle time: hands-free
            // listens without waiting for a click.
            Owed::Nothing => q.listen_again(),
        }
    });
}

/// Loads the mora check (/LF320/). Without its model there are no hints; the settings say where
/// it comes from (`models::mora_model`).
fn load_mora(thread: &CxxQtThread<qobject::Conversation>) {
    match Moras::load() {
        Ok(check) => {
            let _ = thread.queue(move |mut q| q.as_mut().rust_mut().ears.set_moras(check));
        }
        Err(e) if ears::model_missing(&e) => {}
        Err(e) => {
            let _ = thread.queue(move |mut q| q.as_mut().say_mora_failed(&e, MoraFailure::Load));
        }
    }
}
