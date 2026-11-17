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
/// "note", "open", "review", "showCanDo", "showScenarios", "folded" }], "foldedCount",
/// "foldedNames" }]`. `open` is what a click does: `{"scenario": id}`, `{"review": unit}` or
/// null; scenarios carry their `label` and button `emphasis`; `review` is a button of its own
/// with its label. `note` is the line after the title:
/// the due reviews of a done unit, „Als nächstes“, or what unlocks a locked one; `canDo` is a
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

/// One unit of the path: what it shows and what a click on it does.
fn unit_group(st: &Backend, units: &[UnitNode], unit: &UnitNode, due: usize, lang: &str) -> Value {
    let de = lang == "de";
    let locked = unit.state == NodeState::Locked;
    let done = unit.state == NodeState::Done;
    let due = if locked { 0 } else { due };
    let scenarios = scenario_buttons(st, unit, lang);
    // Reviews that go badly are recommended, they lock nothing (T-06).
    let due_text = match (de, unit.shaky) {
        (true, false) => format!("◆ {due} fällig"),
        (true, true) => format!("◆ {due} fällig · empfohlen, bevor es weitergeht"),
        (false, false) => format!("◆ {due} due"),
        (false, true) => format!("◆ {due} due · recommended before you go on"),
    };
    let review = (due > 0).then(|| {
        let word = if de { "Wiederholung" } else { "Review" };
        json!({ "unit": unit.id, "label": format!("{word}: {} · {due_text}", unit.title) })
    });
    let note = match unit.state {
        NodeState::Locked => lock_text(units, unit, de),
        NodeState::Current if de => "Als nächstes".to_string(),
        NodeState::Current => "Up next".to_string(),
        NodeState::Done if due > 0 => due_text,
        _ => String::new(),
    };
    let open = click(unit, done && due > 0);
    json!({
        "id": unit.id,
        "number": unit.id.to_uppercase(),
        "title": unit.title,
        "canDo": unit.can_do,
        "showCanDo": !locked && !done,
        "state": state_word(unit.state),
        "scenarios": scenarios,
        "showScenarios": !locked,
        "review": review,
        "due": due,
        "open": open,
        "note": note,
        "folded": false,
    })
}

/// The scenarios of a unit as buttons: the one to do next is primary, done ones are quiet.
fn scenario_buttons(st: &Backend, unit: &UnitNode, lang: &str) -> Vec<Value> {
    let de = lang == "de";
    unit.scenarios
        .iter()
        .map(|sc| {
            let title = scenario_title(st, &sc.id, lang);
            let (label, emphasis) = match sc.state {
                NodeState::Done if de => (format!("{title} (erledigt)"), "quiet"),
                NodeState::Done => (format!("{title} (done)"), "quiet"),
                NodeState::Current => (title, "primary"),
                _ => (title, "normal"),
            };
            json!({ "id": sc.id, "label": label, "emphasis": emphasis })
        })
        .collect()
}

/// What a click on the unit does: a done unit with words due reviews them, otherwise the
/// first scenario still to do (all done: the first, to play it again); a locked one nothing.
fn click(unit: &UnitNode, review: bool) -> Value {
    if unit.state == NodeState::Locked {
        return Value::Null;
    }
    if review {
        return json!({ "review": unit.id });
    }
    let first = unit
        .scenarios
        .iter()
        .find(|s| s.state != NodeState::Done)
        .or(unit.scenarios.first());
    first.map_or(Value::Null, |s| json!({ "scenario": s.id }))
}

/// What unlocks a locked unit, by the names the path shows: „nach An der Konbini (N5-04)“;
/// several joined with „und“.
fn lock_text(units: &[UnitNode], unit: &UnitNode, de: bool) -> String {
    let names: Vec<String> = unit
        .missing
        .iter()
        .filter_map(|m| units.iter().find(|u| u.id == *m))
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
        assert!(
            review["review"]["label"]
                .as_str()
                .unwrap()
                .ends_with("· ◆ 1 fällig · empfohlen, bevor es weitergeht")
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
        assert_eq!(open["showCanDo"], true);
        assert_eq!(open["scenarios"][0]["emphasis"], "primary");
        assert_eq!(
            open["open"],
            json!({ "scenario": open["scenarios"][0]["id"] })
        );
        assert!(locked["note"].as_str().unwrap().starts_with("nach "));
        assert!(locked["open"].is_null());
        assert_eq!(locked["showScenarios"], false);
        assert_eq!(locked["showCanDo"], false);
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
        assert!(units[1]["review"].is_null());

        // A done unit: the click reviews, every scenario stays a quiet replay button.
        assert_eq!(units[0]["open"], json!({ "review": first.id }));
        assert_eq!(units[0]["showScenarios"], true);
        assert_eq!(units[0]["showCanDo"], false);
        assert!(
            units[0]["scenarios"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["emphasis"] == "quiet")
        );
        let label = units[0]["review"]["label"].as_str().unwrap();
        assert_eq!(
            label,
            format!(
                "Wiederholung: {} · ◆ 2 fällig",
                units[0]["title"].as_str().unwrap()
            )
        );

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
