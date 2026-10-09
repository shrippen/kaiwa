//! Character diff of a correction, as KanteDiffText shows it (`parts` with same, removed,
//! added). Japanese has no spaces, so the unit is the character.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PartKind {
    Same,
    Removed,
    Added,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Part {
    pub text: String,
    pub kind: PartKind,
}

/// Longest-common-subsequence diff from `original` to `better`; neighbouring pieces of the
/// same kind are merged, removals come before additions at the same place.
pub fn diff(original: &str, better: &str) -> Vec<Part> {
    let a: Vec<char> = original.chars().collect();
    let b: Vec<char> = better.chars().collect();
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut parts: Vec<Part> = Vec::new();
    let mut push = |c: char, kind: PartKind| match parts.last_mut() {
        Some(last) if last.kind == kind => last.text.push(c),
        _ => parts.push(Part {
            text: c.to_string(),
            kind,
        }),
    };
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            push(a[i], PartKind::Same);
            i += 1;
            j += 1;
        } else if i < n && (j == m || lcs[i + 1][j] >= lcs[i][j + 1]) {
            push(a[i], PartKind::Removed);
            i += 1;
        } else {
            push(b[j], PartKind::Added);
            j += 1;
        }
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(parts: &[Part]) -> Vec<(&str, PartKind)> {
        parts.iter().map(|p| (p.text.as_str(), p.kind)).collect()
    }

    #[test]
    fn particle_swap() {
        assert_eq!(
            kinds(&diff("コーヒーを好きです", "コーヒーが好きです")),
            vec![
                ("コーヒー", PartKind::Same),
                ("を", PartKind::Removed),
                ("が", PartKind::Added),
                ("好きです", PartKind::Same)
            ]
        );
    }

    #[test]
    fn insertion_and_identity() {
        assert_eq!(
            kinds(&diff("ビール二ください", "ビールを二つください")),
            vec![
                ("ビール", PartKind::Same),
                ("を", PartKind::Added),
                ("二", PartKind::Same),
                ("つ", PartKind::Added),
                ("ください", PartKind::Same)
            ]
        );
        assert_eq!(kinds(&diff("はい", "はい")), vec![("はい", PartKind::Same)]);
        assert!(diff("", "").is_empty());
    }

    #[test]
    fn whole_replacement() {
        let parts = diff("知らない", "わかりません");
        let removed: String = parts
            .iter()
            .filter(|p| p.kind != PartKind::Added)
            .map(|p| p.text.as_str())
            .collect();
        let added: String = parts
            .iter()
            .filter(|p| p.kind != PartKind::Removed)
            .map(|p| p.text.as_str())
            .collect();
        assert_eq!(removed, "知らない");
        assert_eq!(added, "わかりません");
    }
}
