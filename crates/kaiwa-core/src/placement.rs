//! The placement by conversation (Lastenheft /LF200/): short stages at rising levels instead of a
//! test, about 5–8 minutes in all, skippable and repeatable. Each stage plays a scenario at a
//! fixed difficulty for at most [`TURNS_PER_STAGE`] turns; the next stage follows while the
//! learner keeps up (success ≥ [`GO_ON`]). The result is a level per skill, no grade:
//!
//! - listening from the share of lines understood without help, speaking from the share of
//!   turns without a mistake, fluency from goals reached without help, each at the levels of
//!   the stages played;
//! - register from the register mistakes at the polite stages;
//! - reading kana and kanji are not part of a conversation: the learner says it (two choices).

use serde::{Deserialize, Serialize};

use crate::difficulty::{Difficulty, Knob};
use crate::learner::{LearnerModel, Skill, level_index};
use crate::session::Stats;

/// The stages: scenario and level of the material (learner model scale, 0–8).
pub const STAGES: [(&str, f64); 4] = [
    ("n5-02-jikoshoukai", 1.5),
    ("n5-10-yotei", 3.0),
    ("n5-16-kinou", 4.5),
    ("n5-16-kinou", 6.5),
];
pub const TURNS_PER_STAGE: u32 = 4;
/// Keep going while a stage goes at least this well.
pub const GO_ON: f64 = 0.6;

/// A difficulty whose material is at `level`, for a placement stage.
pub fn difficulty_at(level: f64) -> Difficulty {
    // Walk the knobs up one step at a time until the content reaches the level.
    let mut d = Difficulty::default();
    let order = [
        Knob::Length,
        Knob::Vocabulary,
        Knob::Clauses,
        Knob::Speed,
        Knob::Goals,
    ];
    let mut i = 0;
    while d.content_level() < level && i < 64 {
        let k = order[i % order.len()];
        let s = d.get(k);
        if s + 1 < k.steps() {
            d.set(k, s + 1);
        }
        i += 1;
    }
    // Help only on request: the placement measures what the learner can do alone.
    d.set(Knob::Help, Knob::Help.steps() - 1);
    d
}

/// What one stage showed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageResult {
    pub level: f64,
    pub turns: u32,
    pub listening: f64,
    pub speaking: f64,
    pub fluency: f64,
    pub register_mistakes: u32,
}

/// Whether a stage is over: after [`TURNS_PER_STAGE`] turns that can be judged (`judged`:
/// [`Stats::turns`], without those the tutor analysed with gaps, T-11), when its goals are
/// reached, or after twice as many turns in all (a tutor whose analyses all have gaps).
pub fn stage_over(turns: u32, judged: u32, done: bool) -> bool {
    done || judged >= TURNS_PER_STAGE || turns >= 2 * TURNS_PER_STAGE
}

impl StageResult {
    /// What a stage showed; `None` when no turn of it could be judged, which is no result
    /// (not a failed stage).
    pub fn from_stats(level: f64, st: &Stats) -> Option<Self> {
        if st.turns == 0 {
            return None;
        }
        let n = f64::from(st.turns);
        Some(Self {
            level,
            turns: st.turns,
            listening: f64::from(st.understood_unaided) / n,
            speaking: f64::from(st.clean_turns) / n,
            fluency: if st.goals_met > 0 {
                f64::from(st.goals_unaided) / f64::from(st.goals_met)
            } else {
                0.0
            },
            register_mistakes: st.register_mistakes,
        })
    }

    pub fn success(&self) -> f64 {
        (self.listening + self.speaking) / 2.0
    }
}

/// What the learner says about reading (not part of a conversation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Reading {
    #[default]
    None,
    /// Some characters, slowly.
    Some,
    /// Fluently.
    Fluent,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub results: Vec<StageResult>,
    pub kana: Reading,
    pub kanji: Reading,
}

impl Placement {
    /// The stage to play next, or `None` when the placement is over.
    pub fn next_stage(&self) -> Option<usize> {
        match self.results.last() {
            None => Some(0),
            Some(r) if r.success() >= GO_ON && self.results.len() < STAGES.len() => {
                Some(self.results.len())
            }
            _ => None,
        }
    }

    pub fn add(&mut self, r: StageResult) {
        self.results.push(r);
    }

    /// Minutes the placement takes at most (about 30 s per turn).
    pub fn max_minutes() -> u32 {
        (STAGES.len() as u32 * TURNS_PER_STAGE).div_ceil(2)
    }

    /// The level a skill showed: the highest stage passed, plus how far the next one went.
    fn level_of(&self, share: impl Fn(&StageResult) -> f64) -> f64 {
        let mut level: f64 = 1.0;
        for r in &self.results {
            let s = share(r);
            // 80 % at a level is that level (as in the learner model); less is below it.
            let shown = r.level + (s - 0.8) * 2.5;
            level = level.max(shown.min(r.level + 0.5));
        }
        level.clamp(0.0, 8.0)
    }

    /// The learner model the placement gives.
    pub fn result(&self) -> LearnerModel {
        let mut m = LearnerModel::default();
        m.set(Skill::Listening, self.level_of(|r| r.listening));
        m.set(Skill::Speaking, self.level_of(|r| r.speaking));
        m.set(Skill::Fluency, self.level_of(|r| r.fluency));
        // Register shows only where the learner understood something: a stage in which
        // nothing was understood has nothing to correct, which is no sign of a sure register.
        // (Not "kept up": a register slip makes its turn unclean, so the stages with the most
        // slips would be the ones left out.)
        let polite: Vec<&StageResult> = self
            .results
            .iter()
            .filter(|r| r.turns > 0 && r.listening > 0.0)
            .collect();
        let slips: u32 = polite.iter().map(|r| r.register_mistakes).sum();
        let turns: u32 = polite.iter().map(|r| r.turns).sum();
        let register = match (turns, f64::from(slips) / f64::from(turns.max(1))) {
            (0, _) => 0.0,
            (_, s) if s >= 0.5 => 1.0,
            (_, s) if s >= 0.15 => 2.0,
            _ => 3.0,
        };
        m.set(Skill::Register, register);
        let reading = |r: Reading, fluent: &str| match r {
            Reading::None => 0.0,
            Reading::Some => level_index("n5_low").unwrap_or(2.0),
            Reading::Fluent => level_index(fluent).unwrap_or(4.0),
        };
        m.set(Skill::ReadingKana, reading(self.kana, "n3"));
        m.set(Skill::ReadingKanji, reading(self.kanji, "n5"));
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(level: f64, listening: f64, speaking: f64) -> StageResult {
        StageResult {
            level,
            turns: 4,
            listening,
            speaking,
            fluency: speaking,
            register_mistakes: 0,
        }
    }

    #[test]
    fn a_stage_waits_for_turns_that_can_be_judged() {
        // T-11: turns the tutor analysed with gaps do not end a stage as failures.
        assert!(!stage_over(4, 0, false) && !stage_over(7, 3, false));
        assert!(stage_over(4, 4, false) && stage_over(6, 4, false));
        // Its goals reached, or a tutor that never gives a whole analysis: over.
        assert!(stage_over(2, 2, true) && stage_over(8, 0, false));
        // No judged turn is no result, not a stage failed.
        assert_eq!(StageResult::from_stats(1.5, &Stats::default()), None);
        let st = Stats {
            turns: 1,
            understood_unaided: 1,
            clean_turns: 1,
            ..Default::default()
        };
        let r = StageResult::from_stats(1.5, &st).unwrap();
        assert_eq!((r.turns, r.listening, r.speaking), (1, 1.0, 1.0));
    }

    #[test]
    fn nothing_understood_is_no_sure_register() {
        // The first stage failed: no slips, because there was nothing to correct.
        let mut p = Placement::default();
        p.results.push(stage(1.5, 0.0, 0.0));
        assert_eq!(p.result().level(Skill::Register), 0.0);
        // A stage passed without slips shows the polite form.
        let mut p = Placement::default();
        p.results.push(stage(1.5, 1.0, 1.0));
        assert_eq!(p.result().level(Skill::Register), 3.0);
        // Everything understood, every turn in the plain form: the slips make the turns
        // unclean (speaking 0), and they are what shows the register.
        let mut p = Placement::default();
        p.results.push(StageResult {
            register_mistakes: 4,
            ..stage(1.5, 1.0, 0.0)
        });
        assert_eq!(p.result().level(Skill::Register), 1.0);
        // Polite throughout with a particle mistake in every turn: still polite.
        let mut p = Placement::default();
        p.results.push(stage(1.5, 1.0, 0.0));
        assert_eq!(p.result().level(Skill::Register), 3.0);
    }

    #[test]
    fn stages_go_on_while_the_learner_keeps_up() {
        let mut p = Placement::default();
        assert_eq!(p.next_stage(), Some(0));
        p.add(stage(1.5, 1.0, 0.75));
        assert_eq!(p.next_stage(), Some(1));
        p.add(stage(3.0, 0.5, 0.25));
        assert_eq!(p.next_stage(), None, "a weak stage ends it");
        let m = p.result();
        assert!(m.level(Skill::Listening) >= 1.5 && m.level(Skill::Listening) < 3.0);
        assert!(m.level(Skill::Speaking) < m.level(Skill::Listening));
    }

    #[test]
    fn a_strong_speaker_reaches_the_top_in_time() {
        let mut p = Placement::default();
        for (_, level) in STAGES {
            p.add(stage(level, 1.0, 1.0));
        }
        assert_eq!(p.next_stage(), None);
        let m = p.result();
        assert!(m.level(Skill::Listening) >= 6.5);
        assert!(Placement::max_minutes() <= 8);
    }

    #[test]
    fn reading_is_what_the_learner_says() {
        let p = Placement {
            kana: Reading::Fluent,
            kanji: Reading::None,
            ..Default::default()
        };
        let m = p.result();
        assert_eq!(m.level(Skill::ReadingKanji), 0.0);
        assert!(m.level(Skill::ReadingKana) >= 6.0);
    }

    #[test]
    fn stage_difficulties_rise() {
        let levels: Vec<f64> = STAGES
            .iter()
            .map(|(_, l)| difficulty_at(*l).content_level())
            .collect();
        assert!(levels.windows(2).all(|w| w[1] > w[0]), "{levels:?}");
        for (_, l) in STAGES {
            assert!((difficulty_at(l).content_level() - l).abs() < 1.0);
        }
    }
}
