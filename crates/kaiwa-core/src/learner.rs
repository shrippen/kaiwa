//! The learner model (Lastenheft /LF201/, /LF202/, /LD020/): one level per skill, because
//! profiles are uneven (Selin hears at N4 and reads no kanji), and the mistakes per kind.
//!
//! Levels use the scale of the personas' `start_level` (none, a0, n5_low, n5, n4_low, n4, n3, n2,
//! native_casual) as a number from 0 to 8, so the placement can be checked against them. Each
//! session moves a skill towards the level it showed: succeeding at about 80 % on material of
//! level L is evidence for level L (/LF210/). Help counts as evidence of a lower level but never
//! shows as a penalty (/LF212/).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const LEVELS: [&str; 9] = [
    "none",
    "a0",
    "n5_low",
    "n5",
    "n4_low",
    "n4",
    "n3",
    "n2",
    "native_casual",
];
pub const REGISTERS: [&str; 5] = [
    "none",
    "casual_only",
    "desu_masu",
    "desu_masu_and_casual",
    "keigo_basic",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skill {
    Listening,
    /// Grammar and words when speaking.
    Speaking,
    /// Answering without long pauses or help.
    Fluency,
    Pronunciation,
    ReadingKana,
    ReadingKanji,
    /// Scale 0–4 of `REGISTERS`.
    Register,
}

pub const SKILLS: [Skill; 7] = [
    Skill::Listening,
    Skill::Speaking,
    Skill::Fluency,
    Skill::Pronunciation,
    Skill::ReadingKana,
    Skill::ReadingKanji,
    Skill::Register,
];

impl Skill {
    pub fn max(self) -> f64 {
        if self == Skill::Register { 4.0 } else { 8.0 }
    }

    pub fn word(self, lang: &str) -> &'static str {
        let de = lang == "de";
        match self {
            Skill::Listening => {
                if de {
                    "Hören"
                } else {
                    "Listening"
                }
            }
            Skill::Speaking => {
                if de {
                    "Sprechen"
                } else {
                    "Speaking"
                }
            }
            Skill::Fluency => {
                if de {
                    "Flüssigkeit"
                } else {
                    "Fluency"
                }
            }
            Skill::Pronunciation => {
                if de {
                    "Aussprache"
                } else {
                    "Pronunciation"
                }
            }
            Skill::ReadingKana => {
                if de {
                    "Kana lesen"
                } else {
                    "Reading kana"
                }
            }
            Skill::ReadingKanji => {
                if de {
                    "Kanji lesen"
                } else {
                    "Reading kanji"
                }
            }
            Skill::Register => {
                if de {
                    "Höflichkeitsformen"
                } else {
                    "Politeness"
                }
            }
        }
    }
}

/// How the learner did with one kind of mistake (particle, counter …).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ErrorStat {
    /// Times this kind was corrected.
    pub corrected: u32,
    /// Moving rate of the mistake per chance to make it, 0..1.
    pub rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearnerModel {
    pub skills: BTreeMap<Skill, f64>,
    #[serde(default)]
    pub errors: BTreeMap<String, ErrorStat>,
    /// The learner said "I know this already" or "too hard" (/LF202/); kept for the history.
    #[serde(default)]
    pub corrections_by_learner: u32,
}

impl Default for LearnerModel {
    /// A beginner: a0 everywhere, no kana, no register.
    fn default() -> Self {
        let mut skills = BTreeMap::new();
        for s in SKILLS {
            skills.insert(s, if s == Skill::Register { 0.0 } else { 1.0 });
        }
        skills.insert(Skill::ReadingKana, 0.0);
        skills.insert(Skill::ReadingKanji, 0.0);
        Self {
            skills,
            errors: BTreeMap::new(),
            corrections_by_learner: 0,
        }
    }
}

/// The index of a level name (`LEVELS` or, for the register, `REGISTERS`).
pub fn level_index(name: &str) -> Option<f64> {
    LEVELS
        .iter()
        .position(|l| *l == name)
        .or_else(|| REGISTERS.iter().position(|l| *l == name))
        .map(|i| i as f64)
}

/// What the learner showed in one session, per skill: the level of the material and the share
/// that went well (0..1). Skills without evidence are left out.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub at_level: BTreeMap<Skill, (f64, f64)>,
    /// Corrections per kind in the session, and the turns that could have had one.
    pub mistakes: BTreeMap<String, u32>,
    pub turns: u32,
}

/// How fast a skill follows the evidence.
const RATE: f64 = 0.25;

impl LearnerModel {
    pub fn level(&self, s: Skill) -> f64 {
        self.skills.get(&s).copied().unwrap_or(0.0)
    }

    pub fn set(&mut self, s: Skill, v: f64) {
        self.skills.insert(s, v.clamp(0.0, s.max()));
    }

    /// A placement result (/LF200/): levels per skill by name.
    pub fn placed(levels: &[(Skill, &str)]) -> Self {
        let mut m = Self::default();
        for (s, name) in levels {
            if let Some(v) = level_index(name) {
                m.set(*s, v);
            }
        }
        m
    }

    /// Moves each skill towards what the session showed: success `r` at level `l` means about
    /// `l + (r − 0.8) · 2.5`.
    pub fn update(&mut self, e: &Evidence) {
        for (skill, (at, r)) in &e.at_level {
            let shown = at + (r - 0.8) * 2.5;
            let now = self.level(*skill);
            self.set(*skill, now + RATE * (shown - now));
        }
        for (kind, n) in &e.mistakes {
            let stat = self.errors.entry(kind.clone()).or_default();
            stat.corrected += n;
            let r = f64::from(*n) / f64::from(e.turns.max(1));
            stat.rate = 0.7 * stat.rate + 0.3 * r;
        }
        for (kind, stat) in self.errors.iter_mut() {
            if !e.mistakes.contains_key(kind) && e.turns > 0 {
                stat.rate *= 0.7;
            }
        }
    }

    /// The learner corrects the model (/LF202/): "I know this already" (+1) or "too hard" (−1).
    pub fn nudge(&mut self, s: Skill, up: bool) {
        let v = self.level(s) + if up { 0.7 } else { -0.7 };
        self.set(s, v);
        self.corrections_by_learner += 1;
    }

    /// The profile in words, no raw numbers (/LF202/).
    pub fn describe(&self, lang: &str) -> Vec<(Skill, String)> {
        SKILLS
            .iter()
            .map(|s| (*s, describe_level(*s, self.level(*s), lang)))
            .collect()
    }

    /// The kinds of mistakes that still come up most, worst first.
    pub fn weak_spots(&self, n: usize) -> Vec<(&str, f64)> {
        let mut v: Vec<(&str, f64)> = self
            .errors
            .iter()
            .filter(|(_, s)| s.rate >= 0.1)
            .map(|(k, s)| (k.as_str(), s.rate))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(n);
        v
    }
}

fn describe_level(s: Skill, v: f64, lang: &str) -> String {
    let de = lang == "de";
    if s == Skill::Register {
        return match (v.round() as i32, de) {
            (..=0, true) => "Noch keine Höflichkeitsformen.".into(),
            (..=0, false) => "No polite forms yet.".into(),
            (1, true) => "Spricht locker wie unter Freunden; die です/ます-Form fehlt noch.".into(),
            (1, false) => {
                "Speaks casually as among friends; the です/ます form is still missing.".into()
            }
            (2, true) => "Sicher in der です/ます-Form.".into(),
            (2, false) => "Confident in the です/ます form.".into(),
            (3, true) => "Wechselt passend zwischen locker und höflich.".into(),
            (3, false) => "Switches between casual and polite as fits.".into(),
            (_, true) => "Kennt die Grundlagen von 敬語 (Höflichkeitssprache).".into(),
            (_, false) => "Knows the basics of 敬語 (honorific speech).".into(),
        };
    }
    let band = match v {
        x if x < 0.5 => 0,
        x if x < 1.5 => 1,
        x if x < 2.5 => 2,
        x if x < 3.5 => 3,
        x if x < 4.5 => 4,
        x if x < 5.5 => 5,
        _ => 6,
    };
    let what = s.word(lang);
    match (band, de) {
        (0, true) => format!("{what}: noch nicht begonnen."),
        (0, false) => format!("{what}: not started yet."),
        (1, true) => format!("{what}: erste Wörter und Wendungen."),
        (1, false) => format!("{what}: first words and phrases."),
        (2, true) => format!("{what}: einfache Sätze mit Hilfe (Anfang N5)."),
        (2, false) => format!("{what}: simple sentences with help (early N5)."),
        (3, true) => format!("{what}: sicher bei Alltagsthemen auf N5-Niveau."),
        (3, false) => format!("{what}: confident with everyday topics at N5."),
        (4, true) => format!("{what}: erste N4-Themen gelingen."),
        (4, false) => format!("{what}: first N4 topics go well."),
        (5, true) => format!("{what}: sicher auf N4-Niveau."),
        (5, false) => format!("{what}: confident at N4."),
        (_, true) => format!("{what}: über N4 hinaus."),
        (_, false) => format!("{what}: beyond N4."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_follow_success_at_a_level() {
        let mut m = LearnerModel::default();
        let mut e = Evidence::default();
        e.at_level.insert(Skill::Listening, (3.0, 0.8));
        for _ in 0..20 {
            m.update(&e);
        }
        assert!((m.level(Skill::Listening) - 3.0).abs() < 0.05);
        e.at_level.insert(Skill::Listening, (3.0, 1.0));
        m.update(&e);
        assert!(
            m.level(Skill::Listening) > 3.0,
            "doing better than 80 % raises it"
        );
    }

    #[test]
    fn mistakes_fade_when_they_stop() {
        let mut m = LearnerModel::default();
        let mut e = Evidence {
            turns: 5,
            ..Default::default()
        };
        e.mistakes.insert("particle".into(), 2);
        m.update(&e);
        assert_eq!(m.weak_spots(3)[0].0, "particle");
        e.mistakes.clear();
        for _ in 0..8 {
            m.update(&e);
        }
        assert!(m.weak_spots(3).is_empty());
        assert_eq!(m.errors["particle"].corrected, 2);
    }

    #[test]
    fn placement_and_words() {
        let m = LearnerModel::placed(&[
            (Skill::Listening, "n4"),
            (Skill::ReadingKanji, "none"),
            (Skill::Register, "casual_only"),
        ]);
        assert_eq!(m.level(Skill::Listening), 5.0);
        assert_eq!(m.level(Skill::Register), 1.0);
        let words = m.describe("de");
        assert!(
            words
                .iter()
                .any(|(s, w)| *s == Skill::Listening && w.contains("N4"))
        );
        assert!(
            words
                .iter()
                .all(|(_, w)| !w.chars().any(|c| c.is_ascii_digit()) || w.contains("N"))
        );
    }

    #[test]
    fn the_learner_can_correct_the_model() {
        let mut m = LearnerModel::default();
        let before = m.level(Skill::Listening);
        m.nudge(Skill::Listening, true);
        assert!(m.level(Skill::Listening) > before);
        assert_eq!(m.corrections_by_learner, 1);
    }
}
