//! The cost brake (Lastenheft /LF714/): what a cloud tutor costs, estimated from the tokens each
//! answer reports and the list prices in `data/prices.json`, and a limit the learner sets.
//!
//! [`Metered`] wraps the transport of a provider: it reads the usage of every answer (Anthropic
//! `input_tokens`/`output_tokens`, OpenAI `prompt_tokens`/`completion_tokens`) into the
//! process-wide [`meter`], and refuses a call once the budget is spent ([`AiError::Budget`]).
//! The profile keeps the month's total; the app books the meter into it after each turn.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;
use serde_json::Value;

use crate::AiError;
use crate::transport::Transport;

#[derive(Debug, Deserialize)]
struct PriceFile {
    usd_to_eur: f64,
    models: BTreeMap<String, (f64, f64)>,
}

fn prices() -> &'static PriceFile {
    static P: OnceLock<PriceFile> = OnceLock::new();
    P.get_or_init(|| {
        serde_json::from_str(include_str!("../data/prices.json")).expect("bundled prices are valid")
    })
}

/// Euro cents for `input` and `output` tokens of `model`; `None` when the price is unknown.
pub fn cents(model: &str, input: u64, output: u64) -> Option<f64> {
    let p = prices();
    let (i, o) = p.models.get(model)?;
    Some((input as f64 * i + output as f64 * o) / 1e6 * p.usd_to_eur * 100.0)
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Spent {
    /// Euro cents of the calls with a known price.
    pub cents: f64,
    /// Calls whose model has no price (counted, not guessed).
    pub unknown_calls: u32,
    pub calls: u32,
}

/// What the cloud calls of this process cost since the last [`Meter::take`].
#[derive(Debug, Default)]
pub struct Meter {
    spent: Mutex<Spent>,
    /// Cents still allowed this month; `None` without a limit.
    budget: Mutex<Option<f64>>,
}

impl Meter {
    pub fn record(&self, model: &str, input: u64, output: u64) {
        let mut s = self.spent.lock().unwrap_or_else(|e| e.into_inner());
        s.calls += 1;
        match cents(model, input, output) {
            Some(c) => s.cents += c,
            None => s.unknown_calls += 1,
        }
    }

    /// What was spent since the last call, and starts again from zero.
    pub fn take(&self) -> Spent {
        std::mem::take(&mut *self.spent.lock().unwrap_or_else(|e| e.into_inner()))
    }

    pub fn peek(&self) -> Spent {
        self.spent.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The cents left this month (the limit minus what the profile booked); `None`: no limit.
    pub fn set_budget(&self, cents: Option<f64>) {
        *self.budget.lock().unwrap_or_else(|e| e.into_inner()) = cents;
    }

    /// Whether the budget is spent (counting what is not booked yet).
    pub fn exhausted(&self) -> bool {
        let budget = *self.budget.lock().unwrap_or_else(|e| e.into_inner());
        budget.is_some_and(|b| self.peek().cents >= b)
    }
}

/// The meter of this process.
pub fn meter() -> &'static Meter {
    static M: OnceLock<Meter> = OnceLock::new();
    M.get_or_init(Meter::default)
}

/// Tokens of an answer: (input, output).
pub fn usage(answer: &Value) -> Option<(u64, u64)> {
    let u = &answer["usage"];
    let n = |a: &str, b: &str| u[a].as_u64().or_else(|| u[b].as_u64());
    Some((
        n("input_tokens", "prompt_tokens")?,
        n("output_tokens", "completion_tokens")?,
    ))
}

/// A transport that meters and brakes (see the module).
pub struct Metered<T: Transport> {
    pub inner: T,
    pub meter: &'static Meter,
}

impl<T: Transport> Metered<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            meter: meter(),
        }
    }
}

impl<T: Transport> Transport for Metered<T> {
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, String)],
        body: &Value,
    ) -> Result<Value, AiError> {
        if self.meter.exhausted() {
            return Err(AiError::Budget);
        }
        let answer = self.inner.post_json(url, headers, body)?;
        if let Some((i, o)) = usage(&answer) {
            let model = answer["model"]
                .as_str()
                .or_else(|| body["model"].as_str())
                .unwrap_or_default();
            self.meter.record(model, i, o);
        }
        Ok(answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::FakeTransport;
    use serde_json::json;

    #[test]
    fn prices_are_per_million_tokens() {
        // 1 M input tokens of a $1 model: 1 $ = 92 ct.
        let c = cents("claude-haiku-5-5", 1_000_000, 0).unwrap();
        assert!((c - 92.0).abs() < 1e-9);
        assert!(cents("qwen3:8b", 1000, 1000).is_none());
    }

    #[test]
    fn metered_calls_count_and_brake() {
        let m: &'static Meter = Box::leak(Box::default());
        let t = Metered {
            inner: FakeTransport::new(json!({
                "model": "claude-sonnet-5-5",
                "usage": { "input_tokens": 2000, "output_tokens": 300 },
                "content": []
            })),
            meter: m,
        };
        t.post_json("u", &[], &json!({})).unwrap();
        let spent = m.peek();
        assert_eq!(spent.calls, 1);
        assert!(spent.cents > 0.0);
        m.set_budget(Some(spent.cents / 2.0));
        assert!(matches!(
            t.post_json("u", &[], &json!({})),
            Err(AiError::Budget)
        ));
        m.set_budget(None);
        assert!(t.post_json("u", &[], &json!({})).is_ok());
        assert_eq!(m.take().calls, 2);
        assert_eq!(m.peek().calls, 0);
    }

    #[test]
    fn unknown_models_are_counted_not_guessed() {
        let m = Meter::default();
        m.record("local-model", 5000, 500);
        let s = m.peek();
        assert_eq!((s.calls, s.unknown_calls), (1, 1));
        assert_eq!(s.cents, 0.0);
        assert_eq!(
            usage(&json!({ "usage": { "prompt_tokens": 10, "completion_tokens": 3 } })),
            Some((10, 3))
        );
    }
}
