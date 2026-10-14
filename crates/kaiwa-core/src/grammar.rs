//! The grammar catalogue (Q-07, ADR-009): every grammar point a unit names, with an id, the
//! label the unit uses, its level, an explanation in Kaiwa's own words and examples. One file
//! per level in `data/grammar/`. The level of a point is the level of the unit that introduces
//! it ([`crate::Curriculum::check_grammar`]).

use std::collections::BTreeSet;

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
    /// The level files joined, each point with its file's level.
    pub fn merge(files: Vec<File>) -> Catalogue {
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
