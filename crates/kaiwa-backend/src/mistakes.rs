//! The kinds of mistakes of the profile (/LF610/): per kind how often it came up in each of
//! the last talks, and whether it comes up more or less often than before.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Backend;
use crate::views::{kind_word, scenario_title};

/// Talks before a kind of mistake gets a trend, and how often it must have come up in them;
/// with fewer there is only the count (one mistake is no trend).
const TREND_TALKS: usize = 4;
const TREND_MISTAKES: u32 = 3;
/// Talks the profile's charts show, newest last.
const CHART_TALKS: usize = 10;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Change {
    Less,
    More,
    Same,
}

/// How a kind of mistake developed (/LF610/): how often it came up per turn in the later half
/// of the talks against the earlier half. `talks` are the talks in order, each with its turns
/// (without those marked as misrecognised, whose mistakes do not count either) and its
/// mistakes of this kind. Talks, not weeks: a week that has just begun, or the one the
/// learner started in, holds few talks and would read as a change. A quarter more or less
/// counts as a change. `None`: too little for a trend, only the count.
fn change(talks: &[(u32, u32)]) -> Option<Change> {
    let rate = |t: &[(u32, u32)]| {
        let (turns, mistakes) = t
            .iter()
            .fold((0, 0), |(a, b), (turns, m)| (a + turns, b + m));
        f64::from(mistakes) / f64::from(turns.max(1))
    };
    let n: u32 = talks.iter().map(|t| t.1).sum();
    if talks.len() < TREND_TALKS || n < TREND_MISTAKES {
        return None;
    }
    let (early, late) = talks.split_at(talks.len() / 2);
    let (early, late) = (rate(early), rate(late));
    Some(match late {
        l if l < 0.75 * early => Change::Less,
        l if l > 1.25 * early => Change::More,
        _ => Change::Same,
    })
}

/// The trend as a sentence.
fn trend(word: &str, talks: &[(u32, u32)], de: bool) -> String {
    let n: u32 = talks.iter().map(|t| t.1).sum();
    match (change(talks), de) {
        (None, true) => format!("{word}: bisher {n}×"),
        (None, false) => format!("{word}: {n}× so far"),
        (Some(Change::Less), true) => format!("{word}: seltener als in den Gesprächen davor"),
        (Some(Change::Less), false) => format!("{word}: less often than in the talks before"),
        (Some(Change::More), true) => {
            format!("{word}: zuletzt häufiger, ein Fall für die Wiederholung")
        }
        (Some(Change::More), false) => format!("{word}: more often lately, one for the review"),
        (Some(Change::Same), true) => format!("{word}: etwa gleich oft"),
        (Some(Change::Same), false) => format!("{word}: about as often"),
    }
}

/// The trend in one word for the chart's head; empty when there is none yet.
fn trend_word(talks: &[(u32, u32)], de: bool) -> &'static str {
    match (change(talks), de) {
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
    // The talks in order, for the trend and the charts: the turns of each and its mistakes per
    // kind, with its day and scenario.
    let mut talks: Vec<(u32, BTreeMap<String, u32>)> = Vec::new();
    let mut talk_names: Vec<Value> = Vec::new();
    for row in st.store.finished_since(from)? {
        let Ok(state) = st.store.session_state(row.id) else {
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
            talks.push((turns as u32, made));
            talk_names.push(json!({
                "date": row.started.day().iso(),
                "scenario": scenario_title(st, &row.scenario, lang),
            }));
        }
    }
    let mut kinds: Vec<(String, u32)> = totals.into_iter().collect();
    kinds.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let shown = talks.len().saturating_sub(CHART_TALKS);
    let mistakes: Vec<Value> = kinds
        .iter()
        .take(3)
        .map(|(k, _)| {
            let word = kind_word(k, de);
            let of_kind: Vec<(u32, u32)> = talks
                .iter()
                .map(|(turns, made)| (*turns, made.get(k).copied().unwrap_or(0)))
                .collect();
            json!({
                "kind": k,
                "word": word,
                "counts": of_kind[shown..].iter().map(|(_, n)| n).collect::<Vec<_>>(),
                "trend": trend_word(&of_kind, de),
                "text": trend(word, &of_kind, de),
            })
        })
        .collect();
    let talks = talk_names.split_off(shown);
    Ok((mistakes, talks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::stats;

    #[test]
    fn the_trend_compares_talks_not_weeks() {
        // (turns, mistakes) per talk. Fading over eight talks, and the other way round.
        let fading = [
            (4, 3),
            (4, 3),
            (4, 2),
            (4, 2),
            (4, 1),
            (4, 1),
            (4, 0),
            (4, 0),
        ];
        assert!(trend("Partikeln", &fading, true).contains("seltener"));
        assert_eq!(trend_word(&fading, true), "seltener");
        assert_eq!(trend_word(&fading, false), "less often");
        assert_eq!(trend_word(&[(4, 1)], true), "");
        let mut rising = fading;
        rising.reverse();
        assert!(trend("Particles", &rising, false).contains("more often"));
        // A week of one mistake a talk, then a Monday talk with four: more often, although
        // the new week has fewer mistakes than the last (review #67).
        let monday = [
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 4),
        ];
        assert!(trend("Partikeln", &monday, true).contains("häufiger"));
        // Two days of practice are no trend, whichever weeks they fall into.
        assert_eq!(
            trend("Partikeln", &[(4, 1), (4, 3)], true),
            "Partikeln: bisher 4×"
        );
        // Longer talks with as many mistakes per turn: the same.
        let longer = [(4, 1), (4, 1), (8, 2), (8, 2)];
        assert!(trend("Partikeln", &longer, true).contains("gleich"));
        assert_eq!(trend("Particles", &[], false), "Particles: 0× so far");
        // One mistake in twenty talks is no trend, wherever it falls (review #67).
        let mut once = [(4, 0); 20];
        once[10].1 = 1;
        assert_eq!(trend("Partikeln", &once, true), "Partikeln: bisher 1×");
        once[0].1 = 2;
        assert!(trend("Partikeln", &once, true).contains("seltener"));
    }

    #[test]
    fn misrecognised_turns_do_not_thin_out_the_trend() {
        // Four talks of four valid turns with 2, 1, 2, 1 particle mistakes; the third has
        // twelve more turns, all marked as misrecognised (review #68). Per valid turn the
        // later half is as the earlier: 3 of 8 each.
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
                let line = if turn < *mistakes {
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
        assert_eq!(particle["text"], "Partikel: etwa gleich oft", "{particle}");
    }

    #[test]
    fn mistakes_over_twelve_weeks() {
        let b = crate::TempBackend::new("stats", true).unwrap();
        let v = stats(&b, crate::now()).unwrap();
        let m = v["mistakes"].as_array().unwrap();
        let particle = m
            .iter()
            .find(|x| x["kind"] == "particle")
            .expect("particles");
        assert!(
            particle["text"].as_str().unwrap().contains("seltener"),
            "{particle}"
        );
        assert!(m.iter().any(|x| x["kind"] == "counter"), "{m:?}");
        // The charts: the last talks, each with day and scenario, the counts of a kind aligned.
        let talks = v["talks"].as_array().unwrap();
        assert!(!talks.is_empty() && talks.len() <= CHART_TALKS);
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
        assert!(
            v["vocab"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["level"] == "N5")
        );
    }
}
