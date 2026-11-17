//! The learning path view (/LF600/): the content of the path page as JSON.

use kaiwa_core::curriculum::{NodeState, UnitNode};
use serde_json::{Value, json};

use crate::Backend;
use crate::views::scenario_title;

/// Locked units a level shows before the rest folds into one button.
const LOCKED_SHOWN: usize = 3;

/// The learning path (/LF600/): one group per unit with its scenarios, state, due reviews and
/// the unit titles that lock it, in one section per level („N5 · A1“, Q-03).
/// `[{ "level", "units": [{ "id", "number", "title", "canDo", "state", "scenarios", "due",
/// "note", "folded" }], "foldedCount", "foldedNames" }]`. `note` is the line after the title:
/// the due reviews of a unit, „Als nächstes“, or what unlocks a locked one; `canDo` is a
/// sentence of its own that wraps (/LL303/: narrow windows).
pub fn path(st: &Backend) -> anyhow::Result<Value> {
    let p = st.store.profile()?;
    let lang = p.ui_lang.as_str();
    let learning = st.store.learning()?;
    let items = st.store.items()?;
    let today = crate::now().day();
    let units = st.curriculum.path(&learning.course, lang);
    let mut levels: Vec<Value> = Vec::new();
    let mut locked_seen = 0;
    let mut folded: Vec<String> = Vec::new();
    for unit in &units {
        let level = unit.level.clone().unwrap_or_default();
        if levels.last().is_none_or(|l| l["level"] != level.as_str()) {
            close_level(levels.last_mut(), &mut folded);
            locked_seen = 0;
            levels
                .push(json!({ "level": level, "units": [], "foldedCount": 0, "foldedNames": "" }));
        }
        let due = items
            .iter()
            .filter(|i| i.unit.as_deref() == Some(unit.id.as_str()) && i.is_due(today))
            .count();
        let mut node = unit_group(st, &units, unit, due, lang);
        if unit.state == NodeState::Locked {
            locked_seen += 1;
            if locked_seen > LOCKED_SHOWN {
                node["folded"] = json!(true);
                folded.push(unit.title.clone());
            }
        }
        if let Some(list) = levels.last_mut().and_then(|l| l["units"].as_array_mut()) {
            list.push(node);
        }
    }
    close_level(levels.last_mut(), &mut folded);
    Ok(Value::Array(levels))
}

/// Writes what folded into the level just ended, and starts the next count.
fn close_level(level: Option<&mut Value>, folded: &mut Vec<String>) {
    let Some(level) = level else {
        return;
    };
    level["foldedCount"] = json!(folded.len());
    level["foldedNames"] = json!(folded.join(", "));
    folded.clear();
}

fn state_word(state: NodeState) -> &'static str {
    match state {
        NodeState::Done => "done",
        NodeState::Current => "current",
        NodeState::Open => "open",
        NodeState::Locked => "locked",
    }
}

/// One unit of the path: its scenarios and the line after its title.
fn unit_group(st: &Backend, units: &[UnitNode], unit: &UnitNode, due: usize, lang: &str) -> Value {
    let de = lang == "de";
    let scenarios: Vec<Value> = unit
        .scenarios
        .iter()
        .map(|sc| {
            json!({
                "id": sc.id,
                "title": scenario_title(st, &sc.id, lang),
                "state": state_word(sc.state),
            })
        })
        .collect();
    // Reviews that go badly are recommended, they lock nothing (T-06).
    let due_text = match (de, unit.shaky) {
        (true, false) => format!("◆ {due} fällig"),
        (true, true) => format!("◆ {due} fällig · empfohlen, bevor es weitergeht"),
        (false, false) => format!("◆ {due} due"),
        (false, true) => format!("◆ {due} due · recommended before you go on"),
    };
    let due_text = if due > 0 { due_text } else { String::new() };
    let note = match unit.state {
        NodeState::Locked => lock_text(st, units, &unit.id, de),
        NodeState::Current if de => "Als nächstes".to_string(),
        NodeState::Current => "Up next".to_string(),
        NodeState::Done => due_text.clone(),
        NodeState::Open => String::new(),
    };
    json!({
        "id": unit.id,
        "number": unit.id.to_uppercase(),
        "title": unit.title,
        "canDo": unit.can_do,
        "state": state_word(unit.state),
        "scenarios": scenarios,
        "due": if unit.state == NodeState::Locked { 0 } else { due },
        "dueText": if unit.state == NodeState::Locked { String::new() } else { due_text },
        "note": note,
        "folded": false,
    })
}

/// What unlocks a locked unit, by the names the path shows: „nach An der Konbini (N5-04)“;
/// several joined with „und“.
fn lock_text(st: &Backend, units: &[UnitNode], id: &str, de: bool) -> String {
    let Some(unit) = st.curriculum.unit(id) else {
        return String::new();
    };
    let names: Vec<String> = unit
        .requires
        .iter()
        .filter_map(|r| units.iter().find(|u| u.id == *r))
        .filter(|u| u.state != NodeState::Done)
        .map(|u| format!("{} ({})", u.title, u.id.to_uppercase()))
        .collect();
    let (after, and) = if de {
        ("nach", " und ")
    } else {
        ("after", " and ")
    };
    format!("{after} {}", names.join(and))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::today;

    #[test]
    fn reviews_that_go_badly_are_recommended_and_lock_nothing() {
        let b = crate::TempBackend::new("shaky", false).unwrap();
        let first = b.curriculum.units[0].clone();
        let second = b.curriculum.units[1].clone();
        assert!(second.requires.contains(&first.id));
        let mut l = b.store.learning().unwrap();
        l.course.scenarios_done.extend(first.scenarios.clone());
        for _ in 0..3 {
            l.course.reviewed(&first.id, false);
        }
        b.store.set_learning(&l).unwrap();
        let mut item = kaiwa_core::srs::Item::new(
            "はじめまして",
            kaiwa_core::srs::ItemKind::Phrase,
            kaiwa_core::srs::Source::NewWord,
            crate::now(),
        );
        item.unit = Some(first.id.clone());
        item.due = crate::now().day();
        b.store.put_item(&item).unwrap();
        let groups = path_units(&b);
        let review = groups
            .iter()
            .find(|u| u["id"] == first.id.as_str())
            .unwrap();
        assert_eq!(review["due"], 1);
        assert_eq!(
            review["note"],
            "◆ 1 fällig · empfohlen, bevor es weitergeht"
        );
        // T-06: the next unit is open all the same.
        let next = groups
            .iter()
            .find(|u| u["id"] == second.id.as_str())
            .unwrap();
        assert_eq!(next["state"], "current");
        assert!(
            today(&b, crate::now())
                .unwrap()
                .to_string()
                .contains(&second.scenarios[0])
        );
    }

    /// The units of the path in order, flattened over the levels.
    fn path_units(b: &Backend) -> Vec<Value> {
        let levels = path(b).unwrap();
        levels
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|l| l["units"].as_array().unwrap().clone())
            .collect()
    }

    #[test]
    fn a_path_unit_says_what_it_teaches_and_what_locks_it() {
        let b = crate::TempBackend::new("path-text", false).unwrap();
        let units = path_units(&b);
        let open = units.iter().find(|u| u["state"] == "current").unwrap();
        let locked = units.iter().find(|u| u["state"] == "locked").unwrap();

        // The unit's number once, the can-do sentence apart, so it can wrap.
        assert_eq!(open["number"].as_str().unwrap().len(), "N5-01".len());
        assert!(open["canDo"].as_str().unwrap().starts_with("Ich kann"));
        assert_eq!(open["note"], "Als nächstes");
        assert!(locked["note"].as_str().unwrap().starts_with("nach "));
    }

    #[test]
    fn a_path_has_one_group_per_unit_with_its_scenarios() {
        let b = crate::TempBackend::new("path-groups", false).unwrap();
        let units = path_units(&b);
        assert_eq!(units.len(), b.curriculum.units.len());
        for (group, unit) in units.iter().zip(&b.curriculum.units) {
            assert_eq!(group["id"], unit.id.as_str());
            let ids: Vec<&str> = group["scenarios"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["id"].as_str().unwrap())
                .collect();
            assert_eq!(ids, unit.scenarios);
        }
        assert!(
            units
                .iter()
                .any(|u| u["scenarios"].as_array().unwrap().len() > 1)
        );

        // Levels in path order, each unit in the level it belongs to.
        let levels = path(&b).unwrap();
        let first = &levels[0];
        assert_eq!(first["level"], "N5 · A1");
        assert!(levels.as_array().unwrap().len() >= 2);
    }

    #[test]
    fn a_locked_unit_names_its_missing_units_as_the_path_shows_them() {
        let b = crate::TempBackend::new("path-lock", false).unwrap();
        let units = path_units(&b);
        let two = b
            .curriculum
            .units
            .iter()
            .find(|u| u.requires.len() > 1)
            .expect("a unit with two prerequisites");
        let group = units.iter().find(|u| u["id"] == two.id.as_str()).unwrap();
        let names: Vec<String> = two
            .requires
            .iter()
            .map(|r| {
                let g = units.iter().find(|u| u["id"] == r.as_str()).unwrap();
                format!(
                    "{} ({})",
                    g["title"].as_str().unwrap(),
                    g["number"].as_str().unwrap()
                )
            })
            .collect();
        assert_eq!(group["state"], "locked");
        assert_eq!(group["note"], format!("nach {}", names.join(" und ")));

        // A prerequisite that is done is not named.
        let mut l = b.store.learning().unwrap();
        let done = &two.requires[0];
        l.course
            .scenarios_done
            .extend(b.curriculum.unit(done).unwrap().scenarios.clone());
        b.store.set_learning(&l).unwrap();
        let group = path_units(&b)
            .into_iter()
            .find(|u| u["id"] == two.id.as_str())
            .unwrap();
        assert!(
            !group["note"]
                .as_str()
                .unwrap()
                .contains(&done.to_uppercase())
        );
    }

    #[test]
    fn a_unit_counts_its_due_reviews_and_a_level_folds_its_locked_rest() {
        let b = crate::TempBackend::new("path-due", false).unwrap();
        let first = b.curriculum.units[0].clone();
        let mut l = b.store.learning().unwrap();
        l.course.scenarios_done.extend(first.scenarios.clone());
        b.store.set_learning(&l).unwrap();
        for text in ["はじめまして", "おはよう"] {
            let mut item = kaiwa_core::srs::Item::new(
                text,
                kaiwa_core::srs::ItemKind::Phrase,
                kaiwa_core::srs::Source::NewWord,
                crate::now(),
            );
            item.unit = Some(first.id.clone());
            item.due = crate::now().day();
            b.store.put_item(&item).unwrap();
        }
        let units = path_units(&b);
        assert_eq!(units[0]["state"], "done");
        assert_eq!(units[0]["due"], 2);
        assert_eq!(units[0]["note"], "◆ 2 fällig");
        assert_eq!(units[1]["due"], 0);

        // At most three locked units of a level show; the rest folds, named.
        let levels = path(&b).unwrap();
        for level in levels.as_array().unwrap() {
            let units = level["units"].as_array().unwrap();
            let locked: Vec<&Value> = units.iter().filter(|u| u["state"] == "locked").collect();
            let folded = locked.iter().filter(|u| u["folded"] == true).count();
            assert_eq!(folded, locked.len().saturating_sub(3), "{level}");
            assert_eq!(level["foldedCount"], folded);
            let names: Vec<&str> = locked
                .iter()
                .filter(|u| u["folded"] == true)
                .map(|u| u["title"].as_str().unwrap())
                .collect();
            assert_eq!(level["foldedNames"], names.join(", "));
        }
        assert!(
            levels
                .as_array()
                .unwrap()
                .iter()
                .any(|l| l["foldedCount"].as_u64() > Some(0))
        );
    }
}
