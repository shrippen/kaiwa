//! One conversation in a scenario: turns, help, goals, the summary at the end, and what the
//! learning model needs from it (success, evidence, XP).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use kaiwa_ai::{AiError, Analysis, ChatMessage, ChatProvider, Correction, Severity, TurnRequest};
use serde::{Deserialize, Serialize};

use crate::difficulty::{Difficulty, Live};
use crate::feedback::{Amount, Shown, cross_check, visible};
use crate::learner::{Evidence, Skill};
use crate::plain;
use crate::progress::{SessionXp, XP_GOAL, XpReason, speaking_secs, turn_xp};
use crate::prompts::{analysis_system, reply_system};
use crate::scenario::{Goal, Register, Scenario};

/// How the learner gave the turn. Typed turns do not count as speaking (/LF120/).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Input {
    Spoken,
    Typed,
}

/// One step of help (/LF114/).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelpStep {
    pub goal: String,
    /// 1 keyword, 2 sentence start, 3 full example to say after the character.
    pub level: u8,
    pub text: String,
}

/// A listening aid on the character's last line (/LF111/). Each counts as help (/LF212/).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aid {
    Repeat,
    Slow,
    Translate,
}

#[derive(Debug, Clone, Serialize)]
pub struct TurnOutcome {
    pub reply: String,
    pub understood: bool,
    /// Corrections to show, for the feedback amount of the session.
    pub shown: Vec<Shown>,
    /// The one correction that interrupts the conversation, if any (/LF306/).
    pub interrupt: Option<Shown>,
    pub goals_met_now: Vec<String>,
    pub xp: u32,
    pub xp_reason: Option<XpReason>,
    /// Two turns in a row not understood: the rest of the session is easier (/LF213/).
    pub eased: Option<Difficulty>,
    /// Three turns in a row without getting closer to a goal: help comes by itself, at least
    /// the sentence start (/LF213/, Jonas).
    pub help: Option<HelpStep>,
}

/// A learner turn as kept in the session (and in the profile, /LD030/).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnRecord {
    pub learner: String,
    pub input: Input,
    /// Every correction of the analysis (what is shown depends on the amount).
    pub corrections: Vec<Correction>,
    pub new_words: Vec<String>,
    pub understood: bool,
    /// A listening aid was used on the line this turn answers.
    pub aided: bool,
    pub from_help: bool,
    pub xp: u32,
    pub reply: String,
    /// The learner marked the transcript as misrecognised (/LF113/): no learning mistakes from it.
    #[serde(default)]
    pub misrecognized: bool,
    /// Better forms the learner disputed (/LF307/).
    #[serde(default)]
    pub disputed: Vec<String>,
}

/// The end of a session (/LF310/).
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub goals_met: Vec<String>,
    pub goals_total: usize,
    /// At most three corrections, the most severe first, each better form once.
    pub top: Vec<Shown>,
    /// What goes into the review (SRS): the better forms of every shown correction and new words.
    pub review: Vec<String>,
    pub spoken_turns: usize,
    pub typed_turns: usize,
    pub helps_used: u32,
    pub speaking_secs: u32,
    pub xp: SessionXp,
}

/// What the learning model takes from a session (/LF210/, /LF212/).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub turns: u32,
    pub understood: u32,
    /// Understood without a listening aid or help on that line.
    pub understood_unaided: u32,
    pub goals_met: u32,
    /// Reached without asking for help on that goal.
    pub goals_unaided: u32,
    pub helps: u32,
    pub aids: u32,
    pub speaking_secs: u32,
    pub typed_turns: u32,
    /// Corrections of severity unnatural or above, per kind (not misrecognised, not disputed).
    pub mistakes: BTreeMap<String, u32>,
    pub register_mistakes: u32,
    pub eased: u32,
}

impl Stats {
    /// Share of the session that went well: understood lines and goals reached without help.
    pub fn success(&self) -> f64 {
        let n = self.turns + self.goals_met;
        if n == 0 {
            return 0.0;
        }
        f64::from(self.understood_unaided + self.goals_unaided) / f64::from(n)
    }
}

/// Everything needed to continue a session later (/LF121/).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionState {
    pub scenario: String,
    pub lang: String,
    pub amount: Amount,
    pub difficulty: Difficulty,
    pub messages: Vec<ChatMessage>,
    pub turns: Vec<TurnRecord>,
    pub goals_met: BTreeSet<String>,
    pub goals_helped: BTreeSet<String>,
    pub help_level: u8,
    pub helps_used: u32,
    pub aids: BTreeMap<Aid, u32>,
    pub stuck: u32,
    pub live: Live,
    #[serde(default)]
    pub limit_goals: bool,
}

/// Seconds of silence after which help is offered (/LF114/), unless the difficulty says later.
pub const HELP_AFTER: Duration = Duration::from_secs(4);
/// Turns in a row without getting closer to a goal before help comes by itself.
pub const STUCK_AFTER: u32 = 3;

pub struct Session {
    pub scenario: Scenario,
    pub lang: String,
    pub amount: Amount,
    /// Ask a second analysis and keep serious corrections only when both agree (ADR-004).
    pub cross_check: bool,
    /// The difficulty in effect (eased within the session after two misses).
    pub difficulty: Difficulty,
    limit_goals: bool,
    provider: Arc<dyn ChatProvider>,
    messages: Vec<ChatMessage>,
    turns: Vec<TurnRecord>,
    goals_met: BTreeSet<String>,
    goals_helped: BTreeSet<String>,
    help_level: u8,
    helps_used: u32,
    aids: BTreeMap<Aid, u32>,
    aid_on_line: bool,
    stuck: u32,
    live: Live,
}

impl Session {
    pub fn new(
        scenario: Scenario,
        provider: Arc<dyn ChatProvider>,
        amount: Amount,
        lang: &str,
    ) -> Self {
        let opening = ChatMessage::assistant(plain(&scenario.opening));
        Self {
            scenario,
            lang: lang.into(),
            amount,
            cross_check: false,
            difficulty: Difficulty::default(),
            limit_goals: false,
            provider,
            messages: vec![opening],
            turns: Vec::new(),
            goals_met: BTreeSet::new(),
            goals_helped: BTreeSet::new(),
            help_level: 0,
            helps_used: 0,
            aids: BTreeMap::new(),
            aid_on_line: false,
            stuck: 0,
            live: Live::default(),
        }
    }

    /// Plays the session at a difficulty: prompts, goals per scenario, help timing (/LF211/).
    pub fn with_difficulty(mut self, d: Difficulty) -> Self {
        self.difficulty = d;
        self.limit_goals = true;
        self
    }

    /// Continues a session from its state (/LF121/): where it stopped, not from the start.
    pub fn restore(scenario: Scenario, provider: Arc<dyn ChatProvider>, st: SessionState) -> Self {
        Self {
            scenario,
            lang: st.lang,
            amount: st.amount,
            cross_check: false,
            difficulty: st.difficulty,
            limit_goals: st.limit_goals,
            provider,
            messages: st.messages,
            turns: st.turns,
            goals_met: st.goals_met,
            goals_helped: st.goals_helped,
            help_level: st.help_level,
            helps_used: st.helps_used,
            aids: st.aids,
            aid_on_line: false,
            stuck: st.stuck,
            live: st.live,
        }
    }

    pub fn snapshot(&self) -> SessionState {
        SessionState {
            scenario: self.scenario.id.clone(),
            lang: self.lang.clone(),
            amount: self.amount,
            difficulty: self.difficulty.clone(),
            messages: self.messages.clone(),
            turns: self.turns.clone(),
            goals_met: self.goals_met.clone(),
            goals_helped: self.goals_helped.clone(),
            help_level: self.help_level,
            helps_used: self.helps_used,
            aids: self.aids.clone(),
            stuck: self.stuck,
            live: self.live.clone(),
            limit_goals: self.limit_goals,
        }
    }

    /// The character's first line (with furigana markup).
    pub fn opening(&self) -> &str {
        &self.scenario.opening
    }

    /// The character's last line (plain text).
    pub fn last_line(&self) -> &str {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == kaiwa_ai::Role::Assistant)
            .map(|m| m.content.as_str())
            .unwrap_or_default()
    }

    /// The goals the session plays: all of the scenario's, or as many as the difficulty allows
    /// when the session was given one (/LF211/).
    pub fn goals(&self) -> &[Goal] {
        let n = if self.limit_goals {
            self.difficulty
                .goals()
                .max(2)
                .min(self.scenario.goals.len())
        } else {
            self.scenario.goals.len()
        };
        &self.scenario.goals[..n]
    }

    /// The first goal not reached yet.
    pub fn current_goal(&self) -> Option<&Goal> {
        self.goals()
            .iter()
            .find(|g| !self.goals_met.contains(&g.id))
    }

    pub fn done(&self) -> bool {
        self.current_goal().is_none()
    }

    /// Whether to offer help after this much silence.
    /// After 4 s for beginners, later for more fluent learners, never when help comes only on
    /// request (/LF114/, /LF211/).
    pub fn offer_help(&self, silence: Duration) -> bool {
        match self.difficulty.help_after() {
            Some(s) => !self.done() && silence >= Duration::from_secs(u64::from(s)),
            None => false,
        }
    }

    /// The next step of help for the current goal; each call goes one step further, up to 3.
    pub fn help(&mut self) -> Option<HelpStep> {
        let goal = self.current_goal()?.clone();
        self.help_level = (self.help_level + 1).min(3);
        self.helps_used += 1;
        self.goals_helped.insert(goal.id.clone());
        let text = match self.help_level {
            1 => goal.help.keyword,
            2 => goal.help.start,
            _ => goal.help.example,
        };
        Some(HelpStep {
            goal: goal.id,
            level: self.help_level,
            text,
        })
    }

    /// A listening aid on the character's last line: repeat, slower, translate (/LF111/).
    pub fn aid(&mut self, aid: Aid) {
        *self.aids.entry(aid).or_default() += 1;
        self.aid_on_line = true;
    }

    fn request(&self, system: String) -> TurnRequest {
        TurnRequest {
            system,
            messages: self.messages.clone(),
            max_tokens: 600,
        }
    }

    /// The learner's turn: analyse it, then let the character answer.
    pub fn turn(&mut self, text: &str, input: Input) -> Result<TurnOutcome, AiError> {
        self.messages.push(ChatMessage::user(text));
        let analysis_request = self.request(analysis_system(&self.scenario, &self.lang));
        let analysed = self.provider.analyse(&analysis_request);
        let mut analysis: Analysis = match analysed {
            Ok(a) => a,
            Err(e) => {
                self.messages.pop();
                return Err(e);
            }
        };
        if self.cross_check
            && analysis
                .corrections
                .iter()
                .any(|c| c.severity >= kaiwa_ai::Severity::Wrong)
        {
            let second = self.provider.analyse(&analysis_request)?;
            analysis.corrections = cross_check(&analysis.corrections, &second.corrections);
        }
        let reply = match self.provider.reply(&self.request(reply_system(
            &self.scenario,
            &self.lang,
            &self.difficulty,
        ))) {
            Ok(r) => r,
            Err(e) => {
                self.messages.pop();
                return Err(e);
            }
        };
        self.messages.push(ChatMessage::assistant(reply.clone()));

        let playable: Vec<String> = self.goals().iter().map(|g| g.id.clone()).collect();
        let goals_met_now: Vec<String> = analysis
            .goals_met
            .iter()
            .filter(|g| playable.contains(g) && self.goals_met.insert((*g).clone()))
            .cloned()
            .collect();

        let from_help = self.help_level == 3
            && self
                .goals()
                .iter()
                .any(|g| crate::norm(&g.help.example) == crate::norm(text));
        let earlier: Vec<&str> = self.turns.iter().map(|t| t.learner.as_str()).collect();
        let (mut xp, xp_reason) = turn_xp(text, input, &earlier, from_help);
        xp += XP_GOAL * goals_met_now.len() as u32;

        let help = if goals_met_now.is_empty() {
            self.stuck += 1;
            if self.stuck >= STUCK_AFTER && !self.done() {
                self.stuck = 0;
                if self.help_level < 1 {
                    self.help_level = 1;
                }
                self.help()
            } else {
                None
            }
        } else {
            self.help_level = 0;
            self.stuck = 0;
            None
        };

        let eased = self.live.turn(analysis.understood, &self.difficulty);
        if let Some(d) = &eased {
            self.difficulty = d.clone();
        }

        let shown = visible(&analysis.corrections, self.amount);
        let interrupt = shown.iter().find(|s| s.interrupts).cloned();
        self.turns.push(TurnRecord {
            learner: text.into(),
            input,
            corrections: analysis.corrections.clone(),
            new_words: analysis.new_words.clone(),
            understood: analysis.understood,
            aided: self.aid_on_line,
            from_help,
            xp,
            reply: reply.clone(),
            misrecognized: false,
            disputed: Vec::new(),
        });
        self.aid_on_line = false;
        Ok(TurnOutcome {
            reply,
            understood: analysis.understood,
            shown,
            interrupt,
            goals_met_now,
            xp,
            xp_reason,
            eased,
            help,
        })
    }

    /// The learner marks a turn's transcript as misrecognised (/LF113/): its corrections no longer
    /// count as mistakes and do not go into the review.
    pub fn mark_misrecognized(&mut self, turn: usize) -> bool {
        match self.turns.get_mut(turn) {
            Some(t) => {
                t.misrecognized = true;
                true
            }
            None => false,
        }
    }

    /// The learner disputes a correction (/LF307/). It stays visible, marked as disputed, and
    /// is kept for the QA (/LD060/), but it no longer counts as a mistake.
    pub fn dispute(&mut self, turn: usize, better: &str) -> Option<Correction> {
        let t = self.turns.get_mut(turn)?;
        let c = t
            .corrections
            .iter()
            .find(|c| crate::norm(&c.better) == crate::norm(better))?
            .clone();
        if !t.disputed.contains(&c.better) {
            t.disputed.push(c.better.clone());
        }
        Some(c)
    }

    /// The corrections shown for a turn, as now (after misrecognition marks and disputes).
    pub fn shown(&self, turn: usize) -> Vec<Shown> {
        self.turns
            .get(turn)
            .map(|t| visible(&counted(t), self.amount))
            .unwrap_or_default()
    }

    pub fn turns(&self) -> &[TurnRecord] {
        &self.turns
    }

    pub fn summary(&self) -> Summary {
        let mut all: Vec<Shown> = self
            .turns
            .iter()
            .flat_map(|t| visible(&counted(t), self.amount))
            .collect();
        all.sort_by_key(|s| std::cmp::Reverse(s.correction.severity));
        let mut seen = BTreeSet::new();
        let unique: Vec<Shown> = all
            .into_iter()
            .filter(|s| seen.insert(crate::norm(&s.correction.better)))
            .collect();
        let mut review: Vec<String> = unique.iter().map(|s| s.correction.better.clone()).collect();
        for w in self
            .turns
            .iter()
            .filter(|t| !t.misrecognized)
            .flat_map(|t| t.new_words.iter())
        {
            if !review.contains(w) {
                review.push(w.clone());
            }
        }
        let mut xp = SessionXp::default();
        for (i, t) in self.turns.iter().enumerate() {
            let before: Vec<&str> = self.turns[..i].iter().map(|t| t.learner.as_str()).collect();
            let (_, reason) = turn_xp(&t.learner, t.input, &before, t.from_help);
            xp.add_turn((t.xp, reason));
        }
        Summary {
            goals_met: self
                .goals()
                .iter()
                .filter(|g| self.goals_met.contains(&g.id))
                .map(|g| g.id.clone())
                .collect(),
            goals_total: self.goals().len(),
            top: unique.into_iter().take(3).collect(),
            review,
            spoken_turns: self
                .turns
                .iter()
                .filter(|t| t.input == Input::Spoken)
                .count(),
            typed_turns: self
                .turns
                .iter()
                .filter(|t| t.input == Input::Typed)
                .count(),
            helps_used: self.helps_used,
            speaking_secs: self.stats().speaking_secs,
            xp,
        }
    }

    /// What the learning model takes from the session.
    pub fn stats(&self) -> Stats {
        let mut s = Stats {
            helps: self.helps_used,
            aids: self.aids.values().sum(),
            eased: self.live.eased,
            ..Default::default()
        };
        for t in &self.turns {
            s.turns += 1;
            if t.understood {
                s.understood += 1;
                if !t.aided && !t.from_help {
                    s.understood_unaided += 1;
                }
            }
            match t.input {
                Input::Spoken => s.speaking_secs += speaking_secs(&t.learner),
                Input::Typed => s.typed_turns += 1,
            }
            for c in counted(t) {
                if c.also_possible || c.severity < Severity::Unnatural {
                    continue;
                }
                let kind = serde_json::to_value(c.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                if kind == "register" {
                    s.register_mistakes += 1;
                }
                *s.mistakes.entry(kind).or_default() += 1;
            }
        }
        s.goals_met = self.goals_met.len() as u32;
        s.goals_unaided = self
            .goals_met
            .iter()
            .filter(|g| !self.goals_helped.contains(*g))
            .count() as u32;
        s
    }

    /// The session as evidence for the learner model (/LF201/).
    pub fn evidence(&self) -> Evidence {
        let st = self.stats();
        let mut e = Evidence {
            mistakes: st.mistakes.clone(),
            turns: st.turns,
            ..Default::default()
        };
        if st.turns == 0 {
            return e;
        }
        let n = f64::from(st.turns);
        let d = &self.difficulty;
        e.at_level.insert(
            Skill::Listening,
            (d.listening_level(), f64::from(st.understood_unaided) / n),
        );
        let clean = self
            .turns
            .iter()
            .filter(|t| {
                !counted(t)
                    .iter()
                    .any(|c| !c.also_possible && c.severity >= Severity::Wrong)
            })
            .count() as f64;
        e.at_level
            .insert(Skill::Speaking, (d.content_level(), clean / n));
        let fluent = if st.goals_met > 0 {
            f64::from(st.goals_unaided) / f64::from(st.goals_met)
        } else {
            0.5
        };
        e.at_level
            .insert(Skill::Fluency, (d.content_level(), fluent));
        let register_level = match self.scenario.register {
            Register::Casual => 1.0,
            Register::Polite => 2.0,
            Register::Keigo => 4.0,
        };
        e.at_level.insert(
            Skill::Register,
            (register_level, 1.0 - f64::from(st.register_mistakes) / n),
        );
        e
    }

    /// The learner's turns so far (for the transcript and `kaiwa-sim`).
    pub fn learner_turns(&self) -> impl Iterator<Item = &str> {
        self.turns.iter().map(|t| t.learner.as_str())
    }
}

/// The corrections of a turn that still count: none for a misrecognised turn, none disputed.
fn counted(t: &TurnRecord) -> Vec<Correction> {
    if t.misrecognized {
        return Vec::new();
    }
    t.corrections
        .iter()
        .filter(|c| !t.disputed.contains(&c.better))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaiwa_ai::mock::MockProvider;

    fn scenario() -> Scenario {
        Scenario::from_json(include_str!(
            "../../../data/scenarios/n5-02-jikoshoukai.json"
        ))
        .unwrap()
    }

    fn provider() -> Arc<dyn ChatProvider> {
        Arc::new(
            MockProvider::from_json(include_str!("../../../qa/fixtures/n5-02-jikoshoukai.json"))
                .unwrap()
                .with_fallback("すみません、もう一度お願いします。"),
        )
    }

    #[test]
    fn a_conversation_reaches_its_goals() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        assert_eq!(s.current_goal().unwrap().id, "name");
        let out = s
            .turn("マラです。よろしくおねがいします。", Input::Spoken)
            .unwrap();
        assert_eq!(out.goals_met_now, vec!["name".to_string()]);
        assert_eq!(s.current_goal().unwrap().id, "origin");
        s.turn("ドイツのハンブルクから来ました。", Input::Spoken)
            .unwrap();
        s.turn("仕事はプロデューサーです。", Input::Typed).unwrap();
        let out = s.turn("コーヒーを好きです。", Input::Spoken).unwrap();
        assert!(s.done());
        assert_eq!(
            out.shown.len(),
            1,
            "a wrong particle shows with little feedback"
        );
        assert!(
            out.interrupt.is_none(),
            "a wrong particle does not interrupt"
        );
        let sum = s.summary();
        assert_eq!(sum.goals_met.len(), 4);
        assert_eq!((sum.spoken_turns, sum.typed_turns), (3, 1));
        assert!(sum.review.contains(&"コーヒーが好きです".to_string()));
    }

    #[test]
    fn help_escalates_and_resets_with_the_goal() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        assert!(!s.offer_help(Duration::from_secs(2)));
        assert!(s.offer_help(Duration::from_secs(4)));
        assert_eq!(s.help().unwrap().level, 1);
        assert_eq!(s.help().unwrap().level, 2);
        let full = s.help().unwrap();
        assert_eq!(full.level, 3);
        assert_eq!(full.text, "マラです。よろしくおねがいします。");
        assert_eq!(s.help().unwrap().level, 3, "stays at the full example");
        s.turn("マラです。よろしくおねがいします。", Input::Spoken)
            .unwrap();
        assert_eq!(
            s.help().unwrap(),
            HelpStep {
                goal: "origin".into(),
                level: 1,
                text: "ドイツ・ハンブルク".into()
            }
        );
        assert_eq!(s.summary().helps_used, 5);
    }

    #[test]
    fn unknown_turns_are_not_understood_and_reach_nothing() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        let out = s.turn("ええと…", Input::Spoken).unwrap();
        assert!(!out.understood);
        assert!(out.goals_met_now.is_empty());
        assert_eq!(out.reply, "すみません、もう一度お願いします。");
    }

    #[test]
    fn style_hides_with_little_feedback_and_the_summary_keeps_three() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        let out = s
            .turn(
                "わたしはマラです。わたしはプロデューサーです。",
                Input::Spoken,
            )
            .unwrap();
        assert!(out.shown.is_empty(), "style is hidden with little feedback");
        s.amount = Amount::Much;
        s.turn(
            "わたしはマラです。わたしはプロデューサーです。",
            Input::Spoken,
        )
        .unwrap();
        s.turn("ビール二ください。", Input::Spoken).unwrap();
        s.turn("きのう東京に行きます。", Input::Spoken).unwrap();
        s.turn("コーヒーを好きです。", Input::Spoken).unwrap();
        let sum = s.summary();
        assert_eq!(sum.top.len(), 3);
        assert!(
            sum.top
                .iter()
                .all(|t| t.correction.severity == kaiwa_ai::Severity::Wrong),
            "most severe first"
        );
    }

    #[test]
    fn the_opening_goes_to_the_model_without_furigana_markup() {
        let s = Session::new(scenario(), provider(), Amount::Little, "de");
        assert_eq!(s.messages[0].content, "はじめまして。田中です。お名前は？");
    }

    #[test]
    fn a_session_continues_where_it_stopped() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        s.turn("マラです。よろしくおねがいします。", Input::Spoken)
            .unwrap();
        let state = s.snapshot();
        let json = serde_json::to_string(&state).unwrap();
        let back: SessionState = serde_json::from_str(&json).unwrap();
        let mut r = Session::restore(scenario(), provider(), back);
        assert_eq!(r.current_goal().unwrap().id, "origin");
        assert_eq!(r.turns().len(), 1);
        r.turn("ドイツのハンブルクから来ました。", Input::Spoken)
            .unwrap();
        assert_eq!(r.summary().goals_met.len(), 2);
    }

    #[test]
    fn stats_count_success_without_help() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        s.turn("マラです。よろしくおねがいします。", Input::Spoken)
            .unwrap();
        s.help();
        s.aid(Aid::Slow);
        s.turn("ドイツのハンブルクから来ました。", Input::Spoken)
            .unwrap();
        let st = s.stats();
        assert_eq!((st.turns, st.understood, st.understood_unaided), (2, 2, 1));
        assert_eq!((st.goals_met, st.goals_unaided), (2, 1));
        assert!((st.success() - 0.5).abs() < 1e-9);
        assert_eq!((st.helps, st.aids), (1, 1));
        assert!(st.speaking_secs > 0);
    }

    #[test]
    fn misrecognised_and_disputed_corrections_do_not_count() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        s.turn("コーヒーを好きです。", Input::Spoken).unwrap();
        assert_eq!(s.stats().mistakes.get("particle"), Some(&1));
        let disputed = s.dispute(0, "コーヒーが好きです").unwrap();
        assert_eq!(disputed.better, "コーヒーが好きです");
        assert!(s.stats().mistakes.is_empty());
        assert!(s.summary().top.is_empty());
        assert_eq!(s.turns()[0].disputed.len(), 1, "kept, marked as disputed");
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        s.turn("コーヒーを好きです。", Input::Spoken).unwrap();
        assert!(s.mark_misrecognized(0));
        assert!(s.shown(0).is_empty());
        assert!(
            !s.summary()
                .review
                .contains(&"コーヒーが好きです".to_string())
        );
    }

    #[test]
    fn evasive_answers_bring_help_by_themselves() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        assert!(s.turn("はい", Input::Spoken).unwrap().help.is_none());
        assert!(s.turn("いいですね", Input::Spoken).unwrap().help.is_none());
        let third = s.turn("はい", Input::Spoken).unwrap();
        let help = third.help.unwrap();
        assert!(help.level >= 2, "at least the sentence start");
        assert_eq!(third.xp, 0, "a repeat earns nothing");
        let xp = s.summary().xp;
        assert!(!xp.explain("de").is_empty());
    }

    #[test]
    fn the_difficulty_limits_the_goals() {
        let s = Session::new(scenario(), provider(), Amount::Little, "de")
            .with_difficulty(crate::difficulty::Difficulty::default());
        assert_eq!(s.goals().len(), 2);
        assert!(!s.offer_help(Duration::from_secs(3)));
        assert!(s.offer_help(Duration::from_secs(4)));
    }
}
