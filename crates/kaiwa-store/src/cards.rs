//! Review cards: the items, their due dates and grading (/LF220/).

use crate::*;

impl Store {
    pub fn items(&self) -> Result<Vec<Item>> {
        let mut stmt = self.db.prepare("SELECT value FROM items")?;
        let v: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        v.iter()
            .map(|s| serde_json::from_str(s).map_err(StoreError::from))
            .collect()
    }

    pub fn put_item(&self, item: &Item) -> Result<()> {
        self.db.execute(
            "INSERT INTO items (key, due, value) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET due = excluded.due, value = excluded.value",
            params![
                kaiwa_core::norm(&item.text),
                item.due.0,
                serde_json::to_string(item)?
            ],
        )?;
        Ok(())
    }

    /// Adds an item by hand, e.g. from the word lookup (/LF112/).
    pub fn add_item(&self, item: &Item) -> Result<bool> {
        let n = self.db.execute(
            "INSERT OR IGNORE INTO items (key, due, value) VALUES (?1, ?2, ?3)",
            params![
                kaiwa_core::norm(&item.text),
                item.due.0,
                serde_json::to_string(item)?
            ],
        )?;
        Ok(n > 0)
    }

    /// When the learner last practised: the newest conversation or review answer (T-07:
    /// reviewing counts as practice).
    pub fn last_practice(&self) -> Result<Option<Moment>> {
        let r: Option<(i64, i64)> = self
            .db
            .query_row(
                "SELECT COALESCE(ended, started), offset_min FROM sessions ORDER BY started DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let talked = r.map(|(u, o)| moment(u, o));
        // Kept by `review`, so the start page does not read every item for it (/LL020/).
        let reviewed: Option<Moment> = self.state_value("reviewed")?;
        Ok(talked.max(reviewed))
    }

    /// The day of the last re-entry after a break, if there was one.
    pub fn reentered(&self) -> Result<Option<Day>> {
        self.state_value("reentered")
    }

    /// The re-entry after a break (/LF231/, T-07), when [`srs::new_break`] says it is one: ten
    /// items stay due, the rest is spread over the next days. The day is kept, so the
    /// refresher lasts the whole day whatever the learner opens first.
    pub fn reenter(&self, now: Moment) -> Result<()> {
        if !srs::new_break(now, self.last_practice()?, self.reentered()?) {
            return Ok(());
        }
        let mut items = self.items()?;
        let before: Vec<Day> = items.iter().map(|i| i.due).collect();
        srs::spread_after_break(&mut items, now);
        // One transaction: a long break moves thousands of items, and all or none.
        let write = |st: &Store| -> Result<()> {
            for (item, was) in items.iter().zip(&before) {
                if item.due != *was {
                    st.put_item(item)?;
                }
            }
            st.set_state_value("reentered", &now.day())
        };
        if self.db.is_autocommit() {
            self.batch(write)
        } else {
            write(self)
        }
    }

    /// The items to review now (/LF230/, /LF231/).
    pub fn due(&self, now: Moment, limit: usize) -> Result<Vec<Item>> {
        let items = self.items()?;
        Ok(srs::due(&items, now, limit).into_iter().cloned().collect())
    }

    /// Like [`Store::due`], but only the items of `unit`.
    pub fn due_in(&self, unit: &str, now: Moment, limit: usize) -> Result<Vec<Item>> {
        let items: Vec<Item> = self
            .items()?
            .into_iter()
            .filter(|i| i.unit.as_deref() == Some(unit))
            .collect();
        Ok(srs::due(&items, now, limit).into_iter().cloned().collect())
    }

    /// The review item of `text`, if there is one.
    pub fn item(&self, text: &str) -> Result<Option<Item>> {
        self.get_json(
            "SELECT value FROM items WHERE key = ?1",
            &kaiwa_core::norm(text),
        )
    }

    /// One review answer: FSRS, the unit's review record, the day.
    pub fn review(&self, text: &str, grade: Grade, now: Moment) -> Result<Option<Item>> {
        // One transaction: a failure part-way leaves no half a review.
        self.batch(|s| s.review_in(text, grade, now))
    }

    fn review_in(&self, text: &str, grade: Grade, now: Moment) -> Result<Option<Item>> {
        let key = kaiwa_core::norm(text);
        let Some(mut item): Option<Item> =
            self.get_json("SELECT value FROM items WHERE key = ?1", &key)?
        else {
            return Ok(None);
        };
        item.review(grade, now);
        self.put_item(&item)?;
        self.set_state_value("reviewed", &now)?;
        if let Some(unit) = &item.unit {
            let mut l = self.learning()?;
            l.course.reviewed(unit, grade != Grade::Again);
            self.set_learning(&l)?;
        }
        let mut rec = self.day(now.day())?;
        rec.goal = Some(self.profile()?.daily_goal);
        rec.reviews_done += 1;
        rec.xp += XP_REVIEW;
        self.put_day(now.day(), &rec)?;
        Ok(Some(item))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    /// The start page opened at `now`, after the backend spread a break as it does then.
    fn opened(store: &Store, now: Moment) -> Today {
        store.reenter(now).unwrap();
        store.today(now).unwrap()
    }

    #[test]
    fn after_a_break_ten_are_ten_and_the_rest_waits_for_the_next_days() {
        let store = Store::in_memory().unwrap();
        // Reviewing counts as practice (T-07): no conversation was ever held.
        words_before_a_break(&store, at(2, 9));
        assert_eq!(store.last_practice().unwrap(), Some(at(2, 9)));
        assert!(!opened(&store, at(5, 9)).refresher);
        assert_eq!(opened(&store, at(5, 9)).due, 34);

        let back = at(20, 9);
        let today = opened(&store, back);
        assert!(today.refresher);
        assert_eq!(today.due, 10);
        // Each answer takes one off; none moves up.
        let first = store.due(back, 10).unwrap();
        store.review(&first[0].text, Grade::Good, back).unwrap();
        store.review(&first[1].text, Grade::Good, back).unwrap();
        let today = opened(&store, back.plus_secs(3600));
        assert_eq!(today.due, 8);
        assert!(today.refresher, "the whole day of the re-entry");
        // The next days bring ten each, not the pile.
        let next = opened(&store, at(21, 9));
        assert!(!next.refresher);
        assert_eq!(next.due, 8 + 10);
        assert_eq!(opened(&store, at(22, 9)).due, 8 + 20);
    }

    #[test]
    fn opening_the_app_in_a_break_does_not_use_the_break_up() {
        let store = Store::in_memory().unwrap();
        words_before_a_break(&store, at(2, 9));
        // Opened on day 10, nothing practised; the days after are no re-entry.
        assert_eq!(opened(&store, at(10, 9)).due, 10);
        assert!(!opened(&store, at(16, 9)).refresher);
        // Three weeks later the learner is back: welcome, ten, not the heap.
        let back = opened(&store, at(30, 9));
        assert!(back.refresher);
        assert_eq!(back.due, 10);
        assert_eq!(opened(&store, at(31, 9)).due, 20);
    }

    #[test]
    fn a_break_is_spread_once_also_when_nothing_is_practised_on_the_day_back() {
        let store = Store::in_memory().unwrap();
        words_before_a_break(&store, at(2, 9));
        assert_eq!(opened(&store, at(20, 9)).due, 10);
        // Nothing answered: the day after is no second re-entry, the ten and the next ten wait.
        let next = opened(&store, at(21, 9));
        assert!(!next.refresher);
        assert_eq!(next.due, 20);
        // Practice, then another break: spread again.
        let first = store.due(at(21, 9), 1).unwrap();
        store
            .review(&first[0].text, Grade::Good, at(21, 9))
            .unwrap();
        let again = opened(&store, at(40, 9));
        assert!(again.refresher);
        assert_eq!(again.due, 10);
    }
}
