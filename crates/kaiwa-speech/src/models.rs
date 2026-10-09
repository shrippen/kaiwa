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
