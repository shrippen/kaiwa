//! The languages of the interface. Every text the learner reads picks one of them here; no
//! other place decides what an unknown or empty code means.

use serde::{Deserialize, Serialize};

/// A language of the interface (/LL506/).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", from = "String")]
pub enum Lang {
    #[default]
    De,
    En,
}

impl Lang {
    /// The language of a code; an unknown or empty one is German, so a stored value
    /// that is not "en" never breaks a page.
    pub fn of(code: &str) -> Self {
        match code {
            "en" => Self::En,
            _ => Self::De,
        }
    }

    /// The code that is stored, shown to pages and sent to the tutor.
    pub fn code(self) -> &'static str {
        match self {
            Self::De => "de",
            Self::En => "en",
        }
    }

    /// The language's name in English, for prompts.
    pub fn name(self) -> &'static str {
        match self {
            Self::De => "German",
            Self::En => "English",
        }
    }

    /// One of two texts: `Lang::En.pick("Hallo", "Hello")` is "Hello".
    pub fn pick<T>(self, de: T, en: T) -> T {
        match self {
            Self::De => de,
            Self::En => en,
        }
    }
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl From<String> for Lang {
    fn from(code: String) -> Self {
        Self::of(&code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_code_is_german() {
        assert_eq!(Lang::of("en"), Lang::En);
        assert_eq!(Lang::of("de"), Lang::De);
        assert_eq!(Lang::of(""), Lang::De);
        assert_eq!(Lang::of("fr"), Lang::De);
    }

    #[test]
    fn it_reads_and_writes_its_code() {
        assert_eq!(serde_json::to_string(&Lang::En).unwrap(), "\"en\"");
        assert_eq!(serde_json::from_str::<Lang>("\"en\"").unwrap(), Lang::En);
        assert_eq!(serde_json::from_str::<Lang>("\"xx\"").unwrap(), Lang::De);
        assert_eq!(Lang::of(Lang::En.code()), Lang::En);
    }
}
