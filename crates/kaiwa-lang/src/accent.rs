//! The target pitch of a word from its accent kernel (Kanjium), for KantePitchCurve: high or
//! low per mora, plus a following particle that shows heiban and odaka apart. Only the target,
//! never a judgement of the learner (L-03, M-04).

use serde::Serialize;

/// Small kana that belong to the mora before them.
const SMALL: &str = "ゃゅょぁぃぅぇぉゎャュョァィゥェォヮ";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Accent {
    /// The morae of the reading and the particle in brackets: ["た", "べ", "る", "(が)"].
    pub morae: Vec<String>,
    /// 1 high, 0 low, one per mora (particle included).
    pub target: Vec<u8>,
    /// Index of the kernel mora (the last high one before the fall), -1 for heiban.
    pub kernel: i32,
    /// The kernel in morae as written in dictionaries (0 heiban), and other accepted kernels.
    pub pattern: Vec<u32>,
}

/// The morae of a kana text: きょう → [きょ, う].
pub fn morae(kana: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in kana.chars() {
        match out.last_mut() {
            Some(m) if SMALL.contains(c) => m.push(c),
            _ => out.push(c.to_string()),
        }
    }
    out
}

impl Accent {
    /// From a reading and Kanjium's pattern ("2", or "0,3" for two accepted kernels); `None`
    /// when the pattern does not fit the reading.
    pub fn new(reading: &str, pattern: &str) -> Option<Self> {
        let pattern: Vec<u32> = pattern
            .split(',')
            .map(|p| p.trim().parse().ok())
            .collect::<Option<_>>()?;
        let n = *pattern.first()? as usize;
        let mut morae = morae(reading);
        let m = morae.len();
        if m == 0 || n > m {
            return None;
        }
        let mut target: Vec<u8> = (0..m)
            .map(|i| match n {
                0 => u8::from(i > 0),
                1 => u8::from(i == 0),
                _ => u8::from(i > 0 && i < n),
            })
            .collect();
        // A one-mora heiban word starts low and rises on the particle.
        target.push(u8::from(n == 0));
        morae.push("(が)".to_string());
        Some(Self {
            morae,
            target,
            kernel: n as i32 - 1,
            pattern,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_kana_join_the_mora_before() {
        assert_eq!(morae("きょう"), ["きょ", "う"]);
        assert_eq!(morae("がっこう"), ["が", "っ", "こ", "う"]);
        assert_eq!(
            morae("ぷろでゅーさー"),
            ["ぷ", "ろ", "でゅ", "ー", "さ", "ー"]
        );
    }

    #[test]
    fn the_four_types() {
        // heiban さかな (0): low high high, particle high.
        let a = Accent::new("さかな", "0").unwrap();
        assert_eq!(a.target, [0, 1, 1, 1]);
        assert_eq!(a.kernel, -1);
        // atamadaka はし (1, chopsticks): high low, particle low.
        assert_eq!(Accent::new("はし", "1").unwrap().target, [1, 0, 0]);
        // nakadaka たべる (2): low high low, particle low.
        let t = Accent::new("たべる", "2").unwrap();
        assert_eq!(t.target, [0, 1, 0, 0]);
        assert_eq!(t.kernel, 1);
        // odaka はし (2, bridge): low high, particle low; heiban はし (端) differs on the particle.
        assert_eq!(Accent::new("はし", "2").unwrap().target, [0, 1, 0]);
        assert_eq!(Accent::new("はし", "0").unwrap().target, [0, 1, 1]);
    }

    #[test]
    fn more_than_one_kernel_keeps_the_first_and_lists_all() {
        let a = Accent::new("あからさま", "0,3").unwrap();
        assert_eq!(a.kernel, -1);
        assert_eq!(a.pattern, [0, 3]);
    }

    #[test]
    fn a_pattern_that_does_not_fit_is_dropped() {
        assert!(Accent::new("え", "3").is_none());
        assert!(Accent::new("えき", "x").is_none());
        assert!(Accent::new("", "0").is_none());
    }
}
