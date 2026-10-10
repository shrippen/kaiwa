//! What the plasmoid offers (Lastenheft /LF501/–/LF503/, /LF507/): a short talk of two goals
//! from the path, typed or spoken turns with corrections in one line each, and five reviews.
//! A short talk is an ordinary session in the profile, so the app continues it (and the other
//! way round).

use kaiwa_core::clock::Moment;
use kaiwa_core::difficulty::Knob;
use kaiwa_core::session::Input;
use kaiwa_core::srs::Grade;
use kaiwa_core::{Session, TurnOutcome};
use serde_json::{Value, json};

use crate::{Backend, provider};

/// A short talk in progress.
pub struct ShortTalk {
    pub session: Session,
    pub id: Option<i64>,
    pub started: Moment,
    pub demo: bool,
}

/// One correction in one line (/LF503/): "wrong → right".
fn line(o: &TurnOutcome) -> Vec<String> {
    o.shown
        .iter()
        .map(|s| format!("{} → {}", s.correction.original, s.correction.better))
        .collect()
}

impl ShortTalk {
    /// Starts the next scenario on the path with two goals, or continues the open session.
    pub fn start(b: &Backend, now: Moment) -> anyhow::Result<Self> {
        if let Some(row) = b.store.open_session()? {
            let state = b.store.session_state(row.id)?;
            if let Some(sc) = b.scenarios.get(&state.scenario).cloned() {
                let (p, demo) = provider(&sc.id);
                return Ok(Self {
                    session: Session::restore(sc, p, state),
                    id: Some(row.id),
                    started: now,
                    demo,
                });
            }
        }
        let profile = b.store.profile()?;
        let learning = b.store.learning()?;
        let id = b
            .curriculum
            .next_scenario(&learning.course)
            .unwrap_or_else(|| "n5-01-aisatsu".into());
        let sc = b
            .scenarios
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no scenario {id}"))?;
        let (p, demo) = provider(&sc.id);
        b.arm_brake(now)?;
        let mut d = learning.regulator.difficulty.clone();
        d.set(Knob::Goals, 0);
        let session = Session::new(sc, p, profile.feedback, &profile.ui_lang).with_difficulty(d);
        let id = b.store.begin_session(&session.snapshot(), now).ok();
        Ok(Self {
            session,
            id,
            started: now,
            demo,
        })
    }

    /// The talk as the plasmoid shows it.
    pub fn view(&self) -> Value {
        let s = &self.session;
        let lang = s.lang.as_str();
        let met = s.summary().goals_met;
        let last = s.turns().last();
        json!({
            "scenario": s.scenario.id,
            "title": s.scenario.title.get(lang),
            "character": s.scenario.name,
            "opening": s.opening(),
            "goals": s.goals().iter().map(|g| json!({ "text": g.text.get(lang), "done": met.contains(&g.id) })).collect::<Vec<_>>(),
            "turns": s.turns().len(),
            "last": last.map(|t| t.reply.clone()),
            "done": s.done(),
            "demo": self.demo,
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
        let text = if typed && kaiwa_core::romaji::has_romaji(text) {
            kaiwa_core::romaji::to_kana(text)
        } else {
            text.trim().to_string()
        };
        let input = if typed { Input::Typed } else { Input::Spoken };
        let o = self.session.turn(&text, input)?;
        b.book_costs(now, self.session.done())?;
        let mut recorded = Value::Null;
        if let Some(id) = self.id {
            if self.session.done() {
                let r =
                    b.store
                        .record(id, &self.session, self.started, now, Some(&b.curriculum))?;
                recorded = json!({ "minutes": r.day.speaking_secs / 60, "newItems": r.new_items });
                // XP only with gamification (/LF605/).
                if b.store.profile()?.gamification {
                    recorded["xp"] = json!(r.xp);
                }
                self.id = None;
            } else {
                b.store.save_session(id, &self.session.snapshot())?;
            }
        }
        let mut v = self.view();
        v["said"] = json!(text);
        v["reply"] = json!(o.reply);
        v["corrections"] = json!(line(&o));
        v["recorded"] = recorded;
        Ok(v)
    }
}

impl Backend {
    /// Checks a review answer (/LF502/): spoken or typed, compared without punctuation.
    pub fn answer_review(&self, text: &str, said: &str, helped: bool, now: Moment) -> Value {
        let right = kaiwa_core::norm(said) == kaiwa_core::norm(text);
        let grade = Grade::of(right, helped, false);
        let next = match self.store.review(text, grade, now) {
            Ok(Some(item)) => item.due.iso(),
            _ => String::new(),
        };
        json!({ "right": right, "expected": text, "said": said, "next": next })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend(name: &str) -> Backend {
        let dir = std::env::temp_dir().join(format!("kaiwa-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Backend::open_at(dir.join("t.kaiwa"), true).unwrap()
    }

    #[test]
    fn a_short_talk_is_two_goals_and_continues_in_the_app() {
        let b = backend("short");
        let now = Moment::new(1_791_100_000, 120);
        let mut t = ShortTalk::start(&b, now).unwrap();
        assert_eq!(t.session.goals().len(), 2);
        let v = t.say(&b, "はい", true, now).unwrap();
        assert_eq!(v["turns"], 1);
        // The app finds the same session open (/LF507/).
        let open = b.store.open_session().unwrap().expect("open session");
        assert_eq!(Some(open.id), t.id);
        let again = ShortTalk::start(&b, now).unwrap();
        assert_eq!(again.session.turns().len(), 1);
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
}
