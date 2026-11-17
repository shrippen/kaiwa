//! Listening (/LF100/, /LF104/, /LF320/, /LF510/): the microphone, the recognition and the
//! mora check behind the backend, so a front end holds no hardware and no rule of them.
//!
//! ```text
//!  front end ─► Mic ──────► Clip ─► Recognition ─► text
//!               │ poll        │
//!               ▼             └────► Moras ─► [MoraNote]
//!             Heard           └────► Check (loudness)
//! ```

mod text;
mod watch;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use kaiwa_core::Lang;

pub use kaiwa_speech::SpeechError;
use kaiwa_speech::Transcriber;
use kaiwa_speech::capture::{self, Capture, Recording, Verdict};
use kaiwa_speech::g2p::G2p;
use kaiwa_speech::models::{models_dir, reazonspeech_dir};
use kaiwa_speech::mora::{self, Checker, Hint, Kind};
use kaiwa_speech::stt::Recognizer;

use crate::Backend;
use crate::log::{MoraKind, MoraNote};
use watch::Watch;

pub use text::{MoraFailure, mora_text, recognizer_text, speech_text, test_text};
pub use watch::{HANDS_FREE_WAIT_SECS, Heard, Offer, Patience};

/// Shorter recordings (a tap) are not checked for a voice.
const MIN_CHECKED_SECS: f32 = 0.5;
/// Threads the models use.
const THREADS: usize = 2;

/// Who ends a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hands {
    /// The pause after speaking ends it (/LF101/).
    Free,
    /// The learner stops it.
    Button,
}

/// What was recorded: mono samples.
#[derive(Clone)]
pub struct Clip(Recording);

impl Clip {
    pub fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        Self(Recording {
            samples,
            sample_rate,
        })
    }

    /// A WAV file as a clip; `None` if it cannot be read.
    pub fn from_wav(path: &Path) -> Option<Self> {
        let mut r = hound::WavReader::open(path).ok()?;
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
        let mono = capture::to_mono(&samples, channels).collect();
        Some(Self::new(mono, spec.sample_rate))
    }

    pub fn samples(&self) -> &[f32] {
        &self.0.samples
    }

    pub fn sample_rate(&self) -> u32 {
        self.0.sample_rate
    }

    /// How the clip sounds (/LF104/).
    pub fn check(&self) -> Check {
        Check::from(capture::check(&self.0))
    }
}

/// How a recording sounds: loud enough, clear, with a voice in it at all.
#[derive(Debug, Clone, Copy)]
pub struct Check {
    pub speech_db: f32,
    pub noise_db: f32,
    pub seconds: f32,
    verdict: Verdict,
    voice: bool,
}

impl From<capture::Check> for Check {
    fn from(c: capture::Check) -> Self {
        Self {
            speech_db: c.speech_db,
            noise_db: c.noise_db,
            seconds: c.seconds,
            verdict: c.verdict,
            voice: c.voice(),
        }
    }
}

impl Check {
    /// Nothing comes in at all.
    pub fn silent(&self) -> bool {
        self.verdict == Verdict::Silent
    }

    /// No voice in a recording long enough to tell: the recognition would only invent sounds
    /// from it. A tap too short to measure is not counted.
    pub fn no_voice(&self) -> bool {
        self.seconds >= MIN_CHECKED_SECS && !self.voice
    }

    pub fn id(&self) -> &'static str {
        match self.verdict {
            Verdict::Silent => "silent",
            Verdict::Quiet => "quiet",
            Verdict::Noisy => "noisy",
            Verdict::Clipping => "clipping",
            Verdict::Good => "good",
        }
    }

    /// The verdict in words: what it means and what to do.
    pub fn text(&self, lang: Lang) -> &'static str {
        text::verdict_text(self.verdict, lang)
    }
}

/// An input the learner can choose.
pub struct Device {
    pub id: String,
    pub name: String,
    pub default: bool,
}

/// The inputs of this machine.
pub fn devices() -> Vec<Device> {
    capture::input_devices()
        .into_iter()
        .map(|d| Device {
            id: d.id,
            name: d.name,
            default: d.default,
        })
        .collect()
}

/// An open microphone, with the hands-free watch of its turn.
pub struct Mic {
    capture: Capture,
    /// Hands-free: the watch, and when the level was last read.
    watch: Option<(Watch, Instant)>,
}

/// A microphone that opened or why not, and a failure to read the profile on the way (the
/// default microphone and pause stand in, the front end tells the learner).
pub struct Opened {
    pub mic: Result<Mic, SpeechError>,
    pub unread: Option<String>,
}

impl Mic {
    /// Opens the profile's microphone and, hands-free, watches for the end of the turn.
    pub fn open(b: &Backend, hands: Hands) -> Opened {
        let (profile, unread) = match b.profile() {
            Ok(p) => (Some(p), None),
            Err(e) => (None, Some(e.to_string())),
        };
        let microphone = profile.as_ref().and_then(|p| p.microphone.clone());
        let mic = Capture::start_on(microphone.as_deref()).map(|capture| {
            let watch = (hands == Hands::Free).then(|| {
                let pause = profile
                    .as_ref()
                    .map_or(kaiwa_core::turn_end::DEFAULT_PAUSE, |p| p.end_pause_secs);
                (Watch::new(pause), Instant::now())
            });
            Self { capture, watch }
        });
        Opened { mic, unread }
    }

    /// The level now, for the meter.
    pub fn level(&self) -> f32 {
        self.capture.level()
    }

    /// Reads the level at `now`; hands-free tells whether the turn is over.
    pub fn poll(&mut self, now: Instant) -> Heard {
        let level = self.capture.level();
        let Some((watch, last)) = &mut self.watch else {
            return Heard::Listening(level);
        };
        let dt = now.saturating_duration_since(*last).as_secs_f64();
        *last = now;
        watch.feed(level, dt)
    }

    /// Closes the microphone and returns what it recorded.
    pub fn stop(self) -> Result<Clip, SpeechError> {
        self.capture.stop().map(Clip)
    }
}

/// The speech recognition (/LL504/): any engine behind the trait; ReazonSpeech today.
#[derive(Clone)]
pub struct Recognition(Arc<dyn Transcriber>);

impl Recognition {
    /// Loads ReazonSpeech; takes a few seconds, so off the GUI thread.
    pub fn load() -> Result<Self, SpeechError> {
        let r = Recognizer::from_dir(&reazonspeech_dir(), THREADS as i32)?;
        Ok(Self(Arc::new(r)))
    }

    /// What was said, empty if nothing was understood.
    pub fn text(&self, clip: &Clip) -> String {
        self.0.transcribe(clip.samples(), clip.sample_rate())
    }
}

/// Whether the recognition model is simply not there (as against broken).
pub fn model_missing(e: &SpeechError) -> bool {
    matches!(e, SpeechError::ModelMissing(_))
}

/// The mora check's model and the reading of what the learner meant (/LF320/).
struct MoraCheck {
    checker: Checker,
    g2p: G2p,
}

#[derive(Clone)]
pub struct Moras(Arc<Mutex<MoraCheck>>);

impl Moras {
    /// Loads the check; without its model there are no hints (`ModelMissing`).
    pub fn load() -> Result<Self, SpeechError> {
        let checker = Checker::from_dir(&models_dir().join(mora::MODEL), THREADS)?;
        let check = MoraCheck {
            checker,
            g2p: G2p::new()?,
        };
        Ok(Self(Arc::new(Mutex::new(check))))
    }

    /// The hint for a spoken turn: only shortenings, only in sentences, at most one (/LF320/;
    /// K-01, K-02, J-05). Empty when nothing is found; the error when the check fails.
    pub fn check(&self, clip: &Clip, text: &str) -> Result<Vec<MoraNote>, SpeechError> {
        let Ok(mut c) = self.0.lock() else {
            return Ok(Vec::new());
        };
        let Some(words) = c.g2p.pronounced_words(text) else {
            return Ok(Vec::new());
        };
        let hints = c
            .checker
            .hints(clip.samples(), clip.sample_rate(), &words)?;
        Ok(hints.iter().map(mora_note).collect())
    }
}

/// A mora hint for the log: the backend words it.
fn mora_note(h: &Hint) -> MoraNote {
    let kind = match h.kind {
        Kind::GeminateLeftOut => MoraKind::Geminate,
        _ => MoraKind::Vowel,
    };
    MoraNote {
        kind,
        surface: h.surface.clone(),
        meant: h.meant.clone(),
        heard: h.heard.clone(),
    }
}

/// The listening models a front end holds while a talk runs.
#[derive(Default)]
pub struct Ears {
    recognition: Option<Recognition>,
    moras: Option<Moras>,
}

impl Ears {
    pub fn recognition(&self) -> Option<Recognition> {
        self.recognition.clone()
    }

    pub fn set_recognition(&mut self, r: Recognition) {
        self.recognition = Some(r);
    }

    pub fn moras(&self) -> Option<Moras> {
        self.moras.clone()
    }

    pub fn set_moras(&mut self, m: Moras) {
        self.moras = Some(m);
    }

    pub fn loaded(&self) -> bool {
        self.recognition.is_some() || self.moras.is_some()
    }

    /// Frees the models after the idle time; the readings (UniDic) load again with the next
    /// line that needs them.
    pub fn unload(&mut self) {
        self.recognition = None;
        self.moras = None;
        kaiwa_lang::unidic::unload();
    }
}

/// Call before the process exits (see `kaiwa_speech::shutdown`).
pub fn shutdown() {
    kaiwa_speech::shutdown();
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 16_000;

    fn tone(secs: f32, amplitude: f32) -> Clip {
        let n = (RATE as f32 * secs) as usize;
        let samples = (0..n)
            .map(|i| amplitude * (i as f32 * 440.0 * std::f32::consts::TAU / RATE as f32).sin())
            .collect();
        Clip::new(samples, RATE)
    }

    #[test]
    fn silence_is_no_voice() {
        let check = tone(1.0, 0.0).check();
        assert!(check.silent());
        assert!(check.no_voice());
        assert_eq!(check.id(), "silent");
    }

    #[test]
    fn a_tone_has_a_voice() {
        let check = tone(1.0, 0.3).check();
        assert!(!check.silent());
        assert!(!check.no_voice());
    }

    #[test]
    fn a_tap_too_short_to_measure_is_not_counted() {
        assert!(!tone(0.1, 0.0).check().no_voice());
    }

    #[test]
    fn a_shortened_vowel_and_a_left_out_geminate_are_told_apart() {
        let hint = |kind| Hint {
            kind,
            surface: "学校".into(),
            meant: "がっこう".into(),
            heard: "がこう".into(),
            margin: 0.0,
        };
        let note = mora_note(&hint(Kind::GeminateLeftOut));
        assert!(matches!(note.kind, MoraKind::Geminate));
        assert_eq!(
            (note.surface.as_str(), note.heard.as_str()),
            ("学校", "がこう")
        );
        assert!(matches!(
            mora_note(&hint(Kind::LongShort)).kind,
            MoraKind::Vowel
        ));
    }
}
