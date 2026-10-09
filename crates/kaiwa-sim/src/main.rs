//! kaiwa-sim: plays the QA personas in `qa/personas` against Kaiwa (qa/README.md).
//!
//! M0 has `validate`; `run` (text, audio and GUI modes) follows with the headless API in M1.

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
    }
}
