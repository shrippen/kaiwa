//! The aids beside a turn: help, a listening aid, a translation (/LF111/, /LF114/, /LF212/).
//! Each one counts, is saved and, for the translation, books its cost here; a front end only
//! passes what the learner did and shows the result.
//!
//! ```text
//!  help / hear ───────────────► count, save
//!  ask_translation ─► Translation::run (no lock) ─► take_translation ─► count, save, book
//! ```
//!
//! No aid counts while the tutor works on a sent turn: it would count for a turn whose answer
//! came without it. A translation asked before the turn and arriving during it is shown, not
//! counted.

use kaiwa_ai::AiError;
use kaiwa_core::Moment;
use kaiwa_core::session::{Aid, HelpStep, Translating};

use super::Talk;
use crate::Backend;

/// Whether the learner can hear the character: a listening aid plays nothing while the
/// voice still loads (again after the idle time), so it does not count.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VoiceState {
    Ready,
    Loading,
}

/// A listening aid on a line of the character (/LF111/).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Hear {
    Repeat,
    Slow,
}

/// What help did.
pub struct Helped {
    /// The next step; `None` while the tutor works on a turn, or when there is no goal left.
    pub step: Option<HelpStep>,
    /// The help is in the session regardless; this is what could not be kept.
    pub failed: Vec<anyhow::Error>,
}

/// A listening aid that counted.
pub struct Heard {
    pub failed: Vec<anyhow::Error>,
}

/// The tutor's translation of a line, asked but not yet run; it holds no lock.
pub struct Translation {
    asking: Translating,
    line: usize,
}

/// What the tutor answered to a [`Translation`], to hand back to [`Talk::take_translation`].
pub struct Fetched {
    line: usize,
    result: Result<String, AiError>,
}

/// A translation taken in.
pub struct Translated {
    /// The text, or why there is none.
    pub result: Result<String, AiError>,
    /// Not saved, cost not booked.
    pub failed: Vec<anyhow::Error>,
}

impl Translation {
    /// Asks the tutor; run it outside any lock.
    pub fn run(self) -> Fetched {
        Fetched {
            line: self.line,
            result: self.asking.run(),
        }
    }
}

impl Talk {
    /// The next step of help for the current goal, saved at once: help used after the last
    /// turn is there when the talk is continued.
    pub fn help(&mut self, b: &Backend) -> Helped {
        let mut failed = Vec::new();
        if self.asking {
            return Helped { step: None, failed };
        }
        let step = self.session.help();
        failed.extend(self.save(b).err());
        Helped { step, failed }
    }

    /// A listening aid on the character's line `line` (the turns before it). `None` when it
    /// does not count and so is not played: the tutor works on a turn, or the voice loads.
    pub fn hear(
        &mut self,
        b: &Backend,
        aid: Hear,
        line: usize,
        voice: VoiceState,
    ) -> Option<Heard> {
        if self.asking || voice == VoiceState::Loading {
            return None;
        }
        let aid = match aid {
            Hear::Repeat => Aid::Repeat,
            Hear::Slow => Aid::Slow,
        };
        self.session.aid(aid, line);
        Some(Heard {
            failed: self.save(b).err().into_iter().collect(),
        })
    }

    /// The translation of the character's line `line`, given as markup; `None` while the
    /// tutor works on a turn.
    pub fn ask_translation(&self, markup: &str, line: usize) -> Option<Translation> {
        if self.asking {
            return None;
        }
        Some(Translation {
            asking: self.session.translating(&kaiwa_core::plain(markup)),
            line,
        })
    }

    /// Books what the translation cost (/LF714/), also when it failed. It counts as help
    /// once the learner has it, for the line it was asked on only and not during a sent turn
    /// (/LF212/), and is saved.
    pub fn take_translation(&mut self, b: &Backend, now: Moment, got: Fetched) -> Translated {
        let mut failed: Vec<anyhow::Error> = b.book_costs(now, false).err().into_iter().collect();
        // The learner has it now; during a sent turn it would count for an answer given
        // without it (/LF212/).
        if got.result.is_ok() && !self.asking {
            self.session.aid(Aid::Translate, got.line);
            failed.extend(self.save(b).err());
        }
        Translated {
            result: got.result,
            failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TempBackend;
    use crate::flow::Length;
    use kaiwa_core::Input;

    const SCENARIO: &str = "n5-02-jikoshoukai";

    fn now() -> Moment {
        Moment::new(1_791_100_000, 120)
    }

    fn open(b: &Backend) -> Talk {
        Talk::begin(b, SCENARIO, now(), Length::Short).unwrap().talk
    }

    fn aids(t: &Talk) -> u32 {
        t.session().stats().aids
    }

    fn translate(b: &Backend, t: &mut Talk, line: usize) -> Translated {
        let opening = t.session().scenario.opening.clone();
        let got = t.ask_translation(&opening, line).unwrap().run();
        t.take_translation(b, now(), got)
    }

    #[test]
    fn help_goes_a_step_and_is_saved() {
        let b = TempBackend::new("aids-help", true).unwrap();
        let mut t = open(&b);

        let helped = t.help(&b);

        assert_eq!(helped.step.expect("a step").level, 1);
        assert!(helped.failed.is_empty());
        let kept = b.store.session_state(t.id().unwrap()).unwrap();
        assert_eq!(kept.helps_used, 1);
    }

    #[test]
    fn a_listening_aid_counts_and_is_saved() {
        let b = TempBackend::new("aids-hear", true).unwrap();
        let mut t = open(&b);

        let heard = t.hear(&b, Hear::Slow, 0, VoiceState::Ready);

        assert!(heard.expect("counted").failed.is_empty());
        assert_eq!(aids(&t), 1);
        let kept = b.store.session_state(t.id().unwrap()).unwrap();
        assert_eq!(kept.aids.values().sum::<u32>(), 1);
    }

    #[test]
    fn a_listening_aid_does_not_count_while_the_voice_loads() {
        let b = TempBackend::new("aids-loading", true).unwrap();
        let mut t = open(&b);

        assert!(t.hear(&b, Hear::Repeat, 0, VoiceState::Loading).is_none());

        assert_eq!(aids(&t), 0);
    }

    #[test]
    fn no_aid_counts_while_the_tutor_works_on_a_turn() {
        let b = TempBackend::new("aids-busy", true).unwrap();
        let mut t = open(&b);
        let _asking = t.ask("はい");

        assert!(t.hear(&b, Hear::Repeat, 0, VoiceState::Ready).is_none());
        assert!(t.help(&b).step.is_none());
        let opening = t.session().scenario.opening.clone();
        assert!(t.ask_translation(&opening, 0).is_none());
        assert_eq!(aids(&t), 0);
        assert_eq!(t.session().summary().helps_used, 0);
    }

    #[test]
    fn aids_count_again_once_the_turn_is_taken() {
        let b = TempBackend::new("aids-after", true).unwrap();
        let mut t = open(&b);
        t.say(&b, "はい", Input::Typed, now()).unwrap().unwrap();

        assert!(t.hear(&b, Hear::Repeat, 1, VoiceState::Ready).is_some());
        assert_eq!(aids(&t), 1);
    }

    #[test]
    fn a_translation_counts_for_the_asked_line_only() {
        let b = TempBackend::new("aids-translate", true).unwrap();
        let mut t = open(&b);

        let translated = translate(&b, &mut t, 0);
        assert!(translated.result.is_ok(), "{:?}", translated.result);
        assert!(translated.failed.is_empty());
        assert_eq!(aids(&t), 1);

        // Asked on line 0, it arrives after the learner answered: line 1 is open.
        let late = t.ask_translation(&t.session().scenario.opening.clone(), 0);
        let late = late.unwrap().run();
        t.say(&b, "はい", Input::Typed, now()).unwrap().unwrap();
        let before = aids(&t);
        assert!(t.take_translation(&b, now(), late).result.is_ok());
        assert_eq!(aids(&t), before);
    }

    /// /LF212/: asked before a turn, a translation that arrives while the tutor works on the
    /// turn does not count; the answer to that turn came without it.
    #[test]
    fn a_translation_arriving_during_a_turn_does_not_count() {
        let b = TempBackend::new("aids-translate-busy", true).unwrap();
        let mut t = open(&b);
        let asked = t.ask_translation(&t.session().scenario.opening.clone(), 0);
        let got = asked.unwrap().run();

        let _asking = t.ask("はい");
        assert!(t.take_translation(&b, now(), got).result.is_ok());

        assert_eq!(aids(&t), 0);
    }

    /// The turn the translation waited for failed: the learner answers the same line again
    /// with the translation in view, so now it counts.
    #[test]
    fn a_translation_held_during_a_failed_turn_counts() {
        let b = TempBackend::new("aids-translate-failed", true).unwrap();
        let mut t = open(&b);
        let asked = t.ask_translation(&t.session().scenario.opening.clone(), 0);
        let got = asked.unwrap().run();

        let _asking = t.ask("はい");
        t.take_translation(&b, now(), got);
        t.failed_call(&b, now());

        assert_eq!(aids(&t), 1);
    }

    #[test]
    fn a_failed_translation_does_not_count() {
        let b = TempBackend::new("aids-nofixture", true).unwrap();
        let mut t = open(&b);

        let got = Fetched {
            line: 0,
            result: Err(AiError::NoFixture),
        };
        let translated = t.take_translation(&b, now(), got);

        assert!(translated.result.is_err());
        assert_eq!(aids(&t), 0);
    }

    #[test]
    fn a_translation_books_what_it_cost() {
        let b = TempBackend::new("aids-cost", true).unwrap();
        let mut t = open(&b);
        b.meter().record("test", 0, 0, true, Some(0.07));
        let before = b.month_cost(now()).unwrap().cents;

        translate(&b, &mut t, 0);

        assert!(b.month_cost(now()).unwrap().cents > before);
    }
}
