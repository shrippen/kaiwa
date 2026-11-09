//! The character's interim voice (H-02): Kokoro-82M through ONNX Runtime, with the phonemes
//! from [`crate::g2p`]. Kokoro does not know the pitch accent (spike 0.4), so it only speaks
//! the character's replies. Corrections, examples to repeat and shadowing wait for VOICEVOX.
//!
//! ONNX Runtime is the one sherpa-onnx links in (1.28): `ort` uses it without linking its own.

mod cache;
mod vocab;

pub use cache::SpeechCache;

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;

use crate::SpeechError;
use crate::g2p::G2p;

pub const SAMPLE_RATE: u32 = 24_000;
/// Kokoro takes at most 510 phonemes at once.
const MAX_PHONEMES: usize = 510;
const STYLE: usize = 256;

pub struct Voice {
    session: Session,
    /// One style vector per phoneme count (510 × 256).
    styles: Vec<f32>,
    /// The voice the styles belong to, and the archive they come from.
    name: String,
    voices: std::path::PathBuf,
    g2p: G2p,
    vocab: HashMap<char, i64>,
}

/// The model file in `dir`: full precision first, else the int8 one.
fn model(dir: &Path) -> Option<std::path::PathBuf> {
    ["kokoro-v1.0.onnx", "kokoro-v1.0.int8.onnx"]
        .iter()
        .map(|f| dir.join(f))
        .find(|p| p.exists())
}

/// The float32 data of one `.npy` entry (little-endian, C order).
fn npy_f32(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.get(..6)? != b"\x93NUMPY" {
        return None;
    }
    let (header, start) = match bytes.get(6)? {
        1 => (usize::from(u16::from_le_bytes([bytes[8], bytes[9]])), 10),
        _ => (
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize,
            12,
        ),
    };
    let head = std::str::from_utf8(bytes.get(start..start + header)?).ok()?;
    if !head.contains("'<f4'") || head.contains("'fortran_order': True") {
        return None;
    }
    Some(
        bytes
            .get(start + header..)?
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    )
}

/// The style table of one voice from `voices-v1.0.bin` (an npz archive).
fn styles(path: &Path, voice: &str) -> Result<Vec<f32>, SpeechError> {
    let load = |why: String| SpeechError::ModelLoad(format!("{} ({why})", path.display()));
    let file = std::fs::File::open(path).map_err(|e| load(e.to_string()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| load(e.to_string()))?;
    let mut entry = zip
        .by_name(&format!("{voice}.npy"))
        .map_err(|_| load(format!("no voice {voice}")))?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|e| load(e.to_string()))?;
    let styles = npy_f32(&bytes).ok_or_else(|| load("not float32 npy".into()))?;
    if styles.len() != MAX_PHONEMES * STYLE {
        return Err(load(format!("{} values", styles.len())));
    }
    Ok(styles)
}

/// Splits a reply into sentences, so the first one can play while the next is made.
pub fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        current.push(c);
        if "。！？!?\n".contains(c) {
            if !current.trim().is_empty() {
                out.push(current.trim().to_string());
            }
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

impl Voice {
    /// Loads Kokoro from `dir` (`kokoro-v1.0.onnx` or the int8 one, `voices-v1.0.bin`) with
    /// one of its Japanese voices (jf_alpha, jf_gongitsune, jf_nezumi, jf_tebukuro, jm_kumo).
    pub fn from_dir(dir: &Path, voice: &str, threads: usize) -> Result<Self, SpeechError> {
        let missing = || SpeechError::ModelMissing(dir.display().to_string());
        let model = model(dir).ok_or_else(missing)?;
        let voices = dir.join("voices-v1.0.bin");
        if !voices.exists() {
            return Err(missing());
        }
        let load = |e: ort::Error| SpeechError::ModelLoad(format!("{} ({e})", model.display()));
        let onnx = crate::onnx();
        let session = Session::builder()
            .map_err(load)?
            .with_intra_threads(threads)
            .map_err(|e| load(e.into()))?
            // Every sentence has another length, so there is no memory pattern to reuse.
            .with_memory_pattern(false)
            .map_err(|e| load(e.into()))?
            .commit_from_file(&model)
            .map_err(load)?;
        drop(onnx);
        Ok(Self {
            session,
            styles: styles(&voices, voice)?,
            name: voice.to_string(),
            voices,
            g2p: G2p::new()?,
            vocab: vocab::VOCAB.iter().copied().collect(),
        })
    }

    /// The voice it speaks with (`jf_tebukuro`, `jm_kumo`, …).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Speaks with another voice from now on; only its styles load, the model stays. On an
    /// error it keeps the voice it had.
    pub fn select(&mut self, voice: &str) -> Result<(), SpeechError> {
        if voice != self.name {
            self.styles = styles(&self.voices, voice)?;
            self.name = voice.to_string();
        }
        Ok(())
    }

    pub fn phonemes(&self, text: &str) -> String {
        self.g2p.phonemes(text)
    }

    /// One sentence (or a short reply) as mono samples at [`SAMPLE_RATE`]. `speed` 1.0 is
    /// Kokoro's normal pace; a beginner's character speaks slower (0.9).
    pub fn speak(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, SpeechError> {
        let phonemes = self.phonemes(text);
        let mut audio = Vec::new();
        for chunk in chunks(&phonemes, MAX_PHONEMES - 2) {
            let tokens: Vec<i64> = chunk
                .chars()
                .filter_map(|c| self.vocab.get(&c).copied())
                .collect();
            if tokens.is_empty() {
                continue;
            }
            let row = tokens.len() - 1;
            let style = self.styles[row * STYLE..(row + 1) * STYLE].to_vec();
            let mut ids = Vec::with_capacity(tokens.len() + 2);
            ids.push(0);
            ids.extend(tokens);
            ids.push(0);
            let fail = |step: &str| {
                let step = step.to_string();
                move |e: ort::Error| SpeechError::Voice(format!("{step}: {e}"))
            };
            let n = ids.len() as i64;
            let _onnx = crate::onnx();
            let outputs = self
                .session
                .run(ort::inputs![
                    "tokens" => Tensor::from_array((vec![1i64, n], ids)).map_err(fail("tokens"))?,
                    "style" => Tensor::from_array((vec![1i64, STYLE as i64], style)).map_err(fail("style"))?,
                    "speed" => Tensor::from_array((vec![1i64], vec![speed])).map_err(fail("speed"))?,
                ])
                .map_err(fail("run"))?;
            let (_, samples) = outputs["audio"]
                .try_extract_tensor::<f32>()
                .map_err(fail("audio"))?;
            audio.extend_from_slice(samples);
        }
        Ok(audio)
    }
}

/// Phoneme strings longer than Kokoro takes, split at spaces.
fn chunks(phonemes: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for word in phonemes.split(' ') {
        let len = current.chars().count() + word.chars().count() + 1;
        if !current.is_empty() && len > max {
            out.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.extend(word.chars().take(max));
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_split_into_sentences() {
        assert_eq!(
            sentences("はじめまして。田中です。お名前は？ よろしく"),
            vec!["はじめまして。", "田中です。", "お名前は？", "よろしく"]
        );
    }

    #[test]
    fn long_phoneme_strings_split_at_spaces() {
        let long = vec!["abcd"; 300].join(" ");
        let parts = chunks(&long, 508);
        assert!(parts.iter().all(|p| p.chars().count() <= 508));
        assert_eq!(parts.join(" "), long);
    }

    #[test]
    fn npy_reads_float32() {
        let mut bytes = b"\x93NUMPY\x01\x00".to_vec();
        let head = "{'descr': '<f4', 'fortran_order': False, 'shape': (2,), }      \n";
        bytes.extend((head.len() as u16).to_le_bytes());
        bytes.extend(head.as_bytes());
        bytes.extend(1.5f32.to_le_bytes());
        bytes.extend((-2.0f32).to_le_bytes());
        assert_eq!(npy_f32(&bytes), Some(vec![1.5, -2.0]));
    }

    #[test]
    fn a_missing_model_says_where() {
        let err = Voice::from_dir(Path::new("/nonexistent"), "jf_tebukuro", 1).err();
        assert!(matches!(err, Some(SpeechError::ModelMissing(_))));
    }

    /// Kokoro speaks, ReazonSpeech hears it back. Needs both models (`KAIWA_MODELS`).
    #[test]
    fn the_voice_is_understood() {
        let dir = crate::models::kokoro_dir();
        let Ok(mut voice) = Voice::from_dir(&dir, "jf_tebukuro", 2) else {
            eprintln!("skipped: no Kokoro in {}", dir.display());
            return;
        };
        let Ok(rec) = crate::stt::Recognizer::from_dir(&crate::models::reazonspeech_dir(), 2)
        else {
            eprintln!("skipped: no ReazonSpeech");
            return;
        };
        let audio = voice.speak("コーヒーが好きです。", 1.0).unwrap();
        assert!(audio.len() > SAMPLE_RATE as usize / 2);
        let heard = rec.transcribe(&audio, SAMPLE_RATE);
        assert!(heard.contains("コーヒー"), "heard {heard}");
    }

    /// Another scenario's character speaks with its own voice; an unknown one keeps the old.
    /// Needs Kokoro (`KAIWA_MODELS`).
    #[test]
    fn select_switches_the_voice() {
        let dir = crate::models::kokoro_dir();
        let Ok(mut voice) = Voice::from_dir(&dir, "jf_tebukuro", 2) else {
            eprintln!("skipped: no Kokoro in {}", dir.display());
            return;
        };
        let before = voice.speak("はい。", 1.0).unwrap();
        voice.select("jm_kumo").unwrap();
        assert_eq!(voice.name(), "jm_kumo");
        assert_ne!(voice.speak("はい。", 1.0).unwrap(), before);
        assert!(voice.select("xx_none").is_err());
        assert_eq!(voice.name(), "jm_kumo");
    }

    /// Both runtimes at once, as in the app: the learner speaks while the character's line is
    /// still being made. Needs both models.
    #[test]
    fn voice_and_recognizer_run_side_by_side() {
        let (kokoro, reazon) = (
            crate::models::kokoro_dir(),
            crate::models::reazonspeech_dir(),
        );
        if !kokoro.exists() || !reazon.exists() {
            eprintln!("skipped: models missing");
            return;
        }
        let a = std::thread::spawn(move || {
            let mut voice = Voice::from_dir(&kokoro, "jf_tebukuro", 2).unwrap();
            (0..3)
                .map(|_| voice.speak("はじめまして。", 1.0).map(|a| a.len()))
                .collect::<Result<Vec<_>, _>>()
        });
        let b = std::thread::spawn(move || {
            let rec = crate::stt::Recognizer::from_dir(&reazon, 2).unwrap();
            let tone: Vec<f32> = (0..16_000).map(|i| (i as f32 * 0.05).sin() * 0.1).collect();
            (0..3)
                .map(|_| rec.transcribe(&tone, 16_000))
                .collect::<Vec<_>>()
                .len()
        });
        let spoken = a.join().unwrap();
        assert!(spoken.is_ok(), "{spoken:?}");
        assert_eq!(b.join().unwrap(), 3);
    }
}
