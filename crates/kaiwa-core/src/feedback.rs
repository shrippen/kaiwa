//! Which corrections a learner sees, and how (Lastenheft /LF302/ to /LF306/, /LL110/).

use kaiwa_ai::{Correction, Severity};
use serde::{Deserialize, Serialize};

use crate::diff::{Part, diff};

/// How much feedback the learner wants (/LF304/).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Amount {
    /// Only what blocks understanding or is wrong.
    Little,
    /// Plus what is unnatural.
    Normal,
    /// Plus style.
    Much,
}

impl Amount {
    /// The mildest severity still shown.
    pub fn threshold(self) -> Severity {
        match self {
            Amount::Little => Severity::Wrong,
            Amount::Normal => Severity::Unnatural,
            Amount::Much => Severity::Style,
        }
    }
}

/// The tier of KanteHintCard for a severity (decision D-4).
pub fn tier(severity: Severity) -> &'static str {
    match severity {
        Severity::BlocksUnderstanding => "red",
        Severity::Wrong => "yellow",
        Severity::Unnatural => "blue",
        Severity::Style => "none",
    }
}

/// A correction ready for the GUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shown {
    pub correction: Correction,
    pub tier: &'static str,
    pub parts: Vec<Part>,
    /// Interrupts the conversation with a question back (/LF306/).
    pub interrupts: bool,
}

/// Only mistakes that block understanding stop the conversation (/LF306/).
pub fn interrupts(c: &Correction) -> bool {
    c.severity == Severity::BlocksUnderstanding && !c.also_possible
}

/// The corrections to show for `amount`. "Also possible" alternatives are never mistakes
/// (/LF303/): they appear only with `Much`, as style.
pub fn visible(corrections: &[Correction], amount: Amount) -> Vec<Shown> {
    corrections
        .iter()
        .filter(|c| {
            if c.also_possible {
                amount == Amount::Much
            } else {
                c.severity >= amount.threshold()
            }
        })
        .map(|c| Shown {
            correction: c.clone(),
            tier: if c.also_possible {
                tier(Severity::Style)
            } else {
                tier(c.severity)
            },
            parts: diff(&c.original, &c.better),
            interrupts: interrupts(c),
        })
        .collect()
}

/// Cross-check against false alarms (ADR-004): a correction of severity `Wrong` or above is
/// kept only when a second, independent analysis names the same better form. Milder ones
/// come from the first analysis as they are.
pub fn cross_check(first: &[Correction], second: &[Correction]) -> Vec<Correction> {
    let key = |c: &Correction| crate::norm(&c.better);
    first
        .iter()
        .filter(|c| {
            c.severity < Severity::Wrong
                || second
                    .iter()
                    .any(|d| d.severity >= Severity::Unnatural && key(d) == key(c))
        })
        .cloned()
        .collect()
}

/// The word for a severity in the UI language (the tier is never colour alone).
pub fn severity_word(s: Severity, lang: &str) -> &'static str {
    let de = lang == "de";
    match s {
        Severity::BlocksUnderstanding => {
            if de {
                "Verhindert Verständnis"
            } else {
                "Blocks understanding"
            }
        }
        Severity::Wrong => {
            if de {
                "Falsch"
            } else {
                "Wrong"
            }
        }
        Severity::Unnatural => {
            if de {
                "Unnatürlich"
            } else {
                "Unnatural"
            }
        }
        Severity::Style => {
            if de {
                "Stil"
            } else {
                "Style"
            }
        }
    }
}

/// The word for a kind of mistake in the UI language.
pub fn kind_word(k: kaiwa_ai::CorrectionKind, lang: &str) -> &'static str {
    use kaiwa_ai::CorrectionKind as K;
    let de = lang == "de";
    match k {
        K::Particle => {
            if de {
                "Partikel"
            } else {
                "Particle"
            }
        }
        K::Conjugation => {
            if de {
                "Konjugation"
            } else {
                "Conjugation"
            }
        }
        K::WordChoice => {
            if de {
                "Wortwahl"
            } else {
                "Word choice"
            }
        }
        K::Register => "Register",
        K::WordOrder => {
            if de {
                "Satzstellung"
            } else {
                "Word order"
            }
        }
        K::Counter => {
            if de {
                "Zählwort"
            } else {
                "Counter"
            }
        }
        K::Omission => {
            if de {
                "Auslassung"
            } else {
                "Omission"
            }
        }
        K::Loanword => {
            if de {
                "Lehnwort"
            } else {
                "Loanword"
            }
        }
        K::Tense => {
            if de {
                "Zeitform"
            } else {
                "Tense"
            }
        }
        K::Pronunciation => {
            if de {
                "Aussprache"
            } else {
                "Pronunciation"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaiwa_ai::CorrectionKind;

    fn c(severity: Severity, better: &str) -> Correction {
        Correction {
            severity,
            kind: CorrectionKind::Particle,
            original: "コーヒーを好きです".into(),
            better: better.into(),
            explanation: String::new(),
            also_possible: false,
        }
    }

    #[test]
    fn amount_filters_by_severity() {
        let all = vec![
            c(Severity::Style, "a"),
            c(Severity::Unnatural, "b"),
            c(Severity::Wrong, "c"),
            c(Severity::BlocksUnderstanding, "d"),
        ];
        assert_eq!(visible(&all, Amount::Little).len(), 2);
        assert_eq!(visible(&all, Amount::Normal).len(), 3);
        assert_eq!(visible(&all, Amount::Much).len(), 4);
    }

    #[test]
    fn only_blocking_mistakes_interrupt() {
        let shown = visible(
            &[
                c(Severity::Wrong, "x"),
                c(Severity::BlocksUnderstanding, "y"),
            ],
            Amount::Little,
        );
        assert_eq!(
            shown.iter().map(|s| s.interrupts).collect::<Vec<_>>(),
            vec![false, true]
        );
        assert_eq!(shown[0].tier, "yellow");
        assert_eq!(shown[1].tier, "red");
    }

    #[test]
    fn also_possible_is_never_a_mistake() {
        let mut alt = c(Severity::Wrong, "コーヒーが好きです");
        alt.also_possible = true;
        assert!(visible(std::slice::from_ref(&alt), Amount::Normal).is_empty());
        let shown = visible(&[alt], Amount::Much);
        assert_eq!(shown[0].tier, "none");
        assert!(!shown[0].interrupts);
    }

    #[test]
    fn cross_check_drops_unconfirmed_serious_corrections() {
        let first = vec![
            c(Severity::Wrong, "コーヒーが好きです"),
            c(Severity::Wrong, "珈琲が好き"),
            c(Severity::Style, "z"),
        ];
        let second = vec![c(Severity::Unnatural, "コーヒーが 好きです。")];
        let kept = cross_check(&first, &second);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].better, "コーヒーが好きです");
        assert_eq!(kept[1].severity, Severity::Style);
    }

    #[test]
    fn diff_parts_come_with_the_correction() {
        let shown = visible(&[c(Severity::Wrong, "コーヒーが好きです")], Amount::Little);
        assert_eq!(shown[0].parts.len(), 4);
    }
}
