//! Persona checks that run as QML tests (`app/qml/tests`, `tools/qml-tests.sh`): the real
//! pages on stand-ins for the app objects, offscreen. A check holds when what the page does
//! is right, not when a word stands in its source. Without Qt 6 (`qmltestrunner`) such a
//! check is listed as "app only", as the checks of [`crate::app`]; the CI's Qt job runs them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::run::Outcome;

/// The QML test files each check runs, besides what the check tries on the backend.
const TESTS: &[(&str, &[&str])] = &[
    ("mara_ptt", &["tst_space.qml"]),
    ("hinnerk_targets", &["tst_space.qml"]),
    ("mara_listen_tools", &["tst_aids.qml"]),
    ("mara_diff_audio", &["tst_hint.qml"]),
    ("mara_word_lookup", &["tst_lookup.qml"]),
    ("hinnerk_tts_slow", &["tst_sound.qml"]),
    ("lena_suggest_smaller", &["tst_latency.qml"]),
    ("lena_download_announced", &["tst_models.qml"]),
];

/// A test file ends or hangs: the longest it may take.
const LIMIT: Duration = Duration::from_secs(120);
const TESTS_DIR: &str = "app/qml/tests";
const RUNNER_DEFAULT: &str = "/usr/lib/qt6/bin/qmltestrunner";
const RUNNER_ON_PATH: &[&str] = &["qmltestrunner6", "qmltestrunner-qt6"];

/// Whether `id` runs QML tests.
pub fn uses(id: &str) -> bool {
    TESTS.iter().any(|(t, _)| *t == id)
}

/// The outcome of check `id` after the QML tests: `found` is what the check found on the
/// backend (`None` if it has no part there). A check of the app, or one not automated yet,
/// is listed as such.
pub fn finish(id: &str, root: &Path, found: Option<Outcome>) -> Outcome {
    add(id, root, found)
        .or_else(|| crate::app::pending(id))
        .unwrap_or(Outcome::NotAutomated)
}

/// Adds the QML tests of check `id` to what the check found on the backend: a failure of
/// either fails it, without Qt it is app only.
fn add(id: &str, root: &Path, found: Option<Outcome>) -> Option<Outcome> {
    let Some((_, tests)) = TESTS.iter().find(|(t, _)| *t == id) else {
        return found;
    };
    if let Some(Outcome::Fail(_)) = found {
        return found;
    }
    let Some(runner) = runner() else {
        return Some(Outcome::AppOnly);
    };
    for test in *tests {
        if let Err(why) = run(&runner, root, test) {
            return Some(Outcome::Fail(format!("{test}: {why:#}")));
        }
    }
    Some(Outcome::Pass)
}

/// The Qt 6 runner: `$QMLTESTRUNNER`, Qt's own folder, or a suffixed name on the path (not
/// `qmltestrunner` from the path: where Qt 5 is installed too, that is Qt 5's).
fn runner() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("QMLTESTRUNNER") {
        return Some(path.into());
    }
    if Path::new(RUNNER_DEFAULT).is_file() {
        return Some(RUNNER_DEFAULT.into());
    }
    let dirs = std::env::split_paths(&std::env::var_os("PATH")?).collect::<Vec<_>>();
    RUNNER_ON_PATH
        .iter()
        .flat_map(|name| dirs.iter().map(move |d| d.join(name)))
        .find(|p| p.is_file())
}

/// Runs one test file; the error names the failed tests.
fn run(runner: &Path, root: &Path, test: &str) -> Result<()> {
    let log = std::env::temp_dir().join(format!("kaiwa-sim-qml-{}-{test}.log", std::process::id()));
    let file = fs::File::create(&log)?;
    let mut child = Command::new(runner)
        .current_dir(root)
        .args(["-input", &format!("{TESTS_DIR}/{test}")])
        .args([
            "-import",
            "app/qml",
            "-import",
            &format!("{TESTS_DIR}/stub"),
        ])
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .stdin(Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(file)
        .spawn()
        .with_context(|| runner.display().to_string())?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("no end after {} s", LIMIT.as_secs());
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let text = fs::read_to_string(&log).unwrap_or_default();
    let _ = fs::remove_file(&log);
    if status.success() {
        return Ok(());
    }
    let failed: Vec<&str> = text.lines().filter(|l| l.starts_with("FAIL!")).collect();
    anyhow::bail!("{}", failed.join(" / "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_test_file_exists_and_is_named_by_a_persona() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut all = String::new();
        for f in crate::validate::persona_files(&root.join("qa/personas")).unwrap() {
            all.push_str(&fs::read_to_string(f).unwrap());
        }
        for (id, tests) in TESTS {
            assert!(all.contains(&format!("\"id\": \"{id}\"")), "{id}");
            for t in *tests {
                assert!(root.join(TESTS_DIR).join(t).is_file(), "{t}");
            }
        }
    }
}
