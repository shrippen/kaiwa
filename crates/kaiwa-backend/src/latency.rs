//! Response time (Lastenheft /LL010/): from the end of the learner's speech to the character's
//! first sound (or its text, without a voice). The last 50 are kept on the device, not in the
//! export; the settings show median and 95th percentile, and when the median is over 3 s they
//! suggest something faster without switching to it (persona Lena).

use serde_json::{Value, json};

use crate::Backend;

const KEY: &str = "latency";
const KEEP: usize = 50;
/// /LL010/: locally the median should be at most 3 s.
const SLOW_MS: u32 = 3000;

/// The value at `q` (0..1) of sorted milliseconds.
fn quantile(sorted: &[u32], q: f64) -> u32 {
    let i = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[i]
}

fn seconds(ms: u32, de: bool) -> String {
    let s = format!("{:.1}", f64::from(ms) / 1000.0);
    if de { s.replace('.', ",") } else { s }
}

impl Backend {
    /// One response time in milliseconds.
    pub fn add_latency(&self, ms: u32) -> anyhow::Result<()> {
        let mut all: Vec<u32> = self.store.state_value(KEY)?.unwrap_or_default();
        all.push(ms);
        if all.len() > KEEP {
            all.drain(..all.len() - KEEP);
        }
        Ok(self.store.set_state_value(KEY, &all)?)
    }

    /// Median and 95th percentile in words; `null` before five answers.
    pub fn latency_view(&self, lang: &str) -> anyhow::Result<Value> {
        let mut all: Vec<u32> = self.store.state_value(KEY)?.unwrap_or_default();
        if all.len() < 5 {
            return Ok(Value::Null);
        }
        all.sort_unstable();
        let (p50, p95) = (quantile(&all, 0.5), quantile(&all, 0.95));
        let de = lang == "de";
        let text = if de {
            format!(
                "Im Mittel {} s bis zur Antwort, fast immer unter {} s (letzte {} Antworten).",
                seconds(p50, de),
                seconds(p95, de),
                all.len()
            )
        } else {
            format!(
                "{} s to the answer on average, almost always under {} s (last {} answers).",
                seconds(p50, de),
                seconds(p95, de),
                all.len()
            )
        };
        let slow = p50 > SLOW_MS;
        let suggestion = match (slow, de) {
            (false, _) => String::new(),
            (true, true) => "Das ist langsamer als vorgesehen. Schneller wäre ein kleineres lokales Sprachmodell für den Tutor oder ein Cloud-Tutor; Kaiwa stellt nichts von selbst um.".into(),
            (true, false) => "That is slower than intended. A smaller local language model for the tutor or a cloud tutor would be faster; Kaiwa changes nothing by itself.".into(),
        };
        Ok(json!({
            "p50": p50,
            "p95": p95,
            "count": all.len(),
            "text": text,
            "slow": slow,
            "suggestion": suggestion,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend(name: &str) -> crate::TempBackend {
        crate::TempBackend::new(&format!("lat-{name}"), false).unwrap()
    }

    #[test]
    fn nothing_before_five_answers() {
        let b = backend("few");
        for ms in [1000, 1200, 900, 1100] {
            b.add_latency(ms).unwrap();
        }
        assert_eq!(b.latency_view("de").unwrap(), Value::Null);
    }

    #[test]
    fn median_percentile_and_a_suggestion_when_slow() {
        let b = backend("slow");
        for ms in [2000, 4000, 3500, 3200, 5000, 3100] {
            b.add_latency(ms).unwrap();
        }
        let v = b.latency_view("de").unwrap();
        assert_eq!(v["p50"], 3500);
        assert_eq!(v["p95"], 5000);
        assert_eq!(v["slow"], true);
        assert!(v["text"].as_str().unwrap().contains("3,5 s"));
        assert!(!v["suggestion"].as_str().unwrap().is_empty());
    }

    #[test]
    fn keeps_the_last_fifty() {
        let b = backend("keep");
        for ms in 0..60 {
            b.add_latency(1000 + ms).unwrap();
        }
        let v = b.latency_view("en").unwrap();
        assert_eq!(v["count"], 50);
        assert_eq!(v["slow"], false);
        assert_eq!(v["suggestion"], "");
    }
}
