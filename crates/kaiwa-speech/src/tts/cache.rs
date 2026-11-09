//! The sentences a conversation already said, as samples (/LF111/): "again" and "slower"
//! play them at once instead of making them anew. The cache holds the raw voice; louder
//! and clearer are applied on each play, so a changed setting still counts.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// At most this many samples (about five minutes at Kokoro's 24 kHz, ~29 MB); the oldest
/// sentences go first.
pub const MAX_SAMPLES: usize = 24_000 * 60 * 5;

/// One sentence in one voice at one speed.
type Key = (String, String, u32);

#[derive(Default)]
pub struct SpeechCache {
    clips: HashMap<Key, (Arc<Vec<f32>>, u32)>,
    /// Oldest first.
    order: VecDeque<Key>,
    samples: usize,
}

fn key(voice: &str, sentence: &str, speed: f32) -> Key {
    (voice.to_string(), sentence.to_string(), speed.to_bits())
}

impl SpeechCache {
    /// The samples and their rate, if this voice made this sentence at this speed.
    pub fn get(&self, voice: &str, sentence: &str, speed: f32) -> Option<(Arc<Vec<f32>>, u32)> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
