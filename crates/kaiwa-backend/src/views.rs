//! What the app's pages show, as JSON for QML: built from the profile with plain functions,
//! so the content of every view is testable without Qt.

use std::collections::BTreeMap;

use kaiwa_core::clock::Moment;
use kaiwa_core::curriculum::NodeState;
use kaiwa_core::difficulty::KNOBS;
use kaiwa_core::learner::SKILLS;
use kaiwa_core::progress::{DailyGoal, streak_note};
use kaiwa_store::Script;
use serde_json::{Value, json};

use crate::Backend;

fn scenario_title(st: &Backend, id: &str, lang: &str) -> String {
    st.scenarios
        .get(id)
        .map(|s| s.title.get(lang).to_string())
        .unwrap_or_else(|| id.to_string())
}

fn kind_word(kind: &str, de: bool) -> &'static str {
    match (kind, de) {
        ("particle", true) => "Partikeln",
        ("particle", false) => "Particles",
        ("counter", true) => "Zählwörter",
        ("counter", false) => "Counters",
        ("tense", true) => "Zeitformen",
        ("tense", false) => "Tenses",
        ("register", true) => "Höflichkeit",
        ("register", false) => "Politeness",
        ("word_order", true) => "Wortstellung",
        ("word_order", false) => "Word order",
        ("conjugation", true) => "Konjugation",
        ("conjugation", false) => "Conjugation",
        ("word_choice", true) => "Wortwahl",
        ("word_choice", false) => "Word choice",
        (_, true) => "Sonstiges",
        (_, false) => "Other",
    }
}

/// The start page (Lastenheft Kap. 7: today, daily goal, next scenario, review).
pub fn today(st: &Backend, now: Moment) -> anyhow::Result<Value> {
    let p = st.store.profile()?;
    let lang = p.ui_lang.as_str();
    let de = lang == "de";
    let t = st.store.today(now)?;
    let learning = st.store.learning()?;
    let next = st.curriculum.next_scenario(&learning.course).map(|id| {
        let unit = st.curriculum.unit_of(&id);
        json!({
            "id": id,
            "title": scenario_title(st, &id, lang),
            "unit": unit.map(|u| u.title.get(lang)).unwrap_or_default(),
            "canDo": unit.map(|u| u.can_do.get(lang)).unwrap_or_default(),
        })
    });
    let open = t
        .open_session
        .as_ref()
        .map(|r| json!({ "id": r.id, "title": scenario_title(st, &r.scenario, lang) }));
    let (goal_text, goal_value) = match p.daily_goal {
        DailyGoal::Minutes(m) => (
            if de {
                format!("{} von {m} Minuten gesprochen", t.record.speaking_secs / 60)
            } else {
                format!("{} of {m} minutes spoken", t.record.speaking_secs / 60)
            },
            m,
        ),
        DailyGoal::Scenarios(n) => (
            if de {
                format!("{} von {n} Gesprächen", t.record.scenarios_done)
            } else {
                format!("{} of {n} talks", t.record.scenarios_done)
            },
            n,
        ),
    };
    // Free talk (/LF116/): the topic scenarios outside the path, at the learner's difficulty.
    let free: Vec<Value> = st
        .scenarios
        .values()
        .filter(|s| s.unit == "FREI")
        .map(|s| {
            let topic = s.title.get(lang);
            let topic = topic.rsplit(": ").next().unwrap_or(topic);
            json!({ "id": s.id, "topic": topic })
        })
        .collect();
    Ok(json!({
        "free": free,
        "name": p.name,
        "gamification": p.gamification,
        "goalFraction": t.goal_fraction,
        "goalText": goal_text,
        "goalValue": goal_value,
        "goalMet": t.record.met(p.daily_goal),
        "streakDays": t.streak.days,
        "streakNote": if p.gamification { streak_note(&t.streak, t.day, lang) } else { String::new() },
        "xpToday": t.record.xp,
        "due": t.due,
        "refresher": t.refresher,
        "open": open,
        "next": next,
        "placed": p.placed,
        "lang": lang,
        "style": p.style,
    }))
}

/// The learning path (/LF600/): one node per scenario, units as meta, states with words.
pub fn path(st: &Backend) -> anyhow::Result<Value> {
    let p = st.store.profile()?;
    let lang = p.ui_lang.as_str();
    let learning = st.store.learning()?;
    let items = st.store.items()?;
    let today = crate::now().day();
    let mut nodes = Vec::new();
    for unit in st.curriculum.path(&learning.course, lang) {
        for sc in &unit.scenarios {
            let state = match sc.state {
                NodeState::Done => "done",
                NodeState::Current => "current",
                NodeState::Open => "open",
                NodeState::Locked => "locked",
            };
            let meta = match (&unit.locked_because, sc.state) {
                (Some(why), NodeState::Locked) => format!("{} · {}", unit.id.to_uppercase(), why),
                _ => format!("{} · {}", unit.id.to_uppercase(), unit.can_do),
            };
            nodes.push(json!({
                "kind": "scenario",
                "id": sc.id,
                "title": scenario_title(st, &sc.id, lang),
                "meta": meta,
                "state": state,
            }));
        }
        let due = items
            .iter()
            .filter(|i| i.unit.as_deref() == Some(unit.id.as_str()) && i.is_due(today))
            .count();
        if due > 0 && unit.state != NodeState::Locked {
            nodes.push(json!({
                "kind": "review",
                "id": unit.id,
                "title": if lang == "de" { format!("Wiederholung: {}", unit.title) } else { format!("Review: {}", unit.title) },
                "meta": if lang == "de" { format!("{due} fällig") } else { format!("{due} due") },
                "state": "review",
            }));
        }
    }
    Ok(Value::Array(nodes))
}

/// The items to review now, at most `limit` (/LF230/, /LF502/: five in the plasmoid).
pub fn review(st: &Backend, now: Moment, limit: usize) -> anyhow::Result<Value> {
    let due = st.store.due(now, limit)?;
    Ok(Value::Array(
        due.iter()
            .map(|i| {
                json!({
                    "text": i.text,
                    "kind": i.kind,
                    "unit": i.unit,
                    "reps": i.reps,
                    "fromMistake": matches!(i.source, kaiwa_core::srs::Source::Correction { .. }),
                })
            })
            .collect(),
    ))
}

/// The learner model in words (/LF202/), the difficulty in words, weak spots.
pub fn learner(st: &Backend) -> anyhow::Result<Value> {
    let p = st.store.profile()?;
    let lang = p.ui_lang.as_str();
    let de = lang == "de";
    let l = st.store.learning()?;
    let skills: Vec<Value> = SKILLS
        .iter()
        .zip(l.model.describe(lang))
        .map(|(s, (_, words))| {
            json!({
                "skill": serde_json::to_value(s).unwrap_or_default(),
                "name": s.word(lang),
                "words": words.split_once(": ").map(|(_, w)| w.to_string()).unwrap_or(words.clone()),
            })
        })
        .collect();
    let weak: Vec<Value> = l
        .model
        .weak_spots(3)
        .iter()
        .map(|(k, _)| json!({ "kind": k, "word": kind_word(k, de) }))
        .collect();
    let d = &l.regulator.difficulty;
    let knobs: Vec<Value> = KNOBS
        .iter()
        .map(|k| json!({ "word": k.word(lang), "step": d.get(*k), "steps": k.steps() }))
        .collect();
    Ok(json!({
        "skills": skills,
        "weak": weak,
        "knobs": knobs,
        "speed": d.speed(),
        "offerSkip": l.regulator.offer_skip(),
    }))
}

/// Statistics (/LF610/): 12 weeks of practice as a heat map, totals.
pub fn stats(st: &Backend, now: Moment) -> anyhow::Result<Value> {
    let days = st.store.days()?;
    let today = now.day();
    // Monday of the week 11 weeks back, to today: whole weeks in columns.
    let start = today.plus(-(today.weekday() as i64) - 7 * 11);
    let mut levels = Vec::new();
    let mut d = start;
    while d <= today {
        let m = days.get(&d).map(|r| r.speaking_secs / 60).unwrap_or(0);
        levels.push(match m {
            0 => 0,
            1..=2 => 1,
            3..=5 => 2,
            6..=10 => 3,
            _ => 4,
        });
        d = d.plus(1);
    }
    let minutes: u32 = days.values().map(|r| r.speaking_secs / 60).sum();
    let practice_days = days.values().filter(|r| r.speaking_secs > 0).count();
    let items = st.store.items()?;
    // Kinds of mistakes per week over the same twelve weeks (/LF610/).
    let de = st.store.profile()?.ui_lang != "en";
    let from = start.start(now.offset_min).unix;
    let mut weeks: BTreeMap<String, [u32; 12]> = BTreeMap::new();
    for row in st.store.finished_since(from)? {
        let Ok(state) = st.store.session_state(row.id) else {
            continue;
        };
        let w = ((row.started.day().0 - start.0) / 7).clamp(0, 11) as usize;
        for t in &state.turns {
            for k in kaiwa_core::session::mistake_kinds(t) {
                weeks.entry(k).or_default()[w] += 1;
            }
        }
    }
    let mut kinds: Vec<(String, [u32; 12])> = weeks.into_iter().collect();
    kinds.sort_by_key(|(_, w)| std::cmp::Reverse(w.iter().sum::<u32>()));
    let mistakes: Vec<Value> = kinds
        .iter()
        .take(3)
        .map(|(k, w)| {
            let (early, late): (u32, u32) = (w[..6].iter().sum(), w[6..].iter().sum());
            let word = kind_word(k, de);
            let trend = match (late.cmp(&early), de) {
                (std::cmp::Ordering::Less, true) => {
                    format!("{word}: seltener als vor sechs Wochen")
                }
                (std::cmp::Ordering::Less, false) => {
                    format!("{word}: less often than six weeks ago")
                }
                (std::cmp::Ordering::Greater, true) => {
                    format!("{word}: zuletzt häufiger, ein Fall für die Wiederholung")
                }
                (std::cmp::Ordering::Greater, false) => {
                    format!("{word}: more often lately, one for the review")
                }
                (_, true) => format!("{word}: etwa gleich oft"),
                (_, false) => format!("{word}: about as often"),
            };
            json!({ "kind": k, "word": word, "weeks": w, "text": trend })
        })
        .collect();
    // Words and phrases per level, from the unit they came from ("n5-03" is N5).
    let mut vocab: BTreeMap<String, usize> = BTreeMap::new();
    for i in &items {
        let level = i
            .unit
            .as_deref()
            .and_then(|u| u.split('-').next())
            .map(str::to_uppercase)
            .unwrap_or_else(|| "?".into());
        *vocab.entry(level).or_default() += 1;
    }
    Ok(json!({
        "week": week(st, now)?,
        "mistakes": mistakes,
        "vocab": vocab.iter().map(|(l, n)| json!({ "level": l, "count": n })).collect::<Vec<_>>(),
        "heat": levels,
        "minutes": minutes,
        "days": practice_days,
        "items": items.len(),
        "learnt": items.iter().filter(|i| i.stability.is_some_and(|s| s >= 21.0)).count(),
    }))
}

/// The weekly review (/LF604/): the last seven days against the seven before: speaking time,
/// practice days, new words and phrases, and how many turns were understood without help.
pub fn week(st: &Backend, now: Moment) -> anyhow::Result<Value> {
    let today = now.day();
    let this = today.plus(-6);
    let before = today.plus(-13);
    let days = st.store.days()?;
    let span = |from: kaiwa_core::Day, to: kaiwa_core::Day| {
        days.range(from..=to)
            .fold((0u32, 0usize), |(m, n), (_, r)| {
                (m + r.speaking_secs, n + usize::from(r.speaking_secs > 0))
            })
    };
    let (secs, practised) = span(this, today);
    let (secs_before, _) = span(before, this.plus(-1));
    let start = |d: kaiwa_core::Day| d.start(now.offset_min).unix;
    let items = st.store.items()?;
    let mut new: Vec<&kaiwa_core::srs::Item> = items
        .iter()
        .filter(|i| i.added.unix >= start(this))
        .collect();
    new.sort_by_key(|i| std::cmp::Reverse(i.added.unix));
    // Turns understood without help or a listening aid, per week (misheard turns left out).
    let mut unaided = [(0usize, 0usize); 2];
    for row in st.store.finished_since(start(before))? {
        let Ok(state) = st.store.session_state(row.id) else {
            continue;
        };
        let w = usize::from(row.started.unix >= start(this));
        for t in state.turns.iter().filter(|t| !t.misrecognized) {
            unaided[w].1 += 1;
            unaided[w].0 += usize::from(t.understood && !t.aided && !t.from_help);
        }
    }
    let share = |(ok, all): (usize, usize)| (all > 0).then(|| (100 * ok / all) as u32);
    Ok(json!({
        "minutes": secs / 60,
        "minutesBefore": secs_before / 60,
        "days": practised,
        "newItems": new.len(),
        "newExamples": new.iter().take(5).map(|i| i.text.clone()).collect::<Vec<_>>(),
        "unaided": share(unaided[1]),
        "unaidedBefore": share(unaided[0]),
    }))
}

/// The settings page.
pub fn profile(st: &Backend) -> anyhow::Result<Value> {
    let p = st.store.profile()?;
    let goal = match p.daily_goal {
        DailyGoal::Minutes(m) => json!({ "kind": "minutes", "value": m }),
        DailyGoal::Scenarios(n) => json!({ "kind": "scenarios", "value": n }),
    };
    Ok(json!({
        "name": p.name,
        "uiLang": p.ui_lang,
        "goal": goal,
        "gamification": p.gamification,
        "feedback": p.feedback,
        "script": match p.script { Script::Furigana => 0, Script::Kana => 1, Script::Hidden => 2 },
        "endPause": p.end_pause_secs,
        "keepRecordings": p.keep_recordings_days,
        "style": p.style,
        "reminder": p.reminder,
        "costLimit": p.cost_limit_cents,
        "moraHints": p.mora_hints,
        "unloadAfter": p.unload_after_min,
        "handsFree": p.hands_free,
        "textScale": p.text_scale,
        "cost": st.cost_view(crate::now())?,
        "path": st.path.display().to_string(),
        "profiles": st.profiles(),
        "key": key_view(&p.ui_lang),
        "demoData": st.demo_data,
    }))
}

/// The tutor's API key (/LF713/): which provider, where its key comes from, in words.
pub fn key_view(lang: &str) -> Value {
    use kaiwa_ai::secrets::{KeyState, state};
    let env = |k: &str| std::env::var(k).ok();
    let Some(provider) = env("KAIWA_PROVIDER").filter(|p| p == "anthropic" || p == "openai") else {
        return Value::Null;
    };
    let de = lang != "en";
    let s = state(&provider, env);
    let text = match (&s, de) {
        (KeyState::Environment, true) => "Aus der Umgebung (lokaler Test).".to_string(),
        (KeyState::Environment, false) => "From the environment (local test).".to_string(),
        (KeyState::Keyring, true) => "Im Schlüsselbund des Systems gespeichert.".to_string(),
        (KeyState::Keyring, false) => "Kept in the system keyring.".to_string(),
        (KeyState::Missing, true) => {
            "Fehlt. Er wird im Schlüsselbund gespeichert, nie in einer Datei.".to_string()
        }
        (KeyState::Missing, false) => {
            "Missing. It is kept in the keyring, never in a file.".to_string()
        }
        (KeyState::Unavailable(e), true) => format!("Kein Schlüsselbund erreichbar ({e})."),
        (KeyState::Unavailable(e), false) => format!("No keyring reachable ({e})."),
    };
    json!({
        "provider": provider,
        "stored": matches!(s, KeyState::Keyring | KeyState::Environment),
        "canStore": !matches!(s, KeyState::Unavailable(_) | KeyState::Environment),
        "text": text,
    })
}

/// "What leaves the device?" (/LF711/), from `kaiwa_ai::privacy`.
pub fn privacy(lang: &str) -> Value {
    serde_json::to_value(kaiwa_ai::privacy::flows_from_env(lang)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_is_local_without_a_cloud_provider() {
        let rows = privacy("de");
        let rows = rows.as_array().unwrap();
        assert_eq!(rows.len(), 3);
        if std::env::var_os("KAIWA_PROVIDER").is_none() {
            assert!(
                rows.iter()
                    .all(|r| r["cloud"] == false && r["sends"] == "nichts")
            );
        }
    }

    #[test]
    fn free_talk_offers_the_topics_outside_the_path() {
        let dir = std::env::temp_dir().join(format!("kaiwa-free-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let b = Backend::open_at(dir.join("demo.kaiwa"), true).unwrap();
        let v = today(&b, crate::now()).unwrap();
        let free = v["free"].as_array().unwrap();
        assert_eq!(free.len(), 4);
        for f in free {
            let id = f["id"].as_str().unwrap();
            assert!(b.curriculum.unit_of(id).is_none(), "{id} is on the path");
            assert!(!f["topic"].as_str().unwrap().contains(':'));
        }
    }

    #[test]
    fn the_week_adds_up_the_last_seven_days() {
        let dir = std::env::temp_dir().join(format!("kaiwa-week-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let b = Backend::open_at(dir.join("demo.kaiwa"), true).unwrap();
        let now = crate::now();
        let w = week(&b, now).unwrap();
        let days = b.store.days().unwrap();
        let today = now.day();
        let secs: u32 = days
            .range(today.plus(-6)..=today)
            .map(|(_, r)| r.speaking_secs)
            .sum();
        assert_eq!(w["minutes"], secs / 60);
        assert!(w["days"].as_u64().unwrap() <= 7);
        assert!(w["newExamples"].as_array().unwrap().len() <= 5);
        if let Some(u) = w["unaided"].as_u64() {
            assert!(u <= 100);
        }
    }

    #[test]
    fn mistakes_over_twelve_weeks() {
        let dir = std::env::temp_dir().join(format!("kaiwa-stats-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let b = Backend::open_at(dir.join("demo.kaiwa"), true).unwrap();
        let v = stats(&b, crate::now()).unwrap();
        let m = v["mistakes"].as_array().unwrap();
        let particle = m
            .iter()
            .find(|x| x["kind"] == "particle")
            .expect("particles");
        assert!(
            particle["text"].as_str().unwrap().contains("seltener"),
            "{particle}"
        );
        assert!(m.iter().any(|x| x["kind"] == "counter"), "{m:?}");
        assert_eq!(particle["weeks"].as_array().unwrap().len(), 12);
        assert!(
            v["vocab"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["level"] == "N5")
        );
    }
}
