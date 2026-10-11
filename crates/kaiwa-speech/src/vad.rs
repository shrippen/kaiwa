//! The end of a spoken turn when speaking hands-free (/LF100/, /LF101/): the turn ends after
//! the learner has spoken and then been quiet for the pause they set (1.2 to 5 s). There is no
//! limit for the whole turn, and voiced hesitation (えーと, あの) counts as speaking, so it never
//! ends a turn (/LF102/). Works on the capture's level (0 at −60 dB, 1 at 0 dB).

/// Speech must last this long before a pause can end the turn (a cough or a click does not).
const MIN_SPEECH_SECS: f64 = 0.25;
/// Speech lies at least this far above the noise floor (about 9 dB) …
const ABOVE_FLOOR: f32 = 0.15;
/// … and above this level (about −42 dB).
const MIN_LEVEL: f32 = 0.3;
/// The noise floor follows a falling level at once and a rising one slowly (per second).
const FLOOR_RISE: f32 = 0.02;

#[derive(Debug, Clone)]
pub struct EndOfTurn {
    pause: f64,
    floor: Option<f32>,
    spoken: f64,
    quiet: f64,
    listened: f64,
}

impl EndOfTurn {
    pub fn new(pause_secs: f32) -> Self {
        Self {
            pause: f64::from(pause_secs.clamp(1.2, 5.0)),
            floor: None,
            spoken: 0.0,
            quiet: 0.0,
            listened: 0.0,
        }
    }

    /// One level reading `dt` seconds after the last; true once the turn is over.
    pub fn update(&mut self, level: f32, dt: f64) -> bool {
        self.listened += dt;
        let floor = match self.floor {
            Some(f) if level >= f => f + FLOOR_RISE * dt as f32,
            _ => level,
        };
        self.floor = Some(floor);
        let voiced = level > MIN_LEVEL && level > floor + ABOVE_FLOOR;
        if voiced {
            self.spoken += dt;
            self.quiet = 0.0;
            false
        } else {
            self.quiet += dt;
            self.spoken >= MIN_SPEECH_SECS && self.quiet >= self.pause
        }
    }

    /// How long it has listened so far, in seconds.
    pub fn listened_secs(&self) -> f64 {
        self.listened
    }

    /// How long it has been quiet since the last speech, in seconds.
    pub fn quiet_secs(&self) -> f64 {
        self.quiet
    }

    /// Whether the learner has said anything yet.
    pub fn heard(&self) -> bool {
        self.spoken >= MIN_SPEECH_SECS
    }
}

/// Whether a recognised turn looks cut off: it ends on a particle or a connecting form that
/// leaves the sentence open (〜て, 〜が, 〜けど, 〜から, は, を …). A hint that the pause may be
/// too short (/LF101/), never a judgement on the turn.
pub fn sounds_unfinished(text: &str) -> bool {
    let t = text.trim_end_matches(|c: char| c.is_whitespace() || "、,…".contains(c));
    if t.is_empty() || t.ends_with(['。', '？', '?', '！', '!']) {
        return false;
    }
    [
        "けど", "から", "ので", "て", "で", "が", "と", "は", "を", "に", "も", "し",
    ]
    .iter()
    .any(|e| t.ends_with(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOISE: f32 = 0.2;
    const VOICE: f32 = 0.65;

    /// Feeds `secs` of `level` in 80 ms steps; the time at which the turn ended, if it did.
    fn feed(e: &mut EndOfTurn, level: f32, secs: f64, t: &mut f64) -> Option<f64> {
        let mut ended = None;
        let steps = (secs / 0.08).round() as usize;
        for _ in 0..steps {
            *t += 0.08;
            if e.update(level, 0.08) && ended.is_none() {
                ended = Some(*t);
            }
        }
        ended
    }

    #[test]
    fn silence_alone_never_ends_a_turn() {
        let mut e = EndOfTurn::new(1.2);
        let mut t = 0.0;
        assert_eq!(
            feed(&mut e, NOISE, 30.0, &mut t),
            None,
            "no limit before speaking"
        );
        assert!(!e.heard());
        assert!((e.listened_secs() - 30.0).abs() < 0.1);
    }

    #[test]
    fn the_turn_ends_after_the_set_pause() {
        let mut e = EndOfTurn::new(2.0);
        let mut t = 0.0;
        feed(&mut e, NOISE, 1.0, &mut t);
        feed(&mut e, VOICE, 1.5, &mut t);
        let start = t;
        let end = feed(&mut e, NOISE, 4.0, &mut t).expect("ended");
        assert!((end - start - 2.0).abs() < 0.1, "{}", end - start);
    }

    #[test]
    fn a_short_pause_and_fillers_keep_the_turn() {
        let mut e = EndOfTurn::new(3.0);
        let mut t = 0.0;
        feed(&mut e, VOICE, 1.0, &mut t);
        assert_eq!(feed(&mut e, NOISE, 2.5, &mut t), None, "thinking for 2.5 s");
        // えーと: quieter than the sentence, still voice
        assert_eq!(feed(&mut e, 0.5, 2.0, &mut t), None);
        assert_eq!(feed(&mut e, NOISE, 2.9, &mut t), None);
        assert!(feed(&mut e, NOISE, 0.3, &mut t).is_some());
    }

    #[test]
    fn a_click_is_no_speech() {
        let mut e = EndOfTurn::new(1.2);
        let mut t = 0.0;
        feed(&mut e, VOICE, 0.08, &mut t);
        assert_eq!(feed(&mut e, NOISE, 10.0, &mut t), None);
    }

    #[test]
    fn a_loud_room_raises_the_floor() {
        let mut e = EndOfTurn::new(1.2);
        let mut t = 0.0;
        // Steady noise at 0.45: above MIN_LEVEL, but it is the floor, not speech.
        feed(&mut e, 0.45, 5.0, &mut t);
        assert!(!e.heard());
        feed(&mut e, 0.75, 1.0, &mut t);
        assert!(e.heard());
        assert!(feed(&mut e, 0.45, 2.0, &mut t).is_some());
    }

    #[test]
    fn open_endings_sound_unfinished() {
        for t in [
            "ドイツから来て",
            "日本語が",
            "行きたいけど",
            "週末は",
            "コーヒーを",
        ] {
            assert!(sounds_unfinished(t), "{t}");
        }
        for t in [
            "マラです",
            "コーヒーが好きです。",
            "行きましたか？",
            "",
            "そうですね",
        ] {
            assert!(!sounds_unfinished(t), "{t}");
        }
    }

    #[test]
    fn the_pause_stays_within_the_settings() {
        assert!((EndOfTurn::new(0.5).pause - 1.2).abs() < 1e-6);
        assert!((EndOfTurn::new(9.0).pause - 5.0).abs() < 1e-6);
    }
}
