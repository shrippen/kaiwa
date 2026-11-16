//! What the microphone's levels say about a hands-free turn, and the counts that follow a
//! turn. No hardware: the tests feed levels and turns.

use kaiwa_core::turn_end::{CutOffs, EmptyTurns, EndOfTurn};

use crate::Backend;

/// Hands-free: listening stops after this long without any speech.
pub const HANDS_FREE_WAIT_SECS: f64 = 30.0;

/// What one reading of the microphone's level tells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Heard {
    /// Still listening; the level for the meter.
    Listening(f32),
    /// Nothing was said for a long while: close the microphone, send nothing, so it does not
    /// record an empty room for hours.
    GaveUp,
    /// The set pause after speaking is over; the speech ended `quiet_secs` ago, not now.
    Ended { quiet_secs: f64 },
}

/// Hands-free: when the turn ends (/LF101/).
pub(super) struct Watch {
    end: EndOfTurn,
}

impl Watch {
    pub fn new(pause_secs: f32) -> Self {
        Self {
            end: EndOfTurn::new(pause_secs),
        }
    }

    /// One level reading `dt` seconds after the last.
    pub fn feed(&mut self, level: f32, dt: f64) -> Heard {
        if !self.end.heard() && self.end.listened_secs() >= HANDS_FREE_WAIT_SECS {
            return Heard::GaveUp;
        }
        if self.end.update(level, dt) {
            return Heard::Ended {
                quiet_secs: self.end.quiet_secs(),
            };
        }
        Heard::Listening(level)
    }
}

/// What to do with the offer of a longer pause after a recognised turn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Offer {
    /// Offer this pause.
    Make(f32),
    /// The pause was set at least as long in the meantime: the offer is stale.
    Withdraw,
    /// Leave the offer as it is.
    Keep,
}

/// The counts over turns: empty hands-free turns in a row, and turns the pause cut off.
#[derive(Debug, Default)]
pub struct Patience {
    empty: EmptyTurns,
    cut: CutOffs,
}

impl Patience {
    /// A turn was understood, a typed one, or the learner took over: the count starts again.
    pub fn reset_empty(&mut self) {
        self.empty.reset();
    }

    /// A turn the pause ended gave nothing to send: whether to listen again.
    pub fn listen_after_empty(&mut self) -> bool {
        self.empty.after_empty()
    }

    /// A turn was recognised (/LF101/, T-01): the longer pause to offer, given the one
    /// `offered` now (0 for none) and the pause set in the profile.
    pub fn after_turn(&mut self, b: &Backend, by_pause: bool, text: &str, offered: f32) -> Offer {
        let pause = b
            .profile()
            .map(|p| p.end_pause_secs)
            .unwrap_or(kaiwa_core::turn_end::DEFAULT_PAUSE);
        self.judge(by_pause, text, pause, offered)
    }

    fn judge(&mut self, by_pause: bool, text: &str, pause: f32, offered: f32) -> Offer {
        if let Some(next) = self.cut.after_turn(by_pause, text, pause) {
            return Offer::Make(next);
        }
        if offered > 0.0 && pause >= offered {
            return Offer::Withdraw;
        }
        Offer::Keep
    }

    /// The learner took the offer: counting starts afresh.
    pub fn take_offer(&mut self) {
        self.cut.reset();
    }

    /// The learner declined: it comes back only after two more turns that sound cut off.
    pub fn decline_offer(&mut self) {
        self.cut.decline();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 0.05;
    const VOICE: f32 = 0.5;
    const SILENCE: f32 = 0.0;
    const PAUSE: f32 = 1.2;

    /// Feeds `secs` of `level`; the first reading that is not `Listening`.
    fn run(w: &mut Watch, level: f32, secs: f64) -> Option<Heard> {
        let n = (secs / DT).round() as usize;
        (0..n)
            .map(|_| w.feed(level, DT))
            .find(|h| !matches!(h, Heard::Listening(_)))
    }

    #[test]
    fn it_listens_while_nothing_is_decided() {
        let mut w = Watch::new(PAUSE);
        assert_eq!(w.feed(VOICE, DT), Heard::Listening(VOICE));
        assert_eq!(run(&mut w, VOICE, 1.0), None);
    }

    #[test]
    fn it_gives_up_after_the_wait_without_speech() {
        let mut w = Watch::new(PAUSE);
        assert_eq!(run(&mut w, SILENCE, HANDS_FREE_WAIT_SECS - 1.0), None);
        assert_eq!(run(&mut w, SILENCE, 2.0), Some(Heard::GaveUp));
    }

    #[test]
    fn it_ends_after_the_pause_following_speech() {
        let mut w = Watch::new(PAUSE);
        assert_eq!(run(&mut w, SILENCE, 0.5), None);
        assert_eq!(run(&mut w, VOICE, 1.0), None);
        let Some(Heard::Ended { quiet_secs }) = run(&mut w, SILENCE, 3.0) else {
            panic!("the turn did not end");
        };
        assert!((quiet_secs - f64::from(PAUSE)).abs() < 0.1, "{quiet_secs}");
    }

    #[test]
    fn it_does_not_give_up_on_a_learner_who_spoke() {
        let mut w = Watch::new(5.0);
        assert_eq!(run(&mut w, VOICE, 1.0), None);
        assert_eq!(run(&mut w, SILENCE, 4.0), None);
    }

    #[test]
    fn a_cut_off_turn_offers_a_longer_pause_once_stale_it_withdraws() {
        let mut p = Patience::default();
        let cut = "食べて";
        assert_eq!(p.judge(true, cut, PAUSE, 0.0), Offer::Make(3.0));
        assert_eq!(p.judge(false, "はい", PAUSE, 3.0), Offer::Keep);
        assert_eq!(p.judge(false, "はい", 3.0, 3.0), Offer::Withdraw);
    }

    #[test]
    fn three_empty_turns_in_a_row_close_the_microphone() {
        let mut p = Patience::default();
        assert!(p.listen_after_empty());
        assert!(p.listen_after_empty());
        assert!(!p.listen_after_empty());
        p.reset_empty();
        assert!(p.listen_after_empty());
    }
}
