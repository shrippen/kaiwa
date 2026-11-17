//! What a finished talk or a placement changes in the learning state (/LD020/): the one rule
//! the store (when it books a talk, a later correction or a placement) and the long-run
//! simulation apply.

use serde::{Deserialize, Serialize};

use crate::curriculum::CourseProgress;
use crate::difficulty::{Change, Difficulty, Regulator};
use crate::feedback::Amount;
use crate::learner::{Evidence, LearnerModel, Skill};

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

/// Where the amount of feedback comes from (/LF304/, V-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSource {
    /// Neither a placement nor the learner set it.
    #[default]
    Default,
    /// The speaking level of a placement set it.
    Level,
    /// The learner picked it in the settings; a placement leaves it.
    Chosen,
}

/// What a placement sets.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub model: LearnerModel,
    /// The difficulty starts again from the new model.
    pub regulator: Regulator,
    /// The amount of feedback for the new level; `None` when the learner chose one.
    pub feedback: Option<Amount>,
}

/// A placement measured `placed` (/LF200/); `old` is the model before. The history stays:
/// mistakes, the learner's own corrections (/LF202/) and pronunciation, which a talk does not
/// show (typed turns count too). An amount of feedback the learner did not choose follows the
/// speaking level. A placement is no progress, so the store also restarts the week's
/// comparison ([`crate::progress::DayRecord::restart`]).
pub fn book_placement(
    old: &mut LearnerModel,
    placed: LearnerModel,
    source: FeedbackSource,
) -> Placed {
    let mut model = placed;
    model.errors = std::mem::take(&mut old.errors);
    model.corrections_by_learner = old.corrections_by_learner;
    model.set(Skill::Pronunciation, old.level(Skill::Pronunciation));
    let feedback =
        (source != FeedbackSource::Chosen).then(|| Amount::for_level(model.level(Skill::Speaking)));
    Placed {
        regulator: Regulator::new(Difficulty::for_learner(&model)),
        model,
        feedback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn a_placement_keeps_the_history_and_a_chosen_amount() {
        let mut old = LearnerModel::default();
        old.set(Skill::Pronunciation, 2.0);
        old.corrections_by_learner = 3;
        let mut placed = LearnerModel::default();
        placed.set(Skill::Speaking, 5.0);

        let p = book_placement(&mut old.clone(), placed.clone(), FeedbackSource::Default);
        assert_eq!(p.model.level(Skill::Pronunciation), 2.0);
        assert_eq!(p.model.corrections_by_learner, 3);
        assert_eq!(p.feedback, Some(Amount::Normal));
        assert_eq!(
            p.regulator,
            Regulator::new(Difficulty::for_learner(&p.model))
        );

        let p = book_placement(&mut old, placed, FeedbackSource::Chosen);
        assert_eq!(p.feedback, None);
    }
}
