//! Loading ReazonSpeech for a talk.

use cxx_qt::{CxxQtThread, CxxQtType};
use cxx_qt_lib::QString;
use kaiwa_backend::ears::{self, Recognition};

use super::{ERROR, READY, SPEAKING, qobject};

/// Loads ReazonSpeech; without the model the talk button shows why, and the talk page offers
/// the models instead of the talk button (`speech_missing`).
pub(super) fn load_recognizer(thread: &CxxQtThread<qobject::Conversation>, lang: &str) {
    let lang = lang.to_string();
    let loaded = Recognition::load();
    let _ = thread.queue(move |mut q| match loaded {
        Ok(rec) => {
            q.as_mut().rust_mut().ears.set_recognition(rec);
            q.as_mut().set_speech_error(QString::default());
            q.as_mut().set_speech_missing(false);
            if *q.talk_state() != SPEAKING {
                q.as_mut().set_talk_state(READY);
            }
        }
        Err(e) => {
            let text = ears::recognizer_text(&e, &lang);
            q.as_mut().set_speech_error(QString::from(&text));
            q.as_mut().set_speech_missing(ears::model_missing(&e));
            q.as_mut().set_talk_state(ERROR);
        }
    });
}
