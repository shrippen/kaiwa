//! UniDic (owner, 9.11.2026, ADR-004): the readings for furigana where nobody wrote them (the
//! learner's own lines, kanji the tutor left without). UniDic reads in context where IPADIC
//! guesses (角を右へ → かど, 二人 → ふたり; 何 alone it reads なん everywhere, so it goes without). It is a download, not part of the
//! app (about 200 MB built): the source archive of unidic-mecab 2.1.2 from lindera.dev
//! (140 MB, checked by SHA-256) is built here into lindera's format, as lindera's own build
//! does it, and the source is removed again. Without it, [`crate::furigana`] reads with
//! IPADIC, cautiously. Word segmentation for the lookup stays IPADIC's ([`crate::Words`]).

use std::borrow::Cow;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use lindera::dictionary::load_fs_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_dictionary::builder::DictionaryBuilder;
use lindera_dictionary::dictionary::metadata::Metadata;

use crate::words::hiragana;

/// The directory under [`crate::models_dir`].
pub const DIR: &str = "unidic-mecab-2.1.2";
/// The source archive (lindera's mirror of unidic-mecab 2.1.2, as its build fetches it).
pub const URL: &str = "https://Lindera.dev/unidic-mecab-2.1.2.tar.gz";
pub const ARCHIVE: &str = "unidic-mecab-2.1.2.tar.gz";
pub const ARCHIVE_BYTES: u64 = 140_515_741;
pub const ARCHIVE_SHA256: &str = "24ac75fe4e44d973b55e65847c35fa4aec2db7a41a07b49ca125146db9fc1189";
/// What the build leaves; the dictionary is there when all of them are.
pub const BUILT: &[&str] = &[
    "char_def.bin",
    "dict.trie",
    "dict.vals",
    "dict.valsidx",
    "dict.words",
    "dict.wordsidx",
    "matrix.mtx",
    "metadata.json",
    "unk.bin",
    "LICENSE-UNIDIC",
];
/// Bytes on the disk after the build.
pub const DISK_BYTES: u64 = 197_712_000;

/// lindera-unidic's build description and context frequencies (MIT), as its build uses them.
const METADATA: &str = include_str!("../data/unidic/metadata.json");
const CONTEXT_FREQ: &str = include_str!("../data/unidic/context_id_freq.txt");

pub fn path() -> PathBuf {
    crate::models_dir().join(DIR)
}

pub fn available() -> bool {
    let dir = path();
    BUILT.iter().all(|f| dir.join(f).exists())
}

/// Builds the dictionary in `out` from the downloaded `archive`, then removes the archive and
/// the unpacked source (several hundred MB). Files at the archive's top level only; the
/// manual (PDF) stays out.
pub fn build(archive: &Path, out: &Path) -> Result<(), String> {
    let src = out.join("source.part");
    let _ = fs::remove_dir_all(&src);
    fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        let file = File::open(archive).map_err(|e| e.to_string())?;
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
        for entry in tar.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?.into_owned();
            let Some(name) = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if path.components().count() != 2 || name.ends_with(".pdf") {
                continue;
            }
            let mut f = File::create(src.join(&name)).map_err(|e| e.to_string())?;
            io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
        }
        let freq = src.join("context_id_freq.txt");
        fs::write(&freq, CONTEXT_FREQ).map_err(|e| e.to_string())?;
        let metadata: Metadata = serde_json::from_str(METADATA).map_err(|e| e.to_string())?;
        DictionaryBuilder::new(metadata)
            .with_context_id_freq(&freq)
            .build_dictionary(&src, out)
            .map_err(|e| e.to_string())?;
        fs::copy(src.join("BSD"), out.join("LICENSE-UNIDIC")).map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(&src);
    if result.is_ok() {
        let _ = fs::remove_file(archive);
    }
    *loaded() = None;
    result
}

fn loaded() -> MutexGuard<'static, Option<Segmenter>> {
    static SEG: Mutex<Option<Segmenter>> = Mutex::new(None);
    SEG.lock().unwrap_or_else(|e| e.into_inner())
}

/// Frees the loaded dictionary (the idle time of the speech models, /LL012/); the next
/// [`tokens`] loads it again.
pub fn unload() {
    *loaded() = None;
}

/// One word as UniDic reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub surface: String,
    pub start: usize,
    pub end: usize,
    /// In hiragana; `None` when it is not sure ([`reading`]).
    pub reading: Option<String>,
    /// A number (数詞) or a counter (助数詞): readings change together (三本 さんぼん).
    pub counted: bool,
    /// A suffix (接尾辞): read with the word before (日本人, 中国人: じん), which UniDic does
    /// not always see (にん); it gets none.
    pub suffix: bool,
}

/// The words of `text` with their readings; `None` without UniDic.
pub fn tokens(text: &str) -> Option<Vec<Token>> {
    let mut guard = loaded();
    if guard.is_none() {
        if !available() {
            return None;
        }
        let dict = load_fs_dictionary(&path()).ok()?;
        *guard = Some(Segmenter::new(Mode::Normal, dict, None));
    }
    let seg = guard.as_ref()?;
    let found = seg.segment(Cow::Borrowed(text)).ok()?;
    Some(
        found
            .into_iter()
            .map(|mut t| {
                let surface = t.surface.to_string();
                let (start, end) = (t.byte_start, t.byte_end);
                let details: Vec<String> = t.details().iter().map(|s| s.to_string()).collect();
                let d: Vec<&str> = details.iter().map(String::as_str).collect();
                Token {
                    reading: reading(&d),
                    counted: counted(&d),
                    suffix: field(&d, 0) == "接尾辞",
                    surface,
                    start,
                    end,
                }
            })
            .collect(),
    )
}

fn field<'a>(d: &[&'a str], i: usize) -> &'a str {
    d.get(i).copied().filter(|s| *s != "*").unwrap_or_default()
}

/// A number, a counter, or a word that can be one (時間, 分, 月: 助数詞可能); next to a
/// number they read together.
fn counted(d: &[&str]) -> bool {
    field(d, 1) == "数詞" || matches!(field(d, 2), "助数詞" | "助数詞可能")
}

/// Words whose reading depends on the register more than on the sentence: 明日 (あした,
/// あす), 昨日 (きのう, さくじつ), 一日 (いちにち, ついたち), 日本 (にほん, にっぽん).
const REGISTER: &[&str] = &["明日", "昨日", "一日", "日本"];

/// The reading of a token from its UniDic fields (0 part of speech, 1–3 its subcategories,
/// 6 the lexeme's reading, 8 the written form, 9 the pronunciation as written, 10 the base
/// form). The pronunciation is the surface's reading (何 in 何です: ナン, 来 in 来ました: キ)
/// but writes long vowels as ー (センセー); then the lexeme's reading is taken when the
/// surface is the base form (センセイ), else none. No reading either, while the context
/// decides more than UniDic sees: names of people, 日本 (にほん or にっぽん), and the
/// [`crate::AMBIGUOUS`] kanji standing alone (月 as a weekday or the moon).
pub fn reading(d: &[&str]) -> Option<String> {
    let surface = field(d, 8);
    let pron = field(d, 9);
    let base = field(d, 10);
    let lexeme = field(d, 6);
    // From the dictionary form: the lexeme's reading, or the stem's for an inflected form.
    let written = if surface == base && !lexeme.is_empty() {
        Some(lexeme.to_string())
    } else {
        stem_reading(surface, base, lexeme)
    };
    // The pronunciation writes long vowels as ー and づ, ぢ as ズ, ジ (続ける: ツズケル);
    // where that is the only difference, the written reading counts. Where they differ
    // more, the pronunciation is the surface's own (何 in 何です: ナン, not ナニ).
    let kana = match written {
        Some(w) if same_sound(pron, &w) || pron.is_empty() || pron.contains('ー') => w,
        _ if !pron.is_empty() && !pron.contains('ー') => pron.to_string(),
        _ => return None,
    };
    let mut chars = surface.chars();
    let single =
        matches!((chars.next(), chars.next()), (Some(c), None) if crate::AMBIGUOUS.contains(c));
    let person = field(d, 1) == "固有名詞" && field(d, 2) == "人名";
    let register = REGISTER.iter().any(|w| surface.contains(w));
    // 何 with a counter or noun reads by context (何時 なんじ or いつ, 何日 なんにち or なんか).
    let nani = surface.contains('何') && surface.chars().count() > 1;
    if person || register || nani || single && field(d, 0) == "名詞" {
        return None;
    }
    Some(hiragana(&kana))
}

/// Whether a pronunciation (katakana, ー for long vowels, ズ/ジ for づ/ぢ) sounds like a
/// written reading.
fn same_sound(pron: &str, written: &str) -> bool {
    let (p, w): (Vec<char>, Vec<char>) = (
        hiragana(pron).chars().collect(),
        hiragana(written).chars().collect(),
    );
    p.len() == w.len()
        && p.iter().zip(&w).all(|(a, b)| {
            a == b
                || *a == 'ー' && "あいうえお".contains(*b)
                || matches!((a, b), ('ず', 'づ') | ('じ', 'ぢ'))
        })
}

/// The reading of an inflected `surface` from its `base` form and the base's reading: the
/// kanji runs of both must be the same, each keeps the reading it has in the base form, the
/// kana are the surface's own (言い from 言う/イウ: いい).
fn stem_reading(surface: &str, base: &str, base_reading: &str) -> Option<String> {
    let runs = |s: &str| {
        let mut out: Vec<(String, bool)> = Vec::new();
        for c in s.chars() {
            match out.last_mut() {
                Some((r, k)) if *k == crate::is_kanji(c) => r.push(c),
                _ => out.push((c.to_string(), crate::is_kanji(c))),
            }
        }
        out
    };
    let (sr, br) = (runs(surface), runs(base));
    let kanji = |r: &[(String, bool)]| {
        r.iter()
            .filter(|x| x.1)
            .map(|x| x.0.clone())
            .collect::<Vec<_>>()
    };
    if kanji(&sr).is_empty() || kanji(&sr) != kanji(&br) {
        return None;
    }
    let reading: Vec<char> = hiragana(base_reading).chars().collect();
    let parts = crate::align(&br, &reading)?;
    let mut readings = br.iter().zip(parts).filter(|(r, _)| r.1).map(|(_, p)| p);
    Some(
        sr.iter()
            .map(|(r, k)| {
                if *k {
                    readings.next().unwrap_or_default()
                } else {
                    hiragana(r)
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(s: &str) -> Vec<&str> {
        s.split(',').collect()
    }

    #[test]
    fn the_reading_is_the_surfaces() {
        // As lindera's UniDic gives them (probe, 9.11.2026).
        let nan = row("代名詞,*,*,*,*,*,ナニ,何,何,ナン,何,ナン,和,*,*,*,*");
        assert_eq!(reading(&nan).as_deref(), Some("なん"));
        let kado = row("名詞,普通名詞,一般,*,*,*,カド,角,角,カド,角,カド,和,*,*,*,*");
        assert_eq!(
            reading(&kado),
            None,
            "a single ambiguous kanji stays without"
        );
        let tabe = row(
            "動詞,一般,*,*,下一段-バ行,連用形-一般,タベル,食べる,食べ,タベ,食べる,タベル,和,*,*,*,*",
        );
        assert_eq!(reading(&tabe).as_deref(), Some("たべ"));
        let futari =
            row("名詞,普通名詞,副詞可能,*,*,*,フタリ,二人,二人,フタリ,二人,フタリ,和,*,*,*,*");
        assert_eq!(reading(&futari).as_deref(), Some("ふたり"));
        // A long vowel: the lexeme's reading, as the surface is the base form.
        let tokyo = row(
            "名詞,固有名詞,地名,一般,*,*,トウキョウ,トウキョウ,東京,トーキョー,東京,トーキョー,固,*,*,*,*",
        );
        assert_eq!(reading(&tokyo).as_deref(), Some("とうきょう"));
        // Inflected with a long vowel: the stem's reading from the base form.
        let too = row(
            "動詞,一般,*,*,五段-ラ行,連用形-促音便,トオル,通る,通っ,トーッ,通る,トール,和,*,*,*,*",
        );
        assert_eq!(reading(&too).as_deref(), Some("とおっ"));
        let ii =
            row("動詞,一般,*,*,五段-ワア行,連用形-一般,イウ,言う,言い,イー,言う,ユー,和,*,*,*,*");
        assert_eq!(reading(&ii).as_deref(), Some("いい"));
        // Other kanji than the base form's: none.
        assert_eq!(stem_reading("書き", "描く", "カク"), None);
        let nippon =
            row("名詞,固有名詞,地名,国,*,*,ニッポン,日本,日本,ニッポン,日本,ニッポン,固,*,*,*,*");
        assert_eq!(reading(&nippon), None);
        let kin = row("名詞,固有名詞,人名,姓,*,*,キン,キン,金,キン,金,キン,固,*,*,*,*");
        assert_eq!(reading(&kin), None);
    }

    #[test]
    fn numbers_and_counters_go_together() {
        assert!(counted(&row(
            "名詞,数詞,*,*,*,*,フタ,二,二,フタ,二,フタ,和,*,*,*,*"
        )));
        assert!(counted(&row(
            "接尾辞,名詞的,助数詞,*,*,*,ツ,つ,つ,ツ,つ,ツ,和,*,*,*,*"
        )));
        assert!(counted(&row(
            "名詞,普通名詞,助数詞可能,*,*,*,フン,分,分,フン,分,フン,漢,*,*,*,*"
        )));
        assert!(!counted(&row(
            "名詞,普通名詞,一般,*,*,*,ミギ,右,右,ミギ,右,ミギ,和,*,*,*,*"
        )));
        let asu = row("名詞,普通名詞,副詞可能,*,*,*,アス,明日,明日,アス,明日,アス,和,*,*,*,*");
        assert_eq!(reading(&asu), None);
    }

    #[test]
    fn the_written_reading_wins_where_only_the_sound_differs() {
        let tsuzu = row(
            "動詞,一般,*,*,下一段-カ行,連体形-一般,ツヅケル,続ける,続ける,ツズケル,続ける,ツズケル,和,*,*,*,*",
        );
        assert_eq!(reading(&tsuzu).as_deref(), Some("つづける"));
        let sensei =
            row("名詞,普通名詞,一般,*,*,*,センセイ,先生,先生,センセー,先生,センセー,漢,*,*,*,*");
        assert_eq!(reading(&sensei).as_deref(), Some("せんせい"));
        let itsu = row("代名詞,*,*,*,*,*,イツ,何時,何時,イツ,何時,イツ,和,*,*,*,*");
        assert_eq!(reading(&itsu), None);
    }

    #[test]
    fn an_inflected_form_keeps_the_stems_reading() {
        let ii =
            row("動詞,一般,*,*,五段-ワア行,連用形-一般,イウ,言う,言い,イー,言う,ユー,和,*,*,*,*");
        assert_eq!(reading(&ii).as_deref(), Some("いい"));
        let too = row(
            "動詞,一般,*,*,五段-ラ行,連用形-促音便,トオル,通る,通っ,トーッ,通る,トール,和,*,*,*,*",
        );
        assert_eq!(reading(&too).as_deref(), Some("とおっ"));
        // 来 in 来ました: the pronunciation (き), not the base form's く.
        let ki =
            row("動詞,非自立可能,*,*,カ行変格,連用形-一般,クル,来る,来,キ,来る,クル,和,*,*,*,*");
        assert_eq!(reading(&ki).as_deref(), Some("き"));
        assert_eq!(stem_reading("書き", "描く", "カク"), None);
    }
}
