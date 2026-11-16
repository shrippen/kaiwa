//! What a finished talk changes in the learning state (/LD020/): the one rule the store
//! (when it books a talk or a later correction) and the long-run simulation both apply.

use crate::curriculum::CourseProgress;
use crate::difficulty::{Change, Regulator};
use crate::learner::{Evidence, LearnerModel};

/// What a talk did beyond the model's own update.
#[derive(Debug, Clone, PartialEq)]
pub struct Booked {
    /// The knob the regulator moved, if it did.
    pub change: Option<Change>,
    /// The talk completed its scenario in the course.
    pub scenario_done: bool,
}

/// The talk's `scenario` ended with `evidence` and `success` (0..1); `done` says its goals
/// were reached. The model learns from the evidence; the regulator moves only when there was
/// a turn; a talk with its goals reached completes the scenario in the course.
pub fn book_talk(
    model: &mut LearnerModel,
    regulator: &mut Regulator,
    course: &mut CourseProgress,
    evidence: &Evidence,
    scenario: &str,
    success: f64,
    done: bool,
) -> Booked {
    model.update(evidence);
    let change = if evidence.turns > 0 {
        regulator.after_session(success, model)
    } else {
        None
    };
    let scenario_done = done && course.complete(scenario, success);
    Booked {
        change,
        scenario_done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::difficulty::Difficulty;
    use crate::learner::Skill;

    fn evidence(turns: u32) -> Evidence {
        let mut e = Evidence {
            turns,
            ..Default::default()
        };
        if turns > 0 {
            e.at_level.insert(Skill::Speaking, (1.0, 1.0));
        }
        e
    }

    fn state() -> (LearnerModel, Regulator, CourseProgress) {
        let m = LearnerModel::default();
        let r = Regulator::new(Difficulty::for_learner(&m));
        (m, r, CourseProgress::default())
    }

    #[test]
    fn no_turn_leaves_the_regulator_alone() {
        let (mut m, mut r, mut c) = state();
        let b = book_talk(&mut m, &mut r, &mut c, &evidence(0), "n5-01", 1.0, false);
        assert_eq!(b.change, None);
        assert!(r.success.is_empty());
    }

    #[test]
    fn a_turn_counts_for_the_regulator() {
        let (mut m, mut r, mut c) = state();
        book_talk(&mut m, &mut r, &mut c, &evidence(3), "n5-01", 0.9, false);
        assert_eq!(r.success, vec![0.9]);
    }

    #[test]
    fn only_a_talk_with_its_goals_done_completes_the_scenario() {
        let (mut m, mut r, mut c) = state();
        let open = book_talk(&mut m, &mut r, &mut c, &evidence(3), "n5-01", 1.0, false);
        assert!(!open.scenario_done);
        assert!(c.completions.is_empty());

        // One good completion is luck; the second makes it done.
        let first = book_talk(&mut m, &mut r, &mut c, &evidence(3), "n5-01", 1.0, true);
        assert!(!first.scenario_done);
        let second = book_talk(&mut m, &mut r, &mut c, &evidence(3), "n5-01", 1.0, true);
        assert!(second.scenario_done);
        assert!(c.scenarios_done.contains("n5-01"));
    }
}
