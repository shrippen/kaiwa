//! What the character still owes a voice that was not there yet.

/// What a voice that just loaded still has to say.
#[derive(Debug, PartialEq)]
pub enum Owed {
    /// The tutor's reply came while the voice was still loading.
    Reply(String),
    /// The opening line, once per talk.
    Opening,
    Nothing,
}

/// Whether the character can speak right now: the voice is loaded and switched on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Voice {
    Ready,
    Off,
}

/// What the character could not say yet because the voice was not there: a reply that came
/// early, and whether the opening was said (a reload after the idle time must not say it
/// again).
#[derive(Debug, Default)]
pub struct Debt {
    reply: Option<String>,
    opening_said: bool,
}

impl Debt {
    /// A new talk: nothing is owed, the opening is still to say.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The opening was said (or is being said). Without a voice it was dropped: still owed.
    pub fn opening_said(&mut self, voice: Voice) {
        self.opening_said = voice == Voice::Ready;
    }

    /// A reply came and there was no voice to say it; the latest one is kept.
    pub fn reply_early(&mut self, markup: &str) {
        self.reply = Some(markup.to_string());
    }

    /// No voice will come (model missing): nothing is owed any more.
    pub fn forgive(&mut self) {
        self.reply = None;
    }

    /// The voice is ready; what does the talk want now? `lines` is the log's length.
    pub fn voice_ready(&mut self, lines: usize, busy: bool, voice: Voice) -> Owed {
        // A reply owed to a voice the learner switched off meanwhile is not said.
        if let Some(markup) = self.reply.take().filter(|_| voice == Voice::Ready) {
            return Owed::Reply(markup);
        }
        // An opening cannot be said to a switched-off voice; it stays owed.
        if lines != 1 || busy || self.opening_said || voice == Voice::Off {
            return Owed::Nothing;
        }

        self.opening_said = true;
        Owed::Opening
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_before_the_voice_is_said_once_the_voice_is_ready() {
        let mut d = Debt::default();
        d.opening_said(Voice::Ready);
        d.reply_early("もしもし");
        d.reply_early("こんにちは");
        assert_eq!(
            d.voice_ready(3, false, Voice::Ready),
            Owed::Reply("こんにちは".into())
        );
        assert_eq!(
            d.voice_ready(3, false, Voice::Ready),
            Owed::Nothing,
            "only once"
        );
    }

    #[test]
    fn a_missing_voice_forgives_the_reply() {
        let mut d = Debt::default();
        d.reply_early("こんにちは");
        d.forgive();
        assert_eq!(d.voice_ready(3, false, Voice::Ready), Owed::Nothing);
    }

    #[test]
    fn the_opening_is_said_the_first_time_only() {
        let mut d = Debt::default();
        assert_eq!(d.voice_ready(1, false, Voice::Ready), Owed::Opening);
        // Idle unload, a reload while the talk is still at the opening.
        assert_eq!(d.voice_ready(1, false, Voice::Ready), Owed::Nothing);
        d.reset();
        assert_eq!(
            d.voice_ready(1, false, Voice::Ready),
            Owed::Opening,
            "a new talk"
        );
    }

    #[test]
    fn an_opening_said_elsewhere_is_not_said_again() {
        let mut d = Debt::default();
        d.opening_said(Voice::Ready);
        assert_eq!(d.voice_ready(1, false, Voice::Ready), Owed::Nothing);
    }

    #[test]
    fn a_turn_in_flight_has_no_opening() {
        let mut d = Debt::default();
        assert_eq!(d.voice_ready(1, true, Voice::Ready), Owed::Nothing);
    }

    #[test]
    fn an_opening_dropped_for_want_of_a_voice_is_still_owed() {
        let mut d = Debt::default();
        d.opening_said(Voice::Off);
        assert_eq!(d.voice_ready(1, false, Voice::Ready), Owed::Opening);
    }

    #[test]
    fn an_opening_owed_to_a_voice_switched_off_is_not_said() {
        let mut d = Debt::default();
        assert_eq!(d.voice_ready(1, false, Voice::Off), Owed::Nothing);
        assert_eq!(
            d.voice_ready(1, false, Voice::Ready),
            Owed::Opening,
            "still owed once the voice is on"
        );
    }

    #[test]
    fn a_reply_owed_to_a_voice_switched_off_is_dropped() {
        let mut d = Debt::default();
        d.opening_said(Voice::Ready);
        d.reply_early("こんにちは");
        assert_eq!(d.voice_ready(3, false, Voice::Off), Owed::Nothing);
        assert_eq!(d.voice_ready(3, false, Voice::Ready), Owed::Nothing, "gone");
    }
}
