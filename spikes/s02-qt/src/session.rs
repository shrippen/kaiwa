//! A conversation turn as a QObject: the talk button's state, the input level and the
//! transcript, driven from Rust. A worker thread stands in for STT, LLM and TTS and
//! hands its results back to the Qt thread, the path the real pipeline will take.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, talk_state, cxx_name = "talkState")]
        #[qproperty(f64, level)]
        #[qproperty(QString, heard)]
        #[qproperty(QString, reply)]
        type Session = super::SessionRust;

        /// The person pressed the talk button.
        #[qinvokable]
        fn start(self: Pin<&mut Session>);

        /// The person released it: recognise, answer, speak.
        #[qinvokable]
        fn stop(self: Pin<&mut Session>);

        /// The person interrupted the character.
        #[qinvokable]
        fn interrupt(self: Pin<&mut Session>);
    }

    impl cxx_qt::Threading for Session {}
}

use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;
use std::time::Duration;

/// Same order as KanteTalkButton.State.
pub mod state {
    pub const READY: i32 = 0;
    pub const LISTENING: i32 = 1;
    pub const THINKING: i32 = 2;
    pub const SPEAKING: i32 = 3;
}

#[derive(Default)]
pub struct SessionRust {
    talk_state: i32,
    level: f64,
    heard: QString,
    reply: QString,
}

impl qobject::Session {
    fn start(mut self: Pin<&mut Self>) {
        if *self.talk_state() != state::READY {
            return;
        }
        self.as_mut().set_level(0.6);
        self.as_mut().set_talk_state(state::LISTENING);
    }

    fn stop(mut self: Pin<&mut Self>) {
        if *self.talk_state() != state::LISTENING {
            return;
        }
        self.as_mut().set_level(0.0);
        self.as_mut().set_talk_state(state::THINKING);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            // Stand-in for STT and the tutor: results arrive on a worker thread.
            std::thread::sleep(Duration::from_millis(700));
            let queued = thread.queue(|mut s| {
                s.as_mut().set_heard(QString::from("ふくろ、いりません"));
                s.as_mut()
                    .set_reply(QString::from("かしこまりました。{温|あたた|new}めますか。"));
                s.as_mut().set_talk_state(state::SPEAKING);
            });
            if queued.is_err() {
                return; // The window closed meanwhile.
            }
            std::thread::sleep(Duration::from_millis(1500));
            let _ = thread.queue(|s| {
                if *s.talk_state() == state::SPEAKING {
                    s.set_talk_state(state::READY);
                }
            });
        });
    }

    fn interrupt(self: Pin<&mut Self>) {
        if *self.talk_state() == state::SPEAKING {
            self.set_talk_state(state::READY);
        }
    }
}
