//! Pause mode (/LF602/): holiday or illness. On a paused day the streak neither counts nor
//! breaks; ending the pause early makes today count again. The app and `kaiwa-sim` both go
//! through here, so the persona check tests what the buttons do.

use kaiwa_core::clock::Day;
use kaiwa_core::progress::Pause;

use crate::Backend;

impl Backend {
    /// Pauses `days` days from `today` on; returns the day it rests until now. An earlier,
    /// longer pause still holds, so that may be later than this one's end.
    pub fn pause(&self, today: Day, days: u32) -> anyhow::Result<Day> {
        anyhow::ensure!(days > 0, "a pause has at least one day");
        self.store.add_pause(Pause {
            from: today,
            to: today.plus(i64::from(days) - 1),
        })?;
        Ok(self.store.paused_until(today)?.unwrap_or(today))
    }

    /// Ends the pause running on `today`: it held up to yesterday, today counts again.
    pub fn end_pause(&self, today: Day) -> anyhow::Result<()> {
        Ok(self.store.end_pause(today)?)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_shorter_pause_inside_a_longer_one_rests_until_the_longer_ends() {
        let b = crate::TempBackend::new("pause-until", false).unwrap();
        let today = crate::now().day();

        assert_eq!(b.pause(today, 7).unwrap(), today.plus(6));
        assert_eq!(b.pause(today, 2).unwrap(), today.plus(6));
        assert!(b.pause(today, 0).is_err());

        b.end_pause(today).unwrap();
        assert_eq!(b.store.paused_until(today).unwrap(), None);
    }
}
