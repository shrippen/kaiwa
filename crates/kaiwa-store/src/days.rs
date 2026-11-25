//! Days, daily goal and pauses.

use crate::*;

impl Store {
    pub fn day(&self, d: Day) -> Result<DayRecord> {
        let v: Option<String> = self
            .db
            .query_row("SELECT value FROM days WHERE day = ?1", params![d.0], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(match v {
            Some(s) => serde_json::from_str(&s)?,
            None => DayRecord::default(),
        })
    }

    pub fn put_day(&self, d: Day, r: &DayRecord) -> Result<()> {
        self.db.execute(
            "INSERT INTO days (day, value) VALUES (?1, ?2)
             ON CONFLICT(day) DO UPDATE SET value = excluded.value",
            params![d.0, serde_json::to_string(r)?],
        )?;
        Ok(())
    }

    /// The learner chose another daily goal on `today`: a day already begun takes it over, so
    /// it is judged by the goal it ended with, tomorrow as today (/LF602/). Days before keep
    /// theirs.
    pub fn set_day_goal(&self, today: Day, goal: DailyGoal) -> Result<()> {
        // From today on: after a flight west "today" can lie before a day already begun.
        for (day, mut rec) in self.days()?.split_off(&today) {
            rec.goal = Some(goal);
            self.put_day(day, &rec)?;
        }
        Ok(())
    }

    pub fn days(&self) -> Result<BTreeMap<Day, DayRecord>> {
        let mut stmt = self.db.prepare("SELECT day, value FROM days")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        rows.into_iter()
            .map(|(d, v)| Ok((Day(d), serde_json::from_str(&v)?)))
            .collect()
    }

    pub fn pauses(&self) -> Result<Vec<Pause>> {
        let mut stmt = self.db.prepare("SELECT from_day, to_day FROM pauses")?;
        let v = stmt
            .query_map([], |r| {
                Ok(Pause {
                    from: Day(r.get(0)?),
                    to: Day(r.get(1)?),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(v)
    }

    /// Until when a pause runs on `today`: the latest end of the pauses that hold it.
    pub fn paused_until(&self, today: Day) -> Result<Option<Day>> {
        Ok(self
            .pauses()?
            .iter()
            .filter(|p| p.from <= today && today <= p.to)
            .map(|p| p.to)
            .max())
    }

    /// Ends the pauses running on `today` (T-08): they held up to yesterday, today counts
    /// again; one that began today is gone. Pauses that are over stay as they were.
    pub fn end_pause(&self, today: Day) -> Result<()> {
        self.batch(|st| {
            st.db.execute(
                "DELETE FROM pauses WHERE from_day = ?1 AND to_day >= ?1",
                params![today.0],
            )?;
            st.db.execute(
                "UPDATE pauses SET to_day = ?1 - 1 WHERE from_day < ?1 AND to_day >= ?1",
                params![today.0],
            )?;
            Ok(())
        })
    }

    /// Pause mode (holiday, illness): the streak neither counts nor breaks (/LF602/).
    pub fn add_pause(&self, p: Pause) -> Result<()> {
        self.db.execute(
            "INSERT INTO pauses (from_day, to_day) VALUES (?1, ?2)",
            params![p.from.0, p.to.0],
        )?;
        Ok(())
    }

    /// The start page (/LL020/).
    pub fn today(&self, now: Moment) -> Result<Today> {
        let profile = self.profile()?;
        let days = self.days()?;
        let pauses = self.pauses()?;
        let day = now.day();
        let record = days.get(&day).cloned().unwrap_or_default();
        let items = self.items()?;
        Ok(Today {
            day,
            goal_fraction: record.fraction(profile.daily_goal),
            record,
            streak: streak(&days, profile.daily_goal, &pauses, day),
            due: srs::due(&items, now, usize::MAX).len(),
            // The day of the re-entry is the refresher's, whatever was opened since.
            refresher: self.reentered()? == Some(day),
            open_session: self.open_session()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn a_running_pause_can_be_ended() {
        let store = Store::in_memory().unwrap();
        let today = at(10, 9).day();
        assert_eq!(store.paused_until(today).unwrap(), None);
        // One that began three days ago and one set today on top of it; one long over.
        for (from, to) in [(-3, 3), (0, 6), (-20, -15)] {
            store
                .add_pause(Pause {
                    from: today.plus(from),
                    to: today.plus(to),
                })
                .unwrap();
        }
        assert_eq!(store.paused_until(today).unwrap(), Some(today.plus(6)));
        store.end_pause(today).unwrap();
        assert_eq!(store.paused_until(today).unwrap(), None);
        // The days that were paused stay paused; today counts again.
        let mut left = store.pauses().unwrap();
        left.sort_by_key(|p| p.from);
        assert_eq!(
            left,
            vec![
                Pause {
                    from: today.plus(-20),
                    to: today.plus(-15)
                },
                Pause {
                    from: today.plus(-3),
                    to: today.plus(-1)
                },
            ]
        );
        assert_eq!(
            store.paused_until(today.plus(-2)).unwrap(),
            Some(today.plus(-1))
        );
        // Nothing running: ending changes nothing.
        store.end_pause(today).unwrap();
        assert_eq!(store.pauses().unwrap().len(), 2);
    }
}
