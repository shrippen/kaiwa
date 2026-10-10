//! Typing in rōmaji (Lastenheft /LF120/): Hepburn (and the common Kunrei spellings) to hiragana,
//! so a learner without a Japanese keyboard can still answer in Japanese. Doubled consonants
//! become っ, "n" before a consonant or at the end ん ("n'" keeps it apart before a vowel),
//! macrons become long vowels (ō → おう, ā → ああ). Text that already holds kana or kanji stays
//! as it is; only runs of Latin letters are converted.

/// Syllables, longest first within each length.
const TABLE: &[(&str, &str)] = &[
    // three letters
    ("kya", "きゃ"),
    ("kyu", "きゅ"),
    ("kyo", "きょ"),
    ("gya", "ぎゃ"),
    ("gyu", "ぎゅ"),
    ("gyo", "ぎょ"),
    ("sha", "しゃ"),
    ("shu", "しゅ"),
    ("sho", "しょ"),
    ("shi", "し"),
    ("she", "しぇ"),
    ("cha", "ちゃ"),
    ("chu", "ちゅ"),
    ("cho", "ちょ"),
    ("chi", "ち"),
    ("che", "ちぇ"),
    ("tsu", "つ"),
    ("nya", "にゃ"),
    ("nyu", "にゅ"),
    ("nyo", "にょ"),
    ("hya", "ひゃ"),
    ("hyu", "ひゅ"),
    ("hyo", "ひょ"),
    ("bya", "びゃ"),
    ("byu", "びゅ"),
    ("byo", "びょ"),
    ("pya", "ぴゃ"),
    ("pyu", "ぴゅ"),
    ("pyo", "ぴょ"),
    ("mya", "みゃ"),
    ("myu", "みゅ"),
    ("myo", "みょ"),
    ("rya", "りゃ"),
    ("ryu", "りゅ"),
    ("ryo", "りょ"),
    ("sya", "しゃ"),
    ("syu", "しゅ"),
    ("syo", "しょ"),
    ("tya", "ちゃ"),
    ("tyu", "ちゅ"),
    ("tyo", "ちょ"),
    ("zya", "じゃ"),
    ("zyu", "じゅ"),
    ("zyo", "じょ"),
    // two letters
    ("ka", "か"),
    ("ki", "き"),
    ("ku", "く"),
    ("ke", "け"),
    ("ko", "こ"),
    ("ga", "が"),
    ("gi", "ぎ"),
    ("gu", "ぐ"),
    ("ge", "げ"),
    ("go", "ご"),
    ("sa", "さ"),
    ("si", "し"),
    ("su", "す"),
    ("se", "せ"),
    ("so", "そ"),
    ("za", "ざ"),
    ("ji", "じ"),
    ("zi", "じ"),
    ("zu", "ず"),
    ("ze", "ぜ"),
    ("zo", "ぞ"),
    ("ja", "じゃ"),
    ("ju", "じゅ"),
    ("jo", "じょ"),
    ("je", "じぇ"),
    ("ta", "た"),
    ("ti", "ち"),
    ("tu", "つ"),
    ("te", "て"),
    ("to", "と"),
    ("da", "だ"),
    ("di", "ぢ"),
    ("du", "づ"),
    ("de", "で"),
    ("do", "ど"),
    ("na", "な"),
    ("ni", "に"),
    ("nu", "ぬ"),
    ("ne", "ね"),
    ("no", "の"),
    ("ha", "は"),
    ("hi", "ひ"),
    ("fu", "ふ"),
    ("hu", "ふ"),
    ("he", "へ"),
    ("ho", "ほ"),
    ("ba", "ば"),
    ("bi", "び"),
    ("bu", "ぶ"),
    ("be", "べ"),
    ("bo", "ぼ"),
    ("pa", "ぱ"),
    ("pi", "ぴ"),
    ("pu", "ぷ"),
    ("pe", "ぺ"),
    ("po", "ぽ"),
    ("ma", "ま"),
    ("mi", "み"),
    ("mu", "む"),
    ("me", "め"),
    ("mo", "も"),
    ("ya", "や"),
    ("yu", "ゆ"),
    ("yo", "よ"),
    ("ra", "ら"),
    ("ri", "り"),
    ("ru", "る"),
    ("re", "れ"),
    ("ro", "ろ"),
    ("wa", "わ"),
    ("wo", "を"),
    ("fa", "ふぁ"),
    ("fi", "ふぃ"),
    ("fe", "ふぇ"),
    ("fo", "ふぉ"),
    ("n'", "ん"),
    // one letter
    ("a", "あ"),
    ("i", "い"),
    ("u", "う"),
    ("e", "え"),
    ("o", "お"),
];

fn unmacron(c: char) -> Option<&'static str> {
    Some(match c {
        'ā' | 'â' => "aa",
        'ī' | 'î' => "ii",
        'ū' | 'û' => "uu",
        'ē' | 'ê' => "ee",
        'ō' | 'ô' => "ou",
        _ => return None,
    })
}

/// Whether `text` has Latin letters to convert.
pub fn has_romaji(text: &str) -> bool {
    text.chars()
        .any(|c| c.is_ascii_alphabetic() || unmacron(c.to_ascii_lowercase()).is_some())
}

/// Converts the rōmaji in `text` to hiragana; anything else stays.
pub fn to_kana(text: &str) -> String {
    let mut src = String::with_capacity(text.len());
    for c in text.chars() {
        let lc = c.to_lowercase().next().unwrap_or(c);
        match unmacron(lc) {
            Some(s) => src.push_str(s),
            None => src.push(lc),
        }
    }
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(text.len() * 2);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !c.is_ascii_alphabetic() && c != '\'' {
            out.push(match c {
                '.' => '。',
                ',' => '、',
                '?' => '？',
                '!' => '！',
                '-' => 'ー',
                _ => c,
            });
            i += 1;
            continue;
        }
        let next = chars.get(i + 1).copied();
        // Doubled consonant (not n): small tsu. "tch" too (matcha).
        if c != 'n' && !"aeiou".contains(c) && (next == Some(c) || (c == 't' && next == Some('c')))
        {
            out.push('っ');
            i += 1;
            continue;
        }
        // n before a consonant (not y) or at the end: ん.
        if c == 'n' {
            let n_alone = match next {
                None => true,
                Some(n) => {
                    !n.is_ascii_alphabetic() && n != '\'' || (!"aeiouy".contains(n) && n != '\'')
                }
            };
            if n_alone {
                out.push('ん');
                i += 1;
                continue;
            }
        }
        let mut matched = false;
        for len in [3, 2, 1] {
            if i + len > chars.len() {
                continue;
            }
            let piece: String = chars[i..i + len].iter().collect();
            if let Some((_, kana)) = TABLE.iter().find(|(r, _)| *r == piece) {
                out.push_str(kana);
                i += len;
                matched = true;
                break;
            }
        }
        if !matched {
            out.push(c);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hepburn_to_hiragana() {
        for (r, k) in [
            ("watashi wa mara desu.", "わたし わ まら です。"),
            ("kitte", "きって"),
            ("matcha", "まっちゃ"),
            ("shinbun", "しんぶん"),
            ("kon'ya", "こんや"),
            ("konnichiwa", "こんにちわ"),
            ("Tōkyō", "とうきょう"),
            ("kyou wa samui", "きょう わ さむい"),
            ("arigatou gozaimasu!", "ありがとう ございます！"),
            ("koohii ga suki desu", "こおひい が すき です"),
            ("biiru o futatsu kudasai", "びいる お ふたつ ください"),
        ] {
            assert_eq!(to_kana(r), k, "{r}");
        }
    }

    #[test]
    fn kana_and_kanji_stay() {
        assert_eq!(to_kana("駅はどこですか"), "駅はどこですか");
        assert_eq!(to_kana("eki wa どこ"), "えき わ どこ");
        assert!(has_romaji("desu"));
        assert!(!has_romaji("です。"));
    }
}
