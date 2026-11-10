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
    /// The learner's line with the tutor's furigana ([`own_reading`]); empty without.
    pub own_markup: String,
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
    /// The tutor's analysis of this turn had gaps ([`kaiwa_ai::Analysis::incomplete`]): the
    /// page says so, and the turn does not count for the level (T-11).
    pub incomplete: bool,
}

/// The tutor's reading of the learner's line as clean furigana markup, only when it is the
/// line itself: without the markup exactly the text (a reading that changed a character is
/// dropped, the dictionary's furigana stay), and with at least one reading.
pub fn own_reading(text: &str, reading: &str) -> String {
    let markup = crate::clean_markup(reading);
    if markup.contains('{') && crate::plain(&markup) == text {
        markup
    } else {
        String::new()
    }
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
    /// The learner's line with the tutor's furigana; empty when it gave none that fits.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub markup: String,
    /// The analysis of this turn had gaps (T-11): what could be read of it is here and goes
    /// into the review, but the turn is no evidence of the level, in either direction.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub incomplete: bool,
}

/// The end of a session (/LF310/).
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub goals_met: Vec<String>,
    pub goals_total: usize,
    /// At most three corrections, the most severe first, each better form once.
    pub top: Vec<Shown>,
    /// Every shown correction, the most severe first, each better form once: what goes into the
    /// review as a correction (`top` is the first three of it).
    pub corrected: Vec<Shown>,
    /// What goes into the review (SRS): the better forms of every shown correction and new words.
    pub review: Vec<String>,
    /// For each better form (by [`crate::norm`]): the character's line the learner answered
    /// when the mistake happened, the cue of its mini-conversation review (/LF230/).
    pub cues: BTreeMap<String, String>,
    /// For each better form (by [`crate::norm`]): the old words in their place in the learner's
    /// sentence ([`crate::srs::in_context`]), to tell a repeated mistake in the review.
    pub mistakes: BTreeMap<String, String>,
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
    /// Turns without a counted mistake.
    pub clean_turns: u32,
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
    /// Goals reached in a turn whose analysis had gaps (T-11): reached for the talk, no
    /// evidence of the level.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub goals_unjudged: BTreeSet<String>,
    pub help_level: u8,
    pub helps_used: u32,
    pub aids: BTreeMap<Aid, u32>,
    pub stuck: u32,
    pub live: Live,
    #[serde(default)]
    pub limit_goals: bool,
    /// Seconds spent in this talk so far, over every sitting (/LF121/): a talk continued
    /// later counts all of its time as practice, not only the last sitting.
    #[serde(default)]
    pub spent_secs: u32,
}

/// The tutor's two calls for one turn ([`Session::ask`]), free of the session.
pub struct Asking {
    provider: Arc<dyn ChatProvider>,
    analysis: TurnRequest,
    reply: TurnRequest,
    cross_check: bool,
    register: crate::scenario::Register,
}

/// What the tutor said to one turn: the analysis (cross-checked and fitted to the scenario's
/// register) and the character's reply with its readings.
pub struct Answer {
    analysis: Analysis,
    reply: String,
}

impl Asking {
    /// Calls the tutor: the analysis, then the reply. An error leaves nothing behind.
    pub fn run(&self) -> Result<Answer, AiError> {
        let mut analysis = self.provider.analyse(&self.analysis)?;
        if self.cross_check
            && analysis
                .corrections
                .iter()
                .any(|c| c.severity >= kaiwa_ai::Severity::Wrong)
        {
            let second = self.provider.analyse(&self.analysis)?;
            analysis.corrections = cross_check(&analysis.corrections, &second.corrections);
            // A correction the second reading lost to a gap is not one it disagreed with.
            analysis.incomplete |= second.incomplete;
        }
        fit_register(&mut analysis.corrections, self.register);
        // The reply with the readings the tutor wrote (/LF110/); the model's own history
        // gets it without, as the opening.
        let reply = crate::clean_markup(&self.provider.reply(&self.reply)?);
        Ok(Answer { analysis, reply })
    }
}

/// A translation to fetch ([`Session::translating`]), free of the session.
pub struct Translating {
    provider: Arc<dyn ChatProvider>,
    line: String,
    lang: String,
    known: Option<String>,
}

impl Translating {
    pub fn run(&self) -> Result<String, AiError> {
        match &self.known {
            Some(t) => Ok(t.clone()),
            None => self.provider.translate(&self.line, &self.lang),
        }
    }
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
    goals_unjudged: BTreeSet<String>,
    help_level: u8,
    helps_used: u32,
    aids: BTreeMap<Aid, u32>,
    aid_on_line: bool,
    stuck: u32,
    live: Live,
    /// The grammar points of the scenario's unit, for the tutor's instructions; set by the
    /// caller from the curriculum, not part of the saved state.
    grammar: Vec<String>,
    spent_secs: u32,
    /// The learner's own speaking level on the learner model's scale, for a free talk
    /// (/LF116/); set by the caller from the learner model, not part of the saved state.
    learner_level: Option<f64>,
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
            goals_unjudged: BTreeSet::new(),
            help_level: 0,
            helps_used: 0,
            aids: BTreeMap::new(),
            aid_on_line: false,
            stuck: 0,
            live: Live::default(),
            grammar: Vec::new(),
            spent_secs: 0,
            learner_level: None,
        }
    }

    /// What the character is told for its next reply: role, register, the difficulty in
    /// words, the level, the unit's grammar and the goals ([`crate::prompts::reply_system`]).
    pub fn reply_instructions(&self) -> String {
        reply_system(
            &self.scenario,
            &self.lang,
            &self.difficulty,
            &self.grammar,
            self.learner_level,
        )
    }

    /// Adds time the learner spent in this talk (the caller has the clock).
    pub fn spend(&mut self, secs: u32) {
        self.spent_secs = self.spent_secs.saturating_add(secs);
    }

    /// Seconds spent in this talk so far, sittings before a break included (/LF121/).
    pub fn spent_secs(&self) -> u32 {
        self.spent_secs
    }

    /// The unit's grammar labels given with [`Session::with_grammar`].
    pub fn grammar(&self) -> &[String] {
        &self.grammar
    }

    /// The grammar of the scenario's unit (catalogue labels): the character uses it where it
    /// fits, the analysis knows what the unit practises (/LF211/, ADR-009).
    pub fn with_grammar(mut self, labels: Vec<String>) -> Self {
        self.grammar = labels;
        self
    }

    /// The learner's own speaking level ([`crate::learner::LEVELS`]): a free talk, which has
    /// no unit, names it to the character instead of the beginner levels (/LF116/).
    pub fn with_learner_level(mut self, level: f64) -> Self {
        self.learner_level = Some(level);
        self
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
            goals_unjudged: st.goals_unjudged,
            help_level: st.help_level,
            helps_used: st.helps_used,
            aids: st.aids,
            aid_on_line: false,
            stuck: st.stuck,
            live: st.live,
            grammar: Vec::new(),
            spent_secs: st.spent_secs,
            learner_level: None,
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
            goals_unjudged: self.goals_unjudged.clone(),
            help_level: self.help_level,
            helps_used: self.helps_used,
            aids: self.aids.clone(),
            stuck: self.stuck,
            live: self.live.clone(),
            limit_goals: self.limit_goals,
            spent_secs: self.spent_secs,
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

    /// A line of the character (plain text) in the UI language (/LF111/); counts as an aid.
    /// The opening comes from the scenario when it has a translation, every other line from
    /// the tutor.
    pub fn translate(&mut self, line: &str) -> Result<String, AiError> {
        let text = self.translating(line).run()?;
        self.aid(Aid::Translate);
        Ok(text)
    }

    /// The translation of a line, to run without the session (as [`Session::ask`]); the
    /// caller counts the aid ([`Aid::Translate`]) once the learner has it.
    pub fn translating(&self, line: &str) -> Translating {
        let known = self
            .scenario
            .opening_translation
            .as_ref()
            .filter(|_| line == plain(&self.scenario.opening))
            .map(|t| t.get(&self.lang).to_string());
        Translating {
            provider: self.provider.clone(),
            line: line.to_string(),
            lang: self.lang.clone(),
            known,
        }
    }

    /// The learner's turn: analyse it, then let the character answer. The two halves are
    /// [`Session::ask`] (with [`Asking::run`]) and [`Session::take`]; a caller that must not
    /// hold the session while the tutor thinks (the app: its window locks the session for help
    /// and the listening aids) runs them itself.
    pub fn turn(&mut self, text: &str, input: Input) -> Result<TurnOutcome, AiError> {
        let answer = self.ask(text).run()?;
        Ok(self.take(text, input, answer))
    }

    /// What the tutor is asked about the learner's `text`: both calls with everything they
    /// need, so they run without the session. The session itself does not change.
    pub fn ask(&self, text: &str) -> Asking {
        let mut messages = self.messages.clone();
        messages.push(ChatMessage::user(text));
        let request = |system: String| TurnRequest {
            system,
            messages: messages.clone(),
            max_tokens: 600,
        };
        Asking {
            provider: self.provider.clone(),
            analysis: request(analysis_system(&self.scenario, &self.lang, &self.grammar)),
            reply: request(self.reply_instructions()),
            cross_check: self.cross_check,
            register: self.scenario.register,
        }
    }

    /// Takes the tutor's answer to the learner's `text` into the session: the turn with its
    /// corrections, goals, XP and help.
    pub fn take(&mut self, text: &str, input: Input, answer: Answer) -> TurnOutcome {
        let Answer { analysis, reply } = answer;
        self.messages.push(ChatMessage::user(text));
        self.messages.push(ChatMessage::assistant(plain(&reply)));

        let playable: Vec<String> = self.goals().iter().map(|g| g.id.clone()).collect();
        let goals_met_now: Vec<String> = analysis
            .goals_met
            .iter()
            .filter(|g| playable.contains(g) && self.goals_met.insert((*g).clone()))
            .cloned()
            .collect();
        // Reached for the talk, but a turn analysed with gaps shows nothing about the level.
        if analysis.incomplete {
            self.goals_unjudged.extend(goals_met_now.iter().cloned());
        }

        let from_help = self.help_level == 3
            && self
                .goals()
                .iter()
                .any(|g| crate::norm(&g.help.example) == crate::norm(text));
        let earlier: Vec<&str> = self.turns.iter().map(|t| t.learner.as_str()).collect();
        let (mut xp, xp_reason) = turn_xp(text, input, &earlier, from_help);
        // A goal reached by reading out the full example is practice, not an achievement:
        // no goal bonus (/LF603/).
        if !from_help {
            xp += XP_GOAL * goals_met_now.len() as u32;
        }

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

        // The regulator inside the session takes no step from a turn analysed with gaps.
        let eased = if analysis.incomplete {
            None
        } else {
            self.live.turn(analysis.understood, &self.difficulty)
        };
        if let Some(d) = &eased {
            self.difficulty = d.clone();
        }

        let own_markup = own_reading(text, &analysis.reading);
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
            markup: own_markup.clone(),
            incomplete: analysis.incomplete,
        });
        self.aid_on_line = false;
        TurnOutcome {
            own_markup,
            reply,
            understood: analysis.understood,
            shown,
            interrupt,
            goals_met_now,
            xp,
            xp_reason,
            eased,
            help,
            incomplete: analysis.incomplete,
        }
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

    /// The character's line that learner turn `turn` answered (with furigana markup for the
    /// opening): the opening for the first turn, otherwise the reply before it.
    pub fn cue(&self, turn: usize) -> Option<&str> {
        match turn {
            0 => Some(self.opening()),
            n => self.turns.get(n - 1).map(|t| t.reply.as_str()),
        }
        .filter(|c| !c.is_empty())
    }

    pub fn summary(&self) -> Summary {
        let mut cues = BTreeMap::new();
        let mut mistakes = BTreeMap::new();
        // Mistakes only: an "also possible" (shown with much feedback) says the learner's
        // form is right, so it is no item for the review, where that form would then be
        // marked as the mistake, and it is none of the most important corrections (/LF303/).
        let mistaken = |t: &TurnRecord| -> Vec<Shown> {
            visible(&counted(t), self.amount)
                .into_iter()
                .filter(|s| !s.correction.also_possible)
                .collect()
        };
        for (i, t) in self.turns.iter().enumerate() {
            for s in mistaken(t) {
                mistakes
                    .entry(crate::norm(&s.correction.better))
                    .or_insert_with(|| crate::srs::in_context(&t.learner, &s.correction.original));
                if let Some(cue) = self.cue(i) {
                    cues.entry(crate::norm(&s.correction.better))
                        .or_insert_with(|| cue.to_string());
                }
            }
        }
        let mut all: Vec<Shown> = self.turns.iter().flat_map(mistaken).collect();
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
            top: unique.iter().take(3).cloned().collect(),
            corrected: unique,
            review,
            cues,
            mistakes,
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
            // Time spoken is time practised, whatever the tutor made of the turn.
            if t.input == Input::Spoken {
                s.speaking_secs += speaking_secs(&t.learner);
            }
            // A turn the tutor analysed with gaps is no evidence (T-11).
            if t.incomplete {
                continue;
            }
            s.turns += 1;
            if t.understood {
                s.understood += 1;
                if !t.aided && !t.from_help {
                    s.understood_unaided += 1;
                }
            }
            s.typed_turns += u32::from(t.input == Input::Typed);
            let kinds = mistake_kinds(t);
            for kind in &kinds {
                if kind == "register" {
                    s.register_mistakes += 1;
                }
                *s.mistakes.entry(kind.clone()).or_default() += 1;
            }
            s.clean_turns += u32::from(kinds.is_empty());
        }
        let judged = || {
            self.goals_met
                .iter()
                .filter(|g| !self.goals_unjudged.contains(*g))
        };
        s.goals_met = judged().count() as u32;
        s.goals_unaided = judged().filter(|g| !self.goals_helped.contains(*g)).count() as u32;
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
                !t.incomplete
                    && !counted(t)
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

/// Plain forms between friends are right (/LF303/): in a casual scenario a register
/// correction that only asks for more politeness is an alternative ("also possible"), not a
/// mistake. A form that is rude even there (おまえ to a stranger, marked wrong) stays.
pub fn fit_register(corrections: &mut [Correction], register: Register) {
    if register != Register::Casual {
        return;
    }
    for c in corrections.iter_mut() {
        if c.kind == kaiwa_ai::CorrectionKind::Register && c.severity <= Severity::Unnatural {
            c.also_possible = true;
        }
    }
}

/// The kinds of the mistakes a turn counts (severity unnatural or above, not an alternative,
/// not misrecognised, not disputed), e.g. "particle"; for the learner model and the statistics.
pub fn mistake_kinds(t: &TurnRecord) -> Vec<String> {
    counted(t)
        .iter()
        .filter(|c| !c.also_possible && c.severity >= Severity::Unnatural)
        .map(|c| {
            serde_json::to_value(c.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        })
        .collect()
}

/// The corrections of a turn that still count: none for a misrecognised turn, none disputed.
pub(crate) fn counted(t: &TurnRecord) -> Vec<Correction> {
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

    #[test]
    fn a_correction_remembers_the_line_it_answered() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        s.turn("マラです。よろしくおねがいします。", Input::Typed)
            .unwrap();
        s.turn("コーヒーを好きです。", Input::Typed).unwrap();
        assert_eq!(s.cue(0), Some(s.opening()));
        let asked = s.turns()[0].reply.clone();
        assert_eq!(s.cue(1), Some(asked.as_str()));
        assert_eq!(s.cue(3), None, "no line past the last reply");
        let sum = s.summary();
        assert_eq!(
            sum.cues.get(&crate::norm("コーヒーが好きです")),
            Some(&asked),
            "{:?}",
            sum.cues
        );
    }

    fn provider() -> Arc<dyn ChatProvider> {
        Arc::new(
            MockProvider::from_json(include_str!("../../../qa/fixtures/n5-02-jikoshoukai.json"))
                .unwrap()
                .with_fallback("すみません、もう一度お願いします。"),
        )
    }

    /// The recorded answers without a fallback: an unknown line is a tutor that fails.
    fn strict() -> Arc<dyn ChatProvider> {
        Arc::new(
            MockProvider::from_json(include_str!("../../../qa/fixtures/n5-02-jikoshoukai.json"))
                .unwrap(),
        )
    }

    #[test]
    fn another_way_to_say_it_is_no_item_for_the_review() {
        // Among friends the plain form is right; the recorded tutor suggests the polite one
        // anyway, which much feedback shows as "also possible" (/LF303/).
        let scenario =
            Scenario::from_json(include_str!("../../../data/scenarios/n5-09-tomodachi.json"))
                .unwrap();
        let tutor: Arc<dyn ChatProvider> = Arc::new(
            MockProvider::from_json(include_str!("../../../qa/fixtures/n5-09-tomodachi.json"))
                .unwrap()
                .with_fallback("そうなんだ。"),
        );
        let mut s = Session::new(scenario, tutor, Amount::Much, "de");
        let out = s.turn("うん、行く！", Input::Spoken).unwrap();
        assert!(
            out.shown.iter().any(|x| x.correction.also_possible),
            "the fixture shows an alternative"
        );
        let summary = s.summary();
        assert!(summary.corrected.is_empty() && summary.top.is_empty());
        assert!(summary.mistakes.is_empty() && summary.cues.is_empty());
        assert!(
            !summary.review.iter().any(|r| r.contains("行きます")),
            "{:?}",
            summary.review
        );
    }

    #[test]
    fn a_failed_tutor_call_leaves_the_session_as_it_was() {
        let mut s = Session::new(scenario(), strict(), Amount::Normal, "de");
        s.turn("マラです。よろしくおねがいします。", Input::Spoken)
            .unwrap();
        let before = s.snapshot();
        assert!(s.turn("これはきろくにありません。", Input::Spoken).is_err());
        assert_eq!(s.snapshot(), before);
        // A translation that fails is no aid (/LF212/): the learner got nothing.
        assert!(s.translate("これもありません。").is_err());
        assert_eq!(s.snapshot(), before);
    }

    #[test]
    fn asking_without_the_session_gives_the_same_turn() {
        let line = "コーヒーを好きです。";
        let mut whole = Session::new(scenario(), provider(), Amount::Normal, "de");
        let a = whole.turn(line, Input::Spoken).unwrap();
        let mut halves = Session::new(scenario(), provider(), Amount::Normal, "de");
        let asking = halves.ask(line);
        // Asking changes nothing; the session is free meanwhile.
        assert_eq!(halves.turns().len(), 0);
        let b = halves.take(line, Input::Spoken, asking.run().unwrap());
        assert_eq!(
            (a.reply, a.xp, a.shown.len()),
            (b.reply, b.xp, b.shown.len())
        );
        assert_eq!(whole.snapshot(), halves.snapshot());
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
    fn a_reply_keeps_the_tutors_readings_and_the_history_has_none() {
        let p: Arc<dyn ChatProvider> = Arc::new(
            MockProvider::new([]).with_fallback("お{仕事|しごと}は{何|なん}ですか。{角|kado}"),
        );
        let mut s = Session::new(scenario(), p, Amount::Little, "de");
        let out = s.turn("こんにちは。", Input::Typed).unwrap();
        assert_eq!(out.reply, "お{仕事|しごと}は{何|なん}ですか。角");
        assert_eq!(s.messages.last().unwrap().content, "お仕事は何ですか。角");
    }

    #[test]
    fn the_tutors_reading_of_the_learners_line_counts_only_when_it_is_the_line() {
        assert_eq!(
            own_reading("何を食べますか。", "{何|なに}を{食|た}べますか。"),
            "{何|なに}を{食|た}べますか。"
        );
        // Not kana, a changed character, or no reading at all: the dictionary's stay.
        assert_eq!(
            own_reading("何を食べますか。", "{何|nani}を食べますか。"),
            ""
        );
        assert_eq!(
            own_reading("何を食べますか。", "{何|なに}を{食|た}べますか？"),
            ""
        );
        assert_eq!(
            own_reading("ビール二ください。", "ビール{二|ふた}つください。"),
            "",
            "a corrected line is not the learner's"
        );
        assert_eq!(own_reading("はい。", "はい。"), "");
        assert_eq!(own_reading("何を食べますか。", ""), "");
    }

    #[test]
    fn a_turn_brings_the_tutors_reading_and_keeps_it() {
        let fixture = r#"[{ "learner": "何を食べますか。", "reply": "すしです。",
            "analysis": { "understood": true, "goals_met": [], "corrections": [], "new_words": [],
                          "reading": "{何|なに}を{食|た}べますか。" } }]"#;
        let p: Arc<dyn ChatProvider> = Arc::new(MockProvider::from_json(fixture).unwrap());
        let mut s = Session::new(scenario(), p, Amount::Little, "de");
        let out = s.turn("何を食べますか。", Input::Typed).unwrap();
        assert_eq!(out.own_markup, "{何|なに}を{食|た}べますか。");
        assert_eq!(s.turns()[0].markup, out.own_markup);
        let json = serde_json::to_string(&s.snapshot()).unwrap();
        assert!(json.contains("{何|なに}"), "kept for a resumed session");
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
    fn translations_count_as_aids() {
        let mut s = Session::new(scenario(), provider(), Amount::Little, "de");
        let opening = crate::plain(s.opening());
        assert_eq!(
            s.translate(&opening).unwrap(),
            "Freut mich. Ich bin Tanaka. Wie heißen Sie?"
        );
        s.turn("マラです。よろしくおねがいします。", Input::Spoken)
            .unwrap();
        let line = s.last_line().to_string();
        assert_eq!(
            s.translate(&line).unwrap(),
            "Sie sind Mara. Freut mich. Woher kommen Sie?"
        );
        assert_eq!(s.stats().aids, 2);
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
