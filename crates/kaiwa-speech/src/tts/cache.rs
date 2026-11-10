//! The sentences a conversation already said, as samples (/LF111/): "again" and "slower"
//! play them at once instead of making them anew. The cache holds the raw voice; louder
//! and clearer are applied on each play, so a changed setting still counts.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::{SpeechError, Synthesizer};

/// At most this many samples (about five minutes at Kokoro's 24 kHz, ~29 MB); the oldest
/// sentences go first.
pub const MAX_SAMPLES: usize = 24_000 * 60 * 5;

/// A sentence as samples, and their rate.
pub type Clip = (Arc<Vec<f32>>, u32);

/// One sentence in one voice at one speed.
type Key = (String, String, u32);

#[derive(Default)]
pub struct SpeechCache {
    clips: HashMap<Key, Clip>,
    /// Oldest first.
    order: VecDeque<Key>,
    samples: usize,
}

fn key(voice: &str, sentence: &str, speed: f32) -> Key {
    (voice.to_string(), sentence.to_string(), speed.to_bits())
}

impl SpeechCache {
    /// The samples and their rate, if this voice made this sentence at this speed.
    pub fn get(&self, voice: &str, sentence: &str, speed: f32) -> Option<Clip> {
        self.clips.get(&key(voice, sentence, speed)).cloned()
    }

    /// Keeps a sentence; a clip longer than the whole cache is not kept.
    pub fn put(
        &mut self,
        voice: &str,
        sentence: &str,
        speed: f32,
        samples: Arc<Vec<f32>>,
        rate: u32,
    ) {
        if samples.len() > MAX_SAMPLES {
            return;
        }
        let k = key(voice, sentence, speed);
        if let Some((old, _)) = self.clips.remove(&k) {
            self.samples -= old.len();
            self.order.retain(|o| o != &k);
        }
        while self.samples + samples.len() > MAX_SAMPLES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some((old, _)) = self.clips.remove(&oldest) {
                self.samples -= old.len();
            }
        }
        self.samples += samples.len();
        self.order.push_back(k.clone());
        self.clips.insert(k, (samples, rate));
    }

    /// Forgets everything (a new conversation, or the voice unloaded to free memory).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn samples(&self) -> usize {
        self.samples
    }
}

/// One sentence of a character in its own `voice` (/LF117/), from the cache when the
/// conversation said it before (/LF111/), else made and kept. A synthesizer still set to the
/// last conversation's voice switches over first; if it cannot, the voice it has speaks. The
/// sentence is kept under the conversation's voice either way, so "again" finds it. `None`
/// when a lock is poisoned.
pub fn sentence(
    synthesizer: &Mutex<dyn Synthesizer>,
    cache: &Mutex<SpeechCache>,
    voice: &str,
    sentence: &str,
    speed: f32,
) -> Option<Result<Clip, SpeechError>> {
    let cached = || {
        cache
            .lock()
            .ok()
            .and_then(|c| c.get(voice, sentence, speed))
    };
    if let Some(hit) = cached() {
        return Some(Ok(hit));
    }
    let mut v = synthesizer.lock().ok()?;
    // Again now that the synthesizer is ours: a line cut off may have just made it.
    if let Some(hit) = cached() {
        return Some(Ok(hit));
    }
    if v.voice() != voice
        && let Err(e) = v.select(voice)
    {
        eprintln!("kaiwa: {e}");
    }
    let made = v
        .speak(sentence, speed)
        .map(|s| (Arc::new(s), v.sample_rate()));
    if let Ok((samples, rate)) = &made
        && let Ok(mut c) = cache.lock()
    {
        c.put(voice, sentence, speed, samples.clone(), *rate);
    }
    Some(made)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthesizer that counts: each voice gives its own samples.
    struct Fake {
        voice: String,
        known: Vec<&'static str>,
        spoken: usize,
        selects: usize,
    }

    impl Fake {
        fn with(voice: &str) -> Mutex<Fake> {
            Mutex::new(Fake {
                voice: voice.into(),
                known: vec!["jf_tebukuro", "jm_kumo"],
                spoken: 0,
                selects: 0,
            })
        }
    }

    impl Synthesizer for Fake {
        fn speak(&mut self, text: &str, _: f32) -> Result<Vec<f32>, SpeechError> {
            self.spoken += 1;
            Ok(vec![self.voice.len() as f32; text.chars().count()])
        }
        fn sample_rate(&self) -> u32 {
            24_000
        }
        fn voice(&self) -> &str {
            &self.voice
        }
        fn select(&mut self, voice: &str) -> Result<(), SpeechError> {
            self.selects += 1;
            if !self.known.contains(&voice) {
                return Err(SpeechError::ModelLoad(format!("no voice {voice}")));
            }
            self.voice = voice.into();
            Ok(())
        }
    }

    #[test]
    fn the_next_character_speaks_with_its_own_voice() {
        // Still loaded from the last conversation: the woman's voice.
        let fake = Fake::with("jf_tebukuro");
        let cache = Mutex::new(SpeechCache::default());
        let (a, _) = sentence(&fake, &cache, "jf_tebukuro", "はい。", 1.0)
            .unwrap()
            .unwrap();
        // Satō's talk: the same sentence, his voice, not the cached one of hers.
        let (b, _) = sentence(&fake, &cache, "jm_kumo", "はい。", 1.0)
            .unwrap()
            .unwrap();
        assert_ne!(a, b);
        assert_eq!(fake.lock().unwrap().voice(), "jm_kumo");
        assert_eq!(fake.lock().unwrap().selects, 1);
        // "Again" plays from the cache: nothing made, nothing switched.
        let (again, _) = sentence(&fake, &cache, "jm_kumo", "はい。", 1.0)
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&again, &b));
        let fake = fake.lock().unwrap();
        assert_eq!((fake.spoken, fake.selects), (2, 1));
    }

    #[test]
    fn an_unknown_voice_keeps_the_old_one_and_again_still_plays_at_once() {
        let fake = Fake::with("jf_tebukuro");
        let cache = Mutex::new(SpeechCache::default());
        let (a, _) = sentence(&fake, &cache, "xx_typo", "はい。", 1.0)
            .unwrap()
            .unwrap();
        assert_eq!(fake.lock().unwrap().voice(), "jf_tebukuro");
        let (again, _) = sentence(&fake, &cache, "xx_typo", "はい。", 1.0)
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&again, &a));
        assert_eq!(fake.lock().unwrap().spoken, 1);
    }

    fn clip(n: usize) -> Arc<Vec<f32>> {
        Arc::new(vec![0.5; n])
    }

    #[test]
    fn keeps_a_sentence_per_speed() {
        let mut c = SpeechCache::default();
        c.put("v", "こんにちは。", 1.0, clip(10), 24_000);
        assert_eq!(
            c.get("v", "こんにちは。", 1.0).map(|(s, r)| (s.len(), r)),
            Some((10, 24_000))
        );
        // Slower is another recording.
        assert!(c.get("v", "こんにちは。", 0.7).is_none());
        assert!(c.get("v", "こんばんは。", 1.0).is_none());
        // Another character's voice is another recording too.
        assert!(c.get("w", "こんにちは。", 1.0).is_none());
    }

    #[test]
    fn drops_the_oldest_when_full() {
        let mut c = SpeechCache::default();
        let third = MAX_SAMPLES / 3;
        c.put("v", "一。", 1.0, clip(third), 24_000);
        c.put("v", "二。", 1.0, clip(third), 24_000);
        c.put("v", "三。", 1.0, clip(third), 24_000);
        c.put("v", "四。", 1.0, clip(third), 24_000);
        assert!(c.get("v", "一。", 1.0).is_none());
        assert!(c.get("v", "二。", 1.0).is_some() && c.get("v", "四。", 1.0).is_some());
        assert!(c.samples() <= MAX_SAMPLES);
    }

    #[test]
    fn the_same_sentence_again_replaces_it() {
        let mut c = SpeechCache::default();
        c.put("v", "一。", 1.0, clip(10), 24_000);
        c.put("v", "一。", 1.0, clip(20), 24_000);
        assert_eq!(c.samples(), 20);
        assert_eq!(c.order.len(), 1);
    }

    #[test]
    fn skips_a_clip_larger_than_the_cache_and_clears() {
        let mut c = SpeechCache::default();
        c.put("v", "一。", 1.0, clip(10), 24_000);
        c.put("v", "長。", 1.0, clip(MAX_SAMPLES + 1), 24_000);
        assert!(c.get("v", "長。", 1.0).is_none() && c.get("v", "一。", 1.0).is_some());
        c.clear();
        assert_eq!(c.samples(), 0);
        assert!(c.get("v", "一。", 1.0).is_none());
    }
}
