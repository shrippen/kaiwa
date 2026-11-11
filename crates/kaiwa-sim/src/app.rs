//! Checks against the built app (U-03): `kaiwa-sim run --app <binary>` starts it offscreen and
//! looks at what it does. The binary is a release build with the feature `demo` (screenshot
//! mode, recorded answers); the CI's Qt job builds one and runs these. Without `--app` they are
//! listed as "app only".
//!
//!   hinnerk_narrow     every view at the narrowest window: no text cut off (/LL303/)
//!   hinnerk_font       every view at 160 % on a 1366 px laptop: no text cut off (/LL202/)
//!   lena_idle_memory   a fresh profile, idle: PSS at most 150 MB (/LL012/, U-01)
//!   lena_app_egress    a conversation under strace: no connection off the machine (/LL401/)
//!
//! The recorded tutor and no models: what the local tutor, the models and kaiwad send, and
//! whether the models are unloaded, stays in the local test (lena_zero_egress,
//! lena_models_unload), as do overlaps, furigana and a look at the pictures
//! (spec/LOKALER-TEST.md).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::run::Outcome;

/// The checks that need the built app.
pub const APP: &[&str] = &[
    "hinnerk_narrow",
    "hinnerk_font",
    "lena_idle_memory",
    "lena_app_egress",
];

/// /LL012/, measured as PSS (U-01): the app's share of the memory it maps, shared Qt libraries
/// split among the processes that use them. In kB as `/proc` counts (1024 bytes).
const IDLE_LIMIT_KB: u64 = 150 * 1024;
/// How long the app idles before the memory is read.
const IDLE: Duration = Duration::from_secs(10);
/// A run of the app in screenshot mode takes a few seconds; this ends a hung one.
const LIMIT: Duration = Duration::from_secs(120);

/// The large text of hinnerk_font (/LL202/) and the small laptop it is checked on.
const LARGE_TEXT: f64 = 1.6;
const LAPTOP: f64 = 1366.0;
/// Asked of the app for hinnerk_narrow: below its minimum, which it must keep.
const TOO_NARROW: u32 = 400;
/// Tall enough that a page shows all of itself.
const HEIGHT: &str = "1500";

const VIEWS: &[&str] = &["start", "path", "review", "profile", "settings"];
const TALK: &[&str] = &[
    "--view",
    "talk",
    "--scenario",
    "n5-02-jikoshoukai",
    "--say",
    "?|マラです。よろしくおねがいします。|ビール二ください。|コーヒーを好きです。",
];

/// `Some` for a check of [`APP`]: run without an app, it is listed as app only.
pub fn pending(id: &str) -> Option<Outcome> {
    APP.contains(&id).then_some(Outcome::AppOnly)
}

/// Runs one check of [`APP`] against the app at `bin`.
pub fn check(id: &str, root: &Path, bin: &Path) -> Outcome {
    let tmp = std::env::temp_dir().join(format!("kaiwa-sim-app-{}-{id}", std::process::id()));
    let r = fs::create_dir_all(tmp.join("models"))
        .map_err(anyhow::Error::from)
        .and_then(|()| match id {
            "hinnerk_narrow" => narrow(root, bin, &tmp),
            "hinnerk_font" => font(bin, &tmp),
            "lena_idle_memory" => memory(bin, &tmp),
            "lena_app_egress" => egress(bin, &tmp),
            _ => Ok(Some(format!("no app check {id}"))),
        });
    let _ = fs::remove_dir_all(&tmp);
    match r {
        Ok(None) => Outcome::Pass,
        Ok(Some(why)) => Outcome::Fail(why),
        Err(e) => Outcome::Fail(format!("{e:#}")),
    }
}

/// hinnerk_narrow: asked for 400 px, every view opens at the narrowest width `Main.qml` allows
/// and every text fits.
fn narrow(root: &Path, bin: &Path, tmp: &Path) -> Result<Option<String>> {
    let main = fs::read_to_string(root.join("app/qml/Main.qml"))?;
    let narrowest: u32 = regex::Regex::new(r"readonly property int narrowest: (\d+)")?
        .captures(&main)
        .and_then(|c| c[1].parse().ok())
        .context("no narrowest in Main.qml")?;
    texts(bin, tmp, 1.0, TOO_NARROW, Some(narrowest))
}

/// hinnerk_font: at 160 % on a 1366 px laptop the window has 853 px (as `tools/font-160.sh`).
fn font(bin: &Path, tmp: &Path) -> Result<Option<String>> {
    let width = (LAPTOP / LARGE_TEXT) as u32;
    texts(bin, tmp, LARGE_TEXT, width, None)
}

/// Every tab and a conversation, German and English, in screenshot mode with `--check-text`:
/// the app logs each text that is elided, wider than its place or beyond the window. With
/// `want`, the picture must be that wide.
fn texts(
    bin: &Path,
    tmp: &Path,
    scale: f64,
    width: u32,
    want: Option<u32>,
) -> Result<Option<String>> {
    let mut found = Vec::new();
    for lang in ["de", "en"] {
        let mut runs: Vec<(String, Vec<&str>)> = VIEWS
            .iter()
            .map(|v| (format!("{v}-{lang}"), vec!["--view", v]))
            .collect();
        runs.push((format!("talk-{lang}"), TALK.to_vec()));

        for (name, view) in runs {
            let png = tmp.join(format!("{name}.png"));
            let mut cmd = app(bin, tmp);
            cmd.env("KAIWA_DEMO", "1")
                .env("KAIWA_LANG", lang)
                .env("QT_SCALE_FACTOR", scale.to_string())
                .arg("--shot")
                .arg(&png)
                .args(["--check-text", "--kind", "1", "--height", HEIGHT])
                .args(["--width", &width.to_string()])
                .args(view);
            let log = finish(cmd, &tmp.join(format!("{name}.log")))?;

            if !log.contains("kaiwa-text-checked:") {
                found.push(format!("{name}: no text check"));
                continue;
            }
            found.extend(log.lines().filter_map(|l| {
                l.split_once("kaiwa-text: ")
                    .map(|(_, t)| format!("{name}: {t}"))
            }));
            let got = png_width(&png)?;
            if let Some(w) = want.filter(|w| *w != got) {
                found.push(format!("{name}: {got} px wide, not {w}"));
            }
        }
    }
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

/// lena_idle_memory: a fresh profile, no models, the window open and idle. Beside it runs a
/// plain Kirigami window ([`DESKTOP`]): on a Plasma desktop Kaiwa shares Qt and Kirigami with
/// other programs, and PSS splits shared pages among them. Without it, in the CI's container,
/// Kaiwa would carry all of Qt alone.
fn memory(bin: &Path, tmp: &Path) -> Result<Option<String>> {
    let desktop = tmp.join("desktop.qml");
    fs::write(&desktop, DESKTOP)?;
    let mut other = Command::new(qt_bin("qml")?);
    offscreen(&mut other, tmp);
    let mut other = other
        .arg(&desktop)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("starting qml")?;

    let mut cmd = app(bin, tmp);
    cmd.env("KAIWA_PROFILE", tmp.join("fresh/profile.kaiwa"));
    let log = fs::File::create(tmp.join("memory.log"))?;
    let child = cmd.stdout(log.try_clone()?).stderr(log).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            stop(&mut other);
            return Err(anyhow::Error::from(e).context("starting the app"));
        }
    };

    std::thread::sleep(IDLE);
    let pss = match child.try_wait()? {
        Some(status) => Err(anyhow::anyhow!("the app ended while idle: {status}")),
        None => pss_kb(child.id()),
    };
    stop(&mut child);
    stop(&mut other);

    let pss = pss?;
    Ok((pss > IDLE_LIMIT_KB).then(|| {
        format!(
            "{} MB PSS idle, at most {}",
            pss / 1024,
            IDLE_LIMIT_KB / 1024
        )
    }))
}

/// Another Kirigami program on the desktop, for [`memory`].
const DESKTOP: &str = "import QtQuick\nimport org.kde.kirigami as Kirigami\n\
Kirigami.ApplicationWindow { visible: true; pageStack.initialPage: Kirigami.Page {} }\n";

/// A Qt tool by name, from where `qmake6` (or `$QMAKE`) says Qt keeps them.
fn qt_bin(name: &str) -> Result<PathBuf> {
    let qmake = std::env::var("QMAKE").unwrap_or_else(|_| "qmake6".into());
    let out = Command::new(&qmake)
        .args(["-query", "QT_HOST_BINS"])
        .output()
        .with_context(|| format!("{qmake} -query"))?;
    Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim()).join(name))
}

/// lena_app_egress: a conversation with the recorded tutor under strace; every connection
/// and every datagram with an address must stay on the machine.
fn egress(bin: &Path, tmp: &Path) -> Result<Option<String>> {
    let trace = tmp.join("egress.strace");
    let mut cmd = Command::new("strace");
    offscreen(&mut cmd, tmp);
    cmd.env("KAIWA_DEMO", "1")
        .args(["-f", "-qq", "-e", "trace=connect,sendto,sendmsg", "-o"])
        .arg(&trace)
        .arg(bin)
        .arg("--shot")
        .arg(tmp.join("talk.png"))
        .args(TALK);
    let log =
        finish(cmd, &tmp.join("egress.log")).context("strace (installed? ptrace allowed?)")?;

    if !tmp.join("talk.png").exists() {
        return Ok(Some(format!(
            "the conversation did not finish: {}",
            tail(&log)
        )));
    }
    let off: Vec<String> = fs::read_to_string(&trace)?
        .lines()
        .filter(|l| leaves(l))
        .map(|l| l.chars().take(160).collect())
        .collect();
    Ok((!off.is_empty()).then(|| off.join("; ")))
}

/// Does this strace line send to an address off the machine, or ask a name server?
/// `connect(7, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("1.2.3.4")}, 16)`
fn leaves(line: &str) -> bool {
    if line.contains("htons(53)") {
        return true;
    }
    let addr = |marker: &str| {
        line.split_once(marker)
            .and_then(|(_, rest)| rest.split('"').next())
            .map(str::to_string)
    };
    if line.contains("sa_family=AF_INET6") {
        let a = addr("inet_pton(AF_INET6, \"").unwrap_or_default();
        return !(a == "::1" || a.starts_with("::ffff:127."));
    }
    if line.contains("sa_family=AF_INET,") {
        let a = addr("inet_addr(\"").unwrap_or_default();
        return !a.starts_with("127.");
    }
    false
}

/// The app at `bin`, see [`offscreen`].
fn app(bin: &Path, tmp: &Path) -> Command {
    let mut cmd = Command::new(bin);
    offscreen(&mut cmd, tmp);
    cmd
}

/// Offscreen, logging to stderr, with no speech models and none of the developer's `KAIWA_*`
/// settings (a tutor, a profile).
fn offscreen(cmd: &mut Command, tmp: &Path) {
    for (k, _) in std::env::vars_os() {
        if k.to_string_lossy().starts_with("KAIWA_") {
            cmd.env_remove(k);
        }
    }
    cmd.env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("KAIWA_MODELS", tmp.join("models"))
        .stdin(Stdio::null());
}

/// Runs `cmd` to its end, at most [`LIMIT`], and returns what it logged.
fn finish(mut cmd: Command, log: &PathBuf) -> Result<String> {
    let file = fs::File::create(log)?;
    let mut child = cmd.stdout(file.try_clone()?).stderr(file).spawn()?;
    let start = Instant::now();
    while child.try_wait()?.is_none() {
        if start.elapsed() > LIMIT {
            stop(&mut child);
            anyhow::bail!("no end after {} s", LIMIT.as_secs());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(fs::read_to_string(log)?)
}

fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn pss_kb(pid: u32) -> Result<u64> {
    let rollup = fs::read_to_string(format!("/proc/{pid}/smaps_rollup"))?;
    rollup
        .lines()
        .find_map(|l| l.strip_prefix("Pss:"))
        .and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
        .context("no Pss in smaps_rollup")
}

/// The width in a PNG's header (bytes 16 to 19, big-endian).
fn png_width(png: &Path) -> Result<u32> {
    let bytes = fs::read(png).with_context(|| format!("no picture {}", png.display()))?;
    let w = bytes.get(16..20).context("not a PNG")?;
    Ok(u32::from_be_bytes([w[0], w[1], w[2], w[3]]))
}

fn tail(log: &str) -> String {
    let lines: Vec<&str> = log.lines().collect();
    lines[lines.len().saturating_sub(3)..].join(" / ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_addresses_off_the_machine_leave() {
        let off = [
            r#"123 connect(7, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("140.82.1.1")}, 16) = 0"#,
            r#"123 connect(7, {sa_family=AF_INET6, sin6_port=htons(443), sin6_flowinfo=htonl(0), inet_pton(AF_INET6, "2a00::1", &sin6_addr), sin6_scope_id=0}, 28) = 0"#,
            r#"123 sendto(5, "x", 1, 0, {sa_family=AF_INET, sin_port=htons(53), sin_addr=inet_addr("127.0.0.53")}, 16) = 1"#,
        ];
        let local = [
            r#"123 connect(3, {sa_family=AF_UNIX, sun_path="/run/user/1000/bus"}, 110) = 0"#,
            r#"123 connect(7, {sa_family=AF_INET, sin_port=htons(11434), sin_addr=inet_addr("127.0.0.1")}, 16) = 0"#,
            r#"123 connect(7, {sa_family=AF_INET6, sin6_port=htons(11434), sin6_flowinfo=htonl(0), inet_pton(AF_INET6, "::1", &sin6_addr), sin6_scope_id=0}, 28) = 0"#,
            r#"123 sendmsg(4, {msg_name=NULL, msg_namelen=0, msg_iov=[]}, 0) = 8"#,
        ];
        for l in off {
            assert!(leaves(l), "{l}");
        }
        for l in local {
            assert!(!leaves(l), "{l}");
        }
    }

    #[test]
    fn every_app_check_is_named_by_a_persona() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut all = String::new();
        for f in crate::validate::persona_files(&root.join("qa/personas")).unwrap() {
            all.push_str(&fs::read_to_string(f).unwrap());
        }
        for id in APP {
            assert!(all.contains(&format!("\"id\": \"{id}\"")), "{id}");
        }
    }
}
