//! The difficulty regulator (Lastenheft /LF210/–/LF215/).
//!
//! Difficulty has many dimensions (/LF211/), each a knob with a few steps. The regulator aims at
//! about 80 % success per conversation: above 88 % it turns one knob one step harder, below
//! 70 % one knob one step easier, in between nothing changes. Never more than one step per
//! session (/LF214/), except when the learner sets a knob or the placement does. Which knob
//! moves depends on the learner model: harder where a skill has the most headroom, easier where
//! the material is furthest above the learner. Inside a session two turns in a row that were
//! not understood make the rest of the session easier at once (/LF213/). Three sessions at 95 %
//! or more lead to the offer to skip ahead (/LF215/).

use serde::{Deserialize, Serialize};

use crate::learner::{LearnerModel, Skill};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Knob {
    /// Share of words above the level: 0–5 %.
    Vocabulary,
    /// Sentence length.
    Length,
    /// Subordinate clauses.
    Clauses,
    /// Speaking rate of the voice.
    Speed,
    /// Furigana density: all kanji, new kanji only, none.
    Furigana,
    /// Register range: polite only, polite and casual, with 敬語.
    Register,
    /// When help is offered: after 4 s, after 8 s, only when asked.
    Help,
    /// Goals per scenario.
    Goals,
}

pub const KNOBS: [Knob; 8] = [
    Knob::Length,
    Knob::Vocabulary,
    Knob::Speed,
    Knob::Clauses,
    Knob::Goals,
    Knob::Furigana,
    Knob::Help,
    Knob::Register,
];

impl Knob {
    /// Number of steps; step 0 is the easiest.
    pub fn steps(self) -> u8 {
        match self {
            Knob::Vocabulary => 6,
            Knob::Length => 5,
            Knob::Clauses => 3,
            Knob::Speed => 5,
            Knob::Furigana => 3,
            Knob::Register => 3,
            Knob::Help => 3,
            Knob::Goals => 5,
        }
    }

    pub fn word(self, lang: &str) -> &'static str {
        let de = lang == "de";
        match self {
            Knob::Vocabulary => {
                if de {
                    "neue Wörter"
                } else {
                    "new words"
                }
            }
            Knob::Length => {
                if de {
                    "Satzlänge"
                } else {
                    "sentence length"
                }
            }
            Knob::Clauses => {
                if de {
                    "Nebensätze"
                } else {
                    "clauses"
                }
            }
            Knob::Speed => {
                if de {
                    "Sprechtempo"
                } else {
                    "speaking rate"
                }
            }
            Knob::Furigana => {
                if de {
                    "Lesehilfe"
                } else {
                    "reading aid"
                }
            }
            Knob::Register => {
                if de {
                    "Höflichkeitsformen"
                } else {
                    "politeness"
                }
            }
            Knob::Help => {
                if de {
                    "Hilfe-Angebot"
                } else {
                    "help offers"
                }
            }
            Knob::Goals => {
                if de {
                    "Ziele pro Gespräch"
                } else {
                    "goals per talk"
                }
            }
        }
    }
}

pub const SPEEDS: [f64; 5] = [0.7, 0.8, 0.9, 1.0, 1.1];
pub const LENGTHS: [u32; 5] = [4, 6, 8, 11, 14];
pub const HELP_AFTER_SECS: [Option<u32>; 3] = [Some(4), Some(8), None];

/// The step of every knob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Difficulty {
    pub steps: std::collections::BTreeMap<Knob, u8>,
}

impl Default for Difficulty {
    /// The easiest setting, for a beginner without placement.
    fn default() -> Self {
        Self {
            steps: KNOBS.iter().map(|k| (*k, 0)).collect(),
        }
    }
}

impl Difficulty {
    pub fn get(&self, k: Knob) -> u8 {
        self.steps.get(&k).copied().unwrap_or(0)
    }

    pub fn set(&mut self, k: Knob, step: u8) {
        self.steps.insert(k, step.min(k.steps() - 1));
    }

    pub fn speed(&self) -> f64 {
        SPEEDS[usize::from(self.get(Knob::Speed))]
    }

    pub fn max_words(&self) -> u32 {
        LENGTHS[usize::from(self.get(Knob::Length))]
    }

    pub fn unknown_percent(&self) -> u8 {
        self.get(Knob::Vocabulary)
    }

    pub fn help_after(&self) -> Option<u32> {
        HELP_AFTER_SECS[usize::from(self.get(Knob::Help))]
    }

    pub fn goals(&self) -> usize {
        2 + usize::from(self.get(Knob::Goals))
    }

    /// The level of the material, on the learner model's scale: what the character says.
    pub fn content_level(&self) -> f64 {
        (1.0 + f64::from(self.get(Knob::Length))
            + 0.75 * f64::from(self.get(Knob::Clauses))
            + 0.3 * f64::from(self.get(Knob::Vocabulary)))
        .min(8.0)
    }

    /// The level a listener needs: the content at this speed.
    pub fn listening_level(&self) -> f64 {
        (0.8 * self.content_level() + 0.5 * f64::from(self.get(Knob::Speed))).min(8.0)
    }

    /// The step a knob should have for this learner.
    pub fn target(k: Knob, m: &LearnerModel) -> u8 {
        let round = |x: f64, max: u8| x.round().clamp(0.0, f64::from(max)) as u8;
        let s = m.level(Skill::Speaking);
        match k {
            Knob::Speed => round((m.level(Skill::Listening) - 2.0) / 1.0, 4),
            Knob::Furigana => match m.level(Skill::ReadingKanji) {
                x if x < 2.0 => 0,
                x if x < 4.0 => 1,
                _ => 2,
            },
            Knob::Register => match m.level(Skill::Register) {
                x if x < 1.5 => 0,
                x if x < 3.5 => 1,
                _ => 2,
            },
            Knob::Length => round(s - 1.5, 4),
            Knob::Clauses => match s {
                x if x < 3.0 => 0,
                x if x < 5.0 => 1,
                _ => 2,
            },
            Knob::Vocabulary => round(s - 1.0, 5),
            Knob::Help => match m.level(Skill::Fluency) {
                x if x < 2.5 => 0,
                x if x < 4.5 => 1,
                _ => 2,
            },
            Knob::Goals => round(s - 1.0, 4),
        }
    }

    /// The setting that fits a learner, e.g. after the placement (/LF200/, /LF211/).
    pub fn for_learner(m: &LearnerModel) -> Self {
        let mut d = Self::default();
        for k in KNOBS {
            d.set(k, Self::target(k, m));
        }
        d
    }

    /// Text help in kana (romaji-like reading aid) for learners who cannot read kana yet.
    pub fn kana_aid(m: &LearnerModel) -> bool {
        m.level(Skill::ReadingKana) < 3.0
    }
}

/// The knob that follows a skill when the learner says "I know this" or "too hard" (/LF202/):
/// listening → speed, reading kanji → furigana, speaking → sentence length, register → register.
pub fn knob_for(skill: Skill) -> Option<Knob> {
    match skill {
        Skill::Listening => Some(Knob::Speed),
        Skill::ReadingKanji => Some(Knob::Furigana),
        Skill::Speaking => Some(Knob::Length),
        Skill::Register => Some(Knob::Register),
        _ => None,
    }
}

/// The learner corrects the model (/LF202/): the skill moves, and its knob one step at once.
pub fn learner_says(
    m: &mut LearnerModel,
    r: &mut Regulator,
    skill: Skill,
    up: bool,
) -> Option<Change> {
    m.nudge(skill, up);
    let k = knob_for(skill)?;
    let step = r.difficulty.get(k);
    let to = if up {
        step.saturating_add(1)
    } else {
        step.saturating_sub(1)
    };
    (to != step && to < k.steps()).then(|| r.set_by_learner(k, to))
}

/// One move of a knob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub knob: Knob,
    pub from: u8,
    pub to: u8,
    /// By the regulator, or set by the learner.
    pub by_learner: bool,
}

impl Change {
    /// The change in plain words for the summary, e.g. "Die Figur spricht etwas schneller."
    pub fn words(&self, lang: &str) -> &'static str {
        let de = lang == "de";
        let harder = self.to > self.from;
        match (self.knob, harder, de) {
            (Knob::Speed, true, true) => "Die Figur spricht nächstes Mal etwas schneller.",
            (Knob::Speed, true, false) => "Next time the character speaks a little faster.",
            (Knob::Speed, false, true) => "Die Figur spricht nächstes Mal etwas langsamer.",
            (Knob::Speed, false, false) => "Next time the character speaks a little slower.",
            (Knob::Length, true, true) => "Nächstes Mal etwas längere Sätze.",
            (Knob::Length, true, false) => "Next time a little longer sentences.",
            (Knob::Length, false, true) => "Nächstes Mal kürzere Sätze.",
            (Knob::Length, false, false) => "Next time shorter sentences.",
            (Knob::Vocabulary, true, true) => "Nächstes Mal ein paar neue Wörter mehr.",
            (Knob::Vocabulary, true, false) => "Next time a few more new words.",
            (Knob::Vocabulary, false, true) => "Nächstes Mal weniger neue Wörter.",
            (Knob::Vocabulary, false, false) => "Next time fewer new words.",
            (Knob::Clauses, true, true) => "Nächstes Mal auch Nebensätze.",
            (Knob::Clauses, true, false) => "Next time some subordinate clauses too.",
            (Knob::Clauses, false, true) => "Nächstes Mal einfachere Sätze ohne Nebensätze.",
            (Knob::Clauses, false, false) => "Next time simpler sentences without clauses.",
            (Knob::Furigana, true, true) => "Nächstes Mal etwas weniger Lesehilfe über den Kanji.",
            (Knob::Furigana, true, false) => "Next time a little less reading aid over the kanji.",
            (Knob::Furigana, false, true) => "Nächstes Mal wieder Lesehilfe über allen Kanji.",
            (Knob::Furigana, false, false) => "Next time reading aid over every kanji again.",
            (Knob::Register, true, true) => "Nächstes Mal mehr Höflichkeitsformen.",
            (Knob::Register, true, false) => "Next time more polite forms.",
            (Knob::Register, false, true) => "Nächstes Mal weniger Höflichkeitsformen.",
            (Knob::Register, false, false) => "Next time fewer polite forms.",
            (Knob::Help, true, true) => {
                "Hilfe kommt nächstes Mal etwas später; du hast sie kaum gebraucht."
            }
            (Knob::Help, true, false) => {
                "Next time help comes a little later; you hardly needed it."
            }
            (Knob::Help, false, true) => "Hilfe kommt nächstes Mal früher.",
            (Knob::Help, false, false) => "Next time help comes sooner.",
            (Knob::Goals, true, true) => "Nächstes Mal ein Ziel mehr im Gespräch.",
            (Knob::Goals, true, false) => "Next time one more goal in the talk.",
            (Knob::Goals, false, true) => "Nächstes Mal ein Ziel weniger im Gespräch.",
            (Knob::Goals, false, false) => "Next time one goal less in the talk.",
        }
    }
}

/// The regulator between sessions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Regulator {
    pub difficulty: Difficulty,
    /// Success of the past sessions, newest last.
    pub success: Vec<f64>,
    pub changes: Vec<Vec<Change>>,
    /// How many sessions there were when the skip offer was last taken: the same three
    /// sessions offer one unit, not one after the other (/LF215/).
    #[serde(default)]
    pub skip_taken: usize,
}

pub const TARGET: f64 = 0.8;
pub const HARDER_ABOVE: f64 = 0.88;
pub const EASIER_BELOW: f64 = 0.70;
pub const SKIP_AT: f64 = 0.95;

impl Regulator {
    pub fn new(difficulty: Difficulty) -> Self {
        Self {
            difficulty,
            ..Default::default()
        }
    }

    /// After a session with `success` (0..1): at most one knob, one step (/LF214/).
    pub fn after_session(&mut self, success: f64, m: &LearnerModel) -> Option<Change> {
        self.success.push(success);
        let d = &self.difficulty;
        let headroom = |k: Knob| f64::from(Difficulty::target(k, m)) - f64::from(d.get(k));
        let pick = if success > HARDER_ABOVE {
            KNOBS
                .iter()
                .filter(|k| d.get(**k) + 1 < k.steps())
                .max_by(|a, b| {
                    headroom(**a)
                        .partial_cmp(&headroom(**b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|k| (*k, d.get(*k) + 1))
        } else if success < EASIER_BELOW {
            KNOBS
                .iter()
                .filter(|k| d.get(**k) > 0)
                .min_by(|a, b| {
                    headroom(**a)
                        .partial_cmp(&headroom(**b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|k| (*k, d.get(*k) - 1))
        } else {
            None
        };
        let change = pick.map(|(knob, to)| Change {
            knob,
            from: self.difficulty.get(knob),
            to,
            by_learner: false,
        });
        if let Some(c) = &change {
            self.difficulty.set(c.knob, c.to);
        }
        self.changes.push(change.iter().cloned().collect());
        change
    }

    /// The learner sets a knob (/LF202/: "too easy" raises the speed one step). Not bound to the
    /// one-step rule.
    pub fn set_by_learner(&mut self, knob: Knob, to: u8) -> Change {
        let c = Change {
            knob,
            from: self.difficulty.get(knob),
            to: to.min(knob.steps() - 1),
            by_learner: true,
        };
        self.difficulty.set(knob, c.to);
        match self.changes.last_mut() {
            Some(last) => last.push(c.clone()),
            None => self.changes.push(vec![c.clone()]),
        }
        c
    }

    /// Three sessions in a row at 95 % or more: offer to skip ahead (/LF215/). Sessions from
    /// before the last skip do not count again.
    pub fn offer_skip(&self) -> bool {
        self.success.len().saturating_sub(self.skip_taken) >= 3
            && self.success.iter().rev().take(3).all(|s| *s >= SKIP_AT)
    }

    /// The offer was taken: the next one needs three new sessions.
    pub fn took_skip(&mut self) {
        self.skip_taken = self.success.len();
    }

    /// Moving success over the last `n` sessions.
    pub fn moving_success(&self, n: usize) -> Option<f64> {
        if self.success.is_empty() {
            return None;
        }
        let last: Vec<f64> = self.success.iter().rev().take(n).copied().collect();
        Some(last.iter().sum::<f64>() / last.len() as f64)
    }
}

/// Inside a session: two turns in a row not understood → easier at once (/LF213/).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Live {
    misses: u32,
    /// Steps taken back in this session (speed, length).
    pub eased: u32,
}

impl Live {
    /// Returns the easier setting for the rest of the session, when it is time.
    pub fn turn(&mut self, understood: bool, d: &Difficulty) -> Option<Difficulty> {
        if understood {
            self.misses = 0;
            return None;
        }
        self.misses += 1;
        if self.misses < 2 {
            return None;
        }
        self.misses = 0;
        let mut easier = d.clone();
        let mut moved = false;
        for k in [Knob::Speed, Knob::Length, Knob::Help] {
            let s = easier.get(k);
            if s > 0 {
                easier.set(k, s - 1);
                moved = true;
            }
        }
        moved.then(|| {
            self.eased += 1;
            easier
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_knob_one_step_towards_eighty_percent() {
        let m = LearnerModel::placed(&[(Skill::Listening, "n4"), (Skill::Speaking, "n5")]);
        let mut r = Regulator::new(Difficulty::default());
        let c = r.after_session(0.95, &m).unwrap();
        assert_eq!(c.knob, Knob::Speed, "listening has the most headroom");
        assert_eq!((c.from, c.to), (0, 1));
        assert!(r.after_session(0.8, &m).is_none(), "in the band: no change");
        let c = r.after_session(0.5, &m).unwrap();
        assert_eq!(c.to, c.from - 1);
        assert!(r.changes.iter().all(|s| s.len() <= 1));
    }

    #[test]
    fn placement_sets_uneven_profiles() {
        // Selin: hears N4, reads no kanji, casual only.
        let m = LearnerModel::placed(&[
            (Skill::Listening, "n4"),
            (Skill::Speaking, "n5_low"),
            (Skill::ReadingKana, "n5_low"),
            (Skill::ReadingKanji, "none"),
            (Skill::Register, "casual_only"),
        ]);
        let d = Difficulty::for_learner(&m);
        assert!(d.speed() >= 1.0);
        assert_eq!(d.get(Knob::Furigana), 0, "furigana on every kanji");
        assert!(Difficulty::kana_aid(&m));
    }

    #[test]
    fn two_misses_ease_the_rest_of_the_session() {
        let mut d = Difficulty::default();
        d.set(Knob::Speed, 3);
        d.set(Knob::Length, 2);
        let mut live = Live::default();
        assert!(live.turn(false, &d).is_none());
        let easier = live.turn(false, &d).unwrap();
        assert_eq!(easier.get(Knob::Speed), 2);
        assert_eq!(easier.get(Knob::Length), 1);
        assert!(live.turn(true, &easier).is_none());
    }

    #[test]
    fn skip_is_offered_after_three_easy_sessions() {
        let m = LearnerModel::default();
        let mut r = Regulator::new(Difficulty::default());
        for s in [0.96, 0.97, 0.99] {
            r.after_session(s, &m);
        }
        assert!(r.offer_skip());
        r.after_session(0.8, &m);
        assert!(!r.offer_skip());
    }

    #[test]
    fn learner_settings_bypass_the_step_rule() {
        let mut r = Regulator::new(Difficulty::default());
        let c = r.set_by_learner(Knob::Speed, 4);
        assert_eq!((c.from, c.to, c.by_learner), (0, 4, true));
        assert_eq!(r.difficulty.speed(), 1.1);
    }
}
