//! The start page's daily goal in words and its week of minutes per day.

use kaiwa_core::clock::Moment;
use kaiwa_core::progress::DailyGoal;
use serde_json::{Value, json};

use crate::Backend;

/// The minutes spoken today and the rest of the sentence: "2" and "von 5 Minuten gesprochen".
pub(crate) fn goal_words(de: bool, goal_minutes: u32, speaking_secs: u32) -> (u32, String) {
    let rest = if de {
        format!("von {goal_minutes} Minuten gesprochen")
    } else {
        format!("of {goal_minutes} minutes spoken")
    };
    (speaking_secs / 60, rest)
}

/// The calendar week, Monday to Sunday, for the start page's bars: its labels, the minutes
/// spoken per day, whether the goal was met, and which index is today.
pub(crate) fn bars(st: &Backend, now: Moment, goal: DailyGoal, de: bool) -> anyhow::Result<Value> {
    let days = st.store.days()?;
    let today = now.day();
    let monday = today.plus(-i64::from(today.weekday()));
    let labels = if de {
        ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"]
    } else {
        ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
    };
    let week: Vec<_> = (0..7).map(|i| days.get(&monday.plus(i)).cloned()).collect();
    Ok(json!({
        "labels": labels,
        "minutes": week.iter().map(|r| r.as_ref().map_or(0, |r| r.speaking_secs / 60)).collect::<Vec<_>>(),
        "met": week.iter().map(|r| r.as_ref().is_some_and(|r| r.met(goal))).collect::<Vec<_>>(),
        "today": today.weekday(),
    }))
}

#[cfg(test)]
mod tests {
    use crate::views::today;

    #[test]
    fn today_carries_the_week_and_the_review_estimate() {
        let b = crate::TempBackend::new("today-week", true).unwrap();
        let now = crate::now();
        let monday = now.day().plus(-i64::from(now.day().weekday()));
        let rec = |secs| kaiwa_core::progress::DayRecord {
            speaking_secs: secs,
            ..Default::default()
        };
        b.store.put_day(monday, &rec(600)).unwrap();
        b.store.put_day(now.day(), &rec(130)).unwrap();
        let v = today(&b, now).unwrap();
        let w = &v["week"];
        let at = now.day().weekday() as usize;
        assert_eq!(w["labels"].as_array().unwrap().len(), 7);
        assert_eq!(w["minutes"][0], if at == 0 { 2 } else { 10 }, "{w}");
        assert_eq!(w["minutes"][at], 2, "{w}");
        assert_eq!(w["today"], at, "{w}");
        assert_eq!(w["met"][at], false, "{w}");
        assert_eq!(v["goalSpoken"], 2);
        assert_eq!(v["goalRest"], "von 5 Minuten gesprochen");
        assert_eq!(v["goalText"], "2 von 5 Minuten gesprochen");
        let due = v["due"].as_u64().unwrap() as usize;
        assert_eq!(v["reviewMinutes"], kaiwa_core::srs::review_minutes(due));
    }
}
