//! Labels for JMdict's part-of-speech codes in the UI languages. Codes without a label here
//! show JMdict's English description.

/// The label of a code in `lang` ("de" or "en"), `None` for a code without one.
pub fn label(code: &str, lang: &str) -> Option<String> {
    let (de, en) = match code {
        "n" => ("Nomen", "noun"),
        "pn" => ("Pronomen", "pronoun"),
        "v1" | "v1-s" => ("Verb (ichidan)", "verb (ichidan)"),
        c if c.starts_with("v5") => ("Verb (godan)", "verb (godan)"),
        "vk" => ("Verb (unregelmäßig, くる)", "verb (irregular, くる)"),
        "vs" | "vs-i" | "vs-s" => ("Verb mit する", "verb with する"),
        "vz" => ("Verb (ずる)", "verb (ずる)"),
        "adj-i" | "adj-ix" => ("i-Adjektiv", "i-adjective"),
        "adj-na" => ("na-Adjektiv", "na-adjective"),
        "adj-no" => ("mit の", "with の"),
        "adj-pn" => ("Adnominal (連体詞)", "pre-noun adjectival (連体詞)"),
        "adj-t" => ("Adjektiv mit たる", "adjective with たる"),
        "adv" => ("Adverb", "adverb"),
        "adv-to" => ("Adverb mit と", "adverb with と"),
        "prt" => ("Partikel", "particle"),
        "conj" => ("Konjunktion", "conjunction"),
        "int" => ("Interjektion", "interjection"),
        "exp" => ("Ausdruck", "expression"),
        "ctr" => ("Zählwort", "counter"),
        "num" => ("Zahl", "number"),
        "pref" => ("Präfix", "prefix"),
        "suf" | "n-suf" => ("Suffix", "suffix"),
        "aux" | "aux-v" => ("Hilfsverb", "auxiliary verb"),
        "aux-adj" => ("Hilfsadjektiv", "auxiliary adjective"),
        "cop" => ("Kopula", "copula"),
        "vt" => ("transitiv", "transitive"),
        "vi" => ("intransitiv", "intransitive"),
        _ => return None,
    };
    Some(if lang == "de" { de } else { en }.to_string())
}

/// The start of the JMdict codes for an IPADIC part of speech (助詞 → `prt`).
pub fn jmdict_prefix(ipadic: &str) -> Option<&'static str> {
    Some(match ipadic {
        "助詞" => "prt",
        "助動詞" => "aux",
        "動詞" => "v",
        "形容詞" => "adj-i",
        "名詞" => "n",
        "副詞" => "adv",
        "感動詞" => "int",
        "連体詞" => "adj-pn",
        "接続詞" => "conj",
        "接頭詞" => "pref",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_in_both_languages() {
        assert_eq!(label("v5k", "de").as_deref(), Some("Verb (godan)"));
        assert_eq!(label("adj-na", "en").as_deref(), Some("na-adjective"));
        assert_eq!(label("vt", "de").as_deref(), Some("transitiv"));
        assert_eq!(label("unc", "de"), None);
    }
}
