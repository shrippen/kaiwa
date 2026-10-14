//! Grammar lint (INHALT-ENTWURF 3.2, /LF221/): every grammar point a unit names occurs in the
//! unit's texts (opening, help sentences, the character's recorded replies), and no help
//! sentence uses a point that a later unit introduces. A match inside a longer match of a point
//! already introduced is no Vorgriff (ました in ありがとうございました). The patterns are in
//! `data/grammar/*.json`; points without one are listed and skipped.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kaiwa_core::curriculum::Curriculum;
use kaiwa_core::grammar::Catalogue;
use kaiwa_core::scenario::Scenario;
use regex::Regex;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `{漢字|かな}` → `漢字`.
fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        match (tail.find('|'), tail.find('}')) {
            (Some(bar), Some(end)) if bar < end => {
                out.push_str(&tail[..bar]);
                rest = &tail[end + 1..];
            }
            _ => {
                out.push('{');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

struct UnitTexts {
    /// What the learner is helped to say: start and example of each goal.
    learner: Vec<String>,
    /// Everything the unit shows: the above, the openings, the replies without a correction.
    all: Vec<String>,
}

fn texts(scenarios: &[String]) -> UnitTexts {
    let mut learner = Vec::new();
    let mut all = Vec::new();
    for id in scenarios {
        let s = Scenario::from_json(
            &std::fs::read_to_string(root().join(format!("data/scenarios/{id}.json"))).unwrap(),
        )
        .unwrap();
        all.push(plain(&s.opening));
        for g in &s.goals {
            learner.push(plain(&g.help.example));
            learner.push(plain(&g.help.start));
        }
        let fp = root().join(format!("qa/fixtures/{id}.json"));
        if let Ok(f) = std::fs::read_to_string(fp) {
            let turns: serde_json::Value = serde_json::from_str(&f).unwrap();
            for t in turns.as_array().into_iter().flatten() {
                let corrected = t["analysis"]["corrections"]
                    .as_array()
                    .is_some_and(|c| !c.is_empty());
                if !corrected {
                    all.push(t["reply"].as_str().unwrap_or_default().to_string());
                }
            }
        }
    }
    all.extend(learner.iter().cloned());
    UnitTexts { learner, all }
}

#[test]
fn grammar_points_occur_in_their_unit_and_not_before() {
    let c = Curriculum::from_dir(&root().join("data/curriculum")).unwrap();
    let cat = Catalogue::from_dir(&root().join("data/grammar")).unwrap();
    let pattern: BTreeMap<&str, Regex> = cat
        .points
        .iter()
        .filter_map(|p| {
            p.pattern.as_deref().map(|r| {
                (
                    p.label.as_str(),
                    Regex::new(r).unwrap_or_else(|e| panic!("{}: {e}", p.id)),
                )
            })
        })
        .collect();
    // The unit (index in path order) that introduces each point.
    let mut first: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, u) in c.units.iter().enumerate() {
        for g in &u.grammar {
            first.entry(g.as_str()).or_insert(i);
        }
    }
    let mut missing = Vec::new();
    let mut ahead = Vec::new();
    for (i, u) in c.units.iter().enumerate() {
        let t = texts(&u.scenarios);
        for g in &u.grammar {
            if let Some(re) = pattern.get(g.as_str())
                && !t.all.iter().any(|x| re.is_match(x))
            {
                missing.push(format!("{}: {g}", u.id));
            }
        }
        for sentence in &t.learner {
            let shields: Vec<(usize, usize)> = first
                .iter()
                .filter(|(_, j)| **j <= i)
                .filter_map(|(g, _)| pattern.get(g))
                .flat_map(|re| re.find_iter(sentence).map(|m| (m.start(), m.end())))
                .collect();
            for (g, j) in first.iter().filter(|(_, j)| **j > i) {
                let Some(re) = pattern.get(g) else { continue };
                let hit = re.find_iter(sentence).any(|m| {
                    let (a, b) = (m.start(), m.end());
                    !shields
                        .iter()
                        .any(|(x, y)| *x <= a && b <= *y && y - x > b - a)
                });
                if hit {
                    ahead.push(format!(
                        "{}: „{sentence}“ uses {g}, introduced in {}",
                        u.id, c.units[*j].id
                    ));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "grammar points not in their unit: {missing:#?}"
    );
    assert!(ahead.is_empty(), "grammar used before its unit: {ahead:#?}");
}

#[test]
fn only_a_few_points_go_without_a_pattern() {
    let cat = Catalogue::from_dir(&root().join("data/grammar")).unwrap();
    let without: Vec<&str> = cat
        .points
        .iter()
        .filter(|p| p.pattern.is_none())
        .map(|p| p.id.as_str())
        .collect();
    assert_eq!(
        without,
        ["futsuukei", "transitivity"],
        "points without a pattern"
    );
}

#[test]
fn plain_drops_the_readings() {
    assert_eq!(plain("{駅|えき}まで{歩|ある}く"), "駅まで歩く");
}
