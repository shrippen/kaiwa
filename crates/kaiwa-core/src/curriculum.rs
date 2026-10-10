//! The curriculum as a graph of units (Lastenheft /LF220/, /LD040/) and the learning path the
//! learner sees (/LF600/).
//!
//! A unit has can-do statements, grammar points, core words and scenarios, and requires other
//! units. It unlocks when those are done **and** their reviews mostly go well (at least 60 % of
//! the answers right once there are three or more), so new material does not pile up on
//! shaky old material (Hinnerk). Skipped units (/LF215/) count as done. Every state has a word
//! and a symbol in the GUI, never only a colour.

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Curriculum {
    pub id: String,
    pub units: Vec<Unit>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CurriculumError {
    #[error("unit {0} requires unknown unit {1}")]
    UnknownUnit(String, String),
    #[error("units {0:?} require each other in a circle")]
    Cycle(Vec<String>),
    #[error("unit id {0} appears twice")]
    Duplicate(String),
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
    /// skipping at once. A learner who speaks and understands at N4 or above (heritage speakers,
    /// false beginners) does not have to start with N5 greetings.
    pub fn placement_skip(
        &self,
        m: &crate::learner::LearnerModel,
        p: &CourseProgress,
    ) -> Vec<String> {
        use crate::learner::Skill;
        let n4 = crate::learner::level_index("n4").unwrap_or(5.0);
        if m.level(Skill::Speaking) < n4 || m.level(Skill::Listening) < n4 {
            return Vec::new();
        }
        self.units
            .iter()
            .filter(|u| u.id.starts_with("n5") && !self.unit_done(u, p))
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
        }
    }

    fn course() -> Curriculum {
        Curriculum {
            id: "n5".into(),
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
    fn the_n5_curriculum_is_a_sound_graph_of_existing_scenarios() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let c = Curriculum::from_json(
            &std::fs::read_to_string(root.join("data/curriculum/n5.json")).unwrap(),
        )
        .unwrap();
        c.check().unwrap();
        assert_eq!(c.units.len(), 12);
        for u in &c.units {
            for s in &u.scenarios {
                let path = root.join(format!("data/scenarios/{s}.json"));
                let sc =
                    crate::Scenario::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
                assert_eq!(sc.unit.to_lowercase(), u.id, "{s} names its unit");
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
    fn a_scenario_is_done_after_two_good_completions() {
        let mut p = CourseProgress::default();
        assert!(!p.complete("a", 0.9));
        assert!(!p.complete("a", 0.5), "a weak run does not count");
        assert!(p.complete("a", 0.8));
        assert!(p.scenarios_done.contains("a"));
        assert!(!p.complete("a", 0.9), "done stays done");
    }
}
