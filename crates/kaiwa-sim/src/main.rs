//! kaiwa-sim: plays the QA personas in `qa/personas` against Kaiwa (qa/README.md).
//!
//! `validate` checks the personas; `run` plays their journeys in text mode against the engine
//! with recorded tutor answers, and the long journeys over simulated days against the learning
//! model (`days`); `trace` writes the traceability table. Audio and GUI modes follow with the
//! app.

mod days;
mod engine;
mod profile;
mod review;
mod run;
mod trace;
mod ui;
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
        /// Write the report as JSON into this file: every check with its outcome and
        /// requirements, the curves of the long journeys (/LL604/).
        #[arg(long)]
        report: Option<PathBuf>,
        /// A real tutor from KAIWA_PROVIDER instead of the recorded answers (local test).
        #[arg(long)]
        live: bool,
    },
    /// Play one long journey over simulated days and print it session by session.
    Days {
        #[arg(long)]
        persona: String,
        #[arg(long)]
        journey: String,
        /// Instead of the journey, play it with this many seeds (the script's and the ones
        /// after it) and print per seed the lowest and highest moving success over 5 and over
        /// 8 sessions against the band 65–92 % (/LF210/, Fragenpaket 10).
        #[arg(long)]
        seeds: Option<u64>,
    },
    /// Print each unit's content hash for its native speaker review (INHALT-ENTWURF 3.3):
    /// the reviewer enters it as `review.hash` together with `by` and `date`.
    ReviewHash,
    /// Write a review sheet per unit (Markdown) for the native speaker review: texts, recorded
    /// answers, grammar, checklist and the hash to enter (INHALT-ENTWURF 3.3).
    ReviewSheet {
        /// Directory for the sheets.
        #[arg(long)]
        out: PathBuf,
        /// Only this unit.
        #[arg(long)]
        unit: Option<String>,
    },
    /// Print the traceability table (requirement → code → checks).
    Trace {
        /// Write it to spec/RUECKVERFOLGUNG.md instead.
        #[arg(long)]
        write: bool,
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
        Command::Days {
            persona,
            journey,
            seeds: Some(n),
        } => match print_seeds(&cli.root, &persona, &journey, n) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error:#}");
                ExitCode::FAILURE
            }
        },
        Command::Days {
            persona,
            journey,
            seeds: None,
        } => match print_days(&cli.root, &persona, &journey) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error:#}");
                ExitCode::FAILURE
            }
        },
        Command::ReviewSheet { out, unit } => {
            match review::write_sheets(&cli.root, &out, unit.as_deref()) {
                Ok(n) => {
                    println!("{n} review sheets in {}", out.display());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error:#}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::ReviewHash => match review_hashes(&cli.root) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error:#}");
                ExitCode::FAILURE
            }
        },
        Command::Trace { write } => match trace::table(&cli.root) {
            Ok(table) if write => match std::fs::write(cli.root.join(trace::FILE), table) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::FAILURE
                }
            },
            Ok(table) => {
                print!("{table}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error:#}");
                ExitCode::FAILURE
            }
        },
        Command::Run {
            persona,
            report,
            live,
        } => match run_all(&cli.root, persona.as_deref(), report.as_deref(), live) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(error) => {
                eprintln!("error: {error:#}");
                ExitCode::FAILURE
            }
        },
    }
}

fn run_all(
    root: &std::path::Path,
    persona: Option<&str>,
    report: Option<&std::path::Path>,
    live: bool,
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
        all.extend(run::run_persona(root, &id, live)?);
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

/// The success band over several seeds: one line per seed, then how many hold the band.
fn print_seeds(root: &std::path::Path, persona: &str, journey: &str, n: u64) -> anyhow::Result<()> {
    let p: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(format!("qa/personas/{persona}.json")),
    )?)?;
    let script = days::script_for(root, journey)?
        .ok_or_else(|| anyhow::anyhow!("{journey} has no day script"))?;
    let band = 0.65..=0.92;
    let mut held = [0usize; 2];
    println!("seed  sessions  window 5: min  max  band   window 8: min  max  band");
    for k in 0..n {
        let mut s = script.clone();
        s.seed = script.seed.wrapping_add(k);
        let run = days::simulate(root, &p, &s)?;
        let mut cols = Vec::new();
        for (i, w) in [5usize, 8].into_iter().enumerate() {
            let m = days::moving(&run.sessions, w);
            let lo = m.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = m.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let ok = !m.is_empty() && band.contains(&lo) && band.contains(&hi);
            held[i] += usize::from(ok);
            cols.push(format!(
                "{lo:>5.3}  {hi:>5.3}  {}",
                if ok { "ok  " } else { "out " }
            ));
        }
        println!(
            "{:>4}  {:>8}       {}          {}",
            s.seed,
            run.sessions.len(),
            cols[0],
            cols[1]
        );
    }
    println!(
        "in the band: window 5 {} of {n}, window 8 {} of {n}",
        held[0], held[1]
    );
    Ok(())
}

fn print_days(root: &std::path::Path, persona: &str, journey: &str) -> anyhow::Result<()> {
    let p: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(format!("qa/personas/{persona}.json")),
    )?)?;
    let script = days::script_for(root, journey)?
        .ok_or_else(|| anyhow::anyhow!("{journey} has no day script"))?;
    let run = days::simulate(root, &p, &script)?;
    println!("day  date        start  min  scenario             success  level  reviews  xp");
    for s in &run.sessions {
        let secs = s.start.second_of_day();
        println!(
            "{:>3}  {}  {:02}:{:02}  {:>4.1}  {:<20} {:>6.2}  {:>5}  {:>3}{}  {:>4}",
            s.sim_day,
            s.start.day().iso(),
            secs / 3600,
            secs % 3600 / 60,
            s.minutes,
            s.scenario,
            s.success,
            s.level,
            s.reviews,
            if s.refresher { "R" } else { " " },
            s.xp
        );
    }
    let st = kaiwa_core::progress::streak(&run.records, run.goal, &[], run.last_day);
    println!(
        "streak {} (longest {}), freezes {}, ended {}; items {}; units done {}",
        st.days,
        st.longest,
        st.freezes_used.len(),
        st.ended_on.len(),
        run.items.len(),
        run.curriculum
            .path(&run.course, "de")
            .iter()
            .filter(|u| u.state == kaiwa_core::curriculum::NodeState::Done)
            .count()
    );
    for (skill, words) in run.learner.describe("de") {
        println!("  {skill:?}: {words}");
    }
    println!("errors (corrected, reviewed while open): {:?}", run.errors);
    Ok(())
}

/// One line per unit: id, content hash, and whether a recorded review still matches.
fn review_hashes(root: &std::path::Path) -> anyhow::Result<()> {
    use kaiwa_core::curriculum::Curriculum;
    let c = Curriculum::from_dir(&root.join("data/curriculum"))?;
    for u in &c.units {
        let texts = u
            .scenarios
            .iter()
            .map(|s| std::fs::read_to_string(root.join(format!("data/scenarios/{s}.json"))))
            .collect::<Result<Vec<_>, _>>()?;
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let hash = Curriculum::content_hash(u, &refs);
        let state = match u.review.as_ref().and_then(|r| r.hash.as_deref()) {
            None => "not reviewed",
            Some(h) if h == hash => "reviewed",
            Some(_) => "changed since the review",
        };
        println!("{}\t{hash}\t{state}", u.id);
    }
    Ok(())
}
