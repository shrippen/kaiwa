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
        .filter(|s| s.unit == "frei")
        .map(|s| {
            let topic = s.title.get(lang);
            let topic = topic.rsplit(": ").next().unwrap_or(topic);
            json!({ "id": s.id, "topic": topic })
        })
        .collect();
    // No tutor and no recordings (a package without a tutor set up): say so first (Q-15).
    let needs_tutor =
        !crate::DEMO && !kaiwa_ai::provider_from(st.tutor_env()).is_ok_and(|p| p.is_some());
    Ok(json!({
        "free": free,
        "needsTutor": needs_tutor,
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

/// The learning path (/LF600/): one node per scenario, units as meta, states with words, and
/// the level each belongs to („N5 · A1“, Q-03).
pub fn path(st: &Backend) -> anyhow::Result<Value> {
    let p = st.store.profile()?;
    let lang = p.ui_lang.as_str();
    let learning = st.store.learning()?;
    let items = st.store.items()?;
    let today = crate::now().day();
    let mut nodes = Vec::new();
    for unit in st.curriculum.path(&learning.course, lang) {
        let number = unit.id.to_uppercase();
        for sc in &unit.scenarios {
            let state = match sc.state {
                NodeState::Done => "done",
                NodeState::Current => "current",
                NodeState::Open => "open",
                NodeState::Locked => "locked",
            };
            let meta = match (&unit.locked_because, sc.state) {
                (Some(why), NodeState::Locked) => format!("{number} · {why}"),
                _ => format!("{number} · {}", unit.can_do),
            };
            nodes.push(json!({
                "kind": "scenario",
                "id": sc.id,
                "title": scenario_title(st, &sc.id, lang),
                "meta": meta,
                "state": state,
                "level": unit.level,
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
                // Reviews that go badly are recommended, they lock nothing (T-06).
                "meta": match (lang == "de", unit.shaky) {
                    (true, false) => format!("{due} fällig"),
                    (true, true) => format!("{due} fällig · empfohlen, bevor es weitergeht"),
                    (false, false) => format!("{due} due"),
                    (false, true) => format!("{due} due · recommended before you go on"),
                },
                "state": "review",
                "level": unit.level,
            }));
        }
    }
    Ok(Value::Array(nodes))
}

/// The items to review now, at most `limit` (/LF230/, /LF502/: five in the plasmoid).
pub fn review(st: &Backend, now: Moment, limit: usize) -> anyhow::Result<Value> {
    let due = st.store.due(now, limit)?;
    let de = st.store.profile()?.ui_lang == "de";
    Ok(Value::Array(
        due.iter()
            .map(|i| {
                json!({
                    "text": i.text,
                    "kind": i.kind,
                    "unit": i.unit,
                    "reps": i.reps,
                    "fromMistake": matches!(i.source, kaiwa_core::srs::Source::Correction { .. }),
                    // Taken over from the word lookup (/LF112/).
                    "fromLookup": i.source == kaiwa_core::srs::Source::Lookup,
                    // The character's line to answer: a mini conversation (/LF230/).
                    // The line it answered, with furigana over every kanji like in the talk.
                    "cue": i.cue.as_deref().map(|c| kaiwa_lang::complete(crate::lookup::words(), c)),
                    // What the correction was about, in words (Höflichkeit, Partikeln …).
                    "practises": match &i.source {
                        kaiwa_core::srs::Source::Correction { kind } => kind_word(kind, de),
                        _ => "",
                    },
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
    // Grammar points (/LF201/): how many sit, which are still practised, with the catalogue's
    // explanation in the UI language.
    let mut practising: Vec<(&String, &kaiwa_core::grammar::PointState)> = l
        .grammar
        .iter()
        .filter(|(_, s)| s.status() == "practising")
        .collect();
    practising.sort_by_key(|(_, s)| std::cmp::Reverse((s.wrong, s.last)));
    let grammar = json!({
        "firm": l.grammar.values().filter(|s| s.status() == "firm").count(),
        "seen": l.grammar.len(),
        "practising": practising
            .iter()
            .take(5)
            .filter_map(|(id, s)| st.grammar.get(id).map(|p| json!({
                "label": p.label,
                "explain": p.explain.get(lang),
                "wrong": s.wrong,
                "used": s.used,
            })))
            .collect::<Vec<_>>(),
    });
    let offer = skip_offer(st)?;
    Ok(json!({
        "skills": skills,
        "weak": weak,
        "grammar": grammar,
        "knobs": knobs,
        "speed": d.speed(),
        "offerSkip": offer.is_some(),
        // The unit the offer is about (T-08): the one the learner is in now.
        "skipUnit": offer.map(|u| json!({ "id": u.id, "title": u.title })),
    }))
}

/// The unit the skip offer is about (/LF215/, T-08), if there is an offer: three sessions in a
/// row went almost without a slip since the last skip, and the unit the learner is in now is
/// not one with keigo while the learner's register lacks it (as after the placement,
/// `Curriculum::placement_skip`: nobody skips into or over keigo on easy talks in plain form).
pub fn skip_offer(st: &Backend) -> anyhow::Result<Option<kaiwa_core::curriculum::UnitNode>> {
    use kaiwa_core::learner::{Skill, level_index};
    let l = st.store.learning()?;
    if !l.regulator.offer_skip() {
        return Ok(None);
    }
    let lang = st.store.profile()?.ui_lang;
    let Some(unit) = st
        .curriculum
        .path(&l.course, &lang)
        .into_iter()
        .find(|u| u.state == NodeState::Current)
    else {
        return Ok(None);
    };
    let keigo = unit.scenarios.iter().any(|s| {
        st.scenarios
            .get(&s.id)
            .is_some_and(|s| s.register == kaiwa_core::scenario::Register::Keigo)
    });
    let keigo_ok = l.model.level(Skill::Register) >= level_index("keigo_basic").unwrap_or(f64::MAX);
    Ok((keigo_ok || !keigo).then_some(unit))
}

/// Takes the skip offer for `unit` (the button at the path's hint): the unit counts as done and
/// the offer is used up, the next one needs three new sessions. Nothing happens when the offer
/// is not about `unit` (a second click, an old page).
pub fn take_skip(st: &Backend, unit: &str) -> anyhow::Result<bool> {
    if skip_offer(st)?.is_none_or(|u| u.id != unit) {
        return Ok(false);
    }
    let mut l = st.store.learning()?;
    l.course.skipped_units.insert(unit.to_string());
    l.regulator.took_skip();
    st.store.set_learning(&l)?;
    Ok(true)
}

/// Talks before a kind of mistake gets a trend, and how often it must have come up in them;
/// with fewer there is only the count (one mistake is no trend).
const TREND_TALKS: usize = 4;
const TREND_MISTAKES: u32 = 3;

/// How a kind of mistake developed (/LF610/): how often it came up per turn in the later half
/// of the talks against the earlier half. `talks` are the talks in order, each with its turns
/// (without those marked as misrecognised, whose mistakes do not count either) and its
/// mistakes of this kind. Talks, not weeks: a week that has just begun, or the one the
/// learner started in, holds few talks and would read as a change. A quarter more or less
/// counts as a change.
fn trend(word: &str, talks: &[(u32, u32)], de: bool) -> String {
    let rate = |t: &[(u32, u32)]| {
        let (turns, mistakes) = t
            .iter()
            .fold((0, 0), |(a, b), (turns, m)| (a + turns, b + m));
        f64::from(mistakes) / f64::from(turns.max(1))
    };
    let n: u32 = talks.iter().map(|t| t.1).sum();
    if talks.len() < TREND_TALKS || n < TREND_MISTAKES {
        return if de {
            format!("{word}: bisher {n}×")
        } else {
            format!("{word}: {n}× so far")
        };
    }
    let (early, late) = talks.split_at(talks.len() / 2);
    let (early, late) = (rate(early), rate(late));
    match (late, de) {
        (l, true) if l < 0.75 * early => format!("{word}: seltener als in den Gesprächen davor"),
        (l, false) if l < 0.75 * early => format!("{word}: less often than in the talks before"),
        (l, true) if l > 1.25 * early => {
            format!("{word}: zuletzt häufiger, ein Fall für die Wiederholung")
        }
        (l, false) if l > 1.25 * early => format!("{word}: more often lately, one for the review"),
        (_, true) => format!("{word}: etwa gleich oft"),
        (_, false) => format!("{word}: about as often"),
    }
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
    // The totals of the same twelve weeks (not of all time), minutes from the seconds summed:
    // days of 90 s each are not a minute each.
    let shown = || days.range(start..=today).map(|(_, r)| r);
    let minutes: u32 = shown().map(|r| r.speaking_secs).sum::<u32>() / 60;
    let practice_days = shown().filter(|r| r.speaking_secs > 0).count();
    let items = st.store.items()?;
    // Kinds of mistakes per week over the same twelve weeks (/LF610/).
    let de = st.store.profile()?.ui_lang != "en";
    let from = start.start(now.offset_min).unix;
    let mut weeks: BTreeMap<String, [u32; 12]> = BTreeMap::new();
    // The talks in order, for the trend: the turns of each and its mistakes per kind.
    let mut talks: Vec<(u32, BTreeMap<String, u32>)> = Vec::new();
    for row in st.store.finished_since(from)? {
        let Ok(state) = st.store.session_state(row.id) else {
            continue;
        };
        let w = ((row.started.day().0 - start.0) / 7).clamp(0, 11) as usize;
        let mut made: BTreeMap<String, u32> = BTreeMap::new();
        for t in state.turns.iter().filter(|t| !t.incomplete) {
            for k in kaiwa_core::session::mistake_kinds(t) {
                *made.entry(k.clone()).or_default() += 1;
                weeks.entry(k).or_default()[w] += 1;
            }
        }
        let turns = state
            .turns
            .iter()
            .filter(|t| !t.misrecognized && !t.incomplete)
            .count();
        if turns > 0 {
            talks.push((turns as u32, made));
        }
    }
    let mut kinds: Vec<(String, [u32; 12])> = weeks.into_iter().collect();
    kinds.sort_by_key(|(_, w)| std::cmp::Reverse(w.iter().sum::<u32>()));
    let mistakes: Vec<Value> = kinds
        .iter()
        .take(3)
        .map(|(k, w)| {
            let word = kind_word(k, de);
            let of_kind: Vec<(u32, u32)> = talks
                .iter()
                .map(|(turns, made)| (*turns, made.get(k).copied().unwrap_or(0)))
                .collect();
            json!({ "kind": k, "word": word, "weeks": w, "text": trend(word, &of_kind, de) })
        })
        .collect();
    // Words and phrases per level, from the unit they came from ("n5-04" is N5).
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

/// A skill moved over the week when its level changed by at least this much (a tenth of the
/// step between two levels); less is the model settling.
const SKILL_MOVED: f64 = 0.1;

/// "up", "down" or "same": a skill's level now against where the week started.
fn skill_change(from: f64, now: f64) -> &'static str {
    match now - from {
        d if d >= SKILL_MOVED => "up",
        d if d <= -SKILL_MOVED => "down",
        _ => "same",
    }
}

/// The weekly review (/LF604/): the last seven days against the seven before: speaking time,
/// practice days, new words and phrases, how many turns were understood without help, and the
/// progress per skill.
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
        for t in state
            .turns
            .iter()
            .filter(|t| !t.misrecognized && !t.incomplete)
        {
            unaided[w].1 += 1;
            unaided[w].0 += usize::from(t.understood && !t.aided && !t.from_help);
        }
    }
    let share = |(ok, all): (usize, usize)| (all > 0).then(|| (100 * ok / all) as u32);
    // Progress per skill: the learner model now against where the week's first talk started.
    let de = st.store.profile()?.ui_lang != "en";
    let lang = if de { "de" } else { "en" };
    let model = st.store.learning()?.model;
    let skills: Vec<Value> = days
        .range(this..=today)
        .find(|(_, r)| !r.levels_at_start.is_empty())
        .map(|(_, r)| {
            SKILLS
                .iter()
                .filter_map(|s| {
                    let from = *r.levels_at_start.get(s)?;
                    let change = skill_change(from, model.level(*s));
                    let name = s.word(lang);
                    let text = match (change, de) {
                        ("up", true) => format!("{name}: sicherer als zu Beginn der Woche"),
                        ("up", false) => {
                            format!("{name}: more secure than at the start of the week")
                        }
                        ("down", true) => format!("{name}: braucht wieder etwas Übung"),
                        ("down", false) => format!("{name}: could do with some practice again"),
                        (_, true) => format!("{name}: etwa wie zu Beginn der Woche"),
                        (_, false) => format!("{name}: about as at the start of the week"),
                    };
                    Some(json!({
                        "skill": serde_json::to_value(s).unwrap_or_default(),
                        "name": name,
                        "change": change,
                        "text": text,
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(json!({
        "skills": skills,
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
        // A pause that runs today, with the day it ends on (T-08).
        "pausedUntil": st.store.paused_until(crate::now().day())?.map(|d| {
            let (_, month, day) = d.ymd();
            json!({ "day": day, "month": month, "iso": d.iso() })
        }),
        "textScale": p.text_scale.min(kaiwa_store::TEXT_SCALE_MAX),
        "microphone": p.microphone,
        "cost": st.cost_view(crate::now())?,
        "latency": st.latency_view(&p.ui_lang)?,
        "path": st.path.display().to_string(),
        "profiles": st.profiles(),
        "key": key_view(st, &p.ui_lang),
        "tutor": tutor_view(st, &p.ui_lang),
        "demoData": st.demo_data,
        // "About" (/LD050/): the version, the licence and where the dictionary comes from.
        "about": {
            "version": env!("CARGO_PKG_VERSION"),
            "sources": crate::lookup::sources()
                .into_iter()
                .map(|(name, licence, by)| json!({ "name": name, "licence": licence, "by": by }))
                .collect::<Vec<_>>(),
        },
    }))
}

/// The grammar of a scenario's unit (/LF220/, /LL112/): each point with its explanation in the
/// UI language and examples in furigana markup, in the unit's order. Empty for free talk.
pub fn grammar(st: &Backend, scenario: &str, lang: &str) -> Value {
    let Some(unit) = st.curriculum.unit_of(scenario) else {
        return json!([]);
    };
    let points: Vec<Value> = unit
        .grammar
        .iter()
        .filter_map(|label| st.grammar.by_label(label))
        .map(|p| {
            json!({
                "label": p.label,
                "explain": p.explain.get(lang),
                "examples": p.examples,
                "level": p.level.to_uppercase(),
            })
        })
        .collect();
    json!(points)
}

/// The tutor set up in the settings (Q-15): provider, model and server as stored, what the
/// environment overrides (local test), whether a conversation can start, and the presets to
/// choose from.
pub fn tutor_view(st: &Backend, lang: &str) -> Value {
    use kaiwa_ai::presets::{PRESETS, preset};
    let t = st.tutor();
    let env = st.tutor_env();
    let provider = env("KAIWA_PROVIDER").unwrap_or_default();
    let overridden = std::env::var("KAIWA_PROVIDER").is_ok_and(|v| !v.is_empty());
    let ready = kaiwa_ai::provider_from(st.tutor_env()).is_ok_and(|p| p.is_some());
    let de = lang != "en";
    let p = preset(&provider);
    let missing_key =
        p.is_some_and(|p| p.needs_key()) && kaiwa_ai::secrets::key(&provider, &env).is_none();
    let missing_server = p.is_some_and(|p| p.base_url(&env).is_none());
    let status = match (provider.as_str(), p, de) {
        ("", _, true) => "Kein Tutor eingerichtet. Ohne Tutor gibt es keine Gespräche.",
        ("", _, false) => "No tutor set up. Without a tutor there are no conversations.",
        (_, None, true) => "Diesen Anbieter kennt Kaiwa nicht.",
        (_, None, false) => "Kaiwa does not know this provider.",
        _ if ready && de => "Bereit.",
        _ if ready => "Ready.",
        _ if missing_key && de => "Es fehlt der API-Schlüssel.",
        _ if missing_key => "The API key is missing.",
        _ if missing_server && de => "Es fehlt die Adresse des Servers.",
        _ if missing_server => "The server address is missing.",
        (_, _, true) => "Es fehlt der Name des Modells.",
        (_, _, false) => "The model name is missing.",
    };
    let presets: Vec<Value> = PRESETS
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "label": if p.id == "custom" && !de { "OpenAI-compatible" } else { p.label },
                "local": p.local,
                "needsModel": p.needs_model(),
                "needsServer": p.base.is_empty(),
                "server": p.base,
                "exampleModel": p.example_model,
                "keysAt": p.keys_at,
            })
        })
        .collect();
    json!({
        "provider": t.provider,
        "model": t.model,
        "baseUrl": t.base_url,
        "effective": provider,
        "fromEnvironment": overridden,
        "ready": ready,
        "status": status,
        "presets": presets,
    })
}

/// The tutor's API key (/LF713/): which provider, where its key comes from, in words; `null`
/// for a provider without a key (a server on this computer).
pub fn key_view(st: &Backend, lang: &str) -> Value {
    use kaiwa_ai::secrets::{KeyState, state};
    let env = st.tutor_env();
    let Some(p) = env("KAIWA_PROVIDER")
        .and_then(|id| kaiwa_ai::presets::preset(&id))
        .filter(|p| p.key_env.is_some())
    else {
        return Value::Null;
    };
    let provider = p.id.to_string();
    let de = lang != "en";
    let s = state(&provider, env);
    let text = match (&s, de) {
        (KeyState::Environment, true) => "Aus der Umgebung (lokaler Test).".to_string(),
        (KeyState::Environment, false) => "From the environment (local test).".to_string(),
        (KeyState::Keyring, true) => "Im Schlüsselbund des Systems gespeichert.".to_string(),
        (KeyState::Keyring, false) => "Kept in the system keyring.".to_string(),
        (KeyState::Missing, true) if p.needs_key() => format!(
            "Fehlt. Du bekommst ihn unter {}. Er wird im Schlüsselbund gespeichert, nie in einer Datei.",
            p.keys_at
        ),
        (KeyState::Missing, false) if p.needs_key() => format!(
            "Missing. You get it at {}. It is kept in the keyring, never in a file.",
            p.keys_at
        ),
        (KeyState::Missing, true) => {
            "Nur nötig, wenn der Server einen verlangt. Er wird im Schlüsselbund gespeichert, nie in einer Datei.".to_string()
        }
        (KeyState::Missing, false) => {
            "Only needed when the server asks for one. It is kept in the keyring, never in a file.".to_string()
        }
        (KeyState::Unavailable(e), true) => format!("Kein Schlüsselbund erreichbar ({e})."),
        (KeyState::Unavailable(e), false) => format!("No keyring reachable ({e})."),
    };
    json!({
        "provider": provider,
        "label": if p.id == "custom" && !de { "OpenAI-compatible" } else { p.label },
        "stored": matches!(s, KeyState::Keyring | KeyState::Environment),
        "canStore": !matches!(s, KeyState::Unavailable(_) | KeyState::Environment),
        "text": text,
    })
}

/// "What leaves the device?" (/LF711/), from `kaiwa_ai::privacy`, for the tutor set up.
pub fn privacy(st: &Backend, lang: &str) -> Value {
    serde_json::to_value(kaiwa_ai::privacy::flows(st.tutor_env(), lang)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_is_local_without_a_cloud_provider() {
        let b = crate::TempBackend::new("privacy", false).unwrap();
        let rows = privacy(&b, "de");
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
    fn the_tutor_from_the_settings_says_what_is_missing() {
        if std::env::var_os("KAIWA_PROVIDER").is_some() {
            return; // the local test's environment wins over the settings
        }
        let b = crate::TempBackend::new("tutor", false).unwrap();
        let v = tutor_view(&b, "de");
        assert_eq!(v["ready"], false);
        assert!(v["status"].as_str().unwrap().starts_with("Kein Tutor"));
        let mut p = b.store.profile().unwrap();
        p.tutor.provider = "ollama".into();
        b.store.set_profile(&p).unwrap();
        let v = tutor_view(&b, "de");
        assert_eq!(v["ready"], false);
        assert!(v["status"].as_str().unwrap().contains("Modell"));
        p.tutor.model = "qwen3:14b".into();
        b.store.set_profile(&p).unwrap();
        let v = tutor_view(&b, "en");
        assert_eq!(v["ready"], true);
        assert_eq!(v["effective"], "ollama");
        assert!(
            v["presets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["id"] == "openrouter")
        );
        // The local server sends nothing out and takes no key.
        let rows = privacy(&b, "en");
        assert!(rows.as_array().unwrap().iter().all(|r| r["cloud"] == false));
        assert_eq!(key_view(&b, "en"), Value::Null);
        let (_, demo) = b.provider("n5-01-aisatsu");
        assert!(!demo, "a tutor set up is used, not the recordings");
        // Any OpenAI-compatible server needs its address.
        p.tutor.provider = "custom".into();
        b.store.set_profile(&p).unwrap();
        let v = tutor_view(&b, "en");
        assert_eq!(v["ready"], false);
        assert!(v["status"].as_str().unwrap().contains("server address"));
        p.tutor.base_url = "http://localhost:8080/v1".into();
        b.store.set_profile(&p).unwrap();
        assert_eq!(tutor_view(&b, "en")["ready"], true);
        assert_eq!(key_view(&b, "en")["provider"], "custom");
    }

    #[test]
    fn the_learner_view_lists_grammar_still_practised() {
        let b = crate::TempBackend::new("gstate", false).unwrap();
        let mut l = b.store.learning().unwrap();
        l.grammar.insert(
            "ga-suki".into(),
            kaiwa_core::grammar::PointState {
                used: 2,
                wrong: 2,
                last: None,
            },
        );
        l.grammar.insert(
            "desu".into(),
            kaiwa_core::grammar::PointState {
                used: 9,
                wrong: 0,
                last: None,
            },
        );
        b.store.set_learning(&l).unwrap();
        let v = learner(&b).unwrap();
        assert_eq!(v["grammar"]["firm"], 1);
        assert_eq!(v["grammar"]["practising"][0]["label"], "〜が好きです");
        assert!(
            v["grammar"]["practising"][0]["explain"]
                .as_str()
                .unwrap()
                .contains("好き")
        );
    }

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
        let nodes = path(&b).unwrap();
        let nodes = nodes.as_array().unwrap();
        let review = nodes.iter().find(|n| n["kind"] == "review").unwrap();
        assert_eq!(review["id"], first.id.as_str());
        assert_eq!(review["meta"], "1 fällig · empfohlen, bevor es weitergeht");
        // T-06: the next unit is open all the same.
        let next = nodes
            .iter()
            .find(|n| n["id"] == second.scenarios[0].as_str())
            .unwrap();
        assert_eq!(next["state"], "current");
        assert!(
            today(&b, crate::now())
                .unwrap()
                .to_string()
                .contains(&second.scenarios[0])
        );
    }

    #[test]
    fn the_skip_offer_names_the_unit_and_the_settings_a_running_pause() {
        let b = crate::TempBackend::new("promises", false).unwrap();
        let first = b.curriculum.units[0].clone();
        // No offer without three easy talks.
        let v = learner(&b).unwrap();
        assert_eq!(v["offerSkip"], false);
        assert!(v["skipUnit"].is_null());
        assert!(!take_skip(&b, &first.id).unwrap());
        let easy = |n: usize| {
            let mut l = b.store.learning().unwrap();
            l.regulator.success.extend(vec![0.97; n]);
            b.store.set_learning(&l).unwrap();
        };
        easy(3);
        let v = learner(&b).unwrap();
        assert_eq!(v["offerSkip"], true);
        assert_eq!(v["skipUnit"]["id"], first.id.as_str());
        assert_eq!(v["skipUnit"]["title"], first.title.get("de"));
        // Not the unit of the offer: nothing happens.
        assert!(!take_skip(&b, &b.curriculum.units[1].id).unwrap());
        // Taken (what the button does): one unit, and the offer is used up; a second click
        // skips nothing.
        assert!(take_skip(&b, &first.id).unwrap());
        assert!(!take_skip(&b, &first.id).unwrap());
        assert!(!take_skip(&b, &b.curriculum.units[1].id).unwrap());
        let v = learner(&b).unwrap();
        assert_eq!(v["offerSkip"], false);
        assert!(v["skipUnit"].is_null());
        let done = b.store.learning().unwrap().course.skipped_units;
        assert_eq!(done.len(), 1);
        assert!(done.contains(&first.id));
        // Three new easy talks, not two, bring the next offer.
        easy(2);
        assert_eq!(learner(&b).unwrap()["offerSkip"], false);
        easy(1);
        let v = learner(&b).unwrap();
        assert_eq!(v["skipUnit"]["id"], b.curriculum.units[1].id.as_str());
        // A unit with keigo is not offered to a learner without the register.
        let keigo = b
            .curriculum
            .units
            .iter()
            .position(|u| {
                u.scenarios
                    .iter()
                    .any(|s| b.scenarios[s].register == kaiwa_core::scenario::Register::Keigo)
            })
            .unwrap();
        let mut l = b.store.learning().unwrap();
        for u in &b.curriculum.units[..keigo] {
            l.course.skipped_units.insert(u.id.clone());
        }
        b.store.set_learning(&l).unwrap();
        assert!(b.store.learning().unwrap().regulator.offer_skip());
        let v = learner(&b).unwrap();
        assert_eq!(v["offerSkip"], false);
        assert!(!take_skip(&b, &b.curriculum.units[keigo].id).unwrap());
        let mut l = b.store.learning().unwrap();
        l.course.skipped_units.clear();
        l.regulator = Default::default();
        b.store.set_learning(&l).unwrap();

        assert!(profile(&b).unwrap()["pausedUntil"].is_null());
        let today = crate::now().day();
        b.store
            .add_pause(kaiwa_core::progress::Pause {
                from: today,
                to: today.plus(2),
            })
            .unwrap();
        assert_eq!(
            profile(&b).unwrap()["pausedUntil"]["iso"],
            today.plus(2).iso()
        );
        b.store.end_pause(today).unwrap();
        assert!(profile(&b).unwrap()["pausedUntil"].is_null());
    }

    #[test]
    fn a_conversation_shows_its_units_grammar() {
        let b = crate::TempBackend::new("grammar", false).unwrap();
        let g = grammar(&b, "n5-08-onsen", "de");
        let labels: Vec<&str> = g
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["label"].as_str().unwrap())
            .collect();
        assert!(labels.contains(&"〜てもいいですか"), "{labels:?}");
        assert!(g[0]["explain"].as_str().unwrap().len() > 10);
        assert!(g[0]["examples"][0].as_str().unwrap().contains('{'));
        assert_eq!(grammar(&b, "frei-shumi", "de"), json!([]));
    }

    #[test]
    fn free_talk_offers_the_topics_outside_the_path() {
        let b = crate::TempBackend::new("free", true).unwrap();
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
        let b = crate::TempBackend::new("week", true).unwrap();
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
    fn the_week_names_the_progress_per_skill() {
        use kaiwa_core::learner::{LearnerModel, Skill};
        let b = crate::TempBackend::new("week-skills", false).unwrap();
        let now = crate::now();
        // No talk this week: nothing to compare with.
        assert_eq!(week(&b, now).unwrap()["skills"], json!([]));
        let mut l = b.store.learning().unwrap();
        l.model = LearnerModel::placed(&[(Skill::Listening, "n5"), (Skill::Speaking, "n5_low")]);
        b.store.set_learning(&l).unwrap();
        // The week's first talk, three days ago, started lower in listening, higher in
        // speaking; a day before the week does not count.
        let day = |ago: i64, listening: f64, speaking: f64| {
            let mut levels_at_start: BTreeMap<Skill, f64> = kaiwa_core::learner::SKILLS
                .iter()
                .map(|s| (*s, l.model.level(*s)))
                .collect();
            levels_at_start.insert(Skill::Listening, listening);
            levels_at_start.insert(Skill::Speaking, speaking);
            let r = kaiwa_core::progress::DayRecord {
                levels_at_start,
                ..Default::default()
            };
            b.store.put_day(now.day().plus(-ago), &r).unwrap();
        };
        day(9, 0.0, 0.0);
        day(3, 2.6, 2.3);
        day(1, 2.9, 2.0);
        let w = week(&b, now).unwrap();
        let change = |skill: &str| {
            w["skills"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["skill"] == skill)
                .map(|s| {
                    (
                        s["change"].as_str().unwrap().to_string(),
                        s["text"].to_string(),
                    )
                })
                .unwrap()
        };
        assert_eq!(change("listening").0, "up");
        assert!(change("listening").1.contains("Hören: sicherer"), "{w}");
        assert_eq!(change("speaking").0, "down");
        assert_eq!(change("fluency").0, "same");
        assert_eq!(w["skills"].as_array().unwrap().len(), 7);
        assert_eq!(skill_change(3.0, 3.09), "same");
    }

    #[test]
    fn the_trend_compares_talks_not_weeks() {
        // (turns, mistakes) per talk. Fading over eight talks, and the other way round.
        let fading = [
            (4, 3),
            (4, 3),
            (4, 2),
            (4, 2),
            (4, 1),
            (4, 1),
            (4, 0),
            (4, 0),
        ];
        assert!(trend("Partikeln", &fading, true).contains("seltener"));
        let mut rising = fading;
        rising.reverse();
        assert!(trend("Particles", &rising, false).contains("more often"));
        // A week of one mistake a talk, then a Monday talk with four: more often, although
        // the new week has fewer mistakes than the last (review #67).
        let monday = [
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 1),
            (4, 4),
        ];
        assert!(trend("Partikeln", &monday, true).contains("häufiger"));
        // Two days of practice are no trend, whichever weeks they fall into.
        assert_eq!(
            trend("Partikeln", &[(4, 1), (4, 3)], true),
            "Partikeln: bisher 4×"
        );
        // Longer talks with as many mistakes per turn: the same.
        let longer = [(4, 1), (4, 1), (8, 2), (8, 2)];
        assert!(trend("Partikeln", &longer, true).contains("gleich"));
        assert_eq!(trend("Particles", &[], false), "Particles: 0× so far");
        // One mistake in twenty talks is no trend, wherever it falls (review #67).
        let mut once = [(4, 0); 20];
        once[10].1 = 1;
        assert_eq!(trend("Partikeln", &once, true), "Partikeln: bisher 1×");
        once[0].1 = 2;
        assert!(trend("Partikeln", &once, true).contains("seltener"));
    }

    #[test]
    fn misrecognised_turns_do_not_thin_out_the_trend() {
        // Four talks of four valid turns with 2, 1, 2, 1 particle mistakes; the third has
        // twelve more turns, all marked as misrecognised (review #68). Per valid turn the
        // later half is as the earlier: 3 of 8 each.
        let b = crate::TempBackend::new("trend-misrecognised", false).unwrap();
        let scenario = b.scenarios.get("n5-02-jikoshoukai").cloned().unwrap();
        let now = crate::now();
        for (n, (mistakes, misrecognised)) in [(2, 0), (1, 0), (2, 12), (1, 0)].iter().enumerate() {
            let (provider, _) = b.provider(&scenario.id);
            let mut s = kaiwa_core::Session::new(
                scenario.clone(),
                provider,
                kaiwa_core::Amount::Little,
                "de",
            );
            for turn in 0..4 + misrecognised {
                let line = if turn < *mistakes {
                    "コーヒーを好きです。"
                } else {
                    "マラです。よろしくおねがいします。"
                };
                s.turn(line, kaiwa_core::Input::Spoken).unwrap();
                if turn >= 4 {
                    assert!(s.mark_misrecognized(turn));
                }
            }
            let started = now.plus_secs((n as i64 - 4) * 86_400);
            b.store
                .put_finished_session(&s.snapshot(), started, started.plus_secs(300), 0.8)
                .unwrap();
        }
        let v = stats(&b, now).unwrap();
        let particle = v["mistakes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["kind"] == "particle")
            .expect("particles")
            .clone();
        assert_eq!(particle["text"], "Partikeln: etwa gleich oft", "{particle}");
    }

    #[test]
    fn the_twelve_weeks_total_what_they_show() {
        let b = crate::TempBackend::new("twelve-weeks", false).unwrap();
        let now = crate::now();
        let day = |ago: i64, secs: u32| {
            let r = kaiwa_core::progress::DayRecord {
                speaking_secs: secs,
                ..Default::default()
            };
            b.store.put_day(now.day().plus(-ago), &r).unwrap();
        };
        // Four days of 90 s in the weeks shown: six minutes, not four. A day half a year
        // ago is not in the twelve weeks.
        for ago in 0..4 {
            day(ago, 90);
        }
        day(180, 3600);
        let v = stats(&b, now).unwrap();
        assert_eq!(
            (v["minutes"].as_u64(), v["days"].as_u64()),
            (Some(6), Some(4))
        );
        // Never less than the week it contains.
        assert!(v["minutes"].as_u64() >= v["week"]["minutes"].as_u64());
    }

    #[test]
    fn mistakes_over_twelve_weeks() {
        let b = crate::TempBackend::new("stats", true).unwrap();
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
