//! The steps the settings offer for values that are no learning rule of their own: text
//! size, daily goal, unloading the speech models, keeping recordings, the reminder. Each
//! list is defined here once; the store keeps its limits and defaults to them, the profile
//! view passes them to the page.

/// The text sizes the settings offer (/LL202/).
pub const TEXT_SCALE_STEPS: [f32; 4] = [1.0, 1.25, 1.5, 1.6];

/// The daily goals the settings offer, in minutes (/LF602/).
pub const GOAL_STEPS: [u32; 5] = [3, 5, 10, 15, 20];

/// The idle times the settings offer for unloading the speech models, in minutes; `None`
/// keeps them (/LL012/).
pub const UNLOAD_STEPS: [Option<u32>; 4] = [Some(5), Some(10), Some(30), None];

/// The days the settings offer for keeping recordings; `None` keeps none (/LF712/).
pub const KEEP_STEPS: [Option<u32>; 3] = [None, Some(7), Some(30)];

/// The reminder times the settings offer, in minutes after midnight (8:00, 12:30, 19:00);
/// `None` is off.
pub const REMINDER_STEPS: [Option<u32>; 4] = [None, Some(480), Some(750), Some(1140)];

/// The monthly limits the settings offer for a cloud tutor, in cents; `None` is no limit
/// (/LF714/).
pub const COST_STEPS: [Option<u32>; 5] = [None, Some(200), Some(500), Some(1000), Some(2000)];
