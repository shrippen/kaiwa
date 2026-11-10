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
    ("jya", "じゃ"),
    ("jyu", "じゅ"),
    ("jyo", "じょ"),
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

/// Converts the rōmaji in `text` to hiragana, word by word; anything else stays. A word that
/// does not turn into kana completely is no rōmaji (a German or English word, a name) and
/// stays as typed, so the tutor can read it (/LF122/: わたしは Leuchtturm です). A line with
/// several such words is a question in the learner's own language and stays whole.
pub fn to_kana(text: &str) -> String {
    // A Latin word: ASCII letters, the apostrophe of n'ya, letters with marks (ō, ä, ß).
    let latin = |c: char| c == '\'' || (c.is_alphabetic() && (c as u32) < 0x250);
    // The line in pieces: a Latin word with its kana (if it is rōmaji), or anything else.
    let mut pieces: Vec<(String, Option<String>)> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let word = latin(chars[i]);
        let end = (i..chars.len())
            .find(|j| latin(chars[*j]) != word)
            .unwrap_or(chars.len());
        let piece: String = chars[i..end].iter().collect();
        let kana = if word { word_to_kana(&piece) } else { None };
        pieces.push((piece, kana));
        i = end;
    }
    let words = pieces.iter().filter(|(p, _)| p.chars().any(latin)).count();
    let romaji = pieces.iter().filter(|(_, k)| k.is_some()).count();
    // One foreign word in a rōmaji sentence, or up to a quarter of the words; more is a
    // sentence in another language, in which words like "do", "you" or "in" only look like
    // rōmaji.
    let foreign = words - romaji;
    if romaji == 0 || (foreign > 1 && foreign * 4 > words) {
        return text.to_string();
    }
    pieces
        .into_iter()
        .map(|(piece, kana)| {
            kana.unwrap_or_else(|| {
                // Japanese punctuation for a line that is rōmaji.
                piece
                    .chars()
                    .map(|c| match c {
                        '.' => '。',
                        ',' => '、',
                        '?' => '？',
                        '!' => '！',
                        '-' => 'ー',
                        c => c,
                    })
                    .collect()
            })
        })
        .collect()
}

/// One Latin word as hiragana; `None` when some of it is no rōmaji.
fn word_to_kana(word: &str) -> Option<String> {
    let mut src = String::with_capacity(word.len());
    for c in word.chars() {
        let lc = c.to_lowercase().next().unwrap_or(c);
        match unmacron(lc) {
            Some(s) => src.push_str(s),
            None => src.push(lc),
        }
    }
    let chars: Vec<char> = src.chars().collect();
    if chars.iter().all(|c| *c == '\'') {
        return None;
    }
    let mut out = String::with_capacity(word.len() * 2);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            // Only separates n from a vowel (kon'ya); it has no sound of its own.
            i += 1;
            continue;
        }
        if !c.is_ascii_alphabetic() {
            return None;
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
        if c == 'n' && next.is_none_or(|n| !"aeiouy".contains(n)) {
            out.push('ん');
            i += 1;
            continue;
        }
        let piece = [3, 2, 1].into_iter().find_map(|len| {
            let piece: String = chars.get(i..i + len)?.iter().collect();
            TABLE
                .iter()
                .find(|(r, _)| *r == piece)
                .map(|(_, kana)| (len, *kana))
        });
        let (len, kana) = piece?;
        out.push_str(kana);
        i += len;
    }
    Some(out)
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
    fn a_question_in_the_learners_language_stays_as_typed() {
        // /LF122/: the tutor answers these with the Japanese word, so it must read them.
        for q in [
            "Was heißt Bahnhof?",
            "Wie sagt man Bahnhof?",
            "what is station?",
            "How do you say lighthouse in Japanese?",
        ] {
            assert_eq!(to_kana(q), q);
        }
        // One foreign word in a rōmaji sentence stays, the rest is kana.
        assert_eq!(
            to_kana("watashi wa Leuchtturm desu."),
            "わたし わ Leuchtturm です。"
        );
        // Wāpuro spellings.
        assert_eq!(to_kana("jyanai"), "じゃない");
        assert_eq!(to_kana("jyuu"), "じゅう");
    }

    #[test]
    fn kana_and_kanji_stay() {
        assert_eq!(to_kana("駅はどこですか"), "駅はどこですか");
        assert_eq!(to_kana("eki wa どこ"), "えき わ どこ");
        assert!(has_romaji("desu"));
        assert!(!has_romaji("です。"));
    }
}
