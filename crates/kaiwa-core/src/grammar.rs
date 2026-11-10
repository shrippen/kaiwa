//! The grammar catalogue (Q-07, ADR-009): every grammar point a unit names, with an id, the
//! label the unit uses, its level, an explanation in Kaiwa's own words and examples. One file
//! per level in `data/grammar/`. The level of a point is the level of the unit that introduces
//! it ([`crate::Curriculum::check_grammar`]). Each point's `pattern` drives the grammar lint
//! (/LF221/, `tests/grammar_lint.rs`): coverage in its unit and no use before it.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::scenario::Text;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    /// Stable id, ASCII (`te-kudasai`).
    pub id: String,
    /// What the units write, e.g. „〜てください (Bitte)“.
    pub label: String,
    /// Set from the file by [`Catalogue::merge`].
    #[serde(default)]
    pub level: String,
    /// Kaiwa's own explanation, short.
    pub explain: Text,
    /// A regular expression (Rust `regex`) that finds the point in a sentence without
    /// furigana markup, for the grammar lint (INHALT-ENTWURF 3.2); none where a pattern would
    /// not be reliable (plain forms, transitivity pairs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Example sentences in furigana markup (own or Tatoeba, CC BY).
    #[serde(default)]
    pub examples: Vec<String>,
}

/// One level file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct File {
    pub level: String,
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalogue {
    pub points: Vec<Point>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GrammarError {
    #[error("grammar id {0} appears twice")]
    DuplicateId(String),
    #[error("grammar label {0} appears twice")]
    DuplicateLabel(String),
    #[error("grammar point {0} has no explanation in {1}")]
    NoExplanation(String, &'static str),
    #[error("grammar id {0} is not lower-case ASCII with dashes")]
    BadId(String),
}

impl Catalogue {
    /// The level files joined in course order (N5 first, as [`crate::curriculum::LEVELS`]),
    /// each point with its file's level. The order of the files given does not matter.
    pub fn merge(mut files: Vec<File>) -> Catalogue {
        let rank = |l: &str| {
            crate::curriculum::LEVELS
                .iter()
                .position(|x| *x == l)
                .unwrap_or(crate::curriculum::LEVELS.len())
        };
        files.sort_by_key(|f| rank(&f.level));
        let mut points = Vec::new();
        for f in files {
            for mut p in f.points {
                p.level = f.level.clone();
                points.push(p);
            }
        }
        Catalogue { points }
    }

    pub fn from_files(files: &[&str]) -> Result<Self, serde_json::Error> {
        Ok(Self::merge(
            files
                .iter()
                .map(|j| serde_json::from_str(j))
                .collect::<Result<_, _>>()?,
        ))
    }

    /// Every `*.json` in `dir` (`data/grammar`), joined.
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

    pub fn by_label(&self, label: &str) -> Option<&Point> {
        self.points.iter().find(|p| p.label == label)
    }

    pub fn get(&self, id: &str) -> Option<&Point> {
        self.points.iter().find(|p| p.id == id)
    }

    /// Unique ids and labels, ids as `a-z0-9-`, explanations in both languages.
    pub fn check(&self) -> Result<(), GrammarError> {
        let mut ids = BTreeSet::new();
        let mut labels = BTreeSet::new();
        for p in &self.points {
            if p.id.is_empty()
                || !p
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                return Err(GrammarError::BadId(p.id.clone()));
            }
            if !ids.insert(p.id.as_str()) {
                return Err(GrammarError::DuplicateId(p.id.clone()));
            }
            if !labels.insert(p.label.as_str()) {
                return Err(GrammarError::DuplicateLabel(p.label.clone()));
            }
            if p.explain.de.trim().is_empty() {
                return Err(GrammarError::NoExplanation(p.id.clone(), "de"));
            }
            if p.explain.en.trim().is_empty() {
                return Err(GrammarError::NoExplanation(p.id.clone(), "en"));
            }
        }
        Ok(())
    }
}

/// What the learner model knows about one grammar point (/LF201/, /LD020/): how often a turn
/// used it without a mistake on it, how often a counted correction concerned it, and when it
/// last came up.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointState {
    pub used: u32,
    pub wrong: u32,
    #[serde(default)]
    pub last: Option<crate::clock::Day>,
}

impl PointState {
    /// "firm" (used at least three times, at most one in five wrong), "practising" (a mistake
    /// among the last uses) or "new".
    pub fn status(&self) -> &'static str {
        if self.wrong > 0 && self.wrong * 5 > self.used {
            "practising"
        } else if self.used >= 3 {
            "firm"
        } else {
            "new"
        }
    }
}

/// The catalogue's patterns, compiled once.
pub struct Matcher {
    points: Vec<(String, regex::Regex)>,
}

impl Catalogue {
    pub fn matcher(&self) -> Matcher {
        Matcher {
            points: self
                .points
                .iter()
                .filter_map(|p| {
                    let re = regex::Regex::new(p.pattern.as_deref()?).ok()?;
                    Some((p.id.clone(), re))
                })
                .collect(),
        }
    }
}

impl Matcher {
    /// The ids of the points a sentence uses.
    pub fn ids<'a>(&'a self, text: &str) -> Vec<&'a str> {
        self.points
            .iter()
            .filter(|(_, re)| re.is_match(text))
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Counts a session's turns into `states`: a counted correction (/LF303/: not an
    /// alternative, at least unnatural, not disputed or misrecognised) is a mistake on the
    /// points its better form uses and the learner's words lack; the points a turn uses
    /// without such a mistake count as used.
    pub fn track(
        &self,
        states: &mut BTreeMap<String, PointState>,
        turns: &[crate::session::TurnRecord],
        day: crate::clock::Day,
    ) {
        // A turn analysed with gaps (T-11) shows neither a mistake nor a point used well.
        for t in turns.iter().filter(|t| !t.incomplete) {
            let mut wrong: BTreeSet<&str> = BTreeSet::new();
            for c in crate::session::counted(t)
                .iter()
                .filter(|c| !c.also_possible && c.severity >= kaiwa_ai::Severity::Unnatural)
            {
                // Only what the correction changes: points in the better form that the
                // learner's own words lack (を → が concerns 〜が好き, not 〜です).
                let had = self.ids(&c.original);
                wrong.extend(
                    self.ids(&c.better)
                        .into_iter()
                        .filter(|id| !had.contains(id)),
                );
            }
            for id in &wrong {
                let s = states.entry((*id).to_string()).or_default();
                s.wrong += 1;
                s.last = Some(day);
            }
            if t.misrecognized {
                continue;
            }
            for id in self.ids(&t.learner) {
                if !wrong.contains(id) {
                    let s = states.entry(id.to_string()).or_default();
                    s.used += 1;
                    s.last = Some(day);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(id: &str, label: &str) -> Point {
        Point {
            id: id.into(),
            label: label.into(),
            level: String::new(),
            explain: Text {
                de: "x".into(),
                en: "x".into(),
            },
            pattern: None,
            examples: vec![],
        }
    }

    #[test]
    fn points_take_the_level_of_their_file() {
        let c = Catalogue::merge(vec![
            File {
                level: "n5".into(),
                points: vec![point("ne", "〜ね")],
            },
            File {
                level: "n4".into(),
                points: vec![point("tari", "〜たり〜たりします")],
            },
        ]);
        c.check().unwrap();
        assert_eq!(c.by_label("〜ね").unwrap().level, "n5");
        assert_eq!(c.get("tari").unwrap().level, "n4");
    }

    #[test]
    fn files_join_in_course_order_whatever_order_they_come_in() {
        let file = |level: &str, id: &str| File {
            level: level.into(),
            points: vec![point(id, id)],
        };
        let a = Catalogue::merge(vec![file("n4", "tari"), file("n5", "ne")]);
        let b = Catalogue::merge(vec![file("n5", "ne"), file("n4", "tari")]);
        assert_eq!(a, b);
        assert_eq!(a.points[0].id, "ne");
    }

    #[test]
    fn turns_count_into_each_points_state() {
        use crate::session::{Input, TurnRecord};
        use kaiwa_ai::{Correction, CorrectionKind, Severity};
        let mut ga_suki = point("ga-suki", "〜が好きです");
        ga_suki.pattern = Some("が好き".into());
        let mut desu = point("desu", "〜です");
        desu.pattern = Some("です".into());
        let m = Catalogue {
            points: vec![ga_suki, desu],
        }
        .matcher();
        let turn = |learner: &str, better: Option<&str>| TurnRecord {
            learner: learner.into(),
            input: Input::Typed,
            corrections: better
                .map(|b| Correction {
                    severity: Severity::Wrong,
                    kind: CorrectionKind::Particle,
                    original: learner.into(),
                    better: b.into(),
                    explanation: String::new(),
                    also_possible: false,
                })
                .into_iter()
                .collect(),
            new_words: vec![],
            markup: String::new(),
            incomplete: false,
            understood: true,
            aided: false,
            from_help: false,
            xp: 0,
            reply: String::new(),
            misrecognized: false,
            disputed: vec![],
        };
        let mut states = BTreeMap::new();
        let day = crate::clock::Day(10);
        m.track(
            &mut states,
            &[
                turn("アニメが好きです。", None),
                turn("コーヒーを好きです。", Some("コーヒーが好きです。")),
                turn("テニスが好きです。", None),
                turn("音楽が好きです。", None),
            ],
            day,
        );
        let s = &states["ga-suki"];
        assert_eq!((s.used, s.wrong), (3, 1));
        assert_eq!(s.status(), "practising", "one in four wrong");
        assert_eq!(
            states["desu"].used, 4,
            "the correction is about が, not です"
        );
        assert_eq!(states["desu"].wrong, 0);
        assert_eq!(s.last, Some(day));
    }

    #[test]
    fn duplicates_and_bad_ids_are_found() {
        let c = Catalogue {
            points: vec![point("ne", "〜ね"), point("ne", "〜よ")],
        };
        assert_eq!(c.check(), Err(GrammarError::DuplicateId("ne".into())));
        let c = Catalogue {
            points: vec![point("ne", "〜ね"), point("yo", "〜ね")],
        };
        assert_eq!(c.check(), Err(GrammarError::DuplicateLabel("〜ね".into())));
        let c = Catalogue {
            points: vec![point("Ne", "〜ね")],
        };
        assert_eq!(c.check(), Err(GrammarError::BadId("Ne".into())));
    }
}
