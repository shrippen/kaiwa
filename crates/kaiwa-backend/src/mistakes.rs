//! The kinds of mistakes of the profile (/LF610/): per kind how often it came up in each talk
//! of the view's twelve weeks, and the trend the core reads from the same points.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use kaiwa_core::trend::{Change, change};

use crate::Backend;
use crate::views::{kind_word, scenario_title};

/// The trend in one word for the chart's head; empty when there is none yet.
fn trend_word(counts: &[u32], de: bool) -> &'static str {
    match (change(counts), de) {
        (None, _) => "",
        (Some(Change::Less), true) => "seltener",
        (Some(Change::Less), false) => "less often",
        (Some(Change::More), true) => "häufiger",
        (Some(Change::More), false) => "more often",
        (Some(Change::Same), true) => "gleich oft",
        (Some(Change::Same), false) => "as often",
    }
}

/// The three kinds that came up most in the talks finished since `from` (unix seconds): the
/// view's `mistakes` and the `talks` (day and scenario) their `counts` are aligned with.
pub(crate) fn view(st: &Backend, from: i64, de: bool) -> anyhow::Result<(Vec<Value>, Vec<Value>)> {
    let lang = if de { "de" } else { "en" };
    let mut totals: BTreeMap<String, u32> = BTreeMap::new();
    // The talks in order, for the charts and the trend: the mistakes of each per kind, with its
    // day and scenario.
    let mut talks: Vec<BTreeMap<String, u32>> = Vec::new();
    let mut talk_names: Vec<Value> = Vec::new();
    for row in st.store.finished_since(from)? {
        let Ok(state) = st.store.session_state(&row.id) else {
            continue;
        };
        let mut made: BTreeMap<String, u32> = BTreeMap::new();
        for t in state.turns.iter().filter(|t| !t.incomplete) {
            for k in kaiwa_core::session::mistake_kinds(t) {
                *made.entry(k.clone()).or_default() += 1;
                *totals.entry(k).or_default() += 1;
            }
        }
        let turns = state
            .turns
            .iter()
            .filter(|t| !t.misrecognized && !t.incomplete)
            .count();
        if turns > 0 {
            talks.push(made);
            talk_names.push(json!({
                "date": row.started.day().iso(),
                "scenario": scenario_title(st, &row.scenario, lang),
            }));
        }
    }
    let mut kinds: Vec<(String, u32)> = totals.into_iter().collect();
    kinds.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let mistakes: Vec<Value> = kinds
        .iter()
        .take(3)
        .map(|(k, _)| {
            let word = kind_word(k, de);
            let of_kind: Vec<u32> = talks
                .iter()
                .map(|made| made.get(k).copied().unwrap_or(0))
                .collect();
            json!({
                "kind": k,
                "word": word,
                "counts": of_kind,
                "trend": trend_word(&of_kind, de),
            })
        })
        .collect();
    Ok((mistakes, talk_names))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::stats;

    #[test]
    fn the_trend_word_names_the_core_change() {
        assert_eq!(trend_word(&[3, 3, 2, 2, 1, 1, 0, 0], true), "seltener");
        assert_eq!(trend_word(&[3, 3, 2, 2, 1, 1, 0, 0], false), "less often");
        assert_eq!(trend_word(&[0, 0, 1, 1, 2, 2], true), "häufiger");
        assert_eq!(trend_word(&[1], true), "");
    }

    #[test]
    fn misrecognised_turns_do_not_thin_out_the_trend() {
        // Four talks with 2, 1, 2, 1 particle mistakes; the third has twelve more turns with
        // the mistake, all marked as misrecognised (review #68): they are the recogniser's,
        // so the later half is as the earlier, 3 each.
        let b = crate::TempBackend::new("trend-misrecognised", false).unwrap();
        let scenario = b.scenarios.get("n5-02-jikoshoukai").cloned().unwrap();
        let now = crate::now();
        for (n, (mistakes, misrecognised)) in [(2, 0), (1, 0), (2, 12), (1, 0)].iter().enumerate() {
            let (provider, _) = b.provider(&scenario.id);
            let mut s = kaiwa_core::Session::new(
                scenario.clone(),
                provider,
                kaiwa_core::Amount::Little,
                "de",
            );
            for turn in 0..4 + misrecognised {
                let line = if turn < *mistakes || turn >= 4 {
                    "コーヒーを好きです。"
                } else {
                    "マラです。よろしくおねがいします。"
                };
                s.turn(line, kaiwa_core::Input::Spoken).unwrap();
                if turn >= 4 {
                    assert!(s.mark_misrecognized(turn));
                }
            }
            let started = now.plus_secs((n as i64 - 4) * 86_400);
            b.store
                .put_finished_session(&s.snapshot(), started, started.plus_secs(300), 0.8)
                .unwrap();
        }
        let v = stats(&b, now).unwrap();
        let particle = v["mistakes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["kind"] == "particle")
            .expect("particles")
            .clone();
        assert_eq!(particle["counts"], json!([2, 1, 2, 1]), "{particle}");
        assert_eq!(particle["trend"], "gleich oft", "{particle}");
    }

    #[test]
    fn mistakes_over_twelve_weeks() {
        let b = crate::TempBackend::new("stats", true).unwrap();
        let now = crate::now();
        let v = stats(&b, now).unwrap();
        let m = v["mistakes"].as_array().unwrap();
        let particle = m
            .iter()
            .find(|x| x["kind"] == "particle")
            .expect("particles");
        assert!(m.iter().any(|x| x["kind"] == "counter"), "{m:?}");
        // The charts: every talk of the twelve weeks, each with day and scenario, the counts of
        // a kind aligned.
        let talks = v["talks"].as_array().unwrap();
        let today = now.day();
        let monday = today.plus(-(today.weekday() as i64) - 7 * 11);
        let finished = b
            .store
            .finished_since(monday.start(now.offset_min).unix)
            .unwrap();
        assert_eq!(talks.len(), finished.len());
        assert_eq!(particle["counts"].as_array().unwrap().len(), talks.len());
        assert!(
            talks
                .iter()
                .all(|t| t["date"].as_str().unwrap().len() == 10)
        );
        assert!(
            talks
                .iter()
                .all(|t| !t["scenario"].as_str().unwrap().is_empty())
        );
        assert_eq!(particle["trend"], "seltener");
        // The word reads the points the chart shows.
        let counts: Vec<u32> = particle["counts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(
            kaiwa_core::trend::change(&counts),
            Some(kaiwa_core::trend::Change::Less)
        );
        assert!(
            v["vocab"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["level"] == "N5")
        );
    }
}
