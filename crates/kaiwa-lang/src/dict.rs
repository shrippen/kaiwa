//! The dictionary of the data package kaiwa-data (L-06, ADR-007): JMdict with English and
//! German meanings, Kanjium accents, names from JMnedict (Q-13) and the JLPT levels of tanos
//! (Q-01). Opened read-only; the schema is described in the README of
//! kaiwa-data.

use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::Serialize;

use crate::LangError;
use crate::accent::Accent;
use crate::pos;

/// The schema version this reader understands (kaiwa-data `SCHEMA_VERSION`).
pub const SCHEMA: u32 = 2;
/// Senses shown at most.
const MAX_SENSES: usize = 5;

pub struct Dictionary {
    db: Connection,
}

/// What the lookup shows for a word.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    /// JMdict ent_seq.
    pub id: i64,
    /// The spelling looked up (食べる).
    pub word: String,
    /// Its reading in kana (たべる); equal to `word` for a kana word.
    pub reading: String,
    /// Part of speech in the UI language (Verb (ichidan), transitiv).
    pub pos: Vec<String>,
    /// The meanings, one list of glosses per sense.
    pub senses: Vec<Vec<String>>,
    /// The language of the meanings ("de" or "en").
    pub lang: String,
    /// The meanings are English because JMdict has none in the UI language (M-01).
    pub fallback: bool,
    /// A common word (JMdict news1, ichi1, spec, gai1).
    pub common: bool,
    /// The target accent (Kanjium), if known.
    pub accent: Option<Accent>,
}

/// A name (JMnedict) for a word JMdict does not have: 田中, 新宿 (Q-13).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Name {
    /// The spelling looked up.
    pub word: String,
    /// Its reading in kana.
    pub reading: String,
    /// JMnedict kinds: surname, given, place, station …
    pub kinds: Vec<String>,
    /// The first romanisation or translation (Tanaka).
    pub trans: Option<String>,
}

/// Honorifics after a name and their reading: 田中さん is looked up as 田中.
const HONORIFICS: [(&str, &str); 6] = [
    ("さん", "さん"),
    ("様", "さま"),
    ("さま", "さま"),
    ("くん", "くん"),
    ("ちゃん", "ちゃん"),
    ("先生", "せんせい"),
];

/// Kinds a conversation meets first, preferred when a spelling has several entries.
const COMMON_KINDS: [&str; 7] = [
    "surname", "given", "fem", "masc", "place", "station", "person",
];

struct Form {
    kind: String,
    text: String,
    restr: Option<Vec<String>>,
}

struct Sense {
    lang: String,
    pos: Vec<String>,
    stagk: Option<Vec<String>>,
    stagr: Option<Vec<String>>,
    glosses: Vec<String>,
}

/// ASCII letters and digits to their full-width forms (T → Ｔ), as JMdict writes them.
pub fn full_width(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0'..='9' | 'A'..='Z' | 'a'..='z' => char::from_u32(c as u32 + 0xFEE0).unwrap_or(c),
            _ => c,
        })
        .collect()
}

fn json_list(s: Option<String>) -> Option<Vec<String>> {
    s.and_then(|s| serde_json::from_str(&s).ok())
}

impl Dictionary {
    pub fn open(path: &Path) -> Result<Self, LangError> {
        let db = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let schema: Option<String> = db
            .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |r| {
                r.get(0)
            })
            .optional()?;
        match schema.and_then(|s| s.parse::<u32>().ok()) {
            Some(SCHEMA) => Ok(Self { db }),
            other => Err(LangError::Schema(other)),
        }
    }

    /// The version of the package (meta `version`).
    pub fn version(&self) -> Result<String, LangError> {
        Ok(self
            .db
            .query_row("SELECT value FROM meta WHERE key = 'version'", [], |r| {
                r.get(0)
            })?)
    }

    /// The sources with their licence and attribution, for "About" (/LD050/).
    pub fn sources(&self) -> Result<Vec<(String, String, String)>, LangError> {
        let mut st = self
            .db
            .prepare("SELECT name, licence, attribution FROM source ORDER BY id")?;
        let rows = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// The name a word of a line stands for, when JMdict has no entry (Q-13): the spelling as
    /// written or without an honorific (田中さん → 田中); among several readings the one of the
    /// line, among kinds the everyday ones first.
    pub fn lookup_name(&self, w: &crate::LineWord) -> Result<Option<Name>, LangError> {
        let line = w.reading.as_deref().map(crate::words::hiragana);
        let mut spellings = vec![(w.text.clone(), line.clone())];
        if let Some((bare, kana)) = HONORIFICS
            .iter()
            .find_map(|(h, kana)| w.text.strip_suffix(h).map(|b| (b, kana)))
            .filter(|(b, _)| !b.is_empty())
        {
            let reading = line
                .as_deref()
                .and_then(|l| l.strip_suffix(kana))
                .map(String::from);
            spellings.push((bare.to_string(), reading));
        }
        for (spelling, line) in spellings {
            let mut st = self.db.prepare(
                "SELECT reading, types, trans FROM name WHERE kanji = ?1
                 UNION ALL SELECT reading, types, trans FROM name
                     WHERE kanji IS NULL AND reading IN (?1, ?2)",
            )?;
            let rows: Vec<(String, String, Option<String>)> = st
                .query_map(params![spelling, crate::words::hiragana(&spelling)], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                })?
                .collect::<rusqlite::Result<_>>()?;
            let best = rows
                .into_iter()
                .enumerate()
                .map(|(i, (reading, types, trans))| {
                    let kinds: Vec<String> = serde_json::from_str(&types).unwrap_or_default();
                    let mut score = 0;
                    if line
                        .as_deref()
                        .is_some_and(|l| crate::words::hiragana(&reading) == l)
                    {
                        score += 100;
                    }
                    if kinds.iter().any(|k| COMMON_KINDS.contains(&k.as_str())) {
                        score += 10;
                    }
                    // On a tie the first row: JMnedict's own order.
                    ((score, std::cmp::Reverse(i)), reading, kinds, trans)
                })
                .max_by_key(|(key, ..)| *key);
            if let Some((_, reading, kinds, trans)) = best {
                return Ok(Some(Name {
                    word: spelling,
                    reading,
                    kinds,
                    trans,
                }));
            }
        }
        Ok(None)
    }

    /// The JLPT level of a word in the tanos lists (5 = N5 … 1 = N1), the easiest if it has
    /// several; an estimate, one source for Kaiwa's own level (Q-01) and the level check
    /// (/LF221/, `tests/levels.rs`). A word in kana also
    /// matches the reading of a kanji spelling (いる → 居る).
    pub fn jlpt(&self, word: &str, reading: &str) -> Result<Option<u8>, LangError> {
        Ok(self
            .db
            .query_row(
                "SELECT max(level) FROM jlpt WHERE (word = ?1 OR reading = ?1)
                     AND (reading = ?2 OR ?2 = '')",
                params![word, crate::words::hiragana(reading)],
                |r| r.get::<_, Option<u8>>(0),
            )
            .optional()?
            .flatten())
    }

    /// Looks up a word of a line: as written first (いらっしゃいませ is an expression of its
    /// own), then in its dictionary form (食べました → 食べる).
    pub fn lookup_word(&self, w: &crate::LineWord, lang: &str) -> Result<Option<Entry>, LangError> {
        let pos = pos::jmdict_prefix(&w.pos);
        if w.text != w.key
            && let Some(e) = self.lookup_with(&w.text, None, pos, lang)?
        {
            return Ok(Some(e));
        }
        if let Some(e) = self.lookup_with(&w.key, w.reading.as_deref(), pos, lang)? {
            return Ok(Some(e));
        }
        if let Some(f) = &w.fallback
            && let Some(e) = self.lookup_with(f, None, pos, lang)?
        {
            return Ok(Some(e));
        }
        // JMdict writes latin letters in a word full width (Ｔシャツ).
        let wide = full_width(&w.key);
        if wide != w.key {
            return self.lookup_with(&wide, None, pos, lang);
        }
        Ok(None)
    }

    /// Looks a word up: `key` as written or in its dictionary form, `reading` (kana) to choose
    /// between entries with the same spelling, `lang` the UI language ("de", "en").
    pub fn lookup(
        &self,
        key: &str,
        reading: Option<&str>,
        lang: &str,
    ) -> Result<Option<Entry>, LangError> {
        self.lookup_with(key, reading, None, lang)
    }

    /// `pos`: the start of the JMdict code the word's part of speech has (`prt` for a
    /// particle), to choose は the particle over 葉.
    fn lookup_with(
        &self,
        key: &str,
        reading: Option<&str>,
        pos: Option<&str>,
        lang: &str,
    ) -> Result<Option<Entry>, LangError> {
        let mut st = self.db.prepare(
            "SELECT e.id, e.common, e.nf FROM entry e
             WHERE e.id IN (SELECT entry FROM form WHERE text = ?1)",
        )?;
        let candidates: Vec<(i64, bool, Option<i64>)> = st
            .query_map([key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut best: Option<(i64, i64, Vec<Form>, bool)> = None;
        for (id, common, nf) in candidates {
            let forms = self.forms(id)?;
            let reads = reading.is_some_and(|r| forms.iter().any(|f| f.kind == "r" && f.text == r));
            let fits = pos.is_some_and(|p| self.has_pos(id, p).unwrap_or(false));
            // A matching reading first, then the part of speech, then common and frequent words.
            let score = i64::from(reads) * 1000 + i64::from(fits) * 500 + i64::from(common) * 100
                - nf.unwrap_or(50);
            if best.as_ref().is_none_or(|b| score > b.1) {
                best = Some((id, score, forms, common));
            }
        }
        let Some((id, _, forms, common)) = best else {
            return Ok(None);
        };
        let is_kanji = forms.iter().any(|f| f.kind == "k" && f.text == key);
        let reading = forms
            .iter()
            .filter(|f| f.kind == "r")
            .find(|f| reading.is_some_and(|r| r == f.text))
            .or_else(|| {
                forms.iter().filter(|f| f.kind == "r").find(|f| {
                    !is_kanji && f.text == key
                        || is_kanji && f.restr.as_ref().is_none_or(|r| r.iter().any(|k| k == key))
                })
            })
            .map(|f| f.text.clone())
            .unwrap_or_else(|| key.to_string());

        let senses: Vec<Sense> = self
            .senses(id)?
            .into_iter()
            .filter(|s| {
                s.stagk
                    .as_ref()
                    .is_none_or(|k| !is_kanji || k.iter().any(|k| k == key))
                    && s.stagr.as_ref().is_none_or(|r| r.contains(&reading))
            })
            .collect();
        let pos_codes: Vec<String> = senses
            .iter()
            .find(|s| s.lang == "en" && !s.pos.is_empty())
            .map(|s| s.pos.clone())
            .unwrap_or_default();
        let own: Vec<&Sense> = senses.iter().filter(|s| s.lang == lang).collect();
        let (shown, used, fallback) = if own.is_empty() {
            (
                senses.iter().filter(|s| s.lang == "en").collect::<Vec<_>>(),
                "en",
                lang != "en",
            )
        } else {
            (own, lang, false)
        };
        // Kanjium has no accent for particles and endings; its は is the noun 葉.
        let bound = !pos_codes.is_empty()
            && pos_codes
                .iter()
                .all(|p| p.starts_with("prt") || p.starts_with("aux") || p == "cop");
        let accent = if bound {
            None
        } else {
            self.accent(key, &reading)?
        };
        Ok(Some(Entry {
            id,
            word: key.to_string(),
            reading,
            pos: pos_codes
                .iter()
                .filter_map(|c| pos::label(c, lang).or_else(|| self.entity(c)))
                .collect(),
            senses: shown
                .into_iter()
                .take(MAX_SENSES)
                .map(|s| {
                    let mut seen = std::collections::HashSet::new();
                    s.glosses
                        .iter()
                        .filter(|g| seen.insert(g.as_str()))
                        .cloned()
                        .collect()
                })
                .collect(),
            lang: used.to_string(),
            fallback,
            common,
            accent,
        }))
    }

    /// Whether one of the entry's English senses has a part of speech starting with `prefix`.
    fn has_pos(&self, id: i64, prefix: &str) -> Result<bool, LangError> {
        Ok(self
            .senses(id)?
            .iter()
            .filter(|s| s.lang == "en")
            .any(|s| s.pos.iter().any(|p| p.starts_with(prefix))))
    }

    fn forms(&self, id: i64) -> Result<Vec<Form>, LangError> {
        let mut st = self
            .db
            .prepare("SELECT kind, text, restr FROM form WHERE entry = ?1 ORDER BY kind, ord")?;
        let rows = st
            .query_map([id], |r| {
                Ok(Form {
                    kind: r.get(0)?,
                    text: r.get(1)?,
                    restr: json_list(r.get(2)?),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    fn senses(&self, id: i64) -> Result<Vec<Sense>, LangError> {
        let mut st = self.db.prepare(
            "SELECT lang, pos, stagk, stagr, glosses FROM sense WHERE entry = ?1 ORDER BY lang, ord",
        )?;
        let rows = st
            .query_map([id], |r| {
                Ok(Sense {
                    lang: r.get(0)?,
                    pos: json_list(r.get(1)?).unwrap_or_default(),
                    stagk: json_list(r.get(2)?),
                    stagr: json_list(r.get(3)?),
                    glosses: json_list(r.get(4)?).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// The English description of a JMdict code, for codes without a label of our own.
    fn entity(&self, code: &str) -> Option<String> {
        self.db
            .query_row(
                "SELECT description FROM entity WHERE code = ?1",
                [code],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten()
    }

    fn accent(&self, word: &str, reading: &str) -> Result<Option<Accent>, LangError> {
        let pattern: Option<String> = self
            .db
            .query_row(
                "SELECT pattern FROM accent WHERE word = ?1 AND reading = ?2
                 UNION ALL SELECT pattern FROM accent WHERE word = ?2 AND reading = ?2
                 LIMIT 1",
                params![word, reading],
                |r| r.get(0),
            )
            .optional()?;
        Ok(pattern.and_then(|p| Accent::new(reading, &p)))
    }
}
