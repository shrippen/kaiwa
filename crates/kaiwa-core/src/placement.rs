//! The placement by conversation (Lastenheft /LF200/): short stages at rising levels instead of a
//! test, about 5–8 minutes in all, skippable and repeatable. Each stage plays a scenario at a
//! fixed difficulty for [`TURNS_PER_STAGE`] turns (or until its goals are reached); the next
//! stage follows while the learner keeps up (success ≥ [`GO_ON`]). Turns the tutor analysed
//! with gaps (T-11) are not judged: a stage waits for turns that can be, for at most twice
//! its turns, and a placement with a stage that could not be judged changes nothing
//! ([`Placement::after_turn`]). The result is a level per skill, no grade:
//!
//! - listening from the share of lines understood without help, speaking from the share of
//!   turns without a mistake, fluency from goals reached without help, each at the levels of
//!   the stages played;
//! - register from the register mistakes at the polite stages;
//! - reading kana and kanji are not part of a conversation: the learner says it (two choices).

use std::sync::Arc;

use kaiwa_ai::ChatProvider;
use serde::{Deserialize, Serialize};

use crate::difficulty::{Difficulty, Knob};
use crate::learner::{LearnerModel, Skill, level_index};
use crate::session::{Coaching, Session, Stats};
use crate::{Amount, Scenario};

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

/// The session of stage `i`: its scenario at the stage's level, without stepping in by
/// itself (help only on request, the level stays), because the stage is judged at that level.
pub fn stage_session(
    scenario: Scenario,
    provider: Arc<dyn ChatProvider>,
    amount: Amount,
    lang: &str,
    i: usize,
) -> Session {
    Session::new(scenario, provider, amount, lang)
        .with_difficulty(difficulty_at(STAGES[i].1))
        .with_coaching(Coaching::Off)
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

/// What follows a turn of a placement stage ([`Placement::after_turn`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The stage goes on.
    Go,
    /// The stage is judged and the learner kept up: play this stage next.
    Next(usize),
    /// The stage is judged and the placement is over: ask about reading, then the result.
    Over,
    /// The stage could not be judged (too many turns the tutor analysed with gaps): the
    /// placement ends without a result and must change nothing.
    Unjudged,
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

    /// Whether the next stage follows. A stage judged before its turns ran out had all its
    /// goals reached: that is keeping up, and one or two turns are too few to stop the
    /// placement on (a single slip in a single turn would be speaking 0).
    fn kept_up(&self) -> bool {
        self.turns < TURNS_PER_STAGE || self.success() >= GO_ON
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
            Some(r) if r.kept_up() && self.results.len() < STAGES.len() => Some(self.results.len()),
            _ => None,
        }
    }

    pub fn add(&mut self, r: StageResult) {
        self.results.push(r);
    }

    /// After a turn of the stage `s` plays. The stage is judged after [`TURNS_PER_STAGE`]
    /// turns that count ([`Stats::turns`]: without those analysed with gaps, T-11), or when
    /// its goals are reached, all of them in turns that count. Its result is added then. A
    /// stage that is not judged after twice its turns never will be with this tutor.
    pub fn after_turn(&mut self, s: &Session) -> Step {
        let st = s.stats();
        let judged = st.turns >= TURNS_PER_STAGE || (s.done() && s.goals_unjudged().is_empty());
        let level = STAGES[self.results.len().min(STAGES.len() - 1)].1;
        match StageResult::from_stats(level, &st) {
            Some(r) if judged => {
                self.add(r);
                self.next_stage().map_or(Step::Over, Step::Next)
            }
            _ if s.turns().len() as u32 >= 2 * TURNS_PER_STAGE => Step::Unjudged,
            _ => Step::Go,
        }
    }

    /// Minutes the placement takes at most (about 30 s per turn), with a tutor whose
    /// analyses are whole.
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

    /// Stage 1 against a tutor that answers every turn with `analysis` (JSON), in order.
    fn played(analyses: &[&str]) -> (Placement, Vec<Step>) {
        let (p, _, steps, _) = play(analyses);
        (p, steps)
    }

    fn played_session(analyses: &[&str]) -> (Session, Vec<Step>) {
        let (_, s, steps, _) = play(analyses);
        (s, steps)
    }

    fn outcomes(analyses: &[&str]) -> (Placement, Vec<crate::session::TurnOutcome>) {
        let (p, _, _, o) = play(analyses);
        (p, o)
    }

    fn play(
        analyses: &[&str],
    ) -> (
        Placement,
        Session,
        Vec<Step>,
        Vec<crate::session::TurnOutcome>,
    ) {
        use crate::Input;
        use kaiwa_ai::mock::MockProvider;
        let fixtures: Vec<String> = analyses
            .iter()
            .enumerate()
            .map(|(i, a)| format!(r#"{{"learner":"turn {i}","reply":"はい。","analysis":{a}}}"#))
            .collect();
        let tutor = MockProvider::from_json(&format!("[{}]", fixtures.join(","))).unwrap();
        let scenario = Scenario::from_json(include_str!(
            "../../../data/scenarios/n5-02-jikoshoukai.json"
        ))
        .unwrap();
        let mut s = stage_session(scenario, Arc::new(tutor), Amount::Little, "de", 0);
        let mut p = Placement::default();
        let mut steps = Vec::new();
        let mut outs = Vec::new();
        for i in 0..analyses.len() {
            outs.push(s.turn(&format!("turn {i}"), Input::Typed).unwrap());
            steps.push(p.after_turn(&s));
            if steps.last() != Some(&Step::Go) {
                break;
            }
        }
        (p, s, steps, outs)
    }

    const FINE: &str = r#"{"understood":true,"corrections":[]}"#;
    const LOST: &str = r#"{"understood":false,"corrections":[]}"#;
    /// Understood, no mistake named, but the analysis had a gap (T-11).
    const GAP: &str = r#"{"understood":true,"corrections":[],"incomplete":true}"#;
    const GAP_GOALS: &str =
        r#"{"understood":true,"goals_met":["name","origin"],"corrections":[],"incomplete":true}"#;

    #[test]
    fn a_stage_is_judged_from_turns_that_count() {
        // Four whole turns: judged, on to stage 2.
        let (p, steps) = played(&[FINE; 4]);
        assert_eq!(steps, [Step::Go, Step::Go, Step::Go, Step::Next(1)]);
        assert_eq!((p.results[0].turns, p.results[0].listening), (4, 1.0));
        // Gaps in between do not end the stage early and are not failures.
        let (p, steps) = played(&[GAP, FINE, GAP, FINE, FINE, FINE]);
        assert_eq!(steps.last(), Some(&Step::Next(1)));
        assert_eq!(steps.len(), 6);
        assert_eq!((p.results[0].turns, p.results[0].listening), (4, 1.0));
        // Not understood four times: judged, the placement is over.
        let (p, steps) = played(&[LOST; 4]);
        assert_eq!(steps.last(), Some(&Step::Over));
        assert_eq!(p.results.len(), 1);
    }

    /// Both goals of stage 1 in one turn, with a particle mistake.
    const GOALS_SLIP: &str = r#"{"understood":true,"goals_met":["name","origin"],"corrections":[{"original":"ドイツが","better":"ドイツから","severity":"wrong","kind":"particle","explanation":"x"}]}"#;

    #[test]
    fn goals_reached_early_do_not_end_the_placement() {
        // One fluent turn with a slip reaches both goals of stage 1: too few turns to stop
        // the placement, so stage 2 follows.
        let (p, steps) = played(&[GOALS_SLIP]);
        assert_eq!(steps, [Step::Next(1)]);
        assert_eq!(p.results[0].turns, 1);
    }

    #[test]
    fn help_comes_only_on_request() {
        // Three understood turns without a goal: no help by itself, the stage measures what
        // the learner can do alone.
        let (s, steps) = played_session(&[FINE, FINE, FINE]);
        assert_eq!(steps, [Step::Go; 3]);
        assert!(s.turns().iter().all(|t| !t.aided));
        assert_eq!(s.stats().goals_unaided, s.stats().goals_met);
        let (_, outcomes) = outcomes(&[FINE, FINE, FINE]);
        assert!(outcomes.iter().all(|o| o.help.is_none()), "{outcomes:?}");
    }

    #[test]
    fn the_stage_level_stays_the_same() {
        // Two turns not understood ease a normal talk; a stage keeps its level, because it
        // is judged at that level.
        let (_, outcomes) = outcomes(&[LOST, LOST]);
        assert!(outcomes.iter().all(|o| o.eased.is_none()));
    }

    #[test]
    fn a_stage_that_cannot_be_judged_gives_no_result() {
        // A tutor whose every analysis has a gap: eight turns, then no result at all.
        let (p, steps) = played(&[GAP; 8]);
        assert_eq!(steps.len(), 8);
        assert_eq!(steps.last(), Some(&Step::Unjudged));
        assert!(p.results.is_empty());
        // Goals reached in turns with gaps do not end the stage.
        let (p, steps) = played(&[GAP_GOALS, GAP, GAP]);
        assert_eq!(steps, [Step::Go; 3]);
        assert!(p.results.is_empty());
        // One whole turn among seven with gaps is not a stage judged from a single turn.
        let (p, steps) = played(&[LOST, GAP_GOALS, GAP, GAP, GAP, GAP, GAP, GAP]);
        assert_eq!(steps.last(), Some(&Step::Unjudged));
        assert!(p.results.is_empty());
        // No judged turn is no result.
        assert_eq!(StageResult::from_stats(1.5, &Stats::default()), None);
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
