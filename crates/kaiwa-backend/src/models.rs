//! The speech models (F-08, M-03, /LF320/): what the settings list, what a missing dictionary
//! offers, and the download with its progress and its words. A front end holds the thread
//! and shows the progress; the models and the rules around them stay here.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use kaiwa_core::Lang;
use kaiwa_speech::download::{self, DownloadError, MODELS, Model};
use kaiwa_speech::models::{KANA_CTC_URL, models_dir};
use kaiwa_speech::mora;
use serde_json::{Value, json};

/// Progress is reported once per this many bytes, so a front end keeps up.
const PROGRESS_STEP: u64 = 1_000_000;
/// The model's listed sizes are in megabytes.
const MB: f64 = 1e6;

fn mb(bytes: u64) -> u64 {
    (bytes as f64 / MB).round() as u64
}

/// What the learner reads about a model before the download: source, size, licence.
fn offer_of(m: &Model) -> Value {
    json!({
        "dir": m.dir,
        "name": m.name,
        "licence": m.licence,
        "origin": m.origin,
        "downloadMB": mb(m.download_bytes()),
        "diskMB": mb(m.disk_bytes),
    })
}

/// What the download of the model in `dir` would bring (M-03); `None` for an unknown one.
pub fn offer(dir: &str) -> Option<Value> {
    MODELS.iter().find(|m| m.dir == dir).map(offer_of)
}

fn purpose(name: &str, lang: Lang) -> &'static str {
    match (name, lang == Lang::De) {
        ("ReazonSpeech", true) => "Spracherkennung: versteht, was du sagst",
        ("ReazonSpeech", false) => "Speech recognition: understands what you say",
        ("Kokoro", true) => "Übergangsstimme der Figur",
        ("Kokoro", false) => "The character's interim voice",
        ("kaiwa-data", true) => "Wörterbuch: Lesung, Bedeutung und Akzent beim Klick auf ein Wort",
        ("kaiwa-data", false) => "Dictionary: reading, meaning and accent when you click a word",
        ("UniDic", true) => {
            "Lesungen: Furigana über deinen eigenen Sätzen und wo der Tutor keine schreibt"
        }
        ("UniDic", false) => {
            "Readings: furigana over your own sentences and where the tutor writes none"
        }
        (_, true) => "Aussprache-Hinweise zu langen Vokalen und っ",
        (_, false) => "Pronunciation hints on long vowels and っ",
    }
}

/// The models for the settings: what each is for, where it comes from, its size and licence,
/// and whether it is there (F-08).
pub fn list(lang: Lang) -> Value {
    let rows: Vec<Value> = MODELS
        .iter()
        .map(|m| {
            let mut row = offer_of(m);
            row["purpose"] = purpose(m.name, lang).into();
            row["present"] = m.present().into();
            row
        })
        .collect();
    json!(rows)
}

/// The mora check's model for the settings: whether it is there, where it goes and where it
/// comes from (F-08: source, size and licence before any download).
pub fn mora_model() -> Value {
    let dir = models_dir().join(mora::MODEL);
    let download_mb = offer(mora::MODEL).map_or(Value::Null, |o| o["downloadMB"].clone());
    json!({
        "present": dir.join("model.int8.onnx").exists() || dir.join("model.onnx").exists(),
        "id": mora::MODEL,
        "dir": dir.display().to_string(),
        "url": KANA_CTC_URL,
        "downloadMB": download_mb,
        "size": format!("{download_mb} MB"),
        "licence": "Apache-2.0",
    })
}

/// A download that can run: the model and the bytes it will bring.
pub struct Fetch {
    model: Model,
    pub total: u64,
}

/// How a download ended.
pub struct Outcome {
    name: &'static str,
    result: Result<(), DownloadError>,
}

impl Fetch {
    /// The download of the model in `dir`; `None` for an unknown one.
    pub fn of(dir: &str) -> Option<Self> {
        let model = *MODELS.iter().find(|m| m.dir == dir)?;
        Some(Self {
            total: model.download_bytes(),
            model,
        })
    }

    /// Downloads on this thread. `progress` gets the bytes so far, at most once per megabyte;
    /// setting `stop` ends it early.
    pub fn run(&self, stop: &AtomicBool, progress: &dyn Fn(u64)) -> Outcome {
        let last = AtomicU64::new(0);
        let throttled = |done: u64| {
            if !due(done, last.load(Ordering::Relaxed), self.total) {
                return;
            }
            last.store(done, Ordering::Relaxed);
            progress(done);
        };
        Outcome {
            name: self.model.name,
            result: download::download(&self.model, &throttled, stop),
        }
    }
}

/// Whether progress at `done` is worth reporting after `last`: a new megabyte, or the end.
fn due(done: u64, last: u64, total: u64) -> bool {
    done / PROGRESS_STEP != last / PROGRESS_STEP || done >= total
}

impl Outcome {
    /// What the learner reads, in the UI language.
    pub fn text(&self, lang: Lang) -> String {
        let name = self.name;
        match (&self.result, lang == Lang::De) {
            (Ok(()), true) => format!("{name} ist geladen."),
            (Ok(()), false) => format!("{name} is downloaded."),
            (Err(DownloadError::Stopped), true) => "Download abgebrochen.".to_string(),
            (Err(DownloadError::Stopped), false) => "Download stopped.".to_string(),
            (Err(DownloadError::Stalled(s)), true) => format!(
                "{name} ließ sich nicht laden: Die Verbindung hat {s} s lang keine Daten geliefert. Versuch es noch einmal."
            ),
            (Err(DownloadError::Stalled(s)), false) => format!(
                "{name} could not be downloaded: the connection gave no data for {s} s. Please try again."
            ),
            (Err(e), true) => format!("{name} ließ sich nicht laden: {e}"),
            (Err(e), false) => format!("{name} could not be downloaded: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(result: Result<(), DownloadError>) -> Outcome {
        Outcome {
            name: "Kokoro",
            result,
        }
    }

    #[test]
    fn every_model_is_listed_with_its_purpose_and_size() {
        let v = list(Lang::En);
        let rows = v.as_array().unwrap();
        assert_eq!(rows.len(), MODELS.len());
        for (row, m) in rows.iter().zip(MODELS.iter()) {
            assert_eq!(row["dir"], m.dir);
            assert!(row["purpose"].as_str().is_some_and(|p| !p.is_empty()));
            assert!(row["downloadMB"].as_u64().unwrap() > 0);
            assert!(row["present"].is_boolean());
        }
        assert_ne!(list(Lang::De)[0]["purpose"], list(Lang::En)[0]["purpose"]);
    }

    #[test]
    fn the_offer_names_source_size_and_licence() {
        let m = &MODELS[0];
        let v = offer(m.dir).unwrap();
        for key in ["name", "licence", "origin", "downloadMB", "diskMB"] {
            assert!(!v[key].is_null(), "{key}");
        }
        assert_eq!(offer("no-such-model"), None);
        assert!(Fetch::of("no-such-model").is_none());
    }

    #[test]
    fn the_mora_model_says_where_it_comes_from() {
        let v = mora_model();
        assert_eq!(v["url"], KANA_CTC_URL);
        assert!(v["present"].is_boolean());
    }

    #[test]
    fn the_mora_model_takes_its_size_from_the_catalogue() {
        let v = mora_model();
        let listed = offer(mora::MODEL).unwrap();
        assert_eq!(v["id"], mora::MODEL);
        assert!(v["downloadMB"].as_u64().unwrap() > 0);
        assert_eq!(v["downloadMB"], listed["downloadMB"]);
        assert_eq!(v["size"], format!("{} MB", listed["downloadMB"]));
    }

    #[test]
    fn progress_is_reported_per_megabyte_and_at_the_end() {
        let total = 5_000_000;
        assert!(!due(100, 0, total));
        assert!(!due(999_999, 0, total));
        assert!(due(1_000_000, 0, total));
        assert!(!due(1_500_000, 1_000_000, total));
        assert!(due(total, 4_500_000, total));
    }

    #[test]
    fn a_download_ends_in_words_in_both_languages() {
        assert_eq!(outcome(Ok(())).text(Lang::De), "Kokoro ist geladen.");
        assert_eq!(outcome(Ok(())).text(Lang::En), "Kokoro is downloaded.");
        let stopped = || outcome(Err(DownloadError::Stopped));
        assert_eq!(stopped().text(Lang::De), "Download abgebrochen.");
        assert_eq!(stopped().text(Lang::En), "Download stopped.");
        let stalled = || outcome(Err(DownloadError::Stalled(30)));
        assert!(stalled().text(Lang::De).contains("30 s lang keine Daten"));
        assert!(stalled().text(Lang::En).contains("no data for 30 s"));
        let failed = || outcome(Err(DownloadError::Http("503".into())));
        assert_eq!(
            failed().text(Lang::De),
            "Kokoro ließ sich nicht laden: download failed: 503"
        );
        assert_eq!(
            failed().text(Lang::En),
            "Kokoro could not be downloaded: download failed: 503"
        );
    }
}
