//! The microphone and the speech recognition in the service (/LF505/, /LF510/): the plasmoid
//! holds no audio; it asks kaiwad to listen and to recognise. ReazonSpeech is loaded on the
//! first turn and stays until the service quits after its idle time (/LL012/). Built only
//! with the feature `speech` (ALSA and sherpa-onnx); without it the calls say so.

/// A recognised turn.
pub struct Heard {
    pub text: String,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[cfg(feature = "speech")]
mod imp {
    use super::Heard;
    use kaiwa_speech::capture::Capture;
    use kaiwa_speech::models::reazonspeech_dir;
    use kaiwa_speech::stt::Recognizer;

    #[derive(Default)]
    pub struct Ear {
        capture: Option<Capture>,
        recognizer: Option<Recognizer>,
    }

    impl Ear {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn start(&mut self) -> Result<(), String> {
            if self.recognizer.is_none() {
                self.recognizer =
                    Some(Recognizer::from_dir(&reazonspeech_dir(), 2).map_err(|e| e.to_string())?);
            }
            self.capture = Some(Capture::start().map_err(|e| e.to_string())?);
            Ok(())
        }

        /// What was said since `start`: the text ("" when nothing was understood) and the
        /// audio (to keep with the opt-in, /LF712/).
        pub fn stop(&mut self) -> Result<Heard, String> {
            let capture = self.capture.take().ok_or("not listening")?;
            let rec = capture.stop().map_err(|e| e.to_string())?;
            let r = self.recognizer.as_ref().ok_or("no recogniser")?;
            Ok(Heard {
                text: r.transcribe(&rec.samples, rec.sample_rate),
                samples: rec.samples,
                sample_rate: rec.sample_rate,
            })
        }
    }
}

#[cfg(not(feature = "speech"))]
mod imp {
    use super::Heard;

    const NONE: &str = "kaiwad was built without speech (feature `speech`)";

    pub struct Ear;

    impl Ear {
        pub fn new() -> Self {
            Self
        }
        pub fn start(&mut self) -> Result<(), String> {
            Err(NONE.into())
        }
        pub fn stop(&mut self) -> Result<Heard, String> {
            Err(NONE.into())
        }
    }
}

pub use imp::Ear;
