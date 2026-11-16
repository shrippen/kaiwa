//! The placement (/LF200/), once: stage after stage as unstored talks at rising levels, what
//! follows a turn, the end with the learner's word on reading, and the skip it offers
//! (/LF215/). A front end plays the stage's talk through [`Talk`] and only shows what this
//! says.
//!
//! ```text
//!  Placing::start ─► Stage ─► (turns) ─► after_turn ─► Go | Stage | Over | Unjudged
//!                                                          Over ─► finish ─► result view
//! ```

use kaiwa_core::learner::Skill;
use kaiwa_core::placement::{self, Placement, Reading, STAGES, Step};
use kaiwa_core::{Moment, Session};
use kaiwa_store::Profile;
use serde_json::{Value, json};

use crate::Backend;
use crate::flow::{Opened, Talk};
use crate::views;

/// A placement in progress.
pub struct Placing {
    placement: Placement,
}

/// A stage that opened: its unstored talk and what the page needs around it.
pub struct Stage {
    pub opened: Opened,
    /// The opening line of the character.
    pub opening: String,
    /// The learner's interface language.
    pub lang: String,
    /// The stage's place, from 0.
    pub index: usize,
    /// How many stages there are at most.
    pub of: usize,
}

/// What follows a turn.
pub enum Next {
    /// The stage goes on.
    Go,
    /// The learner kept up: this stage follows.
    Stage(Box<Stage>),
    /// Over: ask about reading, then [`Placing::finish`].
    Over,
    /// T-11: too many turns the tutor analysed with gaps. Nothing is asked and nothing
    /// stored; the level stays what it was. The value is the result view saying so.
    Unjudged(Value),
}

/// The end of the placement: the result view and what failed while keeping it.
pub struct Finished {
    pub result: Value,
    pub failed: Vec<anyhow::Error>,
}

/// The learner's word on reading as the page sends it: 2 fluent, 1 some, else none.
pub fn reading(answer: i32) -> Reading {
    match answer {
        2 => Reading::Fluent,
        1 => Reading::Some,
        _ => Reading::None,
    }
}

/// The skip the result offers, taken once, only the offer that ends at `last` (/LF215/).
pub fn skip(b: &Backend, last: &str) -> anyhow::Result<bool> {
    views::take_placement_skip(b, last)
}

impl Placing {
    /// Begins a placement with its first stage.
    pub fn start(b: &Backend, now: Moment) -> anyhow::Result<(Self, Stage)> {
        let stage = Self::stage(b, 0, now)?;
        let placing = Self {
            placement: Placement::default(),
        };
        Ok((placing, stage))
    }

    /// Stage `i` ([`placement::stage_session`]), nothing stored.
    fn stage(b: &Backend, i: usize, now: Moment) -> anyhow::Result<Stage> {
        let id = STAGES[i].0;
        let scenario = b
            .scenario(id)
            .ok_or_else(|| anyhow::anyhow!("scenario {id} is missing"))?;
        let mut failed = Vec::new();
        let profile = b.profile().unwrap_or_else(|e| {
            failed.push(e);
            Profile::default()
        });
        let (provider, demo) = b.provider(&scenario.id);
        let opening = scenario.opening.clone();
        let session =
            placement::stage_session(scenario, provider, profile.feedback, &profile.ui_lang, i);
        let mut opened = Talk::unstored(b, session, demo, now);
        failed.append(&mut opened.failed);
        opened.failed = failed;

        Ok(Stage {
            opened,
            opening,
            lang: profile.ui_lang,
            index: i,
            of: STAGES.len(),
        })
    }

    /// After a turn: the stage ends after its turns (or its goals), then the next stage
    /// follows while the learner keeps up.
    pub fn after_turn(&mut self, b: &Backend, s: &Session, now: Moment) -> anyhow::Result<Next> {
        let step = self.placement.after_turn(s);
        self.advance(b, step, now)
    }

    fn advance(&mut self, b: &Backend, step: Step, now: Moment) -> anyhow::Result<Next> {
        Ok(match step {
            Step::Go => Next::Go,
            Step::Next(i) => Next::Stage(Box::new(Self::stage(b, i, now)?)),
            Step::Over => Next::Over,
            Step::Unjudged => Next::Unjudged(json!({ "placement": [], "unjudged": true })),
        })
    }

    /// Ends the placement with the learner's word on reading: stores the model it gives and
    /// builds the result view, with the skip on offer (none for a beginner).
    pub fn finish(mut self, b: &Backend, kana: Reading, kanji: Reading) -> Finished {
        let mut failed = Vec::new();
        self.placement.kana = kana;
        self.placement.kanji = kanji;

        // The store keeps what a talk does not show (pronunciation) and the history.
        let model = self.placement.result();
        let lang = b.profile().map(|p| p.ui_lang).unwrap_or_else(|e| {
            failed.push(e);
            Profile::default().ui_lang
        });
        let words: Vec<String> = model
            .describe(&lang)
            .into_iter()
            .filter(|(s, _)| *s != Skill::Pronunciation)
            .map(|(_, w)| w)
            .collect();
        if let Err(e) = b.set_placement(model) {
            failed.push(e);
        }

        let offer = views::placement_offer(b).unwrap_or_else(|e| {
            failed.push(e);
            Default::default()
        });
        let skip = (!offer.units.is_empty())
            .then(|| json!({ "last": offer.units.last(), "title": offer.title }));
        let result = json!({
            "placement": words,
            "stages": self.placement.results.len(),
            "minutes": self.placement.minutes(),
            "skip": skip,
        });
        Finished { result, failed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TempBackend;
    use kaiwa_core::learner::Skill;
    use kaiwa_core::placement::StageResult;

    fn backend(name: &str) -> TempBackend {
        TempBackend::new(name, false).unwrap()
    }

    fn now() -> Moment {
        Moment::new(1_791_100_000, 120)
    }

    fn good(level: f64) -> StageResult {
        StageResult {
            level,
            turns: 4,
            listening: 1.0,
            speaking: 1.0,
            fluency: 1.0,
            register_mistakes: 0,
        }
    }

    #[test]
    fn the_stages_come_in_order_and_store_nothing() {
        let b = backend("placing-order");
        let (mut p, first) = Placing::start(&b, now()).unwrap();
        assert!(first.opened.failed.is_empty(), "{:?}", first.opened.failed);
        assert_eq!((first.index, first.of), (0, STAGES.len()));
        assert_eq!(
            first.opened.talk.session().scenario.id,
            STAGES[0].0,
            "stage 1 plays its scenario"
        );
        assert!(!first.opening.is_empty());
        assert!(first.opened.talk.id().is_none(), "nothing stored");

        for i in 1..STAGES.len() {
            p.placement.add(good(STAGES[i - 1].1));
            let Next::Stage(s) = p.advance(&b, Step::Next(i), now()).unwrap() else {
                panic!("stage {i} follows");
            };
            assert_eq!(s.index, i);
            assert_eq!(s.opened.talk.session().scenario.id, STAGES[i].0);
        }
        assert!(b.store.open_session().unwrap().is_none());
    }

    #[test]
    fn go_over_and_unjudged_are_told_apart() {
        let b = backend("placing-steps");
        let (mut p, _) = Placing::start(&b, now()).unwrap();
        assert!(matches!(p.advance(&b, Step::Go, now()).unwrap(), Next::Go));
        assert!(matches!(
            p.advance(&b, Step::Over, now()).unwrap(),
            Next::Over
        ));
        let Next::Unjudged(v) = p.advance(&b, Step::Unjudged, now()).unwrap() else {
            panic!("unjudged");
        };
        assert_eq!(v, json!({ "placement": [], "unjudged": true }));
    }

    #[test]
    fn a_first_turn_does_not_end_the_stage() {
        let b = backend("placing-turn");
        let (mut p, mut stage) = Placing::start(&b, now()).unwrap();
        let t = &mut stage.opened.talk;
        t.say(&b, "はじめまして", kaiwa_core::Input::Spoken, now())
            .unwrap();
        let next = p.after_turn(&b, t.session(), now()).unwrap();
        assert!(matches!(next, Next::Go));
        assert!(p.placement.results.is_empty());
    }

    #[test]
    fn reading_answers_map_to_levels() {
        assert_eq!(reading(2), Reading::Fluent);
        assert_eq!(reading(1), Reading::Some);
        assert_eq!(reading(0), Reading::None);
        assert_eq!(reading(-3), Reading::None);
    }

    #[test]
    fn finish_stores_the_model_and_names_no_pronunciation() {
        let b = backend("placing-finish");
        let (mut p, _) = Placing::start(&b, now()).unwrap();
        for (_, level) in STAGES {
            p.placement.add(good(level));
        }
        let expected = p.placement.result();
        let done = p.finish(&b, Reading::Fluent, Reading::None);

        assert!(done.failed.is_empty(), "{:?}", done.failed);
        let stored = b.store.learning().unwrap().model;
        assert_eq!(
            stored.level(Skill::Listening),
            expected.level(Skill::Listening)
        );
        assert!(stored.level(Skill::Listening) >= 6.5);

        let r = &done.result;
        assert_eq!(r["stages"], STAGES.len());
        assert_eq!(r["minutes"], Placement::max_minutes());
        let words = r["placement"].as_array().unwrap();
        let pron = Skill::Pronunciation;
        let shown = expected.describe("en");
        let named: Vec<_> = shown.iter().filter(|(s, _)| *s != pron).collect();
        assert_eq!(words.len(), named.len());
        let word = shown.iter().find(|(s, _)| *s == pron).map(|(_, w)| w);
        if let Some(w) = word {
            assert!(!words.iter().any(|x| x == w));
        }
    }

    #[test]
    fn minutes_count_the_stages_played() {
        let b = backend("placing-minutes");
        let (mut p, _) = Placing::start(&b, now()).unwrap();
        p.placement.add(good(STAGES[0].1));
        let done = p.finish(&b, Reading::None, Reading::None);
        assert_eq!(done.result["stages"], 1);
        assert_eq!(done.result["minutes"], 2);
    }

    #[test]
    fn an_unknown_skip_changes_nothing() {
        let b = backend("placing-skip");
        assert!(!skip(&b, "no-such-unit").unwrap());
    }
}
