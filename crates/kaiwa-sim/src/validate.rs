//! `kaiwa-sim validate`: personas against their schema and against the Lastenheft.
//!
//! Rules (qa/README.md):
//! - every persona matches `qa/personas/schema.json`;
//! - persona, journey and check ids are unique across all personas;
//! - every requirement a check names is defined in `spec/LASTENHEFT.md`;
//! - every must requirement (priority M) of chapters 4 and 6.1–6.5 is named by at least one
//!   check. Data (/LD/), platform and architecture (/LL5xx/), the QA itself (/LL6xx/) and
//!   the native-speaker review /LF221/ are covered elsewhere.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::Value;

/// Requirements that persona journeys do not have to cover.
fn exempt(id: &str) -> bool {
    id.starts_with("/LD") || id == "/LF221/" || id.starts_with("/LL5") || id.starts_with("/LL6")
}

#[derive(Debug, Default)]
pub struct Report {
    pub personas: usize,
    pub journeys: usize,
    pub checks: usize,
    pub problems: Vec<String>,
    pub uncovered: Vec<String>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.problems.is_empty() && self.uncovered.is_empty()
    }
}

/// Requirement ids in the spec, mapped to their priority (M, S, K).
pub fn spec_requirements(spec: &str) -> BTreeMap<String, char> {
    let row = Regex::new(r"(?m)^\| (/L[FDL]\d{3}/) \| ([MSK]) \|").expect("valid regex");
    row.captures_iter(spec)
        .map(|c| (c[1].to_string(), c[2].chars().next().expect("one letter")))
        .collect()
}

pub fn persona_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter(|p| p.file_name().is_some_and(|n| n != "schema.json"))
        .collect();
    files.sort();
    Ok(files)
}

pub fn validate(root: &Path) -> Result<Report> {
    let dir = root.join("qa/personas");
    let schema: Value = read_json(&dir.join("schema.json"))?;
    let validator = jsonschema::validator_for(&schema).context("compiling persona schema")?;
    let spec = fs::read_to_string(root.join("spec/LASTENHEFT.md"))
        .context("reading spec/LASTENHEFT.md")?;
    let requirements = spec_requirements(&spec);
    if requirements.is_empty() {
        bail!("no requirements found in spec/LASTENHEFT.md");
    }

    let mut report = Report::default();
    let mut ids = BTreeSet::new();
    let mut covered = BTreeSet::new();

    for file in persona_files(&dir)? {
        let name = file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let persona = read_json(&file)?;
        report.personas += 1;
        for error in validator.iter_errors(&persona) {
            report
                .problems
                .push(format!("{name}: {} at {}", error, error.instance_path()));
        }
        let pid = persona["id"].as_str().unwrap_or_default();
        if format!("{pid}.json") != name {
            report
                .problems
                .push(format!("{name}: id '{pid}' does not match the file name"));
        }
        if !ids.insert(pid.to_string()) {
            report
                .problems
                .push(format!("{name}: duplicate id '{pid}'"));
        }
        for journey in persona["journeys"].as_array().into_iter().flatten() {
            report.journeys += 1;
            let jid = journey["id"].as_str().unwrap_or_default();
            if !ids.insert(jid.to_string()) {
                report
                    .problems
                    .push(format!("{name}: duplicate id '{jid}'"));
            }
            for check in journey["checks"].as_array().into_iter().flatten() {
                report.checks += 1;
                let cid = check["id"].as_str().unwrap_or_default();
                if !ids.insert(cid.to_string()) {
                    report
                        .problems
                        .push(format!("{name}: duplicate id '{cid}'"));
                }
                for req in check["requirements"].as_array().into_iter().flatten() {
                    let req = req.as_str().unwrap_or_default();
                    if requirements.contains_key(req) {
                        covered.insert(req.to_string());
                    } else {
                        report.problems.push(format!(
                            "{name}: check '{cid}' names {req}, which the spec does not define"
                        ));
                    }
                }
            }
        }
    }

    report.uncovered = requirements
        .iter()
        .filter(|(id, prio)| **prio == 'M' && !exempt(id) && !covered.contains(*id))
        .map(|(id, _)| id.clone())
        .collect();
    Ok(report)
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn spec_rows_are_parsed_with_priority() {
        let spec =
            "| /LF100/ | M | Push-to-Talk | x |\n| /LF522/ | K | KRunner | x |\n| text | M |";
        let reqs = spec_requirements(spec);
        assert_eq!(reqs.get("/LF100/"), Some(&'M'));
        assert_eq!(reqs.get("/LF522/"), Some(&'K'));
        assert_eq!(reqs.len(), 2);
    }

    #[test]
    fn exemptions_follow_the_qa_readme() {
        assert!(exempt("/LD010/"));
        assert!(exempt("/LF221/"));
        assert!(exempt("/LL503/"));
        assert!(exempt("/LL601/"));
        assert!(!exempt("/LF100/"));
        assert!(!exempt("/LL401/"));
    }

    #[test]
    fn repository_personas_are_valid_and_cover_every_must() {
        let report = validate(&root()).expect("validation runs");
        assert!(report.personas >= 5, "at least five personas");
        assert!(report.problems.is_empty(), "{:#?}", report.problems);
        assert!(
            report.uncovered.is_empty(),
            "uncovered: {:?}",
            report.uncovered
        );
    }
}
