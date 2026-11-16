//! Small helpers of the conversation object that need neither Qt nor its state.

use serde_json::{Value, json};

use crate::state::state;

/// Keeps a response time on the device (/LL010/); the failure, if any, as text.
pub(crate) fn book_latency(since: std::time::Instant) -> Option<String> {
    let ms = u32::try_from(since.elapsed().as_millis()).unwrap_or(u32::MAX);
    state().add_latency(ms).err().map(|e| format!("{e:#}"))
}

/// The value of a store read; a failure is noted in `failed` (for `say_unsaved`) and the
/// default stands in.
pub(crate) fn or_default<T: Default, E: std::fmt::Display>(
    r: Result<T, E>,
    failed: &mut Vec<String>,
) -> T {
    r.unwrap_or_else(|e| {
        failed.push(e.to_string());
        T::default()
    })
}

/// The notice with `msg` on a line of its own below what it says; unchanged
/// when it says `msg` already. Every page adds this way, so two failures both show.
pub(crate) fn append_notice(old: &str, msg: &str) -> String {
    if old.contains(msg) {
        return old.to_string();
    }
    match old.is_empty() {
        true => msg.to_string(),
        false => format!("{old}\n{msg}"),
    }
}

/// The record of the summary with the kept mark in it. `None` without a record (recording the
/// talk failed): the mark has nothing to stay with.
pub(crate) fn keep_mark(recorded: &str) -> Option<String> {
    let mut recorded: Value = serde_json::from_str(recorded).ok()?;
    if !recorded.is_object() {
        return None;
    }
    recorded["markKept"] = json!(true);
    Some(recorded.to_string())
}

#[cfg(test)]
mod tests {
    use super::{append_notice, keep_mark};

    #[test]
    fn two_notices_both_show_and_one_shows_once() {
        let first = append_notice("", "The export failed: disk full");
        let both = append_notice(&first, "Saved, but not fully applied: locked");
        assert_eq!(
            both,
            "The export failed: disk full\nSaved, but not fully applied: locked"
        );
        assert_eq!(append_notice(&both, "The export failed: disk full"), both);
    }

    #[test]
    fn a_kept_mark_needs_a_record() {
        // Recording failed: `recorded` is empty, the page must not get a record from it.
        assert_eq!(keep_mark(""), None);
        assert_eq!(keep_mark("null"), None);

        let kept = keep_mark(r#"{"minutes":3,"xp":null}"#).unwrap();
        let v: serde_json::Value = serde_json::from_str(&kept).unwrap();
        assert_eq!(v["markKept"], true);
        assert_eq!(v["minutes"], 3);
    }
}
