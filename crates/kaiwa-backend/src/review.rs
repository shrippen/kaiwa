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

    /// Back after a break (/LF231/): what is due is spread over the next days, journaled.
    /// Called before anything shows what is due.
    fn spread_break(&self, now: Moment) -> anyhow::Result<()> {
        let (last, reentered) = (self.store.last_practice()?, self.store.reentered()?);
        if kaiwa_core::srs::new_break(now, last, reentered) {
            journal::record(self, &Event::BreakSpread, now)?;
        }
        Ok(())
    }

    /// The items due now, of `unit` or all, after a break is spread.
    pub(crate) fn due(
        &self,
        unit: Option<&str>,
        now: Moment,
        limit: usize,
    ) -> anyhow::Result<Vec<Item>> {
        self.spread_break(now)?;
        Ok(match unit {
            Some(u) => self.store.due_in(u, now, limit)?,
            None => self.store.due(now, limit)?,
        })
    }

    /// The start page's numbers, after a break is spread.
    pub(crate) fn opened_today(&self, now: Moment) -> anyhow::Result<kaiwa_store::Today> {
        self.spread_break(now)?;
        Ok(self.store.today(now)?)
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
    use crate::views::{review, review_scope, review_unit};
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

    #[test]
    fn a_review_node_opens_the_review_of_its_unit() {
        let b = crate::TempBackend::new("review-unit", false).unwrap();
        let now = crate::now();
        let (a, other) = (&b.curriculum.units[0], &b.curriculum.units[1]);
        // The other unit's items are less stable, so they come first in the whole review.
        let put = |text: &str, unit: &str, added: Moment| {
            let mut item = kaiwa_core::srs::Item::new(
                text,
                kaiwa_core::srs::ItemKind::Phrase,
                kaiwa_core::srs::Source::NewWord,
                added,
            );
            item.unit = Some(unit.to_string());
            item.due = now.day();
            b.store.put_item(&item).unwrap();
        };
        put("はじめまして", &a.id, now);
        put("おはよう", &a.id, now);
        for (n, text) in ["ありがとう", "すみません", "さようなら"]
            .iter()
            .enumerate()
        {
            put(text, &other.id, now.plus_secs(-(10 + n as i64) * 86_400));
        }

        let all = review(&b, now, 10).unwrap();
        assert_eq!(all[0]["unit"], other.id.as_str());

        let items = review_unit(&b, now, 10, &a.id).unwrap();
        let items = items.as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i["unit"] == a.id.as_str()));

        let scope = review_scope(&b, now, &a.id).unwrap();
        assert_eq!(scope["id"], a.id.as_str());
        assert_eq!(scope["due"], 2);
        assert!(scope["title"].as_str().is_some_and(|t| !t.is_empty()));
    }

    #[test]
    fn a_unit_without_due_items_reviews_everything_due() {
        let b = crate::TempBackend::new("review-unit-done", false).unwrap();
        let now = crate::now();
        let (a, other) = (&b.curriculum.units[0], &b.curriculum.units[1]);
        let mut item = kaiwa_core::srs::Item::new(
            "ありがとう",
            kaiwa_core::srs::ItemKind::Phrase,
            kaiwa_core::srs::Source::NewWord,
            now,
        );
        item.unit = Some(other.id.clone());
        item.due = now.day();
        b.store.put_item(&item).unwrap();

        assert_eq!(review_scope(&b, now, &a.id).unwrap(), Value::Null);
        let items = review_unit(&b, now, 10, &a.id).unwrap();
        assert_eq!(items.as_array().unwrap().len(), 1);
    }
}
