//! `kaiwa-sim trace`: the traceability table `spec/RUECKVERFOLGUNG.md`, requirement by
//! requirement: where the code names it and which persona checks test it, with the outcome of
//! the automated ones (text mode, recorded tutor answers, as in `run`).
//!
//! The table is generated; a test fails when the file in the repository is out of date.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;

use crate::run::Outcome;
use crate::validate::{persona_files, spec_requirements};

pub const FILE: &str = "spec/RUECKVERFOLGUNG.md";

/// Where code may name a requirement: the crates and the app, without the vendored Kante copy.
/// The QA runner (`sim`) counts only for the requirements on the QA itself (/LL6xx/); its
/// checks name every requirement they test.
fn code_places(root: &Path) -> Result<Vec<(String, std::path::PathBuf)>> {
    let mut places = Vec::new();
    for entry in fs::read_dir(root.join("crates")).context("reading crates/")? {
        let path = entry?.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if path.join("src").is_dir() {
            let short = name.trim_start_matches("kaiwa-").to_string();
            places.push((short, path.join("src")));
        }
    }
    places.sort();
    places.push(("app".into(), root.join("app/src")));
    places.push(("app".into(), root.join("app/qml")));
    Ok(places)
}

fn walk(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
    if !dir.is_dir() || dir.ends_with("qml/Kante") {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|x| ["rs", "qml", "js", "cpp", "h"].contains(&&*x.to_string_lossy()))
        {
            files.push(path);
        }
    }
    Ok(())
}

/// Requirement id → the places (crate short names, `app`) whose code names it.
fn code_refs(root: &Path) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let id = Regex::new(r"\bL([FDL]\d{3})\b").expect("valid regex");
    let mut refs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (place, dir) in code_places(root)? {
        let mut files = Vec::new();
        walk(&dir, &mut files)?;
        for file in files {
            let text = fs::read_to_string(&file).unwrap_or_default();
            for c in id.captures_iter(&text) {
                if place == "sim" && !c[1].starts_with("L6") {
                    continue;
                }
                refs.entry(format!("/L{}/", &c[1]))
                    .or_default()
                    .insert(place.clone());
            }
        }
    }
    Ok(refs)
}

/// The short text of each requirement: its first sentence, at most about 70 characters.
fn titles(spec: &str) -> BTreeMap<String, String> {
    let row = Regex::new(r"(?m)^\| (/L[FDL]\d{3}/) \| [MSK] \| ([^|]+) \|").expect("valid regex");
    row.captures_iter(spec)
        .map(|c| (c[1].to_string(), short(&c[2])))
        .collect()
}

fn short(text: &str) -> String {
    let text = text.replace("**", "").trim().to_string();
    // The first ". " that does not end an abbreviation (z. B., Kap. 9).
    let end = text.match_indices(". ").map(|(i, _)| i).find(|&i| {
        let word = text[..i].rsplit([' ', '(']).next().unwrap_or_default();
        !["z", "B", "Kap", "ca", "bzw", "vgl", "d", "h", "u", "a"].contains(&word)
    });
    let first = match end {
        Some(i) => &text[..=i],
        None => &text,
    };
    if first.chars().count() <= 70 {
        return first.to_string();
    }
    let mut out = String::new();
    for word in first.split(' ') {
        if out.chars().count() + word.chars().count() > 68 {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    format!("{} …", out.trim_end_matches([',', ':', ';']))
}

#[derive(Default)]
struct Checks {
    ids: Vec<String>,
    pass: usize,
    fail: usize,
}

pub fn table(root: &Path) -> Result<String> {
    let spec = fs::read_to_string(root.join("spec/LASTENHEFT.md"))
        .context("reading spec/LASTENHEFT.md")?;
    let requirements = spec_requirements(&spec);
    let titles = titles(&spec);
    let code = code_refs(root)?;

    let mut checks: BTreeMap<String, Checks> = BTreeMap::new();
    for file in persona_files(&root.join("qa/personas"))? {
        let persona: Value = serde_json::from_str(&fs::read_to_string(&file)?)?;
        let pid = persona["id"].as_str().unwrap_or_default();
        let mut outcomes = BTreeMap::new();
        for journey in crate::run::run_persona(root, pid, false)? {
            outcomes.extend(journey.checks);
        }
        for journey in persona["journeys"].as_array().into_iter().flatten() {
            for check in journey["checks"].as_array().into_iter().flatten() {
                let cid = check["id"].as_str().unwrap_or_default();
                for req in check["requirements"].as_array().into_iter().flatten() {
                    let entry = checks
                        .entry(req.as_str().unwrap_or_default().to_string())
                        .or_default();
                    entry.ids.push(cid.to_string());
                    match outcomes.get(cid) {
                        Some(Outcome::Pass) => entry.pass += 1,
                        Some(Outcome::Fail(_)) => entry.fail += 1,
                        _ => {}
                    }
                }
            }
        }
    }

    let mut rows = String::new();
    let mut counts: BTreeMap<(char, &str), usize> = BTreeMap::new();
    for (id, prio) in &requirements {
        let places = code
            .get(id)
            .map(|p| p.iter().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        let c = checks.remove(id).unwrap_or_default();
        let state = if c.ids.is_empty() && id.starts_with("/LL5") {
            "Vorgabe"
        } else if c.fail > 0 {
            "rot"
        } else if c.pass > 0 {
            "geprüft"
        } else if !places.is_empty() {
            "Code"
        } else {
            "offen"
        };
        *counts.entry((*prio, state)).or_default() += 1;
        let tests = if c.ids.is_empty() {
            "–".to_string()
        } else {
            let mut ids = c.ids.clone();
            ids.sort();
            format!(
                "{} von {} automatisch grün: {}",
                c.pass,
                ids.len(),
                ids.iter()
                    .map(|i| format!("`{i}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let title = titles.get(id).map(String::as_str).unwrap_or_default();
        let places = if places.is_empty() { "–" } else { &places };
        rows.push_str(&format!(
            "| {id} | {prio} | {title} | {places} | {state} | {tests} |\n"
        ));
    }

    let mut summary = String::new();
    for (prio, label) in [('M', "Muss"), ('S', "Soll"), ('K', "Kann")] {
        let n = |s: &str| counts.get(&(prio, s)).copied().unwrap_or(0);
        summary.push_str(&format!(
            "| {label} | {} | {} | {} | {} | {} |\n",
            n("geprüft"),
            n("Code"),
            n("offen"),
            n("Vorgabe"),
            n("rot")
        ));
    }

    Ok(format!(
        "# Rückverfolgung: Anforderung → Code → Prüfregel\n\
         \n\
         Erzeugt mit `cargo run -p kaiwa-sim -- trace --write`; ein Test in der CI prüft, dass \
         die Datei aktuell ist. Nicht von Hand bearbeiten.\n\
         \n\
         - **Code:** Crates (`core` = `kaiwa-core` …) und `app`, deren Quelltext die ID nennt.\n\
         - **Stand:** *geprüft* = mindestens eine Prüfregel läuft automatisch und ist grün; \
         *Code* = der Code nennt die ID, aber keine Prüfregel läuft automatisch; *offen* = \
         weder noch; *Vorgabe* = Plattform- oder Architekturvorgabe (/LL5xx/) ohne Prüfregel, \
         geprüft über CI und Pakete; *rot* = eine automatische Prüfregel scheitert. *Geprüft* heißt nicht, dass \
         die Anforderung ganz erfüllt ist: Prüfregeln für Audio, GUI und den echten Tutor laufen \
         noch nicht automatisch.\n\
         - Was bewusst ruht (Plasmoid auf dem Zweig `later/plasmoid`), steht in \
         [`STAND.md`](STAND.md).\n\
         \n\
         | Priorität | geprüft | Code | offen | Vorgabe | rot |\n\
         |---|---|---|---|---|---|\n\
         {summary}\n\
         | ID | Prio | Anforderung | Code | Stand | Prüfregeln |\n\
         |---|---|---|---|---|---|\n\
         {rows}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_keeps_the_first_sentence() {
        assert_eq!(
            short("**Sync** über WebDAV. Mehr dazu."),
            "Sync über WebDAV."
        );
        assert_eq!(
            short("Mehrere Profile (z. B. Familie), getrennt. Mehr."),
            "Mehrere Profile (z. B. Familie), getrennt."
        );
        let long = "Das ist ein sehr langer Satz, der viel länger ist als siebzig Zeichen und \
                    deshalb gekürzt wird";
        let s = short(long);
        assert!(s.ends_with(" …"), "{s}");
        assert!(s.chars().count() <= 72, "{s}");
    }

    #[test]
    fn the_table_in_the_repository_is_current() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let current = fs::read_to_string(root.join(FILE)).unwrap_or_default();
        let table = table(&root).unwrap();
        // The rows that differ: a check that turned red shows here, not only "out of date".
        let differing: Vec<&str> = table
            .lines()
            .filter(|l| !current.lines().any(|c| c == *l))
            .take(5)
            .collect();
        assert!(
            current == table,
            "{FILE} is out of date: cargo run -p kaiwa-sim -- trace --write\n{}",
            differing.join("\n")
        );
    }
}
