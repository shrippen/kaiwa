//! The microphone test in the settings: pick the microphone, record a sentence with a live
//! level, then hear it back, see whether it is loud and clear enough (`capture::check`) and,
//! with ReazonSpeech present, what the recognition understood. Nothing is stored or sent;
//! the recording lives only in this object.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        /// The microphones: [{"id", "name", "default"}], the default first.
        #[qproperty(QString, devices)]
        /// 0 ready, 1 recording, 2 checking (recognition), 3 result shown, 4 error.
        #[qproperty(i32, phase)]
        /// Input level 0..1 (−60..0 dBFS) while recording.
        #[qproperty(f64, level)]
        /// Seconds left of the recording.
        #[qproperty(f64, remaining)]
        /// {"verdict", "text", "speechDb", "noiseDb", "seconds", "heard", "recognition"}.
        #[qproperty(QString, result)]
        /// Why the microphone did not open, in words.
        #[qproperty(QString, error)]
        #[qproperty(bool, playing)]
        type MicTest = super::MicTestRust;

        /// Reads the list of microphones again (a headset plugged in).
        #[qinvokable]
        #[cxx_name = "refreshDevices"]
        fn refresh_devices(self: Pin<&mut MicTest>);

        /// Starts a test recording on the microphone of the profile.
        #[qinvokable]
        fn start(self: Pin<&mut MicTest>);

        /// Ends the recording early and checks it.
        #[qinvokable]
        fn stop(self: Pin<&mut MicTest>);

        /// Level, time left and playback (QML polls it while recording or playing).
        #[qinvokable]
        fn poll(self: Pin<&mut MicTest>);

        /// Plays the test recording back.
        #[qinvokable]
        fn play(self: Pin<&mut MicTest>);
    }

    impl cxx_qt::Threading for MicTest {}
}

use core::pin::Pin;
use std::time::Instant;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use kaiwa_speech::capture::{self, Capture, Recording, Verdict};
use kaiwa_speech::playback::Player;
use kaiwa_speech::stt::Recognizer;
use serde_json::json;

use crate::state::state;

// 0 is ready, the default.
const RECORDING: i32 = 1;
const CHECKING: i32 = 2;
const DONE: i32 = 3;
const ERROR: i32 = 4;

/// Long enough for a sentence like the ones in a conversation.
pub const SECONDS: f64 = 6.0;

#[derive(Default)]
pub struct MicTestRust {
    devices: QString,
    phase: i32,
    level: f64,
    remaining: f64,
    result: QString,
    error: QString,
    playing: bool,
    capture: Option<(Capture, Instant)>,
    recording: Option<Recording>,
    player: Option<Player>,
}

fn lang() -> String {
    state()
        .store
        .profile()
        .map(|p| p.ui_lang)
        .unwrap_or_else(|_| "de".into())
}

/// The verdict in words: what it means and what to do.
fn verdict_text(v: Verdict, de: bool) -> &'static str {
    match (v, de) {
        (Verdict::Silent, true) => {
            "Es kommt nichts an. Ist das Mikrofon stummgeschaltet, das falsche gewählt oder hat Kaiwa keinen Zugriff darauf?"
        }
        (Verdict::Silent, false) => {
            "Nothing comes in. Is the microphone muted, the wrong one chosen, or does Kaiwa have no access to it?"
        }
        (Verdict::Quiet, true) => {
            "Zu leise für die Spracherkennung. Näher ans Mikrofon oder die Eingangslautstärke im System höher stellen."
        }
        (Verdict::Quiet, false) => {
            "Too quiet for the recognition. Move closer to the microphone or raise the input volume in the system."
        }
        (Verdict::Noisy, true) => {
            "Deine Stimme hebt sich kaum vom Hintergrund ab. Ein ruhigerer Ort oder ein Headset hilft."
        }
        (Verdict::Noisy, false) => {
            "Your voice hardly stands out from the background. A quieter place or a headset helps."
        }
        (Verdict::Clipping, true) => {
            "Zu laut, die Spitzen werden abgeschnitten. Etwas Abstand halten oder die Eingangslautstärke senken."
        }
        (Verdict::Clipping, false) => {
            "Too loud, the peaks are cut off. Keep a little distance or lower the input volume."
        }
        (Verdict::Good, true) => "Gut: laut genug und klar.",
        (Verdict::Good, false) => "Good: loud enough and clear.",
    }
}

fn verdict_id(v: Verdict) -> &'static str {
    match v {
        Verdict::Silent => "silent",
        Verdict::Quiet => "quiet",
        Verdict::Noisy => "noisy",
        Verdict::Clipping => "clipping",
        Verdict::Good => "good",
    }
}

impl qobject::MicTest {
    fn refresh_devices(mut self: Pin<&mut Self>) {
        let list: Vec<_> = capture::input_devices()
            .into_iter()
            .map(|d| json!({ "id": d.id, "name": d.name, "default": d.default }))
            .collect();
        self.as_mut()
            .set_devices(QString::from(&serde_json::Value::from(list).to_string()));
    }

    fn start(mut self: Pin<&mut Self>) {
        if *self.phase() == RECORDING || *self.phase() == CHECKING {
            return;
        }
        if let Some(p) = &self.player {
            p.silence();
        }
        self.as_mut().set_playing(false);
        let microphone = state().store.profile().ok().and_then(|p| p.microphone);
        match Capture::start_on(microphone.as_deref()) {
            Ok(c) => {
                self.as_mut().rust_mut().capture = Some((c, Instant::now()));
                self.as_mut().rust_mut().recording = None;
                self.as_mut().set_result(QString::default());
                self.as_mut().set_error(QString::default());
                self.as_mut().set_level(0.0);
                self.as_mut().set_remaining(SECONDS);
                self.as_mut().set_phase(RECORDING);
            }
            Err(e) => {
                let text = crate::conversation::speech_text(&e, &lang());
                self.as_mut().set_error(QString::from(&text));
                self.as_mut().set_phase(ERROR);
            }
        }
    }

    fn poll(mut self: Pin<&mut Self>) {
        if let Some((c, started)) = &self.capture {
            let level = f64::from(c.level());
            let left = (SECONDS - started.elapsed().as_secs_f64()).max(0.0);
            self.as_mut().set_level(level);
            self.as_mut().set_remaining(left);
            if left <= 0.0 {
                self.as_mut().stop();
            }
        }
        let playing = self.player.as_ref().is_some_and(Player::playing);
        if *self.playing() != playing {
            self.as_mut().set_playing(playing);
        }
    }

    fn stop(mut self: Pin<&mut Self>) {
        let Some((c, _)) = self.as_mut().rust_mut().capture.take() else {
            return;
        };
        self.as_mut().set_level(0.0);
        self.as_mut().set_remaining(0.0);
        let lang = lang();
        let de = lang == "de";
        let rec = match c.stop() {
            Ok(r) => r,
            Err(e) => {
                let text = crate::conversation::speech_text(&e, &lang);
                self.as_mut().set_error(QString::from(&text));
                self.as_mut().set_phase(ERROR);
                return;
            }
        };
        let check = capture::check(&rec);
        let base = json!({
            "verdict": verdict_id(check.verdict),
            "text": verdict_text(check.verdict, de),
            "speechDb": check.speech_db.round(),
            "noiseDb": check.noise_db.round(),
            "seconds": (check.seconds * 10.0).round() / 10.0,
            "heard": null,
            "recognition": "checking",
        });
        self.as_mut().set_result(QString::from(&base.to_string()));
        self.as_mut().rust_mut().recording = Some(rec.clone());
        if check.verdict == Verdict::Silent {
            let mut v = base;
            v["recognition"] = "skipped".into();
            self.as_mut().set_result(QString::from(&v.to_string()));
            self.as_mut().set_phase(DONE);
            return;
        }
        // The recognition, when its model is there: loads in a few seconds, off the GUI thread.
        self.as_mut().set_phase(CHECKING);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let mut v = base;
            match Recognizer::from_dir(&kaiwa_speech::models::reazonspeech_dir(), 2) {
                Ok(r) => {
                    let text = r.transcribe(&rec.samples, rec.sample_rate);
                    v["heard"] = text.trim().into();
                    v["recognition"] = "ok".into();
                }
                Err(_) => v["recognition"] = "missing".into(),
            }
            let _ = thread.queue(move |mut q| {
                q.as_mut().set_result(QString::from(&v.to_string()));
                q.as_mut().set_phase(DONE);
            });
        });
    }

    fn play(mut self: Pin<&mut Self>) {
        let Some(rec) = self.recording.clone() else {
            return;
        };
        if self.player.is_none() {
            match Player::start() {
                Ok(p) => self.as_mut().rust_mut().player = Some(p),
                Err(e) => {
                    let text = crate::conversation::speech_text(&e, &lang());
                    self.as_mut().set_error(QString::from(&text));
                    return;
                }
            }
        }
        if let Some(p) = &self.player {
            p.silence();
            p.play(&rec.samples, rec.sample_rate);
        }
        self.as_mut().set_playing(true);
    }
}
