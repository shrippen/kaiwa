//! Speech recognition with ReazonSpeech (zipformer transducer, sherpa-onnx). The utterance
//! gets 0.3 s of silence before and after: without it the model drops the first words
//! (spike 0.3).

use std::path::Path;

use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig};

use crate::SpeechError;

pub const PAD_SECONDS: f32 = 0.3;

pub struct Recognizer {
    inner: OfflineRecognizer,
}

/// `samples` with [`PAD_SECONDS`] of silence on both sides.
pub fn pad(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let n = (sample_rate as f32 * PAD_SECONDS) as usize;
    let mut out = vec![0.0; n];
    out.extend_from_slice(samples);
    out.resize(out.len() + n, 0.0);
    out
}

fn pick(dir: &Path, prefix: &str) -> Option<String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            n.starts_with(prefix) && n.ends_with(".onnx")
        })
        .collect();
    // The int8 models are a quarter of the size and as accurate (spike 0.3).
    files.sort_by_key(|p| !p.to_string_lossy().contains(".int8."));
    files.first().map(|p| p.to_string_lossy().into_owned())
}

impl Recognizer {
    pub fn from_dir(dir: &Path, threads: i32) -> Result<Self, SpeechError> {
        let missing = || SpeechError::ModelMissing(dir.display().to_string());
        let tokens = dir.join("tokens.txt");
        if !tokens.exists() {
            return Err(missing());
        }
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.transducer.encoder = Some(pick(dir, "encoder").ok_or_else(missing)?);
        config.model_config.transducer.decoder = Some(pick(dir, "decoder").ok_or_else(missing)?);
        config.model_config.transducer.joiner = Some(pick(dir, "joiner").ok_or_else(missing)?);
        config.model_config.tokens = Some(tokens.to_string_lossy().into_owned());
        config.model_config.num_threads = threads;
        let inner = OfflineRecognizer::create(&config)
            .ok_or_else(|| SpeechError::ModelLoad(dir.display().to_string()))?;
        Ok(Self { inner })
    }

    /// The text of one utterance (mono, any sample rate; sherpa-onnx resamples).
    pub fn transcribe(&self, samples: &[f32], sample_rate: u32) -> String {
        if samples.is_empty() {
            return String::new();
        }
        let stream = self.inner.create_stream();
        stream.accept_waveform(sample_rate as i32, &pad(samples, sample_rate));
        self.inner.decode(&stream);
        stream
            .get_result()
            .map(|r| r.text.trim().to_string())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_adds_silence_on_both_sides() {
        let p = pad(&[0.5; 10], 100);
        assert_eq!(p.len(), 30 + 10 + 30);
        assert_eq!(p[29], 0.0);
        assert_eq!(p[30], 0.5);
        assert_eq!(p[40], 0.0);
    }

    #[test]
    fn missing_model_is_named() {
        let err = Recognizer::from_dir(Path::new("/nonexistent/kaiwa"), 1)
            .err()
            .unwrap();
        assert!(matches!(err, SpeechError::ModelMissing(_)));
    }

    /// Runs when the model is installed (`KAIWA_MODELS` or the default place); its own test
    /// recordings must come out as Japanese text.
    #[test]
    fn transcribes_the_models_test_recording() {
        let dir = crate::models::reazonspeech_dir();
        let Some(wav) = std::fs::read_dir(dir.join("test_wavs"))
            .ok()
            .and_then(|mut d| d.next())
            .and_then(|e| e.ok())
        else {
            eprintln!("skipped: no ReazonSpeech model in {}", dir.display());
            return;
        };
        let rec = Recognizer::from_dir(&dir, 2).unwrap();
        let mut reader = hound::WavReader::open(wav.path()).unwrap();
        let spec = reader.spec();
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| f32::from(s.unwrap()) / 32768.0)
            .collect();
        let text = rec.transcribe(&samples, spec.sample_rate);
        assert!(
            text.chars().any(|c| ('\u{3040}'..='\u{30ff}').contains(&c)
                || ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "{text}"
        );
    }
}
