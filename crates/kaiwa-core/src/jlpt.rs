//! The JLPT level a learner level stands for. The one place that knows where N5 ends and N4
//! begins on the learner model's scale ([`crate::learner::LEVELS`]); the tutor's prompt, the
//! amount of corrections and the level words all ask here.

/// First level of N4 (`n4_low`).
pub const N4_FROM: f64 = 3.5;
/// First level of N3.
pub const N3_FROM: f64 = 5.5;
const N2_FROM: f64 = 6.5;
const N1_FROM: f64 = 7.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jlpt {
    N5,
    N4,
    N3,
    N2,
    N1,
}

impl Jlpt {
    /// From the easiest to the hardest.
    pub const ALL: [Jlpt; 5] = [Jlpt::N5, Jlpt::N4, Jlpt::N3, Jlpt::N2, Jlpt::N1];

    /// The level a number on the learner model's scale falls in.
    pub fn of(level: f64) -> Self {
        if level < N4_FROM {
            return Jlpt::N5;
        }
        if level < N3_FROM {
            return Jlpt::N4;
        }
        if level < N2_FROM {
            return Jlpt::N3;
        }
        if level < N1_FROM {
            return Jlpt::N2;
        }
        Jlpt::N1
    }

    pub fn name(self) -> &'static str {
        match self {
            Jlpt::N5 => "N5",
            Jlpt::N4 => "N4",
            Jlpt::N3 => "N3",
            Jlpt::N2 => "N2",
            Jlpt::N1 => "N1",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_changes_at_its_borders() {
        for (level, jlpt) in [
            (0.0, Jlpt::N5),
            (3.49, Jlpt::N5),
            (3.5, Jlpt::N4),
            (5.49, Jlpt::N4),
            (5.5, Jlpt::N3),
            (6.49, Jlpt::N3),
            (6.5, Jlpt::N2),
            (7.49, Jlpt::N2),
            (7.5, Jlpt::N1),
            (8.0, Jlpt::N1),
        ] {
            assert_eq!(Jlpt::of(level), jlpt, "{level}");
        }
    }
}
