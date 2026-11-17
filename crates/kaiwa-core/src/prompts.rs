//! Instructions for the two calls of a turn (ADR-004). They go to whichever provider is set;
//! the wording is tried against real models in the owner's local test, not in CI.

use crate::difficulty::{Difficulty, Knob};
use crate::jlpt::Jlpt;
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

/// The JLPT level the scenario's unit is oriented on (`unit`, [`crate::curriculum::Unit::jlpt`]).
/// Free talk has no unit: it takes the learner's own level (/LF116/), `own` on the learner
/// model's scale ([`crate::learner::LEVELS`]); without one it sits at the beginner levels.
fn jlpt(unit: Option<&str>, own: Option<f64>) -> &str {
    if let Some(level) = unit {
        return level;
    }
    match own {
        None => "N5/N4",
        Some(l) => Jlpt::of(l).name(),
    }
}

/// How the difficulty reads for the character (/LF211/), at the level of the scenario.
fn level_words(unit: Option<&str>, d: &Difficulty, own: Option<f64>) -> String {
    let clauses = match d.get(Knob::Clauses) {
        0 => "no subordinate clauses",
        1 => "at most one simple subordinate clause (〜から, 〜て)",
        _ => "subordinate clauses where natural",
    };
    let level = jlpt(unit, own);
    let words = match d.unknown_percent() {
        0 => format!("only words a learner at JLPT {level} knows"),
        p => format!("at most {p} % of the words above the learner's level (JLPT {level})"),
    };
    format!(
        "One or two sentences per turn, each at most {max} words, {clauses}, {words}.",
        max = d.max_words(),
    )
}

/// The character's reply: in role, back to the scene after something inappropriate (/LL302/),
/// in standard Japanese up to N3 (Q-11: dialect only from N2, to listen), at the learner's level, recasting mistakes (/LF305/), giving the Japanese word when the
/// learner asks in their own language (/LF122/). `unit` is the JLPT level of the scenario's
/// unit.
pub fn reply_system(
    s: &Scenario,
    lang: &str,
    d: &Difficulty,
    grammar: &[String],
    unit: Option<&str>,
    own: Option<f64>,
) -> String {
    let grammar = if grammar.is_empty() {
        String::new()
    } else {
        format!(
            "This unit practises: {}. Use these forms where they fit the scene, so the learner hears them; otherwise keep to simple structures.\n",
            grammar.join(", ")
        )
    };
    format!(
        "You are a character in a Japanese speaking exercise. {character}\n\
         Speak only {register}, in standard Japanese (標準語) without any regional dialect. {level}\n\
         {grammar}\
         When the learner makes a mistake, do not point it out: use the correct form naturally in your answer (recast).\n\
         When the learner switches to {lang_name} to ask for a word, give the Japanese word and ask them to say it.\n\
         Lead the conversation towards these goals, one at a time:\n{goals}\n\
         Never leave the role; if the learner says something inappropriate, steer back to the scene kindly.\n\
         Answer with the Japanese utterance only, no translation, no romaji.\n\
         Write the reading of every word with kanji in braces, as it is read in this sentence, \
         in hiragana: {{漢字|かんじ}}; okurigana and kana stay outside: {{仕事|しごと}}は{{何|なん}}ですか。\
         {{食|た}}べました。{{角|かど}}を{{右|みぎ}}へ。Katakana words and kana stay as they are.",
        character = s.character.en,
        register = register_words(s.register),
        level = level_words(unit, d, own),
        lang_name = if lang == "de" { "German" } else { "English" },
        goals = goals(s),
    )
}

/// The analysis of the learner's last turn: structured, strict about false alarms (/LL110/),
/// fillers are no mistakes (/LF102/).
pub fn analysis_system(s: &Scenario, lang: &str, grammar: &[String]) -> String {
    let grammar = if grammar.is_empty() {
        String::new()
    } else {
        format!(
            "The unit practises: {}; a mistake in one of these forms matters most.\n",
            grammar.join(", ")
        )
    };
    format!(
        "You analyse the learner's last turn in a Japanese speaking exercise. The scene: {character} \
         The expected register is {register}.\n\
         {grammar}\
         Report one JSON object with exactly these fields:\n\
         - understood: whether the character could understand what the learner meant;\n\
         - goals_met: ids of the goals the turn reached, from this list:\n{goals}\n\
         - corrections: only real mistakes. A sentence that is correct for the scene gets no correction. \
           If something else would only be another way to say it, set also_possible. \
           Severity: blocks_understanding (the meaning is lost), wrong (a grammar or word mistake), \
           unnatural (correct but odd or the wrong register), style (a nuance). \
           Ignore fillers like えーと and あの. \
           The explanation is one or two short sentences in {lang_name}, with the Japanese example. \
           Each correction is an object with severity, kind (particle, conjugation, word_choice, \
           register, word_order, counter, omission, loanword, tense or pronunciation), original \
           (the learner's words), better (the better form), explanation and also_possible \
           (true or false);\n\
         - new_words: words the learner used or met for the first time, dictionary form;\n\
         - reading: the learner's last turn exactly as given, every character kept (mistakes too), \
           each kanji word followed by its reading in this sentence as {{漢字|かな}} in hiragana \
           (何を → {{何|なに}}を, 何ですか → {{何|なん}}ですか).",
        character = s.character.en,
        register = register_words(s.register),
        goals = goals(s),
        lang_name = if lang == "de" { "German" } else { "English" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_difficulty_shapes_the_reply() {
        let s = Scenario::from_json(include_str!(
            "../../../data/scenarios/n5-02-jikoshoukai.json"
        ))
        .unwrap();
        let easy = reply_system(&s, "de", &Difficulty::default(), &[], Some("N5"), None);
        assert!(easy.contains("at most 4 words") && easy.contains("no subordinate"));
        let mut d = Difficulty::default();
        d.set(Knob::Length, 4);
        d.set(Knob::Vocabulary, 3);
        let hard = reply_system(&s, "de", &d, &[], Some("N5"), None);
        assert!(hard.contains("at most 14 words") && hard.contains("3 %"));
        assert!(easy.contains("JLPT N5") && easy.contains("標準語"));
    }

    #[test]
    fn an_n4_scenario_speaks_at_n4_without_dialect() {
        let s =
            Scenario::from_json(include_str!("../../../data/scenarios/n4-14-guchi.json")).unwrap();
        let p = reply_system(
            &s,
            "de",
            &Difficulty::default(),
            &["〜てしまいました".into()],
            Some("N4"),
            Some(8.0),
        );
        assert!(p.contains("This unit practises: 〜てしまいました."));
        assert!(
            !reply_system(&s, "de", &Difficulty::default(), &[], Some("N4"), None)
                .contains("practises")
        );
        let a = analysis_system(&s, "de", &["〜てしまいました".into()]);
        assert!(a.contains("The unit practises: 〜てしまいました"));
        // A server without schema support learns the fields and the kinds from the prompt.
        for field in [
            "original",
            "better",
            "word_choice",
            "also_possible",
            "JSON object",
        ] {
            assert!(a.contains(field), "{field}");
        }
        assert!(p.contains("JLPT N4"), "{p}");
        assert!(p.contains("without any regional dialect"));
        assert!(p.contains("casual Japanese"));
    }

    #[test]
    fn free_talk_speaks_at_the_learners_own_level() {
        let s =
            Scenario::from_json(include_str!("../../../data/scenarios/frei-shumi.json")).unwrap();
        let at = |own| reply_system(&s, "de", &Difficulty::default(), &[], None, own);
        assert!(at(None).contains("JLPT N5/N4"));
        // a0, n5, n4_low, n4, n3, n2, native_casual
        for (own, level) in [
            (1.0, "N5"),
            (3.0, "N5"),
            (4.0, "N4"),
            (5.0, "N4"),
            (6.0, "N3"),
            (7.0, "N2"),
            (8.0, "N1"),
        ] {
            let p = at(Some(own));
            assert!(
                p.contains(&format!("JLPT {level})")) || p.contains(&format!("JLPT {level} ")),
                "{own}: {p}"
            );
            assert!(!p.contains("N5/N4"), "{own}");
        }
    }

    #[test]
    fn the_level_changes_at_its_borders() {
        for (own, level) in [
            (3.49, "N5"),
            (3.5, "N4"),
            (5.49, "N4"),
            (5.5, "N3"),
            (6.49, "N3"),
            (6.5, "N2"),
            (7.49, "N2"),
            (7.5, "N1"),
        ] {
            assert_eq!(jlpt(None, Some(own)), level, "{own}");
        }
    }
}
