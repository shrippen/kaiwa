//! The speaker and the lines it says: Kokoro, enhancing, playback (needs the feature `speech`).

use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use kaiwa_speech::models::kokoro_dir;
use kaiwa_speech::playback::{self, Player};
use kaiwa_speech::tts::{self, SpeechCache};
use kaiwa_speech::{SpeechError, Synthesizer};

use crate::ears::{self, Clip};

/// Threads Kokoro uses.
const THREADS: usize = 2;
/// How often a line waits to see whether the speaker is quiet.
const QUIET_POLL: Duration = Duration::from_millis(50);

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
    /// runs after each sentence went to the speaker. A sentence the voice could not make is
    /// skipped; the first such failure is returned.
    pub fn say(
        &self,
        said: &Said,
        line: &Line,
        still: &dyn Fn() -> bool,
        on_sound: &mut dyn FnMut(),
    ) -> Option<SpeechError> {
        let player = &self.player;
        let play = |samples: &[f32], rate: u32| {
            player.play(&Clip::new(samples.to_vec(), rate));
            on_sound();
        };
        let failed = render(&*self.voice, said, line, still, play);
        while still() && player.playing() {
            sleep(QUIET_POLL);
        }
        failed
    }
}

/// Makes the sentences of `line` one by one and hands each, louder or clearer as set, to
/// `play`; stops when `still` is false. A sentence made before comes from `said`. A sentence
/// that fails is skipped; the first failure is returned.
fn render(
    voice: &Mutex<dyn Synthesizer>,
    said: &Said,
    line: &Line,
    still: &dyn Fn() -> bool,
    mut play: impl FnMut(&[f32], u32),
) -> Option<SpeechError> {
    let mut failed = None;
    for sentence in tts::sentences(&kaiwa_core::spoken(&line.markup)) {
        if !still() {
            return failed;
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
            Some(Err(e)) => {
                failed.get_or_insert(e);
            }
            _ => {}
        }
    }
    failed
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

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A voice that fails on every sentence.
    struct Broken;

    impl Synthesizer for Broken {
        fn speak(&mut self, _: &str, _: f32) -> Result<Vec<f32>, SpeechError> {
            Err(SpeechError::Voice("no sound".into()))
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

    #[test]
    fn a_voice_that_fails_returns_its_first_failure() {
        let l = line("はい。いいえ。", Enhance::default());
        let mut played = 0;

        let failed = render(
            &Mutex::new(Broken),
            &Said::default(),
            &l,
            &|| true,
            |_, _| {
                played += 1;
            },
        );

        assert_eq!(played, 0);
        assert!(matches!(failed, Some(SpeechError::Voice(_))));
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
