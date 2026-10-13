//! The curriculum as a graph of units (Lastenheft /LF220/, /LD040/) and the learning path the
//! learner sees (/LF600/).
//!
//! A unit has can-do statements, grammar points, core words and scenarios, and requires other
//! units. It unlocks when those are done **and** their reviews mostly go well (at least 60 % of
//! the answers right once there are three or more), so new material does not pile up on
//! shaky old material (Hinnerk). Skipped units (/LF215/) count as done. Every state has a word
//! and a symbol in the GUI, never only a colour.
//!
//! Format v2 (ADR-009): one file per level (`data/curriculum/n5.json`, `n4.json`, …) with the
//! level it is oriented on, units may require units of other levels, and [`Curriculum::merge`]
//! joins the files into one graph that [`Curriculum::check`] checks as a whole. All v2 fields
//! are optional, a v1 file reads as before.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::scenario::Text;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub id: String,
    pub title: Text,
    /// What the learner can do after the unit (JF standard style).
    pub can_do: Text,
    #[serde(default)]
    pub grammar: Vec<String>,
    #[serde(default)]
    pub words: Vec<String>,
    /// Scenario ids, in order.
    pub scenarios: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    /// JF Standard topics 1–15 the unit covers (Q-07), for the coverage check.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<u8>,
    /// Ids of JF Can-do statements the unit's can-do is oriented on (Q-10). Only the ids: the
    /// can-do text is Kaiwa's own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jf_can_do: Vec<u32>,
    /// The native speaker review (Q-07, INHALT-ENTWURF 3.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<Review>,
    /// The level of the file the unit comes from, set by [`Curriculum::merge`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
}

/// What a level is oriented on (Q-03): shown as „N4 · A2“, never as a JLPT course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub jlpt: String,
    pub cefr: String,
}

impl Level {
    /// „N4 · A2“.
    pub fn label(&self) -> String {
        format!("{} · {}", self.jlpt, self.cefr)
    }
}

/// Proof of the native speaker review of a unit: who, when, and a hash over the unit and its
/// scenarios as they were reviewed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub by: Option<String>,
    pub date: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Curriculum {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
    pub units: Vec<Unit>,
}

/// The levels in course order: the files are joined in this order.
pub const LEVELS: [&str; 5] = ["n5", "n4", "n3", "n2", "n1"];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CurriculumError {
    #[error("unit {0} requires unknown unit {1}")]
    UnknownUnit(String, String),
    #[error("units {0:?} require each other in a circle")]
    Cycle(Vec<String>),
    #[error("unit id {0} appears twice")]
    Duplicate(String),
    #[error("unit {0} names JF topic {1}, there are 15")]
    Topic(String, u8),
    #[error("unit {0} has the grammar point {1}, which is not in the catalogue")]
    UnknownGrammar(String, String),
    #[error("grammar point {0} is first used in {1}, but its level is {2}")]
    GrammarLevel(String, String, String),
}

/// What the learner has done, as far as the path is concerned.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourseProgress {
    /// Scenarios done: completed [`COMPLETIONS`] times with at least [`DONE_SUCCESS`].
    pub scenarios_done: BTreeSet<String>,
    /// Good completions so far per scenario.
    #[serde(default)]
    pub completions: BTreeMap<String, u32>,
    pub skipped_units: BTreeSet<String>,
    /// Reviews of each unit's items: (answers, right answers).
    #[serde(default)]
    pub reviews: BTreeMap<String, (u32, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    Done,
    /// The next thing to do.
    Current,
    /// Unlocked, not the next.
    Open,
    Locked,
}

impl NodeState {
    pub fn word(self, lang: &str) -> &'static str {
        let de = lang == "de";
        match self {
            NodeState::Done => {
                if de {
                    "Erledigt"
                } else {
                    "Done"
                }
            }
            NodeState::Current => {
                if de {
                    "Jetzt"
                } else {
                    "Now"
                }
            }
            NodeState::Open => {
                if de {
                    "Offen"
                } else {
                    "Open"
                }
            }
            NodeState::Locked => {
                if de {
                    "Gesperrt"
                } else {
                    "Locked"
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioNode {
    pub id: String,
    pub state: NodeState,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnitNode {
    pub id: String,
    pub title: String,
    pub can_do: String,
    /// „N5 · A1“, from the unit's level file.
    pub level: Option<String>,
    pub state: NodeState,
    pub scenarios: Vec<ScenarioNode>,
    /// Why a locked unit is locked, in words.
    pub locked_because: Option<String>,
}

/// Reviews of a unit go well enough to build on it.
pub const REVIEW_OK: f64 = 0.6;
pub const REVIEW_MIN: u32 = 3;
/// A scenario is done after this many good completions: once is luck, twice is learnt.
pub const COMPLETIONS: u32 = 2;
pub const DONE_SUCCESS: f64 = 0.75;

impl CourseProgress {
    /// A session reached all its goals with `success`. Returns true when the scenario is now done.
    pub fn complete(&mut self, scenario: &str, success: f64) -> bool {
        if success < DONE_SUCCESS || self.scenarios_done.contains(scenario) {
            return false;
        }
        let n = self.completions.entry(scenario.to_string()).or_default();
        *n += 1;
        if *n >= COMPLETIONS {
            self.scenarios_done.insert(scenario.to_string());
            return true;
        }
        false
    }

    /// Records a review answer for a unit's item.
    pub fn reviewed(&mut self, unit: &str, right: bool) {
        let r = self.reviews.entry(unit.to_string()).or_default();
        r.0 += 1;
        r.1 += u32::from(right);
    }
}

impl Curriculum {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// The level files joined into one course, in [`LEVELS`] order (unknown ids last); each
    /// unit keeps its file's level.
    pub fn merge(mut parts: Vec<Curriculum>) -> Curriculum {
        let rank = |id: &str| LEVELS.iter().position(|l| *l == id).unwrap_or(LEVELS.len());
        parts.sort_by_key(|c| rank(&c.id));
        let mut units = Vec::new();
        for c in parts {
            for mut u in c.units {
                if u.level.is_none() {
                    u.level = c.level.clone();
                }
                units.push(u);
            }
        }
        Curriculum {
            id: "kaiwa".into(),
            level: None,
            units,
        }
    }

    /// The level files as JSON, parsed and joined ([`Curriculum::merge`]).
    pub fn from_files(files: &[&str]) -> Result<Self, serde_json::Error> {
        Ok(Self::merge(
            files
                .iter()
                .map(|j| Self::from_json(j))
                .collect::<Result<_, _>>()?,
        ))
    }

    /// Every `*.json` in `dir` (`data/curriculum`), joined.
    pub fn from_dir(dir: &std::path::Path) -> std::io::Result<Self> {
        let mut files = Vec::new();
        for e in std::fs::read_dir(dir)? {
            let p = e?.path();
            if p.extension().is_some_and(|x| x == "json") {
                files.push(std::fs::read_to_string(&p)?);
            }
        }
        let refs: Vec<&str> = files.iter().map(String::as_str).collect();
        Self::from_files(&refs).map_err(std::io::Error::other)
    }

    /// What the native speaker review (INHALT-ENTWURF 3.3, `review.hash`) is about: the unit
    /// without its review and the files of its scenarios, as FNV-1a 64 in hex. Any change to the
    /// unit or a scenario changes it, so a review no longer matches (release gate, M7). A change
    /// detector, not a signature.
    pub fn content_hash(unit: &Unit, scenarios: &[&str]) -> String {
        let mut u = unit.clone();
        u.review = None;
        u.level = None;
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |bytes: &[u8]| {
            for b in bytes {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        eat(serde_json::to_string(&u).unwrap_or_default().as_bytes());
        for s in scenarios {
            eat(b"\n");
            eat(s.as_bytes());
        }
        format!("{h:016x}")
    }

    /// The grammar points of the units against the catalogue: every point is in it, and its
    /// level is the level of the first unit that uses it.
    pub fn check_grammar(
        &self,
        catalogue: &crate::grammar::Catalogue,
    ) -> Result<(), CurriculumError> {
        let mut first: BTreeSet<&str> = BTreeSet::new();
        for u in &self.units {
            for g in &u.grammar {
                let Some(point) = catalogue.by_label(g) else {
                    return Err(CurriculumError::UnknownGrammar(u.id.clone(), g.clone()));
                };
                if first.insert(g.as_str()) {
                    let level = u.id.split('-').next().unwrap_or_default();
                    if point.level != level {
                        return Err(CurriculumError::GrammarLevel(
                            point.id.clone(),
                            u.id.clone(),
                            point.level.clone(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn unit(&self, id: &str) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }

    /// The unit a scenario belongs to.
    pub fn unit_of(&self, scenario: &str) -> Option<&Unit> {
        self.units
            .iter()
            .find(|u| u.scenarios.iter().any(|s| s == scenario))
    }

    /// Unknown references, duplicates and cycles.
    pub fn check(&self) -> Result<(), CurriculumError> {
        let mut seen = BTreeSet::new();
        for u in &self.units {
            if !seen.insert(u.id.as_str()) {
                return Err(CurriculumError::Duplicate(u.id.clone()));
            }
            if let Some(t) = u.topics.iter().find(|t| !(1..=15).contains(*t)) {
                return Err(CurriculumError::Topic(u.id.clone(), *t));
            }
        }
        for u in &self.units {
            for r in &u.requires {
                if !seen.contains(r.as_str()) {
                    return Err(CurriculumError::UnknownUnit(u.id.clone(), r.clone()));
                }
            }
        }
        // Kahn: what cannot be ordered is in a cycle.
        let mut left: Vec<&Unit> = self.units.iter().collect();
        let mut done: BTreeSet<&str> = BTreeSet::new();
        loop {
            let before = left.len();
            left.retain(|u| {
                if u.requires.iter().all(|r| done.contains(r.as_str())) {
                    done.insert(u.id.as_str());
                    false
                } else {
                    true
                }
            });
            if left.is_empty() {
                return Ok(());
            }
            if left.len() == before {
                return Err(CurriculumError::Cycle(
                    left.iter().map(|u| u.id.clone()).collect(),
                ));
            }
        }
    }

    fn unit_done(&self, u: &Unit, p: &CourseProgress) -> bool {
        p.skipped_units.contains(&u.id) || u.scenarios.iter().all(|s| p.scenarios_done.contains(s))
    }

    fn reviews_ok(u: &str, p: &CourseProgress) -> bool {
        match p.reviews.get(u) {
            Some((n, right)) if *n >= REVIEW_MIN => f64::from(*right) / f64::from(*n) >= REVIEW_OK,
            _ => true,
        }
    }

    /// The learning path (/LF600/): every unit and scenario with its state.
    pub fn path(&self, p: &CourseProgress, lang: &str) -> Vec<UnitNode> {
        let de = lang == "de";
        let mut current_given = false;
        let mut out = Vec::new();
        for u in &self.units {
            let done = self.unit_done(u, p);
            let missing: Vec<&str> = u
                .requires
                .iter()
                .filter(|r| self.unit(r).is_some_and(|ru| !self.unit_done(ru, p)))
                .map(String::as_str)
                .collect();
            let shaky: Vec<&str> = u
                .requires
                .iter()
                .filter(|r| !Self::reviews_ok(r, p))
                .map(String::as_str)
                .collect();
            let unlocked = missing.is_empty() && shaky.is_empty();
            let state = if done {
                NodeState::Done
            } else if !unlocked {
                NodeState::Locked
            } else if !current_given {
                current_given = true;
                NodeState::Current
            } else {
                NodeState::Open
            };
            let mut next_given = false;
            let scenarios = u
                .scenarios
                .iter()
                .map(|s| {
                    let st = if p.scenarios_done.contains(s) || p.skipped_units.contains(&u.id) {
                        NodeState::Done
                    } else if state == NodeState::Locked {
                        NodeState::Locked
                    } else if state == NodeState::Current && !next_given {
                        next_given = true;
                        NodeState::Current
                    } else {
                        NodeState::Open
                    };
                    ScenarioNode {
                        id: s.clone(),
                        state: st,
                    }
                })
                .collect();
            let title = |id: &str| {
                self.unit(id)
                    .map(|u| u.title.get(lang).to_string())
                    .unwrap_or_else(|| id.to_string())
            };
            let locked_because = (state == NodeState::Locked).then(|| {
                if !missing.is_empty() {
                    let names: Vec<String> = missing.iter().map(|m| title(m)).collect();
                    if de {
                        format!("Erst: {}", names.join(", "))
                    } else {
                        format!("First: {}", names.join(", "))
                    }
                } else {
                    let names: Vec<String> = shaky.iter().map(|m| title(m)).collect();
                    if de {
                        format!("Erst die Wiederholungen aus {} festigen", names.join(", "))
                    } else {
                        format!("First firm up the reviews from {}", names.join(", "))
                    }
                }
            });
            out.push(UnitNode {
                id: u.id.clone(),
                title: u.title.get(lang).to_string(),
                can_do: u.can_do.get(lang).to_string(),
                level: u.level.as_ref().map(Level::label),
                state,
                scenarios,
                locked_because,
            });
        }
        out
    }

    /// The next scenario to play: the current one on the path.
    pub fn next_scenario(&self, p: &CourseProgress) -> Option<String> {
        self.path(p, "en")
            .into_iter()
            .flat_map(|u| u.scenarios)
            .find(|s| s.state == NodeState::Current)
            .map(|s| s.id)
    }

    /// After the placement (/LF200/, /LF215/): the units below the learner's level, to offer
    /// skipping at once, in path order. A learner who speaks and understands at N4 or above
    /// (heritage speakers, false beginners) does not have to start with N5 greetings; at N3 or
    /// above the N4 units go too. The offer stops at the first unit the learner is not ready
    /// for: one above the level, or one with keigo (`keigo`) while the register lacks it.
    pub fn placement_skip(
        &self,
        m: &crate::learner::LearnerModel,
        p: &CourseProgress,
        keigo: impl Fn(&Unit) -> bool,
    ) -> Vec<String> {
        use crate::learner::{Skill, level_index};
        let level = m.level(Skill::Speaking).min(m.level(Skill::Listening));
        let at = |name: &str| level_index(name).unwrap_or(f64::MAX);
        let ready = |u: &Unit| match u.id.split('-').next() {
            Some("n5") => level >= at("n4"),
            Some("n4") => level >= at("n3"),
            _ => false,
        };
        let keigo_ok = m.level(Skill::Register) >= at("keigo_basic");
        self.units
            .iter()
            .take_while(|u| ready(u) && (keigo_ok || !keigo(u)))
            .filter(|u| !self.unit_done(u, p))
            .map(|u| u.id.clone())
            .collect()
    }

    /// Units not done yet, in path order: the skip offer (/LF215/) lets the learner choose up to
    /// which one, after a short check conversation.
    pub fn skippable(&self, p: &CourseProgress) -> Vec<String> {
        self.units
            .iter()
            .filter(|u| !self.unit_done(u, p))
            .map(|u| u.id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Text {
        Text {
            de: s.into(),
            en: s.into(),
        }
    }

    fn unit(id: &str, scenarios: &[&str], requires: &[&str]) -> Unit {
        Unit {
            id: id.into(),
            title: t(id),
            can_do: t("…"),
            grammar: vec![],
            words: vec![],
            scenarios: scenarios.iter().map(|s| s.to_string()).collect(),
            requires: requires.iter().map(|s| s.to_string()).collect(),
            topics: vec![],
            jf_can_do: vec![],
            review: None,
            level: None,
        }
    }

    fn course() -> Curriculum {
        Curriculum {
            id: "n5".into(),
            level: None,
            units: vec![
                unit("u1", &["a", "b"], &[]),
                unit("u2", &["c"], &["u1"]),
                unit("u3", &["d"], &["u1"]),
                unit("u4", &["e"], &["u2", "u3"]),
            ],
        }
    }

    #[test]
    fn units_unlock_along_the_graph() {
        let c = course();
        c.check().unwrap();
        let mut p = CourseProgress::default();
        let path = c.path(&p, "de");
        assert_eq!(path[0].state, NodeState::Current);
        assert_eq!(path[0].scenarios[0].state, NodeState::Current);
        assert_eq!(path[1].state, NodeState::Locked);
        assert!(path[1].locked_because.as_ref().unwrap().contains("u1"));
        p.scenarios_done.extend(["a".to_string(), "b".to_string()]);
        let path = c.path(&p, "de");
        assert_eq!(path[0].state, NodeState::Done);
        assert_eq!(path[1].state, NodeState::Current);
        assert_eq!(path[2].state, NodeState::Open);
        assert_eq!(path[3].state, NodeState::Locked);
        assert_eq!(c.next_scenario(&p).as_deref(), Some("c"));
    }

    #[test]
    fn shaky_reviews_hold_back_the_next_unit() {
        let c = course();
        let mut p = CourseProgress::default();
        p.scenarios_done.extend(["a".to_string(), "b".to_string()]);
        p.reviews.insert("u1".into(), (5, 2));
        let path = c.path(&p, "en");
        assert_eq!(path[1].state, NodeState::Locked);
        assert!(path[1].locked_because.as_ref().unwrap().contains("reviews"));
        p.reviews.insert("u1".into(), (8, 6));
        assert_eq!(c.path(&p, "en")[1].state, NodeState::Current);
    }

    #[test]
    fn skipped_units_count_as_done() {
        let c = course();
        let mut p = CourseProgress::default();
        p.skipped_units
            .extend(["u1".to_string(), "u2".to_string(), "u3".to_string()]);
        assert_eq!(c.next_scenario(&p).as_deref(), Some("e"));
    }

    #[test]
    fn cycles_and_unknown_units_are_found() {
        let mut c = course();
        c.units[0].requires.push("u4".into());
        assert!(matches!(c.check(), Err(CurriculumError::Cycle(_))));
        let mut c = course();
        c.units[1].requires.push("nope".into());
        assert!(matches!(c.check(), Err(CurriculumError::UnknownUnit(_, _))));
    }
}

#[cfg(test)]
mod bundled {
    use super::*;

    #[test]
    fn the_curriculum_is_a_sound_graph_of_existing_scenarios() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let c = Curriculum::from_dir(&root.join("data/curriculum")).unwrap();
        c.check().unwrap();
        assert_eq!(c.units[0].id, "n5-01", "the levels are joined from N5 up");
        assert_eq!(
            c.units[0].level.as_ref().map(Level::label).as_deref(),
            Some("N5 · A1")
        );
        let grammar = crate::grammar::Catalogue::from_dir(&root.join("data/grammar")).unwrap();
        grammar.check().unwrap();
        c.check_grammar(&grammar).unwrap();
        for p in &grammar.points {
            assert!(
                c.units.iter().any(|u| u.grammar.contains(&p.label)),
                "grammar point {} belongs to no unit",
                p.id
            );
        }
        for u in &c.units {
            for s in &u.scenarios {
                let path = root.join(format!("data/scenarios/{s}.json"));
                let sc =
                    crate::Scenario::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
                assert_eq!(sc.unit, u.id, "{s} names its unit");
            }
        }
        let path = c.path(&CourseProgress::default(), "de");
        assert_eq!(path[0].state, NodeState::Current);
        assert_eq!(
            c.next_scenario(&CourseProgress::default()).as_deref(),
            Some("n5-01-aisatsu")
        );
    }

    #[test]
    fn levels_join_in_course_order_and_require_across_files() {
        let n4 = r#"{"id":"n4","level":{"jlpt":"N4","cefr":"A2"},"units":[
            {"id":"n4-01","title":{"de":"a","en":"a"},"can_do":{"de":"a","en":"a"},
             "scenarios":["x"],"requires":["n5-01"],"topics":[3]}]}"#;
        let n5 = r#"{"id":"n5","units":[
            {"id":"n5-01","title":{"de":"b","en":"b"},"can_do":{"de":"b","en":"b"},
             "scenarios":["y"]}]}"#;
        let c = Curriculum::from_files(&[n4, n5]).unwrap();
        c.check().unwrap();
        assert_eq!(c.units[0].id, "n5-01");
        assert_eq!(c.units[1].level.as_ref().unwrap().label(), "N4 · A2");
        let alone = Curriculum::from_files(&[n4]).unwrap();
        assert!(matches!(
            alone.check(),
            Err(CurriculumError::UnknownUnit(_, _))
        ));
        let mut bad = c.clone();
        bad.units[1].topics = vec![16];
        assert!(matches!(bad.check(), Err(CurriculumError::Topic(_, 16))));
    }

    #[test]
    fn grammar_is_introduced_at_its_level() {
        use crate::grammar::{Catalogue, Point};
        let mut c = Curriculum::from_files(&[r#"{"id":"n5","units":[
            {"id":"n5-01","title":{"de":"b","en":"b"},"can_do":{"de":"b","en":"b"},
             "grammar":["〜ね"],"scenarios":["y"]}]}"#])
        .unwrap();
        let point = |level: &str| Point {
            id: "ne".into(),
            label: "〜ね".into(),
            level: level.into(),
            explain: Text {
                de: "x".into(),
                en: "x".into(),
            },
            examples: vec![],
        };
        c.check_grammar(&Catalogue {
            points: vec![point("n5")],
        })
        .unwrap();
        assert!(matches!(
            c.check_grammar(&Catalogue {
                points: vec![point("n4")]
            }),
            Err(CurriculumError::GrammarLevel(_, _, _))
        ));
        c.units[0].grammar.push("〜よ".into());
        assert!(matches!(
            c.check_grammar(&Catalogue {
                points: vec![point("n5")]
            }),
            Err(CurriculumError::UnknownGrammar(_, _))
        ));
    }

    /// A recorded review still matches its unit and scenarios (INHALT-ENTWURF 3.3).
    #[test]
    fn recorded_reviews_match_the_content() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let c = Curriculum::from_dir(&root.join("data/curriculum")).unwrap();
        for u in &c.units {
            let texts: Vec<String> = u
                .scenarios
                .iter()
                .map(|s| {
                    std::fs::read_to_string(root.join(format!("data/scenarios/{s}.json"))).unwrap()
                })
                .collect();
            let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            let now = Curriculum::content_hash(u, &refs);
            if let Some(h) = u.review.as_ref().and_then(|r| r.hash.as_deref()) {
                assert_eq!(h, now, "{} changed after its review", u.id);
            }
            let mut changed = texts.clone();
            changed[0].push(' ');
            let refs: Vec<&str> = changed.iter().map(String::as_str).collect();
            assert_ne!(Curriculum::content_hash(u, &refs), now);
        }
    }

    /// Q-07: N5 and N4 together reach every everyday topic of the JF Standard (1 self and
    /// family … 13 language and culture); 14 media and 15 science belong to the B levels.
    #[test]
    fn the_a_levels_cover_the_everyday_topics() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let c = Curriculum::from_dir(&root.join("data/curriculum")).unwrap();
        let covered: BTreeSet<u8> = c
            .units
            .iter()
            .filter(|u| u.id.starts_with("n5") || u.id.starts_with("n4"))
            .flat_map(|u| u.topics.iter().copied())
            .collect();
        let missing: Vec<u8> = (1..=13).filter(|t| !covered.contains(t)).collect();
        assert!(missing.is_empty(), "JF topics without a unit: {missing:?}");
        assert!(
            c.units.iter().all(|u| !u.topics.is_empty()),
            "every unit names its topics"
        );
    }

    #[test]
    fn a_native_speaker_skips_up_to_keigo() {
        use crate::learner::{LearnerModel, Skill, level_index};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let c = Curriculum::from_dir(&root.join("data/curriculum")).unwrap();
        let mut m = LearnerModel::default();
        for s in [Skill::Speaking, Skill::Listening] {
            m.set(s, level_index("native_casual").unwrap());
        }
        m.set(
            Skill::Register,
            level_index("desu_masu_and_casual").unwrap(),
        );
        let keigo = |u: &Unit| u.id == "n4-11";
        let offer = c.placement_skip(&m, &CourseProgress::default(), keigo);
        assert!(offer.contains(&"n5-01".to_string()));
        assert!(offer.contains(&"n4-14".to_string()));
        assert!(
            !offer.contains(&"n4-11".to_string()),
            "keigo is where she starts"
        );
        assert!(
            !offer.contains(&"n4-12".to_string()),
            "nothing after the keigo unit"
        );
        // With keigo she may skip it too.
        m.set(Skill::Register, level_index("keigo_basic").unwrap());
        assert!(
            c.placement_skip(&m, &CourseProgress::default(), keigo)
                .contains(&"n4-12".to_string())
        );
        // At N4 only the N5 units.
        for s in [Skill::Speaking, Skill::Listening] {
            m.set(s, level_index("n4").unwrap());
        }
        let offer = c.placement_skip(&m, &CourseProgress::default(), keigo);
        assert!(offer.iter().all(|u| u.starts_with("n5")));
        assert_eq!(offer.len(), 16);
    }

    #[test]
    fn a_scenario_is_done_after_two_good_completions() {
        let mut p = CourseProgress::default();
        assert!(!p.complete("a", 0.9));
        assert!(!p.complete("a", 0.5), "a weak run does not count");
        assert!(p.complete("a", 0.8));
        assert!(p.scenarios_done.contains("a"));
        assert!(!p.complete("a", 0.9), "done stays done");
    }
}
