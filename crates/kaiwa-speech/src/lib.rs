//! Speech for Kaiwa: the microphone ([`capture`]), speech recognition ([`stt`],
//! ReazonSpeech through sherpa-onnx, ADR-002 / H-03) and the character's interim voice
//! ([`tts`], Kokoro with [`g2p`], H-02) through the speaker ([`playback`]); the mora check
//! ([`mora`], ADR-005). Models live outside the binary
//! ([`models`]); the app tells the learner where, when one is missing.

pub mod capture;
pub mod download;
pub mod g2p;
pub mod models;
pub mod mora;
pub mod playback;
pub mod stt;
pub mod tts;
pub mod vad;

#[derive(Debug, thiserror::Error)]
pub enum SpeechError {
    #[error("no microphone found")]
    NoMicrophone,
    #[error("the microphone could not be opened: {0}")]
    Microphone(String),
    #[error("speech model missing in {0}")]
    ModelMissing(String),
    #[error("the speech model could not be loaded from {0}")]
    ModelLoad(String),
    #[error("no speaker found")]
    NoSpeaker,
    #[error("the speaker could not be opened: {0}")]
    Speaker(String),
    #[error("the voice failed: {0}")]
    Voice(String),
}

/// Speech recognition behind a trait (/LL504/): another engine (whisper.cpp, a cloud service)
/// implements it, and the app does not change.
pub trait Transcriber: Send + Sync {
    /// The text of one utterance (mono, any sample rate).
    fn transcribe(&self, samples: &[f32], sample_rate: u32) -> String;
}

/// A voice behind a trait (/LL504/): another engine (VOICEVOX, a cloud voice) implements it.
pub trait Synthesizer: Send {
    /// One sentence as mono samples at [`Synthesizer::sample_rate`]; `speed` 1.0 is normal.
    fn speak(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, SpeechError>;
    fn sample_rate(&self) -> u32;
}

impl Transcriber for stt::Recognizer {
    fn transcribe(&self, samples: &[f32], sample_rate: u32) -> String {
        stt::Recognizer::transcribe(self, samples, sample_rate)
    }
}

impl Synthesizer for tts::Voice {
    fn speak(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, SpeechError> {
        tts::Voice::speak(self, text, speed)
    }
    fn sample_rate(&self) -> u32 {
        tts::SAMPLE_RATE
    }
}

static ONNX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// ONNX Runtime work (loading, recognising, synthesising) holds this lock, one call at a time;
/// a recognition waits for at most one sentence of synthesis.
pub(crate) fn onnx() -> std::sync::MutexGuard<'static, ()> {
    ONNX.lock().unwrap_or_else(|e| e.into_inner())
}

/// Call before the process exits. The runtime's static data is torn down at exit, while a
/// worker thread may still be synthesising the character's line; that run then fails inside
/// ONNX Runtime ("GetElementType is not implemented") or worse. This waits for the running
/// call and keeps every later one from starting.
pub fn shutdown() {
    std::mem::forget(onnx());
}

/// The buffer for a stream on `device`: PulseAudio (also PipeWire's) hands record data in
/// fragments of about 2 s and plays with as much delay unless a size is asked for, so it gets
/// `ms` milliseconds; ALSA keeps its default.
pub(crate) fn buffer_for(
    device: &cpal::Device,
    supported: &cpal::SupportedStreamConfig,
    ms: u32,
) -> cpal::BufferSize {
    use cpal::traits::DeviceTrait;
    let pulse = device
        .id()
        .is_ok_and(|id| id.to_string().starts_with("pulseaudio:"));
    if !pulse {
        return cpal::BufferSize::Default;
    }
    let frames = supported.sample_rate() * ms / 1000;
    match supported.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => {
            cpal::BufferSize::Fixed(frames.clamp(*min, *max))
        }
        cpal::SupportedBufferSize::Unknown => cpal::BufferSize::Fixed(frames),
    }
}

/// The stream config to open: the device's default, but as f32 when the default format is
/// one the streams do not take (PulseAudio offers s24 first and converts to anything).
/// `ranges` is asked only then: under ALSA it opens the device once more.
pub(crate) fn usable_config<R: Iterator<Item = cpal::SupportedStreamConfigRange>>(
    default: cpal::SupportedStreamConfig,
    ranges: impl FnOnce() -> R,
) -> cpal::SupportedStreamConfig {
    use cpal::SampleFormat::{F32, I16, I32, U16};
    if matches!(default.sample_format(), F32 | I16 | U16 | I32) {
        return default;
    }
    let rate = default.sample_rate();
    let channels = default.channels();
    let mut ranges: Vec<_> = ranges().filter(|r| r.sample_format() == F32).collect();
    // The default channel count first, so a mono microphone stays mono.
    ranges.sort_by_key(|r| r.channels() != channels);
    ranges
        .into_iter()
        .find_map(|r| r.try_with_sample_rate(rate))
        .unwrap_or(default)
}

/// Input level 0..1 for the talk button: RMS in dBFS, −60 dB → 0, 0 dB → 1.
pub fn level_of(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    if rms <= 0.0 {
        return 0.0;
    }
    ((20.0 * rms.log10() + 60.0) / 60.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_maps_dbfs_to_0_1() {
        assert_eq!(level_of(&[]), 0.0);
        assert_eq!(level_of(&[0.0; 100]), 0.0);
        assert!((level_of(&[1.0, -1.0]) - 1.0).abs() < 1e-6);
        let quiet = level_of(&[0.001; 100]); // −60 dB
        assert!(quiet < 0.01, "{quiet}");
        let mid = level_of(&[0.0316; 100]); // about −30 dB
        assert!((mid - 0.5).abs() < 0.02, "{mid}");
    }
}
