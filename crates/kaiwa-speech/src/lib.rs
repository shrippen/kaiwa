//! Speech for Kaiwa: the microphone ([`capture`]) and speech recognition ([`stt`],
//! ReazonSpeech through sherpa-onnx, ADR-002 / H-03). Models live outside the binary
//! ([`models`]); the app tells the learner where, when one is missing.

pub mod capture;
pub mod models;
pub mod stt;

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
