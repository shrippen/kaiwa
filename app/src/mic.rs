//! The microphone test in the settings (/LF104/): pick the microphone, record a sentence with a live
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
use kaiwa_backend::ears::{self, Clip, Hands, Mic, Recognition};
use kaiwa_speech::playback::Player;
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
    mic: Option<(Mic, Instant)>,
    recording: Option<Clip>,
    player: Option<Player>,
}

fn lang() -> String {
    state()
        .profile()
        .map(|p| p.ui_lang)
        .unwrap_or_else(|_| "de".into())
}

impl qobject::MicTest {
    fn refresh_devices(mut self: Pin<&mut Self>) {
        let list: Vec<_> = ears::devices()
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
        match Mic::open(&state(), Hands::Button).mic {
            Ok(m) => {
                self.as_mut().rust_mut().mic = Some((m, Instant::now()));
                self.as_mut().rust_mut().recording = None;
                self.as_mut().set_result(QString::default());
                self.as_mut().set_error(QString::default());
                self.as_mut().set_level(0.0);
                self.as_mut().set_remaining(SECONDS);
                self.as_mut().set_phase(RECORDING);
            }
            Err(e) => {
                let text = ears::speech_text(&e, &lang());
                self.as_mut().set_error(QString::from(&text));
                self.as_mut().set_phase(ERROR);
            }
        }
    }

    fn poll(mut self: Pin<&mut Self>) {
        if let Some((m, started)) = &self.mic {
            let level = f64::from(m.level());
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
        let Some((m, _)) = self.as_mut().rust_mut().mic.take() else {
            return;
        };
        self.as_mut().set_level(0.0);
        self.as_mut().set_remaining(0.0);
        let lang = lang();
        let rec = match m.stop() {
            Ok(r) => r,
            Err(e) => {
                let text = ears::speech_text(&e, &lang);
                self.as_mut().set_error(QString::from(&text));
                self.as_mut().set_phase(ERROR);
                return;
            }
        };
        let check = rec.check();
        let base = json!({
            "verdict": check.id(),
            "text": check.text(&lang),
            "speechDb": check.speech_db.round(),
            "noiseDb": check.noise_db.round(),
            "seconds": (check.seconds * 10.0).round() / 10.0,
            "heard": null,
            "recognition": "checking",
        });
        self.as_mut().set_result(QString::from(&base.to_string()));
        self.as_mut().rust_mut().recording = Some(rec.clone());
        if check.silent() {
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
            match Recognition::load() {
                Ok(r) => {
                    let text = r.text(&rec);
                    v["heard"] = text.trim().into();
                    v["recognition"] = "ok".into();
                }
                Err(e) if ears::model_missing(&e) => v["recognition"] = "missing".into(),
                Err(e) => {
                    v["recognition"] = "failed".into();
                    v["why"] = ears::test_text(&e, &lang).into();
                }
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
                    let text = ears::speech_text(&e, &lang());
                    // The result stays; the speaker problem shows under it.
                    self.as_mut().set_error(QString::from(&text));
                    return;
                }
            }
        }
        self.as_mut().set_error(QString::default());
        if let Some(p) = &self.player {
            p.silence();
            p.play(rec.samples(), rec.sample_rate());
        }
        self.as_mut().set_playing(true);
    }
}
