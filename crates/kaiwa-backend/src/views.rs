//! What the app's pages show, as JSON for QML: built from the profile with plain functions,
//! so the content of every view is testable without Qt.

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
    Ok(json!({
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
    Ok(json!({
        "heat": levels,
        "minutes": minutes,
        "days": practice_days,
        "items": items.len(),
        "learnt": items.iter().filter(|i| i.stability.is_some_and(|s| s >= 21.0)).count(),
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
        "cost": st.cost_view(crate::now())?,
        "path": st.path.display().to_string(),
        "demoData": st.demo_data,
    }))
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
}
