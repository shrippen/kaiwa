//! What the plasmoid will offer (M4, before 1.0; the app uses the review answer already),
//! Lastenheft /LF501/–/LF503/, /LF507/: a short talk of two goals from the path, typed or
//! spoken turns with corrections in one line each, and five reviews.
//! A short talk is an ordinary session in the profile, so the app continues it (and the other
//! way round).

use kaiwa_core::Event;
use kaiwa_core::clock::Moment;
use kaiwa_core::session::Input;
use kaiwa_core::srs::{Grade, Item};
use kaiwa_core::{Session, TurnOutcome};
use serde_json::{Value, json};

use kaiwa_store::Recorded;
use kaiwa_store::Result as StoreResult;

use crate::Backend;
use crate::flow::{Length, Talk};
use crate::journal;
use crate::settings::SettingError;
use crate::summary::goals_view;

/// A short talk in progress: the talk flow with two goals.
pub struct ShortTalk {
    talk: Talk,
    /// When the time spent in the talk was last counted.
    counted: Moment,
}

/// One correction in one line (/LF503/): "wrong → right".
fn line(o: &TurnOutcome) -> Vec<String> {
    o.shown
        .iter()
        .map(|s| format!("{} → {}", s.correction.original, s.correction.better))
        .collect()
}

/// The summary of the booked talk; an unreadable profile is added to `failed`.
fn recorded_view(b: &Backend, r: &Recorded, s: &Session, failed: &mut Vec<String>) -> Value {
    let (view, unread) = b.recorded_view(r, s, &s.lang);
    failed.extend(unread);
    view
}

impl ShortTalk {
    /// Starts the next scenario on the path with two goals, or continues the open session.
    pub fn start(b: &Backend, now: Moment) -> anyhow::Result<Self> {
        let opened = Talk::open_or_next(b, now, Length::Short)?;
        if let Some(e) = opened.failed.into_iter().next() {
            return Err(e);
        }
        Ok(Self {
            talk: opened.talk,
            counted: now,
        })
    }

    pub fn session(&self) -> &Session {
        self.talk.session()
    }

    /// The talk as the plasmoid shows it.
    pub fn view(&self) -> Value {
        let s = self.talk.session();
        let lang = s.lang.as_str();
        let last = s.turns().last();
        json!({
            "scenario": s.scenario.id,
            "title": s.scenario.title.get(lang),
            "character": s.scenario.name,
            "opening": s.opening(),
            "goals": goals_view(s, lang),
            "turns": s.turns().len(),
            "last": last.map(|t| t.reply.clone()),
            "done": s.done(),
            "demo": self.talk.demo(),
        })
    }

    /// A turn; the session is saved after it and recorded once its goals are reached.
    pub fn say(
        &mut self,
        b: &Backend,
        text: &str,
        typed: bool,
        now: Moment,
    ) -> anyhow::Result<Value> {
        let input = if typed { Input::Typed } else { Input::Spoken };
        self.talk.spend(Talk::sitting(self.counted, now));
        self.counted = now;
        let Some(mut turned) = self.talk.say(b, text, input, now)? else {
            anyhow::bail!("nothing to say, or the talk is over");
        };
        let o = &turned.outcome;
        // A failure with nothing booked is the error; with a record it is shown beside it.
        if turned.recorded.is_none() && !turned.failed.is_empty() {
            return Err(turned.failed.swap_remove(0));
        }
        let mut failed: Vec<String> = turned.failed.iter().map(|e| format!("{e:#}")).collect();
        // A booked talk is never lost to an unreadable profile: its summary has the defaults
        // and the failure is shown beside it.
        let recorded = match &turned.recorded {
            Some(r) => recorded_view(b, r, self.talk.session(), &mut failed),
            None => Value::Null,
        };
        let mut v = self.view();
        v["said"] = json!(turned.said);
        v["reply"] = json!(o.reply);
        v["corrections"] = json!(line(o));
        // T-11: the tutor's analysis of this turn had gaps; the turn does not count for the level.
        v["incomplete"] = json!(o.incomplete);
        v["recorded"] = recorded;
        if !failed.is_empty() {
            v["failed"] = json!(failed);
        }
        Ok(v)
    }
}

impl Backend {
    /// Checks a review answer (/LF502/): spoken or typed, compared without punctuation.
    pub fn answer_review(&self, text: &str, said: &str, helped: bool, now: Moment) -> Value {
        let item = self.store.item(text);
        answer(text, said, helped, item, |grade| {
            let event = Event::Review {
                text: text.into(),
                grade,
            };
            journal::record(self, &event, now).map_err(|e| match e {
                SettingError::NotSaved(e) | SettingError::NotApplied(e) => e,
                SettingError::Spare => anyhow::anyhow!("a review never refuses on a spare"),
            })?;
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

    fn backend(name: &str) -> crate::TempBackend {
        crate::TempBackend::new(name, true).unwrap()
    }

    #[test]
    fn a_review_answer_is_graded_by_its_item() {
        let b = backend("review-grade");
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
    fn a_short_talk_is_two_goals_and_continues_in_the_app() {
        let b = backend("short");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = ShortTalk::start(&b, now).unwrap();
        assert_eq!(t.session().goals().len(), 2);
        assert!(
            !t.session().grammar().is_empty(),
            "the tutor knows the unit's grammar"
        );
        let v = t.say(&b, "はい", true, now).unwrap();
        assert_eq!(v["turns"], 1);
        // The app finds the same session open (/LF507/).
        let open = b.store.open_session().unwrap().expect("open session");
        assert_eq!(Some(open.id), t.talk.id());
        let again = ShortTalk::start(&b, now).unwrap();
        assert_eq!(again.session().turns().len(), 1);
        assert_eq!(
            again.session().grammar(),
            t.session().grammar(),
            "also after a restore"
        );
    }

    #[test]
    fn a_booked_talk_keeps_its_summary_when_the_profile_is_unreadable() {
        let b = backend("short-unread");
        let t = ShortTalk::start(&b, Moment::new(1_791_100_000, 120)).unwrap();
        let recorded = kaiwa_store::Recorded {
            success: 1.0,
            change: None,
            scenario_done: false,
            new_items: 2,
            xp: 40,
            day: Default::default(),
        };
        let db = rusqlite::Connection::open(&b.path).unwrap();
        db.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('profile', '\"unreadable\"')",
            [],
        )
        .unwrap();
        let mut failed = Vec::new();

        let v = recorded_view(&b, &recorded, t.session(), &mut failed);

        assert_eq!(v["newItems"], 2, "the summary stays: {v}");
        assert_eq!(v["xp"], 40, "with the defaults: {v}");
        assert_eq!(failed.len(), 1, "the failure is shown beside it");
    }

    #[test]
    fn reviews_are_checked_without_punctuation() {
        let b = backend("review");
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
            Err(StoreError::NoSession(1))
        });
        assert!(
            v["error"].as_str().unwrap().contains("no session 1"),
            "the page gets the error: {v}"
        );
        assert!(v.get("next").is_none(), "no date for what was not saved");
    }

    #[test]
    fn an_unreadable_item_is_no_answer() {
        let v = answer("袋", "袋", false, Err(StoreError::NoSession(2)), |_| {
            Ok::<_, StoreError>(None)
        });
        assert!(v["error"].is_string(), "the item is not guessed: {v}");
        assert!(v.get("right").is_none());
    }
}
