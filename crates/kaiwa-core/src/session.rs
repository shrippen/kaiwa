//! One conversation in a scenario: turns, help, goals and the summary at the end.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use kaiwa_ai::{AiError, Analysis, ChatMessage, ChatProvider, TurnRequest};
use serde::Serialize;

use crate::feedback::{Amount, Shown, cross_check, visible};
use crate::plain;
use crate::prompts::{analysis_system, reply_system};
use crate::scenario::{Goal, Scenario};

/// How the learner gave the turn. Typed turns do not count as speaking (/LF120/).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Input {
    Spoken,
    Typed,
}

/// One step of help (/LF114/).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HelpStep {
    pub goal: String,
    /// 1 keyword, 2 sentence start, 3 full example to say after the character.
    pub level: u8,
    pub text: String,
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
}

#[derive(Debug, Clone, Serialize)]
struct TurnRecord {
    learner: String,
    input: Input,
    shown: Vec<Shown>,
    new_words: Vec<String>,
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
}

/// Seconds of silence after which help is offered (/LF114/).
pub const HELP_AFTER: Duration = Duration::from_secs(4);

pub struct Session {
    pub scenario: Scenario,
    pub lang: String,
    pub amount: Amount,
    /// Ask a second analysis and keep serious corrections only when both agree (ADR-004).
    pub cross_check: bool,
    provider: Arc<dyn ChatProvider>,
    messages: Vec<ChatMessage>,
    turns: Vec<TurnRecord>,
    goals_met: BTreeSet<String>,
    help_level: u8,
    helps_used: u32,
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
            provider,
            messages: vec![opening],
            turns: Vec::new(),
            goals_met: BTreeSet::new(),
            help_level: 0,
            helps_used: 0,
        }
    }

    /// The character's first line (with furigana markup).
    pub fn opening(&self) -> &str {
        &self.scenario.opening
    }

    /// The first goal not reached yet.
    pub fn current_goal(&self) -> Option<&Goal> {
        self.scenario
            .goals
            .iter()
            .find(|g| !self.goals_met.contains(&g.id))
    }

    pub fn done(&self) -> bool {
        self.current_goal().is_none()
    }

    /// Whether to offer help after this much silence.
    pub fn offer_help(&self, silence: Duration) -> bool {
        !self.done() && silence >= HELP_AFTER
    }

    /// The next step of help for the current goal; each call goes one step further, up to 3.
    pub fn help(&mut self) -> Option<HelpStep> {
        let goal = self.current_goal()?.clone();
        self.help_level = (self.help_level + 1).min(3);
        self.helps_used += 1;
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
        let mut analysis: Analysis = self.provider.analyse(&analysis_request)?;
        if self.cross_check
            && analysis
                .corrections
                .iter()
                .any(|c| c.severity >= kaiwa_ai::Severity::Wrong)
        {
            let second = self.provider.analyse(&analysis_request)?;
            analysis.corrections = cross_check(&analysis.corrections, &second.corrections);
        }
        let reply = self
            .provider
            .reply(&self.request(reply_system(&self.scenario, &self.lang)))?;
        self.messages.push(ChatMessage::assistant(reply.clone()));

        let goals_met_now: Vec<String> = analysis
            .goals_met
            .iter()
            .filter(|g| self.scenario.goal(g).is_some() && self.goals_met.insert((*g).clone()))
            .cloned()
            .collect();
        if !goals_met_now.is_empty() {
            self.help_level = 0;
        }
        let shown = visible(&analysis.corrections, self.amount);
        let interrupt = shown.iter().find(|s| s.interrupts).cloned();
        self.turns.push(TurnRecord {
            learner: text.into(),
            input,
            shown: shown.clone(),
            new_words: analysis.new_words.clone(),
        });
        Ok(TurnOutcome {
            reply,
            understood: analysis.understood,
            shown,
            interrupt,
            goals_met_now,
        })
    }

    pub fn summary(&self) -> Summary {
        let mut all: Vec<Shown> = self
            .turns
            .iter()
            .flat_map(|t| t.shown.iter().cloned())
            .collect();
        all.sort_by_key(|s| std::cmp::Reverse(s.correction.severity));
        let mut seen = BTreeSet::new();
        let unique: Vec<Shown> = all
            .into_iter()
            .filter(|s| seen.insert(crate::norm(&s.correction.better)))
            .collect();
        let mut review: Vec<String> = unique.iter().map(|s| s.correction.better.clone()).collect();
        for w in self.turns.iter().flat_map(|t| t.new_words.iter()) {
            if !review.contains(w) {
                review.push(w.clone());
            }
        }
        Summary {
            goals_met: self
                .scenario
                .goals
                .iter()
                .filter(|g| self.goals_met.contains(&g.id))
                .map(|g| g.id.clone())
                .collect(),
            goals_total: self.scenario.goals.len(),
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
        }
    }

    /// The learner's turns so far (for the transcript and `kaiwa-sim`).
    pub fn learner_turns(&self) -> impl Iterator<Item = &str> {
        self.turns.iter().map(|t| t.learner.as_str())
    }
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
}
