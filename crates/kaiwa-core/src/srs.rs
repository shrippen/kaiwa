//! Spaced repetition with FSRS-5 (Lastenheft /LF230/, /LF231/, /LD020/).
//!
//! Items are words, phrases and patterns from the learner's own conversations: the better forms
//! of corrections, new words and words taken over from the lookup (/LF112/). A review is never a
//! flash card: the app asks for the item in a mini conversation or as something to say after the
//! character, and grades the answer (again, hard, good, easy). FSRS turns the grades into
//! stability, difficulty and the next due date, for a retention of 90 %.
//!
//! After a break of seven days or more, the learner starts with a refresher of at most ten items,
//! the ones closest to being forgotten, instead of a pile of due reviews (/LF231/). Ten are ten
//! (T-07, owner 10.11.2026): the rest of what is due is spread once over the next days.

use serde::{Deserialize, Serialize};

use crate::clock::{Day, Moment};

/// FSRS-5 default parameters (open-spaced-repetition, fsrs-rs 2.0).
pub const W: [f64; 19] = [
    0.40255, 1.18385, 3.173, 15.69105, 7.1949, 0.5345, 1.4604, 0.0046, 1.54575, 0.1192, 1.01925,
    1.9395, 0.11, 0.29605, 2.2698, 0.2315, 2.9898, 0.51655, 0.6621,
];
const DECAY: f64 = -0.5;
const FACTOR: f64 = 19.0 / 81.0;
/// Target retention.
pub const RETENTION: f64 = 0.9;
/// A break this long starts with a refresher.
pub const BREAK_DAYS: i64 = 7;
pub const REFRESHER_SIZE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grade {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

impl Grade {
    /// The grade of a review answer: wrong → again, right with help → hard, right → good,
    /// right at once and fluent → easy.
    pub fn of(right: bool, helped: bool, fluent: bool) -> Grade {
        match (right, helped, fluent) {
            (false, _, _) => Grade::Again,
            (true, true, _) => Grade::Hard,
            (true, false, true) => Grade::Easy,
            (true, false, false) => Grade::Good,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Word,
    Phrase,
    /// A grammar pattern, e.g. 〜が好きです.
    Pattern,
}

/// Where an item came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "from")]
pub enum Source {
    /// The better form of a correction, with its kind (particle, counter …).
    Correction {
        kind: String,
    },
    NewWord,
    /// Taken over from the word lookup (/LF112/).
    Lookup,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// The Japanese text, e.g. コーヒーが好きです.
    pub text: String,
    pub kind: ItemKind,
    pub source: Source,
    /// The unit it belongs to, if any.
    #[serde(default)]
    pub unit: Option<String>,
    /// The character's line the learner answered when the mistake happened (plain text; the
    /// opening keeps its furigana markup). The review plays it again as a mini conversation:
    /// the character says it, the learner answers with the better form (/LF230/). `None`: the
    /// item is said after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cue: Option<String>,
    /// The learner's words the correction replaced, with up to [`CONTEXT`] characters before
    /// them from the learner's sentence (少し話されます for 話せます; see [`in_context`]). An
    /// answer in the mini conversation that still has them is not right; the context keeps a
    /// correct use elsewhere in the answer from counting (駅の前で next to 仕事の前に).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
    pub added: Moment,
    /// FSRS memory state; `None` before the first review.
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    pub last_review: Option<Moment>,
    pub due: Day,
    pub reps: u32,
    pub lapses: u32,
}

impl Item {
    /// A new item, due the day after it was met.
    pub fn new(text: impl Into<String>, kind: ItemKind, source: Source, at: Moment) -> Self {
        Self {
            text: text.into(),
            kind,
            source,
            unit: None,
            cue: None,
            original: None,
            added: at,
            stability: None,
            difficulty: None,
            last_review: None,
            due: at.day().plus(1),
            reps: 0,
            lapses: 0,
        }
    }

    /// Whether a review answer is right (/LF230/). Said after: the whole answer is the item.
    /// A mini conversation answers with a sentence: it has the better form, and once the better
    /// form is taken out, not the old mistake (英語が話せます。日本語も少し話されます。 still has
    /// 話されます). Compared by [`crate::norm`].
    pub fn answered(&self, said: &str) -> bool {
        let (said, text) = (crate::norm(said), crate::norm(&self.text));
        if text.is_empty() {
            return false;
        }
        if self.cue.is_none() {
            return said == text;
        }
        let rest = said.replace(&text, "\u{0}");
        let mistake = self
            .original
            .as_deref()
            .map(crate::norm)
            .filter(|o| !o.is_empty() && *o != text);
        // The old words again. Where they contain the better form (駅にに for 駅に), taking
        // the better form out would hide them: look in the whole answer.
        let repeated = mistake.is_some_and(|o| {
            if o.contains(&text) {
                said.contains(&o)
            } else {
                rest.contains(&o)
            }
        });
        said.contains(&text) && !repeated
    }

    /// Probability of recall now (1.0 for an item never reviewed but just met).
    pub fn retrievability(&self, now: Moment) -> f64 {
        match (self.stability, self.last_review) {
            (Some(s), Some(last)) => forgetting(now.days_since(last).max(0.0), s),
            _ => {
                // Never reviewed: treat it like an item with the stability of a "good" first answer.
                forgetting(now.days_since(self.added).max(0.0), W[2])
            }
        }
    }

    /// Applies a review.
    pub fn review(&mut self, grade: Grade, now: Moment) {
        let g = grade as u8 as f64;
        let (s, d) = match (self.stability, self.difficulty, self.last_review) {
            (Some(s), Some(d), Some(last)) => {
                // Whole days between the review dates, as in FSRS-5 (fsrs-rs `delta_t: u32`).
                let elapsed = (now.day().0 - last.day().0).max(0) as f64;
                let r = forgetting(elapsed, s);
                let d2 = next_difficulty(d, g);
                let s2 = if now.day() == last.day() {
                    // Same day: the short-term formula of FSRS-5.
                    s * (W[17] * (g - 3.0 + W[18])).exp()
                } else if grade == Grade::Again {
                    let fail = W[11]
                        * d.powf(-W[12])
                        * ((s + 1.0).powf(W[13]) - 1.0)
                        * (W[14] * (1.0 - r)).exp();
                    // Capped below the old stability, as in fsrs-rs 2.0.
                    fail.min(s / (W[17] * W[18]).exp())
                } else {
                    let hard = if grade == Grade::Hard { W[15] } else { 1.0 };
                    let easy = if grade == Grade::Easy { W[16] } else { 1.0 };
                    s * (W[8].exp()
                        * (11.0 - d)
                        * s.powf(-W[9])
                        * ((W[10] * (1.0 - r)).exp() - 1.0)
                        * hard
                        * easy
                        + 1.0)
                };
                (s2, d2)
            }
            _ => (W[g as usize - 1], initial_difficulty(g)),
        };
        let s = s.max(0.01);
        self.stability = Some(s);
        self.difficulty = Some(d.clamp(1.0, 10.0));
        self.last_review = Some(now);
        self.reps += 1;
        if grade == Grade::Again {
            self.lapses += 1;
        }
        let days = if grade == Grade::Again {
            1
        } else {
            interval(s).max(1)
        };
        self.due = now.day().plus(days);
    }

    pub fn is_due(&self, today: Day) -> bool {
        self.due <= today
    }
}

fn forgetting(elapsed_days: f64, stability: f64) -> f64 {
    (1.0 + FACTOR * elapsed_days / stability).powf(DECAY)
}

fn interval(stability: f64) -> i64 {
    let days = stability / FACTOR * (RETENTION.powf(1.0 / DECAY) - 1.0);
    days.round().clamp(1.0, 36_500.0) as i64
}

fn initial_difficulty(g: f64) -> f64 {
    (W[4] - (W[5] * (g - 1.0)).exp() + 1.0).clamp(1.0, 10.0)
}

fn next_difficulty(d: f64, g: f64) -> f64 {
    let delta = -W[6] * (g - 3.0);
    // Linear damping (FSRS-5), then mean reversion towards the difficulty of an easy start.
    let d1 = d + delta * (10.0 - d) / 9.0;
    (W[7] * initial_difficulty(4.0) + (1.0 - W[7]) * d1).clamp(1.0, 10.0)
}

/// Which items to review now (/LF230/): every due item, the most at risk first, up to `limit`.
/// After a break, [`spread_after_break`] has left only the refresher due (/LF231/).
pub fn due(items: &[Item], now: Moment, limit: usize) -> Vec<&Item> {
    let today = now.day();
    let mut list: Vec<&Item> = items.iter().filter(|i| i.is_due(today)).collect();
    list.sort_by(|a, b| {
        a.retrievability(now)
            .partial_cmp(&b.retrievability(now))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    list.truncate(limit);
    list
}

/// The re-entry after a break (/LF231/, T-07): the `REFRESHER_SIZE` due items closest to being
/// forgotten stay due, the refresher. The rest of what is due moves to the next days,
/// `REFRESHER_SIZE` a day, the most at risk first, so no answer pulls up another one today and
/// no pile waits tomorrow. Only the day an item is due changes, not what the learner knows of
/// it. Returns how many items moved; the caller does this once per break.
pub fn spread_after_break(items: &mut [Item], now: Moment) -> usize {
    let today = now.day();
    let mut order: Vec<(usize, f64)> = items
        .iter()
        .enumerate()
        .filter(|(_, i)| i.is_due(today))
        .map(|(n, i)| (n, i.retrievability(now)))
        .collect();
    order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let rest = order.iter().skip(REFRESHER_SIZE);
    for (k, (n, _)) in rest.clone().enumerate() {
        items[*n].due = today.plus(1 + (k / REFRESHER_SIZE) as i64);
    }
    rest.count()
}

/// Whether a break calls for the refresher.
pub fn refresher(now: Moment, last_practice: Option<Moment>) -> bool {
    last_practice.is_some_and(|l| now.day().0 - l.day().0 >= BREAK_DAYS)
}

/// Characters of the learner's sentence kept before the old words.
pub const CONTEXT: usize = 2;

/// The old words of a correction with up to [`CONTEXT`] characters before them, as they stood in
/// the learner's `sentence`, compared by [`crate::norm`]. The old words alone when the sentence
/// does not have them as heard, or has them more than once: a correction carries no position, so
/// the corrected place is not known (家の前で… 仕事の前で with 前で → 前に).
pub fn in_context(sentence: &str, original: &str) -> String {
    let (s, o) = (crate::norm(sentence), crate::norm(original));
    let once = !o.is_empty() && s.matches(o.as_str()).count() == 1;
    match s.find(&o) {
        Some(at) if once => {
            let before: Vec<char> = s[..at].chars().collect();
            let from = before.len().saturating_sub(CONTEXT);
            before[from..].iter().collect::<String>() + &o
        }
        _ => o,
    }
}

/// A correction as it goes into the review: the better form, its kind (particle, register …),
/// the learner's words it replaced and the character's line it answered.
#[derive(Debug, Clone, PartialEq)]
pub struct Corrected {
    pub better: String,
    pub kind: String,
    pub original: Option<String>,
    pub cue: Option<String>,
}

/// Adds the forms from a session's summary to the items, without duplicates (by the
/// comparison key). Returns how many were new.
pub fn add_from_summary(
    items: &mut Vec<Item>,
    review: &[String],
    corrections: &[Corrected],
    unit: Option<&str>,
    at: Moment,
) -> usize {
    let mut added = 0;
    for text in review {
        let key = crate::norm(text);
        if key.is_empty() || items.iter().any(|i| crate::norm(&i.text) == key) {
            continue;
        }
        let found = corrections.iter().find(|c| crate::norm(&c.better) == key);
        let (kind, source) = match found {
            Some(c) => (
                ItemKind::Pattern,
                Source::Correction {
                    kind: c.kind.clone(),
                },
            ),
            None if text.chars().count() <= 6 => (ItemKind::Word, Source::NewWord),
            None => (ItemKind::Phrase, Source::NewWord),
        };
        let mut item = Item::new(text.clone(), kind, source, at);
        item.unit = unit.map(str::to_string);
        item.cue = found.and_then(|c| c.cue.clone());
        item.original = found.and_then(|c| c.original.clone());
        items.push(item);
        added += 1;
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: i64) -> Moment {
        Day::from_ymd(2026, 10, 1)
            .plus(day)
            .start(120)
            .plus_secs(9 * 3600)
    }

    fn item() -> Item {
        Item::new(
            "コーヒーが好きです",
            ItemKind::Pattern,
            Source::Correction {
                kind: "particle".into(),
            },
            at(0),
        )
    }

    /// FSRS-5 counts whole days between review dates (fsrs-rs: `delta_t: u32`), so a review
    /// just after midnight is one day later. Reference value 3.4452 (S after Hard, then Good
    /// at delta_t = 1).
    #[test]
    fn elapsed_days_are_calendar_days() {
        let late = Day::from_ymd(2026, 10, 1)
            .start(120)
            .plus_secs(23 * 3600 + 50 * 60);
        let mut i = item();
        i.review(Grade::Hard, late);
        i.review(Grade::Good, late.plus_secs(20 * 60));
        let s = i.stability.unwrap();
        assert!((s - 3.4452).abs() < 1e-3, "{s}");
    }

    /// fsrs-rs 2.0 `stability_after_failure` caps the new stability at
    /// `s / exp(w17 * w18)`, not at `s`. Reference value 2.2539.
    #[test]
    fn lapse_stability_is_capped_below_old() {
        let mut i = item();
        i.review(Grade::Good, at(0));
        i.review(Grade::Again, at(59));
        let s = i.stability.unwrap();
        assert!((s - 2.2539).abs() < 1e-3, "{s}");
    }

    #[test]
    fn good_answers_stretch_the_intervals() {
        let mut i = item();
        assert!(i.is_due(at(1).day()));
        let mut day = 1;
        let mut gaps = Vec::new();
        for _ in 0..5 {
            i.review(Grade::Good, at(day));
            let gap = i.due.0 - at(day).day().0;
            gaps.push(gap);
            day += gap;
        }
        assert!(gaps.windows(2).all(|w| w[1] > w[0]), "{gaps:?}");
        assert_eq!(gaps[0], 3, "first good answer: about three days");
    }

    #[test]
    fn again_brings_it_back_tomorrow_and_counts_a_lapse() {
        let mut i = item();
        i.review(Grade::Good, at(1));
        let s = i.stability.unwrap();
        i.review(Grade::Again, at(4));
        assert_eq!(i.due, at(5).day());
        assert_eq!(i.lapses, 1);
        assert!(i.stability.unwrap() < s);
        assert!(i.difficulty.unwrap() > initial_difficulty(3.0));
    }

    #[test]
    fn easy_beats_good_beats_hard() {
        let next = |g| {
            let mut i = item();
            i.review(Grade::Good, at(1));
            i.review(g, at(4));
            i.due.0
        };
        assert!(next(Grade::Easy) > next(Grade::Good));
        assert!(next(Grade::Good) > next(Grade::Hard));
    }

    #[test]
    fn a_long_break_starts_with_a_refresher() {
        let items: Vec<Item> = (0..30)
            .map(|n| Item::new(format!("ことば{n}"), ItemKind::Word, Source::NewWord, at(0)))
            .collect();
        assert_eq!(due(&items, at(3), 50).len(), 30);
        assert!(!refresher(at(3), Some(at(2))));
        assert!(refresher(at(20), Some(at(9))));
    }

    #[test]
    fn after_a_break_ten_stay_due_and_the_rest_is_spread() {
        let mut items: Vec<Item> = (0..34)
            .map(|n| Item::new(format!("ことば{n}"), ItemKind::Word, Source::NewWord, at(0)))
            .collect();
        // One learnt well and not due: it stays where it is.
        items[33].due = at(40).day();
        // One reviewed before: more at risk than the new ones is not the point, it is due.
        items[0].review(Grade::Good, at(1));
        let now = at(20);
        let first: Vec<String> = due(&items, now, REFRESHER_SIZE)
            .iter()
            .map(|i| i.text.clone())
            .collect();
        assert_eq!(spread_after_break(&mut items, now), 23);
        // Ten are ten, the same ten that were most at risk; answering one pulls up no other.
        let today: Vec<String> = due(&items, now, 50)
            .iter()
            .map(|i| i.text.clone())
            .collect();
        assert_eq!(today, first);
        // The rest: ten a day from tomorrow, nothing later than it has to be.
        let on = |d: i64| items.iter().filter(|i| i.due == now.day().plus(d)).count();
        assert_eq!((on(1), on(2), on(3), on(4)), (10, 10, 3, 0));
        assert_eq!(items[33].due, at(40).day());
        // Nothing left to move: a second call changes nothing.
        assert_eq!(spread_after_break(&mut items, now), 0);
    }

    #[test]
    fn summary_forms_become_items_once() {
        let mut items = Vec::new();
        let review = vec!["コーヒーが好きです".to_string(), "駅".to_string()];
        let corr = vec![Corrected {
            better: "コーヒーが好きです".into(),
            kind: "particle".into(),
            original: Some("コーヒーを好きです".into()),
            cue: Some("何が好きですか。".into()),
        }];
        assert_eq!(
            add_from_summary(&mut items, &review, &corr, Some("n5-02"), at(0)),
            2
        );
        assert_eq!(add_from_summary(&mut items, &review, &corr, None, at(1)), 0);
        assert_eq!(items[0].kind, ItemKind::Pattern);
        assert_eq!(items[0].cue.as_deref(), Some("何が好きですか。"));
        assert_eq!(items[0].original.as_deref(), Some("コーヒーを好きです"));
        assert_eq!(items[1].kind, ItemKind::Word);
        assert_eq!(items[1].cue, None, "a new word is said after");
    }

    fn talk(text: &str, original: &str) -> Item {
        let mut i = item();
        i.text = text.into();
        i.original = Some(original.into());
        i.cue = Some("どうですか。".into());
        i
    }

    #[test]
    fn a_mini_conversation_answer_has_the_form_in_a_sentence() {
        let i = talk("お約束しております", "お約束しています");
        assert!(i.answered("ミュラーと申します。二時にお約束しております。"));
        assert!(!i.answered("二時にお約束しています。"));
        assert!(!i.answered(""));
    }

    #[test]
    fn the_old_mistake_next_to_the_form_is_not_right() {
        // n4-06-baito: 話されます → 話せます, while the first clause had 話せます already.
        let i = talk("話せます", "話されます");
        assert!(!i.answered("英語が話せます。日本語も少し話されます。"));
        assert!(i.answered("英語が話せます。日本語も少し話せます。"));
        // The better form contains the old words: only what is left over counts.
        let j = talk("が好きです", "好きです");
        assert!(j.answered("コーヒーが好きです。"));
        // The old words contain the better form: said again, they are still the mistake.
        let k = talk("駅に", "駅にに");
        assert!(!k.answered("駅ににいます。"));
        assert!(k.answered("駅にいます。"));
    }

    #[test]
    fn the_old_words_keep_their_place_in_the_sentence() {
        assert_eq!(
            in_context("仕事の前で、コーヒーを飲みます。", "前で"),
            "事の前で"
        );
        assert_eq!(in_context("うん、行く！", "行く"), "うん行く");
        assert_eq!(in_context("行く", "行く"), "行く");
        assert_eq!(in_context("ちがう文", "前で"), "前で", "not heard as such");
        assert_eq!(
            in_context("家の前で会います。仕事の前で、コーヒーを飲みます。", "前で"),
            "前で",
            "twice: the place is not known"
        );
        // n5-11-ichinichi: 前で → 前に. A right 駅の前で elsewhere is no repeated mistake.
        let i = talk(
            "前に",
            &in_context("仕事の前で、コーヒーを飲みます。", "前で"),
        );
        assert!(i.answered("仕事の前に、駅の前でコーヒーを飲みます。"));
        assert!(!i.answered("仕事の前で、コーヒーを飲みます。"));
        // n5-09-tomodachi: 行く → 行きます.
        let j = talk("行きます", &in_context("うん、行く！", "行く"));
        assert!(j.answered("土曜日、映画を見に行くなら、私も行きます。"));
        // n4-06-baito still: the mistake in its place is wrong.
        let k = talk(
            "話せます",
            &in_context("英語が話せます。日本語も少し話されます。", "話されます"),
        );
        assert!(!k.answered("英語が話せます。日本語も少し話されます。"));
    }

    #[test]
    fn a_word_said_after_is_the_whole_answer() {
        let mut w = item();
        w.text = "袋".into();
        assert!(w.answered("袋"));
        assert!(!w.answered("手袋"), "a longer word is another word");
    }

    #[test]
    fn an_item_without_a_cue_reads_as_before() {
        let old = r#"{"text":"駅","kind":"word","source":{"from":"new_word"},"added":{"unix":0,"offset_min":0},"stability":null,"difficulty":null,"last_review":null,"due":1,"reps":0,"lapses":0}"#;
        let i: Item = serde_json::from_str(old).unwrap();
        assert_eq!(i.cue, None);
        assert!(!serde_json::to_string(&i).unwrap().contains("cue"));
    }
}
