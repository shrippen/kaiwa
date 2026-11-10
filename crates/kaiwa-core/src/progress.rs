//! Daily goal, streak and XP (Lastenheft /LF601/–/LF606/, /LF620/, /LL300/).
//!
//! - A session counts for the local day it **started** on (a session from 23:58 to 00:03 fills
//!   the first day only).
//! - The streak counts days with the daily goal met. A missed day uses a freeze (two per
//!   calendar month, automatic); without a freeze the streak ends, said plainly and without
//!   blame. Pause mode (holiday, illness) neither counts nor breaks.
//! - XP reward speaking: typed turns earn less, one-word answers, stock phrases and repeats
//!   earn little or nothing, reading out the help's full example earns half (/LF603/).
//! - With gamification off there is no XP and no streak anywhere (/LF605/). There are no hearts,
//!   lives or locks (/LF606/): nothing here can take something away.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::clock::Day;
use crate::learner::Skill;
use crate::session::Input;

/// The daily goal (/LF601/), chosen by the learner. Default: five minutes of speaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DailyGoal {
    Minutes(u32),
    Scenarios(u32),
}

impl Default for DailyGoal {
    fn default() -> Self {
        DailyGoal::Minutes(5)
    }
}

/// What happened on one local day.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DayRecord {
    /// Spoken seconds only; typed turns do not count (/LF120/, /LF601/).
    pub speaking_secs: u32,
    pub typed_turns: u32,
    pub scenarios_done: u32,
    pub reviews_done: u32,
    pub xp: u32,
    /// The learner model's level per skill before the day's first talk: where the week
    /// started from, for the progress per skill in the weekly review (/LF604/). Empty on days
    /// without a talk and on days from before this was kept.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub levels_at_start: BTreeMap<Skill, f64>,
    /// The daily goal as it was on this day: the streak judges a day by the goal it had, so
    /// choosing another goal later neither ends nor invents a streak (/LF601/, /LF602/).
    /// `None` on days from before this was kept; they are judged by the goal of today.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<DailyGoal>,
}

impl DayRecord {
    pub fn met(&self, goal: DailyGoal) -> bool {
        match goal {
            DailyGoal::Minutes(m) => self.speaking_secs >= m * 60,
            DailyGoal::Scenarios(n) => self.scenarios_done >= n,
        }
    }

    /// Progress towards the goal, 0..1 (the ring of the plasmoid and the start page).
    pub fn fraction(&self, goal: DailyGoal) -> f64 {
        let f = match goal {
            DailyGoal::Minutes(m) => f64::from(self.speaking_secs) / f64::from((m * 60).max(1)),
            DailyGoal::Scenarios(n) => f64::from(self.scenarios_done) / f64::from(n.max(1)),
        };
        f.clamp(0.0, 1.0)
    }
}

pub const FREEZES_PER_MONTH: usize = 2;

/// A stretch of pause mode, both days included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pause {
    pub from: Day,
    pub to: Day,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Streak {
    /// Days with the goal met in the running streak.
    pub days: u32,
    pub longest: u32,
    /// Days covered by a freeze.
    pub freezes_used: Vec<Day>,
    /// Days on which a streak ended (no freeze left).
    pub ended_on: Vec<Day>,
    pub today_met: bool,
}

/// The streak as of `today` (/LF602/).
pub fn streak(
    records: &BTreeMap<Day, DayRecord>,
    goal: DailyGoal,
    pauses: &[Pause],
    today: Day,
) -> Streak {
    let mut s = Streak::default();
    let Some((&first, _)) = records.iter().next() else {
        return s;
    };
    let mut used: BTreeMap<(i32, u32), usize> = BTreeMap::new();
    let mut d = first;
    while d <= today {
        let paused = pauses.iter().any(|p| p.from <= d && d <= p.to);
        // A past day by the goal it had; today by the goal as it is now.
        let met = records.get(&d).is_some_and(|r| {
            r.met(if d == today {
                goal
            } else {
                r.goal.unwrap_or(goal)
            })
        });
        if met {
            s.days += 1;
            s.longest = s.longest.max(s.days);
        } else if paused || d == today {
            // A pause neither counts nor breaks; today is not over yet.
        } else if s.days > 0 {
            let n = used.entry(d.month()).or_default();
            if *n < FREEZES_PER_MONTH {
                *n += 1;
                s.freezes_used.push(d);
            } else {
                s.days = 0;
                s.ended_on.push(d);
            }
        }
        d = d.plus(1);
    }
    s.today_met = records.get(&today).is_some_and(|r| r.met(goal));
    s
}

/// Freezes still free in `today`'s month.
pub fn freezes_left(s: &Streak, today: Day) -> usize {
    FREEZES_PER_MONTH.saturating_sub(
        s.freezes_used
            .iter()
            .filter(|d| d.month() == today.month())
            .count(),
    )
}

/// What the start page says about the streak, plain and without blame (/LF620/, /LL300/).
pub fn streak_note(s: &Streak, today: Day, lang: &str) -> String {
    let de = lang == "de";
    if s.ended_on.last() == Some(&today.plus(-1)) && s.days == 0 {
        return if de {
            "Die Serie hat neu begonnen. Heute ist Tag eins.".into()
        } else {
            "The streak has started over. Today is day one.".into()
        };
    }
    if s.freezes_used.last() == Some(&today.plus(-1)) {
        return if de {
            format!(
                "Gestern hat ein Freeze die Serie gehalten. Diesen Monat bleiben {}.",
                freezes_left(s, today)
            )
        } else {
            format!(
                "A freeze kept the streak yesterday. {} left this month.",
                freezes_left(s, today)
            )
        };
    }
    match (de, s.days) {
        (true, 1) => "1 Tag in Folge.".into(),
        (true, n) => format!("{n} Tage in Folge."),
        (false, 1) => "1 day in a row.".into(),
        (false, n) => format!("{n} days in a row."),
    }
}

/// Words that never appear in what Kaiwa says about progress or reminders (/LF620/, /LL300/).
pub const BLAME_WORDS: &[&str] = &[
    "vermisst",
    "verloren",
    "verlierst",
    "verpasst",
    "enttäusch",
    "schade",
    "leider",
    "miss you",
    "lose your",
    "lost",
    "don't break",
    "disappoint",
];

pub fn blameless(text: &str) -> bool {
    let t = text.to_lowercase();
    !BLAME_WORDS.iter().any(|w| t.contains(w))
}

/// Why a turn earned less than full XP; the summary explains it (/LF603/).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XpReason {
    Typed,
    OneWord,
    Repeated,
    FromHelp,
}

pub const XP_SPOKEN: u32 = 10;
pub const XP_TYPED: u32 = 4;
pub const XP_ONE_WORD: u32 = 1;
pub const XP_FROM_HELP: u32 = 5;
pub const XP_GOAL: u32 = 10;
pub const XP_REVIEW: u32 = 2;

/// Short answers that carry no information on their own.
const STOCK: &[&str] = &[
    "はい",
    "いいえ",
    "ええ",
    "うん",
    "いいですね",
    "そうですね",
    "そうです",
    "なるほど",
    "すごい",
    "ありがとう",
    "ありがとうございます",
    "わかりました",
    "だいじょうぶです",
    "大丈夫です",
];

/// XP for one learner turn. `earlier`: the learner's earlier turns in this session;
/// `from_help`: the turn repeats the full example of help step 3.
pub fn turn_xp(
    text: &str,
    input: Input,
    earlier: &[&str],
    from_help: bool,
) -> (u32, Option<XpReason>) {
    let key = crate::norm(text);
    if earlier.iter().any(|e| crate::norm(e) == key) {
        return (0, Some(XpReason::Repeated));
    }
    if key.chars().count() <= 3 || STOCK.iter().any(|s| crate::norm(s) == key) {
        return (XP_ONE_WORD, Some(XpReason::OneWord));
    }
    if from_help {
        return (XP_FROM_HELP, Some(XpReason::FromHelp));
    }
    match input {
        Input::Spoken => (XP_SPOKEN, None),
        Input::Typed => (XP_TYPED, Some(XpReason::Typed)),
    }
}

/// The XP of a session with the reasons for what was held back.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionXp {
    pub xp: u32,
    /// How many turns earned less, per reason.
    pub reduced: BTreeMap<XpReason, u32>,
}

impl SessionXp {
    pub fn add_turn(&mut self, (xp, reason): (u32, Option<XpReason>)) {
        self.xp += xp;
        if let Some(r) = reason {
            *self.reduced.entry(r).or_default() += 1;
        }
    }

    /// One line per reason, for the summary.
    pub fn explain(&self, lang: &str) -> Vec<String> {
        let de = lang == "de";
        self.reduced
            .iter()
            .map(|(r, n)| match (r, de) {
                (XpReason::Typed, true) => {
                    format!("{n} getippte Züge zählen weniger als gesprochene.")
                }
                (XpReason::Typed, false) => format!("{n} typed turns count less than spoken ones."),
                (XpReason::OneWord, true) => {
                    format!("{n} Antworten aus einem Wort zählen kaum. Ganze Sätze bringen mehr.")
                }
                (XpReason::OneWord, false) => {
                    format!("{n} one-word answers count very little. Whole sentences earn more.")
                }
                (XpReason::Repeated, true) => {
                    format!("{n} Wiederholungen desselben Satzes zählen nicht.")
                }
                (XpReason::Repeated, false) => {
                    format!("{n} repeats of the same sentence do not count.")
                }
                (XpReason::FromHelp, true) => {
                    format!("{n} Sätze aus der Hilfe zählen halb, und ein so erreichtes Ziel bringt keine Extrapunkte. Eigene Sätze zählen voll.")
                }
                (XpReason::FromHelp, false) => {
                    format!("{n} sentences from the help count half, and a goal reached that way earns no bonus. Your own count in full.")
                }
            })
            .collect()
    }
}

/// Seconds a spoken turn takes, estimated from its length (for recognised text without a
/// recording, and for `kaiwa-sim`): about 6 morae per second plus a second to start.
pub fn speaking_secs(text: &str) -> u32 {
    let morae = crate::norm(text).chars().count() as f64;
    (1.0 + morae / 6.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(min: u32) -> DayRecord {
        DayRecord {
            speaking_secs: min * 60,
            ..Default::default()
        }
    }

    fn days(start: Day, mins: &[u32]) -> BTreeMap<Day, DayRecord> {
        mins.iter()
            .enumerate()
            .filter(|(_, m)| **m > 0)
            .map(|(i, m)| (start.plus(i as i64), rec(*m)))
            .collect()
    }

    #[test]
    fn another_goal_leaves_the_days_before_as_they_were() {
        // Twenty days of five to six minutes with a goal of five.
        let mut days = BTreeMap::new();
        for n in 0..20 {
            let mut r = rec(5 + n % 2);
            r.goal = Some(DailyGoal::Minutes(5));
            days.insert(Day(100 + i64::from(n)), r);
        }
        let today = Day(120);
        // The learner picks ten minutes: the streak stands, today is still open.
        let s = streak(&days, DailyGoal::Minutes(10), &[], today);
        assert_eq!((s.days, s.freezes_used.len()), (20, 0));
        // A lower goal invents nothing either: a day that missed its goal stays missed.
        let mut missed = rec(2);
        missed.goal = Some(DailyGoal::Minutes(5));
        days.insert(Day(110), missed);
        let s = streak(&days, DailyGoal::Minutes(1), &[], today);
        assert_eq!(s.freezes_used, vec![Day(110)]);
        // Days from before the goal was kept are judged by today's goal, as before.
        let old: BTreeMap<Day, DayRecord> = (0..3).map(|n| (Day(200 + n), rec(6))).collect();
        assert_eq!(streak(&old, DailyGoal::Minutes(10), &[], Day(203)).days, 0);
        assert_eq!(streak(&old, DailyGoal::Minutes(5), &[], Day(203)).days, 3);
        // Today counts by the goal as it is now.
        let mut now = rec(6);
        now.goal = Some(DailyGoal::Minutes(5));
        days.insert(today, now);
        assert!(!streak(&days, DailyGoal::Minutes(10), &[], today).today_met);
    }

    #[test]
    fn two_freezes_a_month_then_the_streak_ends() {
        let d0 = Day::from_ymd(2026, 10, 1);
        // Five days met, three missed, then met again.
        let r = days(d0, &[5, 5, 5, 5, 5, 0, 0, 0, 5]);
        let s = streak(&r, DailyGoal::default(), &[], d0.plus(8));
        assert_eq!(s.freezes_used, vec![d0.plus(5), d0.plus(6)]);
        assert_eq!(s.ended_on, vec![d0.plus(7)]);
        assert_eq!(s.days, 1);
        assert_eq!(s.longest, 5);
        assert!(blameless(&streak_note(&s, d0.plus(8), "de")));
    }

    #[test]
    fn freezes_renew_with_the_month() {
        let d0 = Day::from_ymd(2026, 10, 28);
        // Missed the 30th and 31st (two freezes in October) and the 1st (a new one).
        let r = days(d0, &[5, 5, 0, 0, 0, 5]);
        let s = streak(&r, DailyGoal::default(), &[], d0.plus(5));
        assert_eq!(s.freezes_used.len(), 3);
        assert!(s.ended_on.is_empty());
        assert_eq!(s.days, 3);
    }

    #[test]
    fn pause_mode_neither_counts_nor_breaks() {
        let d0 = Day::from_ymd(2026, 10, 1);
        let r = days(d0, &[5, 5, 0, 0, 0, 0, 0, 5]);
        let p = [Pause {
            from: d0.plus(2),
            to: d0.plus(6),
        }];
        let s = streak(&r, DailyGoal::default(), &p, d0.plus(7));
        assert_eq!((s.days, s.freezes_used.len()), (3, 0));
    }

    #[test]
    fn today_does_not_break_before_it_is_over() {
        let d0 = Day::from_ymd(2026, 10, 1);
        let r = days(d0, &[5, 5]);
        let s = streak(&r, DailyGoal::default(), &[], d0.plus(2));
        assert_eq!(s.days, 2);
        assert!(!s.today_met);
        assert!(s.freezes_used.is_empty());
    }

    #[test]
    fn xp_rewards_speaking_not_gaming() {
        let spoken = turn_xp(
            "ドイツのハンブルクから来ました。",
            Input::Spoken,
            &[],
            false,
        );
        assert_eq!(spoken, (XP_SPOKEN, None));
        assert_eq!(
            turn_xp("ドイツのハンブルクから来ました。", Input::Typed, &[], false).1,
            Some(XpReason::Typed)
        );
        assert_eq!(
            turn_xp("はい", Input::Spoken, &[], false).1,
            Some(XpReason::OneWord)
        );
        assert_eq!(
            turn_xp("いいですね。", Input::Spoken, &[], false).1,
            Some(XpReason::OneWord)
        );
        assert_eq!(
            turn_xp(
                "ビールをください",
                Input::Spoken,
                &["ビールをください。"],
                false
            ),
            (0, Some(XpReason::Repeated))
        );
        let mut s = SessionXp::default();
        s.add_turn(turn_xp("はい", Input::Spoken, &[], false));
        s.add_turn(turn_xp("はい", Input::Spoken, &["はい"], false));
        assert_eq!(s.explain("de").len(), 2);
        assert!(s.explain("en").iter().all(|l| blameless(l)));
    }

    #[test]
    fn goals_by_minutes_or_scenarios() {
        let r = DayRecord {
            speaking_secs: 150,
            scenarios_done: 1,
            ..Default::default()
        };
        assert!(!r.met(DailyGoal::Minutes(5)));
        assert!((r.fraction(DailyGoal::Minutes(5)) - 0.5).abs() < 1e-9);
        assert!(r.met(DailyGoal::Scenarios(1)));
    }
}
