//! Checks on what the app and the plasmoid show, without a screen: the English UI has no
//! German left (/LL506/), the plasmoid's short talk and its five reviews (/LF501/, /LF502/).
//! They run on the backend the app and `kaiwad` share, with the demo profile.

use std::path::Path;

use anyhow::Result;
use kaiwa_backend::short::ShortTalk;
use kaiwa_backend::{Backend, views};
use kaiwa_core::clock::Moment;
use serde_json::Value;

use crate::run::Outcome;

pub fn check(id: &str, root: &Path) -> Option<Outcome> {
    let r = match id {
        "theo_ui_en" => ui_en(root),
        "mara_plasmoid_short" => short(),
        "mara_plasmoid_reviews" => reviews(),
        "lena_retention" => retention(),
        "hinnerk_gamification_off" => gamification_off(root),
        _ => return None,
    };
    Some(match r {
        Ok(None) => Outcome::Pass,
        Ok(Some(why)) => Outcome::Fail(why),
        Err(e) => Outcome::Fail(format!("{e:#}")),
    })
}

const NOW: Moment = Moment {
    unix: 1_791_100_000,
    offset_min: 120,
};

fn demo(name: &str, lang: &str) -> Result<Backend> {
    let dir = std::env::temp_dir().join(format!("kaiwa-sim-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let b = Backend::open_at(dir.join("demo.kaiwa"), true)?;
    let mut p = b.store.profile()?;
    p.ui_lang = lang.into();
    b.store.set_profile(&p)?;
    Ok(b)
}

/// German in a text meant for English: umlauts, ß or common German words.
fn german(s: &str) -> bool {
    const WORDS: &str = "und der die das den dem nicht mit ist ein eine für du dich dein deine \
        noch zum beim oder wird auf aber ohne sich wenn nur auch bei von im ich wir spricht \
        umstritten erledigt gesperrt offen jetzt weiter zurück fertig";
    s.chars().any(|c| "äöüÄÖÜß".contains(c))
        || s.split(|c: char| !c.is_alphabetic())
            .any(|w| WORDS.split_whitespace().any(|x| x == w))
}

fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "path" | "file" | "id" | "unit"))
            .for_each(|(_, x)| strings(x, out)),
        _ => {}
    }
}

/// The string literals of a QML file: the English halves of `t(de, en)` calls and the
/// literals outside any `t(…)`. A small lexer: comments skipped, calls tracked with their
/// argument index, so concatenations and calls over several lines count too.
fn qml_texts(src: &str) -> (Vec<String>, Vec<String>) {
    let mut english = Vec::new();
    let mut bare = Vec::new();
    // (call name, argument index) per open parenthesis.
    let mut calls: Vec<(String, usize)> = Vec::new();
    let mut ident = String::new();
    let c: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && c.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < c.len() && !(c[i] == '*' && c[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        match ch {
            '"' => {
                let mut lit = String::new();
                i += 1;
                while i < c.len() && c[i] != '"' {
                    if c[i] == '\\' {
                        i += 1;
                    }
                    if let Some(x) = c.get(i) {
                        lit.push(*x);
                    }
                    i += 1;
                }
                match calls.iter().rev().find(|(n, _)| n == "t") {
                    Some((_, 1)) => english.push(lit),
                    Some(_) => {}
                    None if lit.chars().any(char::is_alphabetic) => bare.push(lit),
                    None => {}
                }
                ident.clear();
            }
            '(' => {
                calls.push((std::mem::take(&mut ident), 0));
            }
            ')' => {
                calls.pop();
                ident.clear();
            }
            ',' => {
                if let Some(top) = calls.last_mut() {
                    top.1 += 1;
                }
                ident.clear();
            }
            x if x.is_alphanumeric() || x == '_' => ident.push(x),
            '.' => ident.clear(),
            x if x.is_whitespace() => {}
            _ => ident.clear(),
        }
        i += 1;
    }
    // import lines and the like are not shown.
    bare.retain(|s| !s.starts_with("qrc:") && !s.contains('/') || s.contains(' '));
    (english, bare)
}

/// /LL506/: with the UI in English no German shows: the English half of every t(de, en), no
/// German literal outside t(), and the content of every view of an English profile.
fn ui_en(root: &Path) -> Result<Option<String>> {
    let mut found = Vec::new();
    for dir in ["app/qml", "plasmoid/package/contents/ui"] {
        for e in std::fs::read_dir(root.join(dir))? {
            let p = e?.path();
            if p.extension().is_none_or(|x| x != "qml") {
                continue;
            }
            let (english, bare) = qml_texts(&std::fs::read_to_string(&p)?);
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            found.extend(
                english
                    .into_iter()
                    .filter(|s| german(s))
                    .map(|s| format!("{name}: t(…, \"{s}\")")),
            );
            found.extend(
                bare.into_iter()
                    .filter(|s| german(s))
                    .map(|s| format!("{name}: \"{s}\" without t()")),
            );
        }
    }
    let b = demo("en", "en")?;
    let mut texts = Vec::new();
    for v in [
        views::today(&b, NOW)?,
        views::path(&b)?,
        views::review(&b, NOW, 10)?,
        views::learner(&b)?,
        views::stats(&b, NOW)?,
        views::profile(&b)?,
        views::privacy("en"),
    ] {
        strings(&v, &mut texts);
    }
    found.extend(
        texts
            .into_iter()
            .filter(|s| german(s))
            .map(|s| format!("view: {s}")),
    );
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

/// /LF501/: the short talk has at most 6 goals (two from the path) and ends by itself with a
/// mini summary once they are reached.
fn short() -> Result<Option<String>> {
    let b = demo("short", "de")?;
    let mut t = ShortTalk::start(&b, NOW)?;
    let goals = t.session.goals().to_vec();
    if goals.is_empty() || goals.len() > 6 {
        return Ok(Some(format!("{} goals", goals.len())));
    }
    let mut last = Value::Null;
    for g in &goals {
        last = t.say(&b, &g.help.example, false, NOW)?;
    }
    if last["done"] != true {
        return Ok(Some(format!("not done after the goals' examples: {last}")));
    }
    Ok(last["recorded"]
        .is_null()
        .then(|| "no mini summary at the end".into()))
}

/// /LF502/: "five reviews" offers exactly five due items when at least five are due.
fn reviews() -> Result<Option<String>> {
    let b = demo("reviews", "de")?;
    let due = b.store.today(NOW)?.due;
    let v = views::review(&b, NOW, 5)?;
    let n = v.as_array().map(Vec::len).unwrap_or(0);
    let want = due.min(5);
    Ok((n != want).then(|| format!("{n} offered, {due} due")))
}

/// /LF712/: with recordings kept for 7 days, after 8 days they are gone and the transcripts
/// (the sessions' turns) stay.
fn retention() -> Result<Option<String>> {
    let b = demo("retention", "de")?;
    let mut p = b.store.profile()?;
    p.keep_recordings_days = Some(7);
    b.store.set_profile(&p)?;
    let sessions = b.store.finished_since(0)?;
    let turns_before: usize = sessions
        .iter()
        .map(|r| {
            b.store
                .session_state(r.id)
                .map(|s| s.turns.len())
                .unwrap_or(0)
        })
        .sum();
    for (i, r) in sessions.iter().take(3).enumerate() {
        b.keep_recording(r.id, i, &[0.05; 8000], 16_000, NOW)?;
    }
    let files = |b: &Backend| {
        std::fs::read_dir(b.recordings_dir())
            .map(|d| d.count())
            .unwrap_or(0)
    };
    if files(&b) != 3 {
        return Ok(Some(format!("{} recordings kept, 3 expected", files(&b))));
    }
    b.purge_recordings(NOW.plus_secs(8 * 86_400))?;
    let turns_after: usize = b
        .store
        .finished_since(0)?
        .iter()
        .map(|r| {
            b.store
                .session_state(r.id)
                .map(|s| s.turns.len())
                .unwrap_or(0)
        })
        .sum();
    if files(&b) != 0 {
        return Ok(Some(format!("{} recordings left after 8 days", files(&b))));
    }
    Ok((turns_after != turns_before || turns_before == 0)
        .then(|| format!("transcripts {turns_before} → {turns_after}")))
}

/// /LF605/: with gamification off nothing shows XP, the streak or the goal meter: the views
/// carry no streak note, a short talk ends without XP, and in the QML every XP, streak and
/// meter display sits behind the gamification setting.
fn gamification_off(root: &Path) -> Result<Option<String>> {
    let b = demo("gamification", "de")?;
    let mut p = b.store.profile()?;
    p.gamification = false;
    b.store.set_profile(&p)?;
    let mut found = Vec::new();
    let today = views::today(&b, NOW)?;
    if today["streakNote"].as_str().is_some_and(|s| !s.is_empty()) {
        found.push(format!("streak note: {}", today["streakNote"]));
    }
    let mut t = ShortTalk::start(&b, NOW)?;
    let goals = t.session.goals().to_vec();
    let mut last = Value::Null;
    for g in &goals {
        last = t.say(&b, &g.help.example, false, NOW)?;
    }
    if last["recorded"].get("xp").is_some() {
        found.push("XP after a short talk".into());
    }
    for dir in ["app/qml", "plasmoid/package/contents/ui"] {
        for e in std::fs::read_dir(root.join(dir))? {
            let path = e?.path();
            if path.extension().is_none_or(|x| x != "qml") {
                continue;
            }
            let src = std::fs::read_to_string(&path)?;
            let lines: Vec<&str> = src.lines().collect();
            for (i, l) in lines.iter().enumerate() {
                let shows = l.contains("label: \"XP\"")
                    || l.contains("KanteGoalMeter {")
                    || (l.contains("streakDays") && !l.trim_start().starts_with("//"))
                    || l.contains(" XP");
                if !shows {
                    continue;
                }
                let near = &lines[i.saturating_sub(8)..(i + 8).min(lines.len())];
                if !near
                    .iter()
                    .any(|x| x.contains("gamification") || x.contains(".meter"))
                {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    found.push(format!("{name}:{}: {}", i + 1, l.trim()));
                }
            }
        }
    }
    Ok((!found.is_empty()).then(|| found.join("; ")))
}
