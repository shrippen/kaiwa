//! kaiwa-sim: plays the QA personas in `qa/personas` against Kaiwa (qa/README.md).
//!
//! `validate` checks the personas; `run` plays their journeys in text mode against the engine
//! with recorded tutor answers. Audio and GUI modes follow with the app.

mod run;
mod validate;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "QA runner for the Kaiwa personas")]
struct Cli {
    /// Repository root (holds qa/ and spec/).
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check the personas against their schema and the requirements in the Lastenheft.
    Validate,
    /// Play the journeys in text mode and run their checks.
    Run {
        /// Persona id; all personas when left out.
        #[arg(long)]
        persona: Option<String>,
        /// Write the report as JSON into this file.
        #[arg(long)]
        report: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate => match validate::validate(&cli.root) {
            Ok(report) => {
                for problem in &report.problems {
                    eprintln!("error: {problem}");
                }
                for id in &report.uncovered {
                    eprintln!("error: must requirement {id} is not named by any check");
                }
                println!(
                    "{} personas, {} journeys, {} checks: {}",
                    report.personas,
                    report.journeys,
                    report.checks,
                    if report.ok() { "ok" } else { "failed" }
                );
                if report.ok() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(error) => {
                eprintln!("error: {error:#}");
                ExitCode::FAILURE
            }
        },
        Command::Run { persona, report } => {
            match run_all(&cli.root, persona.as_deref(), report.as_deref()) {
                Ok(true) => ExitCode::SUCCESS,
                Ok(false) => ExitCode::FAILURE,
                Err(error) => {
                    eprintln!("error: {error:#}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn run_all(
    root: &std::path::Path,
    persona: Option<&str>,
    report: Option<&std::path::Path>,
) -> anyhow::Result<bool> {
    let ids: Vec<String> = match persona {
        Some(p) => vec![p.to_string()],
        None => validate::persona_files(&root.join("qa/personas"))?
            .iter()
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .collect(),
    };
    let mut all = Vec::new();
    for id in ids {
        all.extend(run::run_persona(root, &id)?);
    }
    let (mut pass, mut fail, mut open) = (0, 0, 0);
    for j in &all {
        for (cid, outcome) in &j.checks {
            match outcome {
                run::Outcome::Pass => {
                    pass += 1;
                    println!("pass  {} / {}", j.journey, cid);
                }
                run::Outcome::Fail(why) => {
                    fail += 1;
                    println!("FAIL  {} / {}: {}", j.journey, cid, why);
                }
                run::Outcome::NotAutomated => open += 1,
            }
        }
    }
    println!("{pass} passed, {fail} failed, {open} not automated yet");
    if let Some(path) = report {
        std::fs::write(path, serde_json::to_string_pretty(&all)?)?;
    }
    Ok(fail == 0)
}
