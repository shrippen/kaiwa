//! Time for the learning model: moments with the local offset they happened in, and local days.
//! The days of the daily goal and the streak are local days (/LF602/): a session belongs to the
//! day it started on, in the time zone the learner was in at that moment. A clock is a trait, so
//! `kaiwa-sim` can run 30 days in a second (/LL602/).

use serde::{Deserialize, Serialize};

/// A point in time: seconds since 1970 (UTC) and the local offset in minutes at that moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Moment {
    pub unix: i64,
    pub offset_min: i32,
}

/// A local calendar day: days since 1970-01-01.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Day(pub i64);

impl Moment {
    pub fn new(unix: i64, offset_min: i32) -> Self {
        Self { unix, offset_min }
    }

    /// The local day this moment falls on.
    pub fn day(self) -> Day {
        Day((self.unix + i64::from(self.offset_min) * 60).div_euclid(86_400))
    }

    /// Seconds since the local midnight.
    pub fn second_of_day(self) -> i64 {
        (self.unix + i64::from(self.offset_min) * 60).rem_euclid(86_400)
    }

    pub fn plus_secs(self, secs: i64) -> Self {
        Self {
            unix: self.unix + secs,
            ..self
        }
    }

    /// Days (fractional) from `earlier` to `self`.
    pub fn days_since(self, earlier: Moment) -> f64 {
        (self.unix - earlier.unix) as f64 / 86_400.0
    }
}

impl Day {
    /// The day from a calendar date (proleptic Gregorian).
    pub fn from_ymd(y: i32, m: u32, d: u32) -> Self {
        // Howard Hinnant, days_from_civil.
        let y = i64::from(if m <= 2 { y - 1 } else { y });
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(m);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Day(era * 146_097 + doe - 719_468)
    }

    /// Year, month (1–12), day (1–31).
    pub fn ymd(self) -> (i32, u32, u32) {
        let z = self.0 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = (yoe + era * 400 + i64::from(m <= 2)) as i32;
        (y, m, d)
    }

    /// Year and month, for the freezes per month.
    pub fn month(self) -> (i32, u32) {
        let (y, m, _) = self.ymd();
        (y, m)
    }

    /// ISO date, e.g. "2026-10-09".
    pub fn iso(self) -> String {
        let (y, m, d) = self.ymd();
        format!("{y:04}-{m:02}-{d:02}")
    }

    pub fn plus(self, days: i64) -> Day {
        Day(self.0 + days)
    }

    /// Monday 0 … Sunday 6.
    pub fn weekday(self) -> u32 {
        (self.0 + 3).rem_euclid(7) as u32
    }

    /// Local midnight of this day at `offset_min`.
    pub fn start(self, offset_min: i32) -> Moment {
        Moment::new(self.0 * 86_400 - i64::from(offset_min) * 60, offset_min)
    }
}

/// Where "now" comes from.
pub trait Clock: Send + Sync {
    fn now(&self) -> Moment;
}

/// A clock that only moves when told to (tests, `kaiwa-sim`).
#[derive(Debug)]
pub struct ManualClock(std::sync::Mutex<Moment>);

impl ManualClock {
    pub fn new(at: Moment) -> Self {
        Self(std::sync::Mutex::new(at))
    }
    pub fn set(&self, at: Moment) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = at;
    }
    pub fn advance(&self, secs: i64) {
        let mut m = self.0.lock().unwrap_or_else(|e| e.into_inner());
        *m = m.plus_secs(secs);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Moment {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_round_trip() {
        for (y, m, d) in [
            (1970, 1, 1),
            (2026, 10, 9),
            (2028, 2, 29),
            (2000, 3, 1),
            (1969, 12, 31),
        ] {
            assert_eq!(Day::from_ymd(y, m, d).ymd(), (y, m, d));
        }
        assert_eq!(Day::from_ymd(1970, 1, 1), Day(0));
        assert_eq!(Day::from_ymd(2026, 10, 9).iso(), "2026-10-09");
        assert_eq!(Day::from_ymd(2026, 10, 9).weekday(), 4, "a Friday");
    }

    #[test]
    fn local_days_follow_the_offset() {
        let day = Day::from_ymd(2026, 10, 9);
        // 23:58 in Berlin (UTC+2) is still the 9th there, 21:58 UTC.
        let late = day.start(120).plus_secs(23 * 3600 + 58 * 60);
        assert_eq!(late.day(), day);
        assert_eq!(late.plus_secs(5 * 60).day(), day.plus(1));
        // The same instant seen from UTC+0 is the 9th as well.
        assert_eq!(Moment::new(late.unix, 0).day(), day);
        assert_eq!(late.second_of_day(), 23 * 3600 + 58 * 60);
    }
}
