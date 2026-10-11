//! Checks on what the app shows, without a screen: the English UI has no German left
//! (/LL506/), recordings, gamification and accessible names. They run on the backend with the
//! demo profile and on the app's QML.

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
        "lena_retention" => retention(),
        "hinnerk_gamification_off" => gamification_off(root),
        "hinnerk_a11y_names" => a11y_names(root),
        "theo_keyboard" => keyboard(root),
        "lena_download_announced" => download_announced(root),
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
    for dir in ["app/qml"] {
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
    for dir in ["app/qml"] {
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

/// The blocks `Type { … }` of a QML file: (line, type, body).
fn blocks<'a>(src: &'a str, types: &[&str]) -> Vec<(usize, String, &'a str)> {
    let mut out = Vec::new();
    for t in types {
        let open = format!("{t} {{");
        let mut from = 0;
        while let Some(i) = src[from..].find(&open) {
            let start = from + i;
            // A whole word: not the tail of a longer type name.
            let before = src[..start].chars().last();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '.') {
                from = start + open.len();
                continue;
            }
            let body_start = start + open.len();
            let mut depth = 1;
            let mut end = body_start;
            for (j, c) in src[body_start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    end = body_start + j;
                    break;
                }
            }
            let line = src[..start].lines().count() + 1;
            out.push((line, (*t).to_string(), &src[body_start..end]));
            from = body_start;
        }
    }
    out
}

/// /LL201/ (static part; the AT-SPI tree follows with the gui mode): every control of the
/// app has a name for assistive technology:
/// a visible text, or an Accessible.name (text fields always need the latter). The Kante
/// controls give the role; KanteSegmented names each segment from its model.
/// /LL401/, F-08 from the source: the settings name source, licence and size of each model
/// before its download button, the download starts only from that button, and no other code
/// of the app downloads anything.
fn download_announced(root: &Path) -> Result<Option<String>> {
    let page = std::fs::read_to_string(root.join("app/qml/SettingsPage.qml"))?;
    let mut found = Vec::new();
    for field in [
        "modelData.origin",
        "modelData.licence",
        "modelData.downloadMB",
    ] {
        if !page.contains(field) {
            found.push(format!("the model row does not show {field}"));
        }
    }
    let calls: Vec<&str> = page
        .lines()
        .filter(|l| l.contains("downloadModel("))
        .collect();
    if calls.is_empty()
        || calls
            .iter()
            .any(|l| !l.trim_start().starts_with("onClicked:"))
    {
        found.push(format!("downloadModel outside a button: {calls:?}"));
    }
    let mut callers = Vec::new();
    for dir in ["app/src", "app/qml"] {
        for e in std::fs::read_dir(root.join(dir))? {
            let path = e?.path();
            let src = std::fs::read_to_string(&path).unwrap_or_default();
            if src.contains("download::download(") || src.contains("ureq::") {
                callers.push(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                );
            }
        }
    }
    if callers != ["app.rs"] {
        found.push(format!("downloads from {callers:?}, expected only app.rs"));
    }
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

/// /LL200/ from the source: the app's pages have no mouse-only handlers and no control taken out
/// of the tab order, and every interactive Kante control they use draws its focus. Walking a
/// journey by keyboard needs the GUI mode.
fn keyboard(root: &Path) -> Result<Option<String>> {
    const MOUSE_ONLY: &[&str] = &[
        "MouseArea",
        "TapHandler",
        "focusPolicy: Qt.NoFocus",
        "activeFocusOnTab: false",
    ];
    const INTERACTIVE: &[&str] = &[
        "KanteButton",
        "KanteSegmented",
        "KanteTalkButton",
        "KanteTextField",
        "KanteComboBox",
        "KanteTabBar",
        "KantePath",
        "KanteCheckSkin",
        "KanteChip",
    ];
    let mut found = Vec::new();
    let mut used = std::collections::BTreeSet::new();
    for e in std::fs::read_dir(root.join("app/qml"))? {
        let path = e?.path();
        if path.extension().is_none_or(|x| x != "qml") {
            continue;
        }
        let src = std::fs::read_to_string(&path)?;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for (i, line) in src.lines().enumerate() {
            if let Some(m) = MOUSE_ONLY.iter().find(|m| line.contains(*m)) {
                found.push(format!("{name}:{} {m}", i + 1));
            }
        }
        for t in INTERACTIVE {
            if src.contains(&format!("{t} {{")) {
                used.insert(*t);
            }
        }
    }
    // A control draws its focus itself or through a Kante skin it uses (KanteFieldSkin).
    let kante = |t: &str| std::fs::read_to_string(root.join(format!("app/qml/Kante/{t}.qml")));
    let draws = |src: &str| src.contains("visualFocus") || src.contains("activeFocus");
    for t in used {
        let src = kante(t)?;
        let skins = src
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.starts_with("Kante") && w.ends_with("Skin"));
        let by_skin = skins.filter_map(|w| kante(w).ok()).any(|skin| draws(&skin));
        if !draws(&src) && !by_skin {
            found.push(format!("{t} draws no focus"));
        }
    }
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

fn a11y_names(root: &Path) -> Result<Option<String>> {
    const NAMED_BY_TEXT: &[&str] = &[
        "KanteButton",
        "KantePlasmaButton",
        "KantePlasmaToolButton",
        "KanteToolButton",
        "QQC2.Button",
    ];
    const FIELDS: &[&str] = &[
        "KanteTextField",
        "KanteComboBox",
        "QQC2.ComboBox",
        "QQC2.TextField",
    ];
    let mut found = Vec::new();
    for dir in ["app/qml"] {
        for e in std::fs::read_dir(root.join(dir))? {
            let path = e?.path();
            if path.extension().is_none_or(|x| x != "qml") {
                continue;
            }
            let src = std::fs::read_to_string(&path)?;
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            for (line, t, body) in blocks(&src, NAMED_BY_TEXT) {
                // Only the block's own properties, not those of nested items.
                let own: String = own_lines(body);
                if !own.contains("text:") && !own.contains("Accessible.name:") {
                    found.push(format!("{name}:{line} {t} without text"));
                }
            }
            for (line, t, body) in blocks(&src, FIELDS) {
                let own = own_lines(body);
                // Kirigami.FormData.label names a field in a form.
                if !own.contains("Accessible.name:") && !own.contains("FormData.label:") {
                    found.push(format!("{name}:{line} {t} without Accessible.name"));
                }
            }
        }
    }
    Ok((!found.is_empty()).then(|| found.join("; ")))
}

/// The lines of a block body at its own depth (nested blocks left out).
fn own_lines(body: &str) -> String {
    let mut depth = 0;
    let mut out = String::new();
    for l in body.lines() {
        if depth == 0 {
            out.push_str(l);
            out.push('\n');
        }
        depth += l.matches('{').count() as i32 - l.matches('}').count() as i32;
        depth = depth.max(0);
    }
    out
}
