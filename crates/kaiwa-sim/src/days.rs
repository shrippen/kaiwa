//! Long runs over simulated days (Lastenheft /LL602/): the learning model over 30 days with an
//! accelerated clock, for adaptivity, spaced repetition, the learning path and the streak.
//!
//! The learner is simulated at the level of turns, not text: a persona has a true level per
//! skill (its `start_level`), and a turn goes well with 80 % probability when the material is
//! at that level, more often below it and less often above (the premise of /LF210/). Planted
//! errors come with the persona's `rate` and fade after `fades_after` corrections. Everything
//! else is Kaiwa's real code: learner model, regulator, curriculum, FSRS, daily goal, streak, XP.
//! A day script (`qa/scripts/<journey>.json` with `"days"`) sets the start date, the seed and
//! special events (sessions over midnight, a time zone change).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::curriculum::{CourseProgress, Curriculum, NodeState};
use kaiwa_core::difficulty::{Difficulty, KNOBS, Regulator};
use kaiwa_core::learner::{Evidence, LearnerModel, Skill, level_index};
use kaiwa_core::progress::{
    self, DailyGoal, DayRecord, XP_GOAL, XP_REVIEW, XP_SPOKEN, XP_TYPED, streak, streak_note,
};
use kaiwa_core::srs::{self, Grade, Item};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::run::Outcome;

#[derive(Debug, Clone, Deserialize)]
pub struct DaysScript {
    #[allow(dead_code)]
    pub journey: String,
    pub days: u32,
    #[serde(default)]
    pub seed: u64,
    /// First day, ISO date.
    pub start: String,
    /// Local offset in hours at the start (Germany in winter: 1).
    #[serde(default = "one")]
    pub offset_hours: i32,
    #[serde(default)]
    pub daily_goal_minutes: Option<u32>,
    /// Share of turns typed instead of spoken.
    #[serde(default)]
    pub typed_share: f64,
    /// Practise every day outside the persona's breaks (instead of `days_per_week`).
    #[serde(default)]
    pub every_day: bool,
    /// Days (0-based) on which a session starts at 23:58 and ends at 00:03.
    #[serde(default)]
    pub midnight_days: Vec<u32>,
    #[serde(default)]
    pub tz_change: Option<TzChange>,
}

fn one() -> i32 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct TzChange {
    pub day: u32,
    pub offset_hours: i32,
}

/// One simulated session.
#[derive(Debug, Clone, Serialize)]
pub struct SimSession {
    pub sim_day: u32,
    pub start: Moment,
    pub minutes: f64,
    pub scenario: String,
    pub success: f64,
    pub turns: u32,
    pub typed: u32,
    pub reviews: u32,
    pub refresher: bool,
    pub changes: usize,
    /// Sum of all knob steps after the session.
    pub level: u32,
    pub xp: u32,
}

/// The result of a long run.
pub struct DaysRun {
    pub script: DaysScript,
    pub goal: DailyGoal,
    pub gamification: bool,
    pub sessions: Vec<SimSession>,
    pub records: BTreeMap<Day, DayRecord>,
    pub items: Vec<Item>,
    /// Planted error id → (times corrected, times its item was reviewed while its rate was ≥ 0.1).
    pub errors: BTreeMap<String, (u32, u32)>,
    pub course: CourseProgress,
    pub curriculum: Curriculum,
    pub regulator: Regulator,
    pub learner: LearnerModel,
    pub first_day: Day,
    pub last_day: Day,
    /// Everything said to the learner about progress (notes, XP lines).
    pub texts: Vec<String>,
}

/// SplitMix64: small, fixed, reproducible (/LL603/).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
    fn chance(&mut self, p: f64) -> bool {
        self.next() < p
    }
    fn between(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Probability of success with ability `a` on material at level `l`: 80 % when they match.
pub(crate) fn p_success(a: f64, l: f64) -> f64 {
    logistic(4f64.ln() + 2.0 * (a - l))
}

fn level_of(persona: &Value, key: &str) -> f64 {
    persona["start_level"][key]
        .as_str()
        .and_then(level_index)
        .unwrap_or(1.0)
}

/// Every "HH:MM" in a session plan's `when`, as seconds; 19:00 if there is none.
fn times_of(when: &str) -> Vec<i64> {
    let re = regex::Regex::new(r"(\d{1,2}):(\d{2})").expect("valid regex");
    let t: Vec<i64> = re
        .captures_iter(when)
        .filter_map(|c| Some(c[1].parse::<i64>().ok()? * 3600 + c[2].parse::<i64>().ok()? * 60))
        .collect();
    if t.is_empty() { vec![19 * 3600] } else { t }
}

struct PlannedError {
    id: String,
    kind: String,
    right: String,
    rate: f64,
    fades_after: Option<f64>,
    shown: bool,
}

pub fn simulate(root: &Path, persona: &Value, script: &DaysScript) -> Result<DaysRun> {
    let curriculum =
        Curriculum::from_dir(&root.join("data/curriculum")).context("data/curriculum")?;
    let mut rng = Rng(script.seed ^ 0x006B_6169_7761);
    let first_day = {
        let p: Vec<u32> = script
            .start
            .split('-')
            .filter_map(|x| x.parse().ok())
            .collect();
        Day::from_ymd(p[0] as i32, p[1], p[2])
    };
    let amount_little = persona["learning"]["feedback_preference"].as_str() == Some("little");
    let gamification = persona["learning"]["gamification"].as_str() != Some("off");
    let goal = DailyGoal::Minutes(script.daily_goal_minutes.unwrap_or(5));
    let hint_usage = persona["learning"]["hint_usage"].as_f64().unwrap_or(0.3);
    let forgetting = match persona["learning"]["forgetting"].as_str() {
        Some("high") => 0.012,
        Some("low") => 0.003,
        _ => 0.006,
    };
    // The truth the simulation plays, and the model Kaiwa builds of it (placement: the truth).
    let mut ability: BTreeMap<Skill, f64> = BTreeMap::new();
    ability.insert(Skill::Listening, level_of(persona, "listening"));
    ability.insert(Skill::Speaking, level_of(persona, "speaking"));
    let start_ability = ability.clone();
    let mut learner = LearnerModel::placed(&[
        (
            Skill::Listening,
            persona["start_level"]["listening"].as_str().unwrap_or("a0"),
        ),
        (
            Skill::Speaking,
            persona["start_level"]["speaking"].as_str().unwrap_or("a0"),
        ),
        (
            Skill::Fluency,
            persona["start_level"]["speaking"].as_str().unwrap_or("a0"),
        ),
        (
            Skill::ReadingKana,
            persona["start_level"]["reading_kana"]
                .as_str()
                .unwrap_or("none"),
        ),
        (
            Skill::ReadingKanji,
            persona["start_level"]["reading_kanji"]
                .as_str()
                .unwrap_or("none"),
        ),
        (
            Skill::Register,
            persona["start_level"]["register"]
                .as_str()
                .unwrap_or("none"),
        ),
    ]);
    let mut regulator = Regulator::new(Difficulty::for_learner(&learner));
    let mut planned: Vec<PlannedError> = persona["language_errors"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| PlannedError {
            id: e["id"].as_str().unwrap_or_default().into(),
            kind: e["type"].as_str().unwrap_or_default().into(),
            right: e["right"]
                .as_str()
                .unwrap_or_default()
                .split('／')
                .next()
                .unwrap_or_default()
                .into(),
            rate: e["rate"].as_f64().unwrap_or(0.0),
            fades_after: e["fades_after"].as_f64(),
            shown: match e["expected_severity"].as_str() {
                Some("blocks_understanding" | "wrong") => true,
                Some("unnatural") => !amount_little,
                _ => false,
            },
        })
        .collect();
    let mut corrected: BTreeMap<String, u32> = BTreeMap::new();
    let mut errors: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let breaks: Vec<(u32, u32)> = persona["learning"]["breaks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| {
            let after = b["after_day"].as_u64()? as u32;
            Some((after + 1, after + b["days"].as_u64()? as u32))
        })
        .collect();
    let plans: Vec<Value> = persona["learning"]["sessions"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut course = CourseProgress::default();
    let mut items: Vec<Item> = Vec::new();
    let mut records: BTreeMap<Day, DayRecord> = BTreeMap::new();
    let mut sessions = Vec::new();
    let mut texts = Vec::new();
    let mut last_practice: Option<Moment> = None;
    let mut offset = script.offset_hours * 60;

    for sim_day in 0..script.days {
        if let Some(tz) = &script.tz_change
            && sim_day == tz.day
        {
            offset = tz.offset_hours * 60;
        }
        let date = first_day.plus(i64::from(sim_day));
        let on_break = breaks.iter().any(|(a, b)| (*a..=*b).contains(&sim_day));
        // The truth forgets a little every day.
        for (s, a) in ability.iter_mut() {
            *a = (*a - forgetting).max(start_ability[s] - 0.3);
        }
        if on_break {
            continue;
        }
        let mut todays: Vec<(i64, f64, String)> = Vec::new();
        for plan in &plans {
            let when = plan["when"].as_str().unwrap_or("");
            let dpw = plan["days_per_week"].as_f64().unwrap_or(3.0);
            let weekday = date.weekday();
            // "werktags" plays Monday to Friday, "Sonntag…" Sundays, anything else any day.
            let (fits, p) = if when.contains("werktags") {
                (weekday < 5, dpw / 5.0)
            } else if when.contains("Sonntag") {
                (weekday == 6, dpw)
            } else {
                (true, dpw / 7.0)
            };
            let times = times_of(when);
            let m = plan["minutes"].as_array();
            let lo = m.and_then(|m| m[0].as_f64()).unwrap_or(5.0);
            let hi = m.and_then(|m| m[1].as_f64()).unwrap_or(lo);
            let surface: String = plan["surface"].as_str().unwrap_or("app_plasma").into();
            if script.every_day {
                todays.push((times[0], rng.between(lo, hi).max(6.0), surface));
                break;
            }
            if fits && rng.chance(p.min(1.0)) {
                for t in times {
                    todays.push((t, rng.between(lo, hi), surface.clone()));
                }
            }
        }
        if script.midnight_days.contains(&sim_day) {
            todays = vec![(23 * 3600 + 58 * 60, 5.0, "app_plasma".into())];
        }
        if todays.is_empty() && script.every_day {
            todays.push((19 * 3600, 6.0, "app_plasma".into()));
        }
        for (secs, minutes, surface) in todays {
            let at = date.start(offset).plus_secs(secs);
            let mut xp = 0;

            // Reviews first (/LF230/, /LF231/): five in the plasmoid, ten in the app.
            let limit = if surface == "plasmoid" { 5 } else { 10 };
            let refresher = srs::refresher(at, last_practice);
            if refresher {
                texts.push(streak_note(
                    &streak(&records, goal, &[], at.day()),
                    at.day(),
                    "de",
                ));
            }
            let due: Vec<String> = srs::due(&items, at, last_practice, limit)
                .iter()
                .map(|i| i.text.clone())
                .collect();
            let mut reviews = 0;
            for text in &due {
                let Some(item) = items.iter_mut().find(|i| &i.text == text) else {
                    continue;
                };
                let p = 0.45 + 0.5 * item.retrievability(at);
                let right = rng.chance(p);
                let helped = rng.chance(hint_usage * 0.5);
                item.review(Grade::of(right, helped, false), at);
                if let Some(unit) = &item.unit {
                    course.reviewed(unit, right);
                }
                if let Some(e) = planned.iter().find(|e| e.right == item.text) {
                    let rate = fade(e, &corrected);
                    if rate >= 0.1 {
                        errors.entry(e.id.clone()).or_default().1 += 1;
                    }
                }
                reviews += 1;
                xp += XP_REVIEW;
            }

            // The conversation.
            let scenario = curriculum
                .next_scenario(&course)
                .unwrap_or_else(|| "n5-12-kinou".into());
            let unit = curriculum.unit_of(&scenario).map(|u| u.id.clone());
            let d = regulator.difficulty.clone();
            let (lis, con) = (d.listening_level(), d.content_level());
            let turns = (minutes * 2.0).round().max(2.0) as u32;
            let mut st = kaiwa_core::Stats::default();
            let mut clean = 0u32;
            let mut typed = 0u32;
            for _ in 0..turns {
                st.turns += 1;
                let p_l = p_success(ability[&Skill::Listening], lis);
                let p_s = p_success(ability[&Skill::Speaking], con);
                if rng.chance(script.typed_share) {
                    typed += 1;
                    xp += XP_TYPED;
                } else {
                    xp += XP_SPOKEN;
                }
                if rng.chance(p_l) {
                    st.understood += 1;
                    st.understood_unaided += 1;
                } else if rng.chance(hint_usage) {
                    st.understood += 1;
                    st.aids += 1;
                }
                if rng.chance(p_s) {
                    clean += 1;
                }
            }
            for e in planned.iter_mut() {
                let rate = fade(e, &corrected);
                if rng.chance(rate) {
                    *st.mistakes.entry(e.kind.clone()).or_default() += 1;
                    if e.shown {
                        *corrected.entry(e.id.clone()).or_default() += 1;
                        errors.entry(e.id.clone()).or_default().0 += 1;
                        let _ = srs::add_from_summary(
                            &mut items,
                            std::slice::from_ref(&e.right),
                            &[(e.right.clone(), e.kind.clone())],
                            unit.as_deref(),
                            at,
                        );
                    }
                }
            }
            let goals = d.goals().min(4) as u32;
            for _ in 0..goals {
                let p_s = p_success(ability[&Skill::Speaking], con);
                if rng.chance(p_s) {
                    st.goals_met += 1;
                    st.goals_unaided += 1;
                } else if rng.chance(hint_usage + 0.3) {
                    st.goals_met += 1;
                    st.helps += 1;
                }
            }
            xp += XP_GOAL * st.goals_met;
            let success = st.success();
            if st.goals_met == goals {
                course.complete(&scenario, success);
            }
            // Two new words of the unit per session go into the review (the app takes them
            // from the analysis).
            if let Some(u) = curriculum.unit_of(&scenario) {
                let fresh: Vec<String> = u
                    .words
                    .iter()
                    .filter(|w| !items.iter().any(|i| &i.text == *w))
                    .take(2)
                    .cloned()
                    .collect();
                srs::add_from_summary(&mut items, &fresh, &[], Some(&u.id), at);
            }

            // The model learns from the session; the regulator moves one knob at most.
            let mut ev = Evidence {
                mistakes: st.mistakes.clone(),
                turns: st.turns,
                ..Default::default()
            };
            ev.at_level.insert(
                Skill::Listening,
                (lis, f64::from(st.understood_unaided) / f64::from(st.turns)),
            );
            ev.at_level.insert(
                Skill::Speaking,
                (con, f64::from(clean) / f64::from(st.turns)),
            );
            if st.goals_met > 0 {
                ev.at_level.insert(
                    Skill::Fluency,
                    (con, f64::from(st.goals_unaided) / f64::from(st.goals_met)),
                );
            }
            learner.update(&ev);
            let change = regulator.after_session(success, &learner);

            // The truth learns from practice, with diminishing returns per session: about a
            // tenth of a level per week of short daily sessions.
            for a in ability.values_mut() {
                *a += 0.05 * (1.0 - (-minutes / 20.0).exp());
            }

            let spoken_share = 1.0 - f64::from(typed) / f64::from(turns);
            let rec = records.entry(at.day()).or_default();
            rec.speaking_secs += (minutes * 60.0 * spoken_share).round() as u32;
            rec.typed_turns += typed;
            rec.reviews_done += reviews;
            rec.xp += xp;
            if st.goals_met == goals {
                rec.scenarios_done += 1;
            }
            last_practice = Some(at.plus_secs((minutes * 60.0) as i64));
            sessions.push(SimSession {
                sim_day,
                start: at,
                minutes,
                scenario,
                success,
                turns,
                typed,
                reviews,
                refresher,
                changes: usize::from(change.is_some()),
                level: KNOBS
                    .iter()
                    .map(|k| u32::from(regulator.difficulty.get(*k)))
                    .sum(),
                xp,
            });
        }
    }
    let last_day = first_day.plus(i64::from(script.days) - 1);
    if gamification {
        texts.push(streak_note(
            &streak(&records, goal, &[], last_day),
            last_day,
            "de",
        ));
    }
    Ok(DaysRun {
        script: script.clone(),
        goal,
        gamification,
        sessions,
        records,
        items,
        errors,
        course,
        curriculum,
        regulator,
        learner,
        first_day,
        last_day,
        texts,
    })
}

/// The rate of a planted error after its corrections: halves with every `fades_after` of them.
fn fade(e: &PlannedError, corrected: &BTreeMap<String, u32>) -> f64 {
    match e.fades_after {
        Some(n) if n > 0.0 => {
            e.rate * 0.5f64.powf(f64::from(*corrected.get(&e.id).unwrap_or(&0)) / n)
        }
        _ => e.rate,
    }
}

fn moving(sessions: &[SimSession], n: usize) -> Vec<f64> {
    sessions
        .windows(n)
        .map(|w| w.iter().map(|s| s.success).sum::<f64>() / n as f64)
        .collect()
}

/// The automated form of a day-run check. `None`: not automated yet.
pub fn check(id: &str, run: &DaysRun) -> Option<Outcome> {
    let fail = |m: String| Some(Outcome::Fail(m));
    let pass = Some(Outcome::Pass);
    match id {
        // /LF210/: moving success over 5 sessions between 65 % and 92 %.
        "mara_success_band" => {
            let m = moving(&run.sessions, 5);
            match m.iter().find(|s| !(0.65..=0.92).contains(*s)) {
                Some(s) => fail(format!("moving success {s:.2} outside 0.65–0.92")),
                None if m.is_empty() => fail("fewer than five sessions".into()),
                None => pass,
            }
        }
        // /LF214/: at most one knob, one step per session; harder over the first 17 days.
        "mara_gradual" => {
            if run.regulator.changes.iter().any(|c| c.len() > 1) {
                return fail("a session changed more than one knob".into());
            }
            let first = run.sessions.first()?.level;
            let day17 = run.sessions.iter().rev().find(|s| s.sim_day <= 17)?.level;
            if day17 <= first {
                return fail(format!("difficulty {first} → {day17} over 17 days"));
            }
            pass
        }
        // /LF230/: the corrected forms come back in the review while the error persists.
        "mara_srs_errors" => {
            for id in ["mara_suki_o", "mara_counter"] {
                match run.errors.get(id) {
                    Some((c, r)) if *c > 0 && *r == 0 => {
                        return fail(format!(
                            "{id} corrected {c}×, never reviewed while it lasted"
                        ));
                    }
                    None => return fail(format!("{id} never came up in 30 days")),
                    _ => {}
                }
            }
            pass
        }
        // /LF231/, /LF602/: after the break a refresher of at most ten, no blame.
        "mara_comeback" => {
            let back = run.sessions.iter().find(|s| s.refresher)?;
            if back.reviews > srs::REFRESHER_SIZE as u32 {
                return fail(format!("{} reviews after the break", back.reviews));
            }
            match run.texts.iter().find(|t| !progress::blameless(t)) {
                Some(t) => fail(format!("blame in: {t}")),
                None => pass,
            }
        }
        // /LF212/: help lowers the model's evidence, never shows as a penalty.
        "mara_hints_silent" => {
            let mut with = Evidence::default();
            with.at_level.insert(Skill::Fluency, (3.0, 0.5));
            let mut without = Evidence::default();
            without.at_level.insert(Skill::Fluency, (3.0, 1.0));
            let (mut a, mut b) = (LearnerModel::default(), LearnerModel::default());
            a.update(&with);
            b.update(&without);
            if a.level(Skill::Fluency) >= b.level(Skill::Fluency) {
                return fail("help does not lower the evidence".into());
            }
            match run.texts.iter().find(|t| {
                let t = t.to_lowercase();
                t.contains("abzug") || t.contains("strafe") || t.contains("penalty")
            }) {
                Some(t) => fail(format!("shown as a penalty: {t}")),
                None => pass,
            }
        }
        // /LF601/: the goal is the learner's; typed turns add no speaking time.
        "mara_daily_goal" => {
            let typed: u32 = run.sessions.iter().map(|s| s.typed).sum();
            if run.script.typed_share > 0.0 && typed == 0 {
                return fail("the run typed nothing".into());
            }
            for s in &run.sessions {
                let spoken = s.minutes * 60.0 * (1.0 - f64::from(s.typed) / f64::from(s.turns));
                if spoken > s.minutes * 60.0 + 0.5 {
                    return fail("typed turns counted as speaking".into());
                }
            }
            let met = run.records.values().filter(|r| r.met(run.goal)).count();
            let goal_secs = match run.goal {
                DailyGoal::Minutes(m) => m * 60,
                DailyGoal::Scenarios(_) => 0,
            };
            if run
                .records
                .values()
                .any(|r| r.met(run.goal) != (r.speaking_secs >= goal_secs))
            {
                return fail("the daily goal ignores the chosen minutes".into());
            }
            if met == 0 {
                return fail("the goal was never met".into());
            }
            pass
        }
        // /LF600/, /LF220/: units unlock along the graph; states have words.
        "mara_path" | "hinnerk_pace" => {
            let path = run.curriculum.path(&run.course, "de");
            for u in &path {
                if u.state != NodeState::Locked {
                    for r in &run.curriculum.unit(&u.id)?.requires {
                        let req = path.iter().find(|p| &p.id == r)?;
                        if req.state != NodeState::Done {
                            return fail(format!("{} open before {r} is done", u.id));
                        }
                    }
                }
                if u.state.word("de").is_empty() {
                    return fail("a state without a word".into());
                }
            }
            if !path.iter().any(|u| u.state == NodeState::Done) {
                return fail("no unit done in 30 days".into());
            }
            if id == "hinnerk_pace" {
                let m = moving(&run.sessions, 5);
                if let Some(s) = m.iter().find(|s| **s < 0.65) {
                    return fail(format!("moving success {s:.2} below 0.65"));
                }
            }
            pass
        }
        // /LF620/, /LL300/: nothing about missed days, nothing with blame; no streak without
        // gamification.
        "hinnerk_no_pressure" => {
            if !run.gamification
                && run
                    .texts
                    .iter()
                    .any(|t| t.contains("Serie") || t.contains("in Folge"))
            {
                return fail("streak text with gamification off".into());
            }
            match run.texts.iter().find(|t| !progress::blameless(t)) {
                Some(t) => fail(format!("blame in: {t}")),
                None => pass,
            }
        }
        // /LF210/: never two sessions in a row at 98 % or more (underchallenged).
        "emi_no_baby_talk" => {
            match run
                .sessions
                .windows(2)
                .find(|w| w[0].success >= 0.98 && w[1].success >= 0.98)
            {
                Some(w) => fail(format!(
                    "days {} and {} at {:.2} and {:.2}",
                    w[0].sim_day, w[1].sim_day, w[0].success, w[1].success
                )),
                None if run.sessions.len() < 2 => fail("fewer than two sessions".into()),
                None => pass,
            }
        }
        // /LF602/: a session from 23:58 to 00:03 counts for the day it started.
        "jonas_midnight" => {
            for d in &run.script.midnight_days {
                let s = run.sessions.iter().find(|s| s.sim_day == *d)?;
                let start_day = s.start.day();
                if start_day != run.first_day.plus(i64::from(*d)) {
                    return fail(format!(
                        "the session of day {d} landed on {}",
                        start_day.iso()
                    ));
                }
                let next = run.records.get(&start_day.plus(1));
                let midnight_secs = (s.minutes * 60.0) as u32;
                if next.is_some_and(|r| r.speaking_secs >= midnight_secs && r.met(run.goal))
                    && !run
                        .sessions
                        .iter()
                        .any(|o| o.start.day() == start_day.plus(1))
                {
                    return fail("the next day was met by the midnight session".into());
                }
            }
            pass
        }
        // /LF602/: of three missed days two are covered by freezes, the third ends the streak.
        "jonas_freeze" => {
            let s = streak(&run.records, run.goal, &[], run.last_day);
            if s.freezes_used.len() != 2 || s.ended_on.len() != 1 {
                return fail(format!(
                    "freezes {:?}, ended {:?}",
                    s.freezes_used.iter().map(|d| d.iso()).collect::<Vec<_>>(),
                    s.ended_on.iter().map(|d| d.iso()).collect::<Vec<_>>()
                ));
            }
            let note = streak_note(&s, s.ended_on[0].plus(1), "de");
            if !progress::blameless(&note) {
                return fail(format!("blame in: {note}"));
            }
            pass
        }
        // /LF602/: the time zone change neither costs nor gives a day.
        "jonas_timezone" => {
            let tz = run.script.tz_change.as_ref()?;
            let around: Vec<Day> = (tz.day.saturating_sub(2)..tz.day + 3)
                .map(|d| run.first_day.plus(i64::from(d)))
                .collect();
            for d in &around {
                let n = run.sessions.iter().filter(|s| s.start.day() == *d).count();
                if n != 1 {
                    return fail(format!("{} has {n} sessions around the change", d.iso()));
                }
            }
            let before = streak(&run.records, run.goal, &[], around[1]).days;
            let after = streak(&run.records, run.goal, &[], around[3]).days;
            if after != before + 2 {
                return fail(format!("streak {before} → {after} over two days"));
            }
            pass
        }
        _ => None,
    }
}

/// A run's day script, if the journey has one.
pub fn script_for(root: &Path, journey: &str) -> Result<Option<DaysScript>> {
    let path = root.join(format!("qa/scripts/{journey}.json"));
    if !path.exists() {
        return Ok(None);
    }
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    if v.get("days").is_none() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_value(v)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn persona(id: &str) -> Value {
        serde_json::from_str(
            &std::fs::read_to_string(root().join(format!("qa/personas/{id}.json"))).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn runs_are_reproducible() {
        let s = script_for(&root(), "mara_30_days").unwrap().unwrap();
        let a = simulate(&root(), &persona("mara"), &s).unwrap();
        let b = simulate(&root(), &persona("mara"), &s).unwrap();
        assert_eq!(a.sessions.len(), b.sessions.len());
        assert_eq!(
            a.sessions.iter().map(|s| s.success).collect::<Vec<_>>(),
            b.sessions.iter().map(|s| s.success).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_broken_regulator_fails_the_band() {
        let s = script_for(&root(), "mara_30_days").unwrap().unwrap();
        let mut run = simulate(&root(), &persona("mara"), &s).unwrap();
        for x in run.sessions.iter_mut().take(5) {
            x.success = 0.4;
        }
        assert!(matches!(
            check("mara_success_band", &run),
            Some(Outcome::Fail(_))
        ));
    }
}
