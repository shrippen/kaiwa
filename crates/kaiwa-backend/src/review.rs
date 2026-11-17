//! The review answer (/LF502/): checks what the learner said against the card and journals the
//! grade. The app's review page and the plasmoid's short talk both use it.

use kaiwa_core::Event;
use kaiwa_core::clock::Moment;
use kaiwa_core::srs::{Grade, Item};
use kaiwa_store::Result as StoreResult;
use serde_json::{Value, json};

use crate::Backend;
use crate::journal;

impl Backend {
    /// Checks a review answer (/LF502/): spoken or typed, compared without punctuation.
    pub fn answer_review(&self, text: &str, said: &str, helped: bool, now: Moment) -> Value {
        let item = self.store.item(text);
        answer(text, said, helped, item, |grade| {
            let event = Event::Review {
                text: text.into(),
                grade,
            };
            journal::record(self, &event, now)?;
            self.store.item(text).map_err(anyhow::Error::from)
        })
    }
}

/// The answer's words for the page; `{"error": …}` when nothing was saved.
fn answer<E: std::fmt::Display>(
    text: &str,
    said: &str,
    helped: bool,
    item: StoreResult<Option<Item>>,
    review: impl FnOnce(Grade) -> Result<Option<Item>, E>,
) -> Value {
    // A mini conversation looks for the form in the sentence, a word said after is the whole
    // answer (/LF230/).
    let right = match item {
        Ok(Some(item)) => item.answered(said),
        Ok(None) => kaiwa_core::norm(said) == kaiwa_core::norm(text),
        Err(e) => return json!({ "error": e.to_string() }),
    };
    let next = match review(Grade::of(right, helped, false)) {
        Ok(Some(item)) => item.due.iso(),
        Ok(None) => String::new(),
        Err(e) => return json!({ "error": e.to_string() }),
    };
    json!({ "right": right, "expected": text, "said": said, "next": next })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaiwa_store::StoreError;

    #[test]
    fn a_review_answer_is_graded_by_its_item() {
        let b = crate::TempBackend::new("review-grade", true).unwrap();
        let now = crate::now();
        let coffee = b
            .store
            .item("コーヒーが好きです")
            .unwrap()
            .expect("demo item");
        assert_eq!(
            coffee.cue.as_deref(),
            Some("プロデューサーですか。すごいですね。{何|なに}が{好|す}きですか。"),
            "the recorded line before the mistake, with the tutor's readings"
        );
        assert_eq!(coffee.original.as_deref(), Some("コーヒーを好きです"));
        let v = b.answer_review("コーヒーが好きです", "コーヒーを好きです。", false, now);
        assert_eq!(v["right"], false);
        let v = b.answer_review(
            "コーヒーが好きです",
            "はい、コーヒーが好きです。",
            false,
            now,
        );
        assert_eq!(v["right"], true);
        // A word without a line is the whole answer.
        let v = b.answer_review("袋", "手袋", false, now);
        assert_eq!(v["right"], false);
    }

    #[test]
    fn reviews_are_checked_without_punctuation() {
        let b = crate::TempBackend::new("review", true).unwrap();
        let v = b.answer_review(
            "おはようございます",
            "おはようございます。",
            false,
            Moment::new(1_791_100_000, 120),
        );
        assert_eq!(v["right"], true);
    }

    #[test]
    fn a_review_that_was_not_saved_says_so() {
        let v = answer("袋", "袋", false, Ok(None), |_| {
            Err(StoreError::NoSession("x-1".into()))
        });
        assert!(
            v["error"].as_str().unwrap().contains("no session x-1"),
            "the page gets the error: {v}"
        );
        assert!(v.get("next").is_none(), "no date for what was not saved");
    }

    #[test]
    fn an_unreadable_item_is_no_answer() {
        let v = answer(
            "袋",
            "袋",
            false,
            Err(StoreError::NoSession("x-2".into())),
            |_| Ok::<_, StoreError>(None),
        );
        assert!(v["error"].is_string(), "the item is not guessed: {v}");
        assert!(v.get("right").is_none());
    }
}
