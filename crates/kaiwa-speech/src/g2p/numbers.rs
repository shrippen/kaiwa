//! Numbers as kana, with the sound changes of 300, 600, 800, 3000, 8000 and 1兆.
//! Counters (三本, 一つ) are the dictionary's job once the number is written in kanji.

const DIGITS: [&str; 10] = [
    "ぜろ",
    "いち",
    "に",
    "さん",
    "よん",
    "ご",
    "ろく",
    "なな",
    "はち",
    "きゅう",
];
const GROUPS: [&str; 4] = ["", "まん", "おく", "ちょう"];

/// A run of ASCII digits as kana. Leading zeros and runs past 兆 read digit by digit.
pub(super) fn kana(digits: &str) -> String {
    let by_digit = || {
        digits
            .bytes()
            .map(|b| DIGITS[usize::from(b - b'0')])
            .collect::<String>()
    };
    if digits.len() > 1 && digits.starts_with('0') || digits.len() > 16 {
        return by_digit();
    }
    let Ok(n) = digits.parse::<u64>() else {
        return by_digit();
    };
    if n == 0 {
        return DIGITS[0].into();
    }
    let mut out = String::new();
    for g in (0..GROUPS.len()).rev() {
        let part = (n / 10_000u64.pow(g as u32)) % 10_000;
        if part == 0 {
            continue;
        }
        if g == 3 && part == 1 {
            out.push_str("いっちょう");
        } else {
            out.push_str(&below_10000(part, g > 0));
            out.push_str(GROUPS[g]);
        }
    }
    out
}

/// A number in kanji (三百, 二千五百, 一万) as ASCII digits, when it has 百, 千, 万 or 億;
/// other runs (一, 十分, 一緒) stay words for the dictionary.
pub(super) fn from_kanji(run: &str) -> Option<String> {
    if run.chars().count() < 2 || !run.chars().any(|c| "百千万億".contains(c)) {
        return None;
    }
    let (mut total, mut section, mut current) = (0u64, 0u64, 0u64);
    for c in run.chars() {
        let digit = "〇一二三四五六七八九".chars().position(|d| d == c);
        match (digit, c) {
            (Some(d), _) => current = d as u64,
            (None, '十' | '百' | '千') => {
                let unit = match c {
                    '十' => 10,
                    '百' => 100,
                    _ => 1000,
                };
                section += current.max(1) * unit;
                current = 0;
            }
            (None, '万' | '億') => {
                let unit = if c == '万' { 10_000 } else { 100_000_000 };
                total += (section + current).max(1) * unit;
                section = 0;
                current = 0;
            }
            _ => return None,
        }
    }
    Some((total + section + current).to_string())
}

/// 1–9999. `big` marks a group before 万/億, where 1000 reads いっせん.
fn below_10000(n: u64, big: bool) -> String {
    let (th, hu, te, on) = (n / 1000, n / 100 % 10, n / 10 % 10, n % 10);
    let mut out = String::new();
    out.push_str(match th {
        0 => "",
        1 if big => "いっせん",
        1 => "せん",
        3 => "さんぜん",
        8 => "はっせん",
        d => DIGITS[d as usize],
    });
    if th > 1 && th != 3 && th != 8 {
        out.push_str("せん");
    }
    out.push_str(match hu {
        0 => "",
        1 => "ひゃく",
        3 => "さんびゃく",
        6 => "ろっぴゃく",
        8 => "はっぴゃく",
        d => DIGITS[d as usize],
    });
    if hu > 1 && ![3, 6, 8].contains(&hu) {
        out.push_str("ひゃく");
    }
    if te > 1 {
        out.push_str(DIGITS[te as usize]);
    }
    if te > 0 {
        out.push_str("じゅう");
    }
    if on > 0 {
        out.push_str(DIGITS[on as usize]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{from_kanji, kana};

    #[test]
    fn kanji_numbers() {
        assert_eq!(from_kanji("三百").as_deref(), Some("300"));
        assert_eq!(from_kanji("二千五百").as_deref(), Some("2500"));
        assert_eq!(from_kanji("一万二千").as_deref(), Some("12000"));
        assert_eq!(from_kanji("十五"), None);
        assert_eq!(from_kanji("一"), None);
    }

    #[test]
    fn readings() {
        for (n, want) in [
            ("0", "ぜろ"),
            ("7", "なな"),
            ("10", "じゅう"),
            ("24", "にじゅうよん"),
            ("300", "さんびゃく"),
            ("680", "ろっぴゃくはちじゅう"),
            ("1000", "せん"),
            ("3800", "さんぜんはっぴゃく"),
            ("10000", "いちまん"),
            ("10000000", "いっせんまん"),
            ("120000000", "いちおくにせんまん"),
            ("1000000000000", "いっちょう"),
            ("007", "ぜろぜろなな"),
        ] {
            assert_eq!(kana(n), want, "{n}");
        }
    }
}
