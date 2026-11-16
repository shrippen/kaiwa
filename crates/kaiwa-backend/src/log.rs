//! The conversation's log as the page shows it: one entry per line, the learner's and the
//! character's. The format lives here and nowhere else; the app only passes what happens and
//! shows [`Log::view`].
//!
//! The log and the session's turns are not the same list: a line the tutor never answered
//! (the network, the cost limit) stays in the log, marked `failed`, but is no turn of the
//! session. Everything the page does to "this line" (misrecognised /LF113/, dispute /LF307/,
//! the recording's number /LF712/) goes through [`Log::turn_of`].

use kaiwa_core::{Input, TurnOutcome, feedback, session::SessionState};
use serde_json::{Value, json};

use crate::{lookup, summary};

/// What a mora check heard in a word, as far as the page words it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoraKind {
    /// A っ left out.
    Geminate,
    /// A long vowel spoken short.
    Vowel,
}

/// One mora hint: in `surface` (as written) the word `meant` (kana) sounded like `heard`.
#[derive(Clone, Debug)]
pub struct MoraNote {
    pub kind: MoraKind,
    pub surface: String,
    pub meant: String,
    pub heard: String,
}

#[derive(Debug, Default)]
pub struct Log {
    entries: Vec<Value>,
}

impl Log {
    /// A new talk: the character's opening line.
    pub fn opened(opening: &str) -> Self {
        Self {
            entries: vec![json!({ "who": "character", "markup": opening })],
        }
    }

    /// The log of a continued talk: the opening, then each turn with its corrections and the
    /// character's reply. Equal to the log the same turns built live.
    pub fn restored(opening: &str, state: &SessionState, lang: &str) -> Self {
        let mut log = Self::opened(opening);
        for t in &state.turns {
            let shown = feedback::visible(
                if t.misrecognized { &[] } else { &t.corrections },
                state.amount,
            );
            let corrections: Vec<Value> = shown
                .iter()
                .map(|x| {
                    let mut v = summary::shown_view(x, lang);
                    v["disputed"] = json!(t.disputed.contains(&x.correction.better));
                    v
                })
                .collect();
            let mut line = json!({
                "who": "learner",
                "text": t.learner,
                "typed": t.input == Input::Typed,
                "corrections": corrections,
                "pending": false,
                "misrecognized": t.misrecognized,
                "incomplete": t.incomplete,
            });
            if !t.markup.is_empty() {
                line["markup"] = t.markup.clone().into();
            }
            log.entries.push(line);
            log.add_reply(&t.reply);
        }
        log
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The opening line's markup.
    pub fn opening(&self) -> &str {
        self.entries
            .first()
            .and_then(|e| e["markup"].as_str())
            .unwrap_or_default()
    }

    /// The markup of the character's line at `index`; `None` for the learner's and beyond.
    pub fn character_line(&self, index: usize) -> Option<&str> {
        let entry = self
            .entries
            .get(index)
            .filter(|e| e["who"] == "character")?;
        Some(entry["markup"].as_str().unwrap_or_default())
    }

    /// The learner's line goes in, waiting for the tutor; returns its index.
    pub fn add_learner(&mut self, text: &str, input: Input) -> usize {
        self.entries.push(json!({
            "who": "learner",
            "text": text,
            "typed": input == Input::Typed,
            "corrections": [],
            "pending": true,
            "misrecognized": false,
        }));
        self.entries.len() - 1
    }

    /// The character's reply.
    pub fn add_reply(&mut self, markup: &str) {
        self.entries
            .push(json!({ "who": "character", "markup": markup }));
    }

    /// The tutor answered the learner's line at `index`: it shows what the tutor made of it,
    /// and the reply follows.
    pub fn answer(&mut self, index: usize, outcome: &TurnOutcome, lang: &str) {
        if let Some(line) = self.entries.get_mut(index) {
            show_outcome(line, outcome, lang);
        }
        self.add_reply(&outcome.reply);
    }

    /// The tutor did not answer: the line stays to be read, but it is no turn of the session.
    pub fn fail(&mut self, index: usize) {
        let Some(line) = self.entries.get_mut(index) else {
            return;
        };
        line["pending"] = json!(false);
        line["failed"] = json!(true);
    }

    /// The mora check's hints for the learner's line at `index`, if it still says `text`.
    pub fn add_mora(&mut self, index: usize, text: &str, notes: &[MoraNote], lang: &str) {
        let Some(line) = self.entries.get_mut(index) else {
            return;
        };
        if line["text"] != text.trim() {
            return;
        }
        line["mora"] = notes.iter().map(|n| mora_view(n, lang)).collect();
    }

    /// The learner marked the line at `index` as misrecognised: no corrections stand.
    pub fn misrecognized(&mut self, index: usize) {
        let Some(line) = self.entries.get_mut(index) else {
            return;
        };
        line["corrections"] = json!([]);
        line["misrecognized"] = json!(true);
    }

    /// The learner disputed the correction `better` of the line at `index`.
    pub fn disputed(&mut self, index: usize, better: &str) {
        let Some(list) = self
            .entries
            .get_mut(index)
            .and_then(|e| e["corrections"].as_array_mut())
        else {
            return;
        };
        for c in list.iter_mut().filter(|c| c["better"] == better) {
            c["disputed"] = json!(true);
        }
    }

    /// Whether the character's line at `index` has its translation or is being translated.
    pub fn is_translated(&self, index: usize) -> bool {
        let Some(line) = self.entries.get(index) else {
            return false;
        };
        line.get("translation").is_some() || line["translating"] == true
    }

    pub fn translating(&mut self, index: usize) {
        if let Some(line) = self.entries.get_mut(index) {
            line["translating"] = json!(true);
        }
    }

    /// The translation is no longer asked for (it came or it failed).
    pub fn translated_not(&mut self, index: usize) {
        let Some(line) = self.entries.get_mut(index).and_then(|e| e.as_object_mut()) else {
            return;
        };
        line.remove("translating");
    }

    pub fn translation(&mut self, index: usize, text: &str) {
        if let Some(line) = self.entries.get_mut(index) {
            line["translation"] = json!(text);
        }
    }

    /// The word `word` of the line at `index`, as the page offered it for a look-up.
    pub fn word(&self, index: usize, word: usize) -> Option<kaiwa_lang::LineWord> {
        let line = self.entries.get(index)?;
        serde_json::from_value(line["words"].get(word)?.clone()).ok()
    }

    /// The session's turn index of the log entry at `index`; `None` for the character's
    /// lines, for a line the tutor never answered and beyond the log.
    pub fn turn_of(&self, index: usize) -> Option<usize> {
        is_turn(self.entries.get(index)?).then(|| self.turns_before(index))
    }

    /// How many turns of the session the log holds before `index`.
    pub fn turns_before(&self, index: usize) -> usize {
        let end = index.min(self.entries.len());
        self.entries[..end].iter().filter(|e| is_turn(e)).count()
    }

    /// How many turns of the session the log holds: the index the next one gets.
    pub fn turns(&self) -> usize {
        self.turns_before(self.entries.len())
    }

    /// The JSON the page reads. Every line comes as words to look up (/LF112/, Kante 1.33),
    /// once per line: the character's with the tutor's furigana, the learner's own with the
    /// dictionary's until the tutor's reading comes with its answer (/LF110/, ADR-004).
    pub fn view(&mut self) -> String {
        for entry in &mut self.entries {
            if entry.get("segments").is_some() {
                continue;
            }
            let line = match entry["who"].as_str() {
                Some("character") => lookup::line(entry["markup"].as_str().unwrap_or_default()),
                Some("learner") => match entry["markup"].as_str() {
                    Some(markup) => lookup::line(markup),
                    None => lookup::own_line(entry["text"].as_str().unwrap_or_default()),
                },
                _ => continue,
            };
            entry["segments"] = line["segments"].clone();
            entry["words"] = line["words"].clone();
        }
        Value::Array(self.entries.clone()).to_string()
    }
}

/// Whether a log entry is a line of the learner's that became a turn of the session.
fn is_turn(entry: &Value) -> bool {
    entry["who"] == "learner" && entry["failed"] != true
}

/// The learner's line of the log with what the tutor made of it.
fn show_outcome(turn: &mut Value, outcome: &TurnOutcome, lang: &str) {
    turn["corrections"] = outcome
        .shown
        .iter()
        .map(|x| {
            let mut v = summary::shown_view(x, lang);
            v["disputed"] = json!(false);
            v
        })
        .collect();
    turn["pending"] = json!(false);
    turn["incomplete"] = json!(outcome.incomplete);
    // The tutor's furigana replace the dictionary's (ADR-004).
    if outcome.own_markup.is_empty() {
        return;
    }
    turn["markup"] = outcome.own_markup.clone().into();
    if let Some(t) = turn.as_object_mut() {
        t.remove("segments");
        t.remove("words");
    }
}

/// A mora hint as the page shows it: cautious, in the UI language, no severity (K-02).
fn mora_view(n: &MoraNote, lang: &str) -> Value {
    let de = lang == "de";
    let word = if n.surface == n.meant {
        n.meant.clone()
    } else {
        format!("{} ({})", n.surface, n.meant)
    };
    let heard = &n.heard;
    let text = match (n.kind, de) {
        (MoraKind::Geminate, true) => format!(
            "Das っ in {word} war kaum zu hören, es klang eher wie {heard}. Vor dem nächsten Laut eine kurze Pause halten, so lang wie eine Silbe. Die Prüfung irrt sich manchmal; das zählt nicht als Fehler."
        ),
        (MoraKind::Geminate, false) => format!(
            "The っ in {word} was hard to hear; it sounded more like {heard}. Pause briefly before the next sound, as long as one syllable. The check is sometimes wrong; this does not count as a mistake."
        ),
        (MoraKind::Vowel, true) => format!(
            "Der lange Vokal in {word} klang eher kurz, wie {heard}. Ihn etwa doppelt so lang halten. Die Prüfung irrt sich manchmal; das zählt nicht als Fehler."
        ),
        (MoraKind::Vowel, false) => format!(
            "The long vowel in {word} sounded short, like {heard}. Hold it about twice as long. The check is sometimes wrong; this does not count as a mistake."
        ),
    };
    json!({
        "title": format!("{} → {}?", n.meant, n.heard),
        "tierText": if de { "Aussprache · Hinweis" } else { "Pronunciation · hint" },
        "meta": if de { "Moren-Länge" } else { "Mora length" },
        "text": text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TempBackend;
    use crate::flow::Talk;
    use kaiwa_core::Moment;

    const SCENARIO: &str = "n5-02-jikoshoukai";
    const LANG: &str = "en";

    fn entry(log: &Log, i: usize) -> &Value {
        &log.entries[i]
    }

    /// A talk with the recorded tutor, and a live log of what it said to the first goal's
    /// example line.
    fn spoken() -> (Talk, Log, TempBackend, String) {
        let b = TempBackend::new("log-live", true).unwrap();
        let mut talk = Talk::begin(
            &b,
            SCENARIO,
            Moment::new(1_791_100_000, 0),
            crate::flow::Length::Full,
        )
        .unwrap()
        .talk;
        let mut log = Log::opened(talk.session().opening());
        let said = talk.session().goals()[0].help.example.clone();

        let index = log.add_learner(&said, Input::Spoken);
        assert_eq!(entry(&log, index)["pending"], true);
        let turned = talk
            .say(&b, &said, Input::Spoken, Moment::new(1_791_100_000, 0))
            .unwrap()
            .unwrap();
        log.answer(index, &turned.outcome, LANG);
        (talk, log, b, said)
    }

    #[test]
    fn a_turn_goes_from_pending_to_answered() {
        let (_talk, log, _b, said) = spoken();
        assert_eq!(log.len(), 3);
        let line = entry(&log, 1);
        assert_eq!(line["who"], "learner");
        assert_eq!(line["text"], said.as_str());
        assert_eq!(line["pending"], false);
        assert_eq!(line["incomplete"], false);
        assert_eq!(entry(&log, 2)["who"], "character");
        assert_eq!(log.character_line(2), entry(&log, 2)["markup"].as_str());
        assert_eq!(log.character_line(1), None);
        assert_eq!(log.turns(), 1);
    }

    #[test]
    fn a_continued_talk_has_the_log_the_live_talk_built() {
        let (talk, live, _b, _said) = spoken();
        let state = talk.session().snapshot();
        let restored = Log::restored(talk.session().opening(), &state, LANG);
        assert_eq!(restored.entries, live.entries);
    }

    #[test]
    fn a_line_the_tutor_never_answered_is_no_turn() {
        // A fails (network), B is the same line again, C follows.
        let mut log = Log::opened("もしもし");
        let a = log.add_learner("A", Input::Typed);
        log.fail(a);
        assert_eq!(entry(&log, a)["pending"], false);
        let b = log.add_learner("B", Input::Typed);
        log.add_reply("r");
        let c = log.add_learner("C", Input::Typed);
        log.add_reply("r");

        assert_eq!(log.turn_of(a), None);
        assert_eq!(log.turn_of(b), Some(0), "B is the session's first turn");
        assert_eq!(log.turn_of(c), Some(1));
        assert_eq!(log.turn_of(0), None);
        assert_eq!(log.turn_of(9), None);
        assert_eq!(log.turns(), 2);
        assert_eq!(log.turns_before(c), 1);
    }

    #[test]
    fn a_misrecognised_line_loses_its_corrections() {
        let (_talk, mut log, _b, _said) = spoken();
        log.entries[1]["corrections"] = json!([{ "better": "x", "disputed": false }]);
        log.misrecognized(1);
        assert_eq!(entry(&log, 1)["misrecognized"], true);
        assert_eq!(entry(&log, 1)["corrections"], json!([]));
    }

    #[test]
    fn a_dispute_marks_the_matching_correction_only() {
        let mut log = Log::opened("もしもし");
        let i = log.add_learner("A", Input::Typed);
        log.entries[i]["corrections"] = json!([
            { "better": "x", "disputed": false },
            { "better": "y", "disputed": false },
        ]);
        log.disputed(i, "y");
        assert_eq!(entry(&log, i)["corrections"][0]["disputed"], false);
        assert_eq!(entry(&log, i)["corrections"][1]["disputed"], true);
    }

    #[test]
    fn a_mora_hint_stays_with_its_line() {
        let mut log = Log::opened("もしもし");
        let i = log.add_learner("きって", Input::Spoken);
        let note = MoraNote {
            kind: MoraKind::Geminate,
            surface: "きって".into(),
            meant: "きって".into(),
            heard: "きて".into(),
        };
        log.add_mora(i, "other", std::slice::from_ref(&note), LANG);
        assert!(entry(&log, i).get("mora").is_none(), "another line now");

        log.add_mora(i, " きって ", &[note], LANG);
        assert_eq!(entry(&log, i)["mora"][0]["title"], "きって → きて?");
    }

    #[test]
    fn a_translation_is_asked_once() {
        let mut log = Log::opened("もしもし");
        assert!(!log.is_translated(0));
        log.translating(0);
        assert!(log.is_translated(0));
        log.translated_not(0);
        assert!(!log.is_translated(0));
        log.translation(0, "Hello");
        assert!(log.is_translated(0));
    }
}
