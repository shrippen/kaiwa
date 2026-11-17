//! How a kind of mistake develops (/LF610/): the profile's chart shows the mistakes of the
//! last talks, one point per talk, and the trend reads exactly those points.

/// The talks a chart shows and a trend reads, newest last.
pub const WINDOW: usize = 10;
/// Talks before a kind of mistake gets a trend, and how many mistakes it must have in them;
/// with fewer there is only the line (one mistake is no trend).
const MIN_TALKS: usize = 4;
const MIN_MISTAKES: u32 = 3;
/// A quarter more or less than before counts as a change.
const CHANGE: f64 = 0.25;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Change {
    Less,
    More,
    Same,
}

/// The change from the earlier half of the talks to the later one, in mistakes per talk.
/// `counts` are the mistakes of one kind per talk in order; only the last `WINDOW` count.
/// Talks, not weeks: a week that has just begun holds few talks and would read as a change.
/// `None`: too little for a trend.
pub fn change(counts: &[u32]) -> Option<Change> {
    let shown = &counts[counts.len().saturating_sub(WINDOW)..];
    if shown.len() < MIN_TALKS || shown.iter().sum::<u32>() < MIN_MISTAKES {
        return None;
    }
    let mean = |c: &[u32]| f64::from(c.iter().sum::<u32>()) / c.len() as f64;
    let (early, late) = shown.split_at(shown.len() / 2);
    let (early, late) = (mean(early), mean(late));
    Some(match late {
        l if l < (1.0 - CHANGE) * early => Change::Less,
        l if l > (1.0 + CHANGE) * early => Change::More,
        _ => Change::Same,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fading_and_rising_over_the_talks() {
        let fading = [3, 3, 2, 2, 1, 1, 0, 0];
        assert_eq!(change(&fading), Some(Change::Less));
        let mut rising = fading;
        rising.reverse();
        assert_eq!(change(&rising), Some(Change::More));
        assert_eq!(change(&[1, 1, 2, 2]), Some(Change::More));
        assert_eq!(change(&[2, 2, 2, 2]), Some(Change::Same));
    }

    #[test]
    fn too_little_is_no_trend() {
        assert_eq!(change(&[]), None);
        // Two days of practice, or one mistake in twenty talks.
        assert_eq!(change(&[1, 3]), None);
        let mut once = [0; 20];
        once[10] = 1;
        assert_eq!(change(&once), None);
    }

    #[test]
    fn only_the_talks_the_chart_shows_count() {
        // Ten talks of four mistakes, then ten that rise from none: the line rises, so the
        // trend does, although all twenty together read as fewer.
        let mut counts = vec![4; 10];
        counts.extend([0, 0, 1, 1, 2, 2, 3, 3, 3, 3]);
        assert_eq!(change(&counts), Some(Change::More));
    }

    #[test]
    fn a_new_week_with_a_busy_monday_is_more() {
        // A week of one mistake a talk, then a Monday talk with four (review #67).
        assert_eq!(change(&[1, 1, 1, 1, 1, 1, 1, 4]), Some(Change::More));
    }
}
