//! Instructions for the two calls of a turn (ADR-004). They go to whichever provider is set;
//! the wording is tried against real models in the owner's local test, not in CI.

use crate::scenario::{Register, Scenario};

fn register_words(r: Register) -> &'static str {
    match r {
        Register::Casual => "casual Japanese (plain forms) as between friends",
        Register::Polite => "polite Japanese (です/ます forms)",
        Register::Keigo => "respectful business Japanese (尊敬語 and 謙譲語 where they belong)",
    }
}

fn goals(s: &Scenario) -> String {
    s.goals
        .iter()
        .map(|g| format!("- {}: {}", g.id, g.text.en))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The character's reply: in role, at the learner's level, recasting mistakes (/LF305/).
pub fn reply_system(s: &Scenario, lang: &str) -> String {
    format!(
        "You are a character in a Japanese speaking exercise. {character}\n\
         Speak only {register}. One or two short sentences per turn, words a beginner (JLPT N5) knows; \
         at most one new word per turn.\n\
         When the learner makes a mistake, do not point it out: use the correct form naturally in your answer (recast).\n\
         When the learner switches to {lang_name} to ask for a word, give the Japanese word and ask them to say it.\n\
         Lead the conversation towards these goals, one at a time:\n{goals}\n\
         Never leave the role; if the learner says something inappropriate, steer back to the scene kindly.\n\
         Answer with the Japanese utterance only, no translation, no romaji.",
        character = s.character.en,
        register = register_words(s.register),
        lang_name = if lang == "de" { "German" } else { "English" },
        goals = goals(s),
    )
}

/// The analysis of the learner's last turn: structured, strict about false alarms (/LL110/).
pub fn analysis_system(s: &Scenario, lang: &str) -> String {
    format!(
        "You analyse the learner's last turn in a Japanese speaking exercise. The scene: {character} \
         The expected register is {register}.\n\
         Report:\n\
         - understood: whether the character could understand what the learner meant;\n\
         - goals_met: ids of the goals the turn reached, from this list:\n{goals}\n\
         - corrections: only real mistakes. A sentence that is correct for the scene gets no correction. \
           If something else would only be another way to say it, set also_possible. \
           Severity: blocks_understanding (the meaning is lost), wrong (a grammar or word mistake), \
           unnatural (correct but odd or the wrong register), style (a nuance). \
           Ignore fillers like えーと and あの. \
           The explanation is one or two short sentences in {lang_name}, with the Japanese example;\n\
         - new_words: words the learner used or met for the first time, dictionary form.",
        character = s.character.en,
        register = register_words(s.register),
        goals = goals(s),
        lang_name = if lang == "de" { "German" } else { "English" },
    )
}
