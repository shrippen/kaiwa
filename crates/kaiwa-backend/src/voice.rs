//! The character's voice (/LF103/, /LF111/, /LF117/, /LL204/, /LF510/): loading it, the pace,
//! speaking a line sentence by sentence, playing a recording back, and what the voice still
//! owes the talk while it loads. A front end holds no speaker and no rule of them.
//!
//! ```text
//!  front end ─► Speaker::say ─► Said (cache) ─► Synthesizer ─► enhance ─► Playback
//!               pace(), Debt     "again" at once   Kokoro        louder/clearer  speaker
//! ```

use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use kaiwa_core::difficulty::Difficulty;
use kaiwa_speech::models::kokoro_dir;
use kaiwa_speech::playback::{self, Player};
use kaiwa_speech::tts::{self, SpeechCache};
use kaiwa_speech::{SpeechError, Synthesizer};

use crate::ears::{self, Clip};

/// The voice's pace without a session: a little below Kokoro's normal one.
pub const DEFAULT_PACE: f32 = 0.9;
/// "Slower" (/LF212/) plays at this share of the pace.
const SLOW_SHARE: f32 = 0.7;
/// Threads Kokoro uses.
const THREADS: usize = 2;
/// How often a line waits to see whether the speaker is quiet.
const QUIET_POLL: Duration = Duration::from_millis(50);

/// How fast a line is said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// The pace the difficulty sets.
    Normal,
    /// The aid "slower".
    Slow,
}

/// The pace of the voice, 0.7 to 1.1 (/LF211/): the session's difficulty sets it (eased
/// within the session after two misses); without a session [`DEFAULT_PACE`].
pub fn pace(difficulty: Option<&Difficulty>, how: Pace) -> f32 {
    let base = difficulty.map_or(DEFAULT_PACE, |d| d.speed() as f32);
    match how {
        Pace::Normal => base,
        Pace::Slow => base * SLOW_SHARE,
    }
}

/// Louder and clearer (/LL204/), as the learner set them in the profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Enhance {
    pub louder: bool,
    pub clear: bool,
}

/// One line to say: furigana markup, the character's voice, the pace.
pub struct Line {
    pub markup: String,
    pub voice: String,
    pub speed: f32,
    pub enhance: Enhance,
}

/// What a conversation already said, so "again" plays at once (/LF111/).
#[derive(Clone, Default)]
pub struct Said(Arc<Mutex<SpeechCache>>);

impl Said {
    /// Forgets everything (another conversation, or the voice unloaded).
    pub fn clear(&self) {
        if let Ok(mut c) = self.0.lock() {
            c.clear();
        }
    }
}

/// The speaker: what the learner hears played, one clip after the other.
pub struct Playback(Player);

impl Playback {
    pub fn open() -> Result<Self, SpeechError> {
        Player::start().map(Self)
    }

    /// Queues the clip behind what is playing.
    pub fn play(&self, clip: &Clip) {
        self.0.play(clip.samples(), clip.sample_rate());
    }

    pub fn playing(&self) -> bool {
        self.0.playing()
    }

    /// Stops at once.
    pub fn silence(&self) {
        self.0.silence();
    }
}

/// The interim voice and the speaker it plays through.
#[derive(Clone)]
pub struct Speaker {
    voice: Arc<Mutex<dyn Synthesizer>>,
    player: Arc<Playback>,
}

impl Speaker {
    /// Loads Kokoro with the character's `voice` and opens the speaker; takes a few seconds,
    /// so off the GUI thread.
    pub fn load(voice: &str) -> Result<Self, SpeechError> {
        let kokoro = tts::Voice::from_dir(&kokoro_dir(), voice, THREADS)?;
        Ok(Self {
            voice: Arc::new(Mutex::new(kokoro)),
            player: Arc::new(Playback::open()?),
        })
    }

    /// Stops at once.
    pub fn silence(&self) {
        self.player.silence();
    }

    /// Says `line` sentence by sentence (the first plays while the next is made) and returns
    /// when the speaker is quiet, or `still` turns false (the character moved on). `on_sound`
    /// runs after each sentence went to the speaker.
    pub fn say(
        &self,
        said: &Said,
        line: &Line,
        still: &dyn Fn() -> bool,
        on_sound: &mut dyn FnMut(),
    ) {
        let player = &self.player;
        let play = |samples: &[f32], rate: u32| {
            player.play(&Clip::new(samples.to_vec(), rate));
            on_sound();
        };
        render(&*self.voice, said, line, still, play);
        while still() && player.playing() {
            sleep(QUIET_POLL);
        }
    }
}

/// Makes the sentences of `line` one by one and hands each, louder or clearer as set, to
/// `play`; stops when `still` is false. A sentence made before comes from `said`.
fn render(
    voice: &Mutex<dyn Synthesizer>,
    said: &Said,
    line: &Line,
    still: &dyn Fn() -> bool,
    mut play: impl FnMut(&[f32], u32),
) {
    for sentence in tts::sentences(&kaiwa_core::spoken(&line.markup)) {
        if !still() {
            return;
        }
        // In the character's own voice (/LF117/); what this conversation said before plays at
        // once (/LF111/).
        let made = tts::sentence(voice, &said.0, &line.voice, &sentence, line.speed);
        match made {
            Some(Ok((samples, rate))) if still() => {
                let mut samples = samples.to_vec();
                let Enhance { louder, clear } = line.enhance;
                playback::enhance(&mut samples, rate, louder, clear);
                play(&samples, rate);
            }
            Some(Err(e)) => eprintln!("kaiwa: {e}"),
            _ => {}
        }
    }
}

/// Why the character answers without a voice, in the UI language; a missing model says where
/// to get it.
pub fn voice_text(e: &SpeechError, lang: &str) -> String {
    let de = lang == "de";
    match (e, de) {
        (SpeechError::ModelMissing(_), true) => "Die Figur antwortet ohne Stimme. Die Übergangsstimme Kokoro lässt sich in den Einstellungen unter „Modelle“ laden.".to_string(),
        (SpeechError::ModelMissing(_), false) => "The character answers without a voice. Download the interim voice Kokoro in the settings under “Models”.".to_string(),
        (e, true) => format!("Die Figur antwortet ohne Stimme: {}", ears::speech_text(e, "de")),
        (e, false) => format!("The character answers without a voice: {}", ears::speech_text(e, "en")),
    }
}

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

    use std::cell::Cell;

    /// A voice that makes four samples of a constant and counts its calls.
    struct Fake(Arc<Mutex<u32>>);

    impl Synthesizer for Fake {
        fn speak(&mut self, _: &str, _: f32) -> Result<Vec<f32>, SpeechError> {
            *self.0.lock().unwrap() += 1;
            Ok(vec![0.25; 4])
        }
        fn sample_rate(&self) -> u32 {
            24_000
        }
        fn voice(&self) -> &str {
            "jf_tebukuro"
        }
        fn select(&mut self, _: &str) -> Result<(), SpeechError> {
            Ok(())
        }
    }

    fn line(markup: &str, enhance: Enhance) -> Line {
        Line {
            markup: markup.into(),
            voice: "jf_tebukuro".into(),
            speed: 0.9,
            enhance,
        }
    }

    fn said_clips(
        l: &Line,
        said: &Said,
        calls: &Arc<Mutex<u32>>,
        still: &dyn Fn() -> bool,
    ) -> Vec<Vec<f32>> {
        let voice = Mutex::new(Fake(calls.clone()));
        let mut out = Vec::new();
        render(&voice, said, l, still, |s, _| out.push(s.to_vec()));
        out
    }

    #[test]
    fn a_line_is_said_sentence_by_sentence_from_its_plain_text() {
        let calls = Arc::default();
        let l = line("{今日|きょう}は。はい。", Enhance::default());
        let out = said_clips(&l, &Said::default(), &calls, &|| true);
        assert_eq!(out.len(), 2);
        assert_eq!(*calls.lock().unwrap(), 2);
        assert_eq!(out[0], vec![0.25; 4], "no enhancing unless set");
    }

    #[test]
    fn what_was_said_before_plays_without_making_it_again() {
        let (calls, said) = (Arc::default(), Said::default());
        let l = line("はい。", Enhance::default());
        said_clips(&l, &said, &calls, &|| true);
        said_clips(&l, &said, &calls, &|| true);
        assert_eq!(*calls.lock().unwrap(), 1);
        said.clear();
        said_clips(&l, &said, &calls, &|| true);
        assert_eq!(*calls.lock().unwrap(), 2, "a new conversation says it anew");
    }

    #[test]
    fn louder_and_clearer_change_the_samples_but_not_the_cache() {
        let (calls, said) = (Arc::default(), Said::default());
        let plain = said_clips(&line("はい。", Enhance::default()), &said, &calls, &|| true);
        let loud = Enhance {
            louder: true,
            clear: false,
        };
        let out = said_clips(&line("はい。", loud), &said, &calls, &|| true);
        assert_ne!(out, plain);
        assert_eq!(*calls.lock().unwrap(), 1, "applied on each play");
        let again = said_clips(&line("はい。", Enhance::default()), &said, &calls, &|| true);
        assert_eq!(again, plain);
    }

    #[test]
    fn a_line_stops_when_the_character_moved_on() {
        let calls = Arc::default();
        let l = line("はい。いいえ。ええ。", Enhance::default());
        let n = Cell::new(0);
        let still = || {
            n.set(n.get() + 1);
            n.get() <= 2
        };
        let out = said_clips(&l, &Said::default(), &calls, &still);
        assert!(out.len() < 3);
    }

    #[test]
    fn the_pace_follows_the_difficulty() {
        let d = Difficulty::default();
        assert_eq!(pace(Some(&d), Pace::Normal), d.speed() as f32);
        assert!(pace(Some(&d), Pace::Slow) < pace(Some(&d), Pace::Normal));
        assert_eq!(pace(None, Pace::Normal), DEFAULT_PACE);
        assert_eq!(pace(None, Pace::Slow), DEFAULT_PACE * SLOW_SHARE);
    }

    #[test]
    fn a_voice_that_is_missing_says_where_to_get_it() {
        let e = SpeechError::ModelMissing("x".into());
        assert!(voice_text(&e, "de").contains("Modelle"));
        assert!(voice_text(&e, "en").contains("Models"));
        let e = SpeechError::NoSpeaker;
        assert_eq!(
            voice_text(&e, "en"),
            "The character answers without a voice: No speaker found."
        );
        assert!(voice_text(&e, "de").starts_with("Die Figur antwortet ohne Stimme: Kein"));
    }
}
