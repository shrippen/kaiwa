//! Review sheets for the native speaker review (INHALT-ENTWURF 3.3, /LF221/): one Markdown file
//! per unit with everything the reviewer reads (can-do, grammar with explanations, core words,
//! each scenario's character, opening, goals with help, the recorded answers with their planted
//! mistakes and corrections), the checklist, and the content hash to enter as `review.hash`.
//!
//!   cargo run -p kaiwa-sim -- review-sheet --out target/review [--unit n4-13]

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use kaiwa_core::curriculum::{Curriculum, Unit};
use kaiwa_core::grammar::Catalogue;
use kaiwa_core::scenario::{Register, Scenario};
use serde_json::Value;

const CHECKLIST: &[&str] = &[
    "Klingt jeder Satz natürlich, so wie man ihn in dieser Lage sagt? / Does every sentence sound natural for the situation?",
    "Passt das Register zur Figur (höflich, Freundesform, Keigo)? / Does the register fit the character?",
    "Ist die Szene kulturell stimmig? / Is the scene culturally right?",
    "Stimmen Lesungen und Furigana? / Are readings and furigana right?",
    "Sind die Korrekturen der eingebauten Fehler richtig, und stimmt die Erklärung? / Are the corrections of the planted mistakes right, and their explanations?",
    "Stimmen die Grammatik-Erklärungen und Beispiele? / Are the grammar explanations and examples right?",
    "Ist der Kann-Satz für die Stufe realistisch? / Is the can-do realistic for the level?",
];

fn register(r: Register) -> &'static str {
    match r {
        Register::Casual => "Freundesform / casual",
        Register::Polite => "höflich / polite (です・ます)",
        Register::Keigo => "Keigo",
    }
}

/// The sheet for one unit.
pub fn sheet(root: &Path, c: &Curriculum, grammar: &Catalogue, u: &Unit) -> Result<String> {
    let mut texts = Vec::new();
    for s in &u.scenarios {
        texts.push(
            std::fs::read_to_string(root.join(format!("data/scenarios/{s}.json")))
                .with_context(|| format!("scenario {s}"))?,
        );
    }
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let hash = Curriculum::content_hash(u, &refs);
    let level = u.level.as_ref().map(|l| l.label()).unwrap_or_default();
    let mut m = String::new();
    writeln!(m, "# {} {} / {} ({level})\n", u.id, u.title.de, u.title.en)?;
    writeln!(m, "**Kann / Can-do:** {} / {}\n", u.can_do.de, u.can_do.en)?;
    let requires: Vec<String> = u
        .requires
        .iter()
        .map(|r| {
            c.unit(r)
                .map(|x| format!("{r} {}", x.title.de))
                .unwrap_or_else(|| r.clone())
        })
        .collect();
    if !requires.is_empty() {
        writeln!(m, "Setzt voraus / requires: {}\n", requires.join(", "))?;
    }
    writeln!(m, "## Grammatik / Grammar\n")?;
    for label in &u.grammar {
        match grammar.by_label(label) {
            Some(p) => {
                writeln!(m, "- **{label}** ({})", p.level.to_uppercase())?;
                writeln!(m, "  - de: {}", p.explain.de)?;
                writeln!(m, "  - en: {}", p.explain.en)?;
                for e in &p.examples {
                    writeln!(m, "  - 例: {e}")?;
                }
            }
            None => writeln!(m, "- **{label}**")?,
        }
    }
    writeln!(
        m,
        "\n## Kernwörter / Core words\n\n{}\n",
        u.words.join("、")
    )?;
    for text in &texts {
        let s = Scenario::from_json(text)?;
        writeln!(m, "## Szenario / Scenario {} „{}“\n", s.id, s.title.de)?;
        writeln!(
            m,
            "- Figur / character: {} — {}",
            s.name.as_deref().unwrap_or("–"),
            s.character.de
        )?;
        writeln!(m, "- Register: {}", register(s.register))?;
        write!(m, "- Eröffnung / opening: {}", s.opening)?;
        if let Some(t) = &s.opening_translation {
            write!(m, " ({} / {})", t.de, t.en)?;
        }
        writeln!(
            m,
            "\n\n| Ziel / goal | Stichwort | Anfang | Beispiel |\n|---|---|---|---|"
        )?;
        for g in &s.goals {
            writeln!(
                m,
                "| {} | {} | {} | {} |",
                g.text.de, g.help.keyword, g.help.start, g.help.example
            )?;
        }
        let fp = root.join(format!("qa/fixtures/{}.json", s.id));
        if let Ok(f) = std::fs::read_to_string(fp) {
            let turns: Value = serde_json::from_str(&f)?;
            writeln!(
                m,
                "\n**Aufgezeichnete Antworten / recorded answers** (Lernende → Figur)\n"
            )?;
            for t in turns.as_array().into_iter().flatten() {
                let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
                writeln!(
                    m,
                    "- {} → {} ({})",
                    s(&t["learner"]),
                    s(&t["reply"]),
                    s(&t["translation"]["de"])
                )?;
                for c in t["analysis"]["corrections"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    writeln!(
                        m,
                        "  - Eingebauter Fehler / planted mistake: {} → {} ({}, {}): {}",
                        s(&c["original"]),
                        s(&c["better"]),
                        s(&c["severity"]),
                        s(&c["kind"]),
                        s(&c["explanation"])
                    )?;
                }
            }
        }
        writeln!(m)?;
    }
    writeln!(m, "## Checkliste / Checklist\n")?;
    for item in CHECKLIST {
        writeln!(m, "- [ ] {item}")?;
    }
    writeln!(
        m,
        "\nFundstellen bitte mit Szenario und Satz notieren. / Note findings with scenario and sentence.\n"
    )?;
    writeln!(m, "## Nachweis / Proof\n")?;
    writeln!(
        m,
        "Nach dem Review in `data/curriculum/{}.json` bei {} eintragen / enter after the review:\n",
        u.id.split('-').next().unwrap_or_default(),
        u.id
    )?;
    writeln!(
        m,
        "```json\n\"review\": {{ \"by\": \"…\", \"date\": \"…\", \"hash\": \"{hash}\" }}\n```"
    )?;
    Ok(m)
}

/// Writes the sheets of all units (or one) to `out`; returns how many.
pub fn write_sheets(root: &Path, out: &Path, only: Option<&str>) -> Result<usize> {
    let c = Curriculum::from_dir(&root.join("data/curriculum"))?;
    let g = Catalogue::from_dir(&root.join("data/grammar"))?;
    std::fs::create_dir_all(out)?;
    let mut n = 0;
    for u in c.units.iter().filter(|u| only.is_none_or(|o| o == u.id)) {
        std::fs::write(out.join(format!("{}.md", u.id)), sheet(root, &c, &g, u)?)?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_has_the_texts_the_checklist_and_the_hash() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let c = Curriculum::from_dir(&root.join("data/curriculum")).unwrap();
        let g = Catalogue::from_dir(&root.join("data/grammar")).unwrap();
        let u = c.unit("n5-08").unwrap();
        let s = sheet(&root, &c, &g, u).unwrap();
        assert!(s.contains("〜てもいいですか"));
        assert!(s.contains("お風呂で写真を撮ってもいいですか。"));
        assert!(s.contains("Eingebauter Fehler"));
        assert!(s.contains("- [ ] Klingt jeder Satz natürlich"));
        let texts: Vec<String> = u
            .scenarios
            .iter()
            .map(|x| {
                std::fs::read_to_string(root.join(format!("data/scenarios/{x}.json"))).unwrap()
            })
            .collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        assert!(s.contains(&Curriculum::content_hash(u, &refs)));
    }
}
