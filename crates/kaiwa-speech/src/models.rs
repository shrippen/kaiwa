//! Where the speech models live: `$KAIWA_MODELS`, else `$XDG_DATA_HOME/kaiwa/models`, else
//! `~/.local/share/kaiwa/models`. The models are downloaded once (F-08: source, size and
//! licence shown first); they never go into the repository or the binary.

use std::path::PathBuf;

/// The ReazonSpeech zipformer from sherpa-onnx's model releases (Apache-2.0).
pub const REAZONSPEECH: &str = "sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01";
pub const REAZONSPEECH_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01.tar.bz2";

pub fn models_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("KAIWA_MODELS") {
        return PathBuf::from(dir);
    }
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    data.unwrap_or_else(|| PathBuf::from("."))
        .join("kaiwa/models")
}

pub fn reazonspeech_dir() -> PathBuf {
    models_dir().join(REAZONSPEECH)
}

/// Kokoro-82M v1.0 as ONNX (Apache-2.0), from the kokoro-onnx releases: `kokoro-v1.0.onnx`
/// (310 MB) or `kokoro-v1.0.int8.onnx` (88 MB), and `voices-v1.0.bin` (27 MB).
pub const KOKORO: &str = "kokoro-v1.0";
pub const KOKORO_URL: &str =
    "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/";

/// Kaiwa's own kana CTC model for the mora check (ADR-005, K-04 to K-06; Apache-2.0), on
/// Hugging Face at a fixed revision: `model.int8.onnx` (356 MB), `vocab.json` and
/// `thresholds.json`, into the directory [`crate::mora::MODEL`].
pub const KANA_CTC_URL: &str = "https://huggingface.co/shrippen/kaiwa-kana-ctc/resolve/99dd836ea672ee18abc88c7d6bb54fbbd187835b/";

pub fn kokoro_dir() -> PathBuf {
    models_dir().join(KOKORO)
}
