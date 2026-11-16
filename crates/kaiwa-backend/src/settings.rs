//! The settings and the small profile actions (Lastenheft /LF202/, /LF602/, /LF712/): the
//! limits of each setting and what a change sets off live here, so no front end repeats them.
//! An unknown key or a value that does not parse leaves the profile as it is.

use kaiwa_core::clock::Moment;
use kaiwa_core::learner::Skill;
use kaiwa_core::progress::DailyGoal;
use kaiwa_store::{Profile, Script};
use serde_json::json;

use crate::Backend;

const GOAL_MIN_MINUTES: u32 = 1;
const GOAL_MAX_MINUTES: u32 = 120;
/// The highest style: 0 system, 1 Kante, 2 Kante Light.
const STYLE_MAX: u8 = 2;
const LANG_EN: &str = "en";
const LANG_DE: &str = "de";

/// The text sizes as the page gets them: f32 1.6 would become 1.600000023841858 in JSON, so
/// they are rounded to hundredths like the profile's own `textScale`.
pub(crate) fn text_scale_steps() -> Vec<f64> {
    kaiwa_store::TEXT_SCALE_STEPS
        .iter()
        .map(|s| (f64::from(*s) * 100.0).round() / 100.0)
        .collect()
}

/// The "yes" a switch is stored with.
fn on(value: &str) -> bool {
    value == "true"
}

/// The pick of the script: "1" kana only, "2" hidden, anything else furigana.
fn script_of(value: &str) -> Script {
    match value {
        "1" => Script::Kana,
        "2" => Script::Hidden,
        _ => Script::Furigana,
    }
}

/// A new provider starts without the model and the server of the old one, so no server of
/// the old one is used by mistake; an unknown preset means none.
fn set_provider(p: &mut Profile, value: &str) {
    let id = kaiwa_ai::presets::preset(value)
        .map(|p| p.id.to_string())
        .unwrap_or_default();
    if id == p.tutor.provider {
        return;
    }
    p.tutor = kaiwa_ai::Tutor {
        provider: id,
        ..Default::default()
    };
}

/// How the interface looks and what it shows. `false` when `key` is not one of these.
fn apply_look(p: &mut Profile, key: &str, value: &str) -> bool {
    match key {
        "gamification" => p.gamification = on(value),
        "moraHints" => p.mora_hints = on(value),
        "handsFree" => p.hands_free = on(value),
        "script" => p.script = script_of(value),
        "style" => p.style = value.parse().unwrap_or(0).min(STYLE_MAX),
        "uiLang" => p.ui_lang = if value == LANG_EN { LANG_EN } else { LANG_DE }.into(),
        "name" => p.name = value.to_string(),
        "textScale" => {
            if let Ok(s) = value.parse::<f32>() {
                p.text_scale = s.clamp(
                    kaiwa_store::TEXT_SCALE_STEPS[0],
                    kaiwa_store::TEXT_SCALE_MAX,
                );
            }
        }
        _ => return false,
    }
    true
}

/// Practice, timing, privacy and the tutor; anything else is ignored.
fn apply_rest(p: &mut Profile, key: &str, value: &str) {
    match key {
        "goal" => {
            if let Ok(m) = value.parse::<u32>() {
                p.daily_goal = DailyGoal::Minutes(m.clamp(GOAL_MIN_MINUTES, GOAL_MAX_MINUTES));
            }
        }
        "endPause" => {
            if let Ok(s) = value.parse::<f32>() {
                p.end_pause_secs = kaiwa_core::turn_end::clamp_pause(s);
            }
        }
        "reminder" => p.reminder = value.parse().ok(),
        "unloadAfter" => p.unload_after_min = value.parse().ok(),
        "keepRecordings" => p.keep_recordings_days = value.parse().ok(),
        "tutorProvider" => set_provider(p, value),
        // A device id from the microphone list; "" is the default.
        "microphone" => p.microphone = (!value.is_empty()).then(|| value.to_string()),
        "tutorModel" => p.tutor.model = value.trim().to_string(),
        "tutorBase" => p.tutor.base_url = value.trim().to_string(),
        _ => {}
    }
}

/// How a skill felt to the learner, for [`Backend::nudge`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Felt {
    TooEasy,
    TooHard,
}

/// Why a setting did not fully take.
#[derive(Debug)]
pub enum SettingError {
    /// The profile could not be read or written: the setting is as it was.
    NotSaved(anyhow::Error),
    /// The setting is saved, but what it sets off failed (today's goal, deleting recordings).
    NotApplied(anyhow::Error),
}

impl Backend {
    /// Changes one setting: goal, gamification, feedback, script, endPause, style, uiLang,
    /// reminder, unloadAfter, costLimit, moraHints, handsFree, textScale, keepRecordings,
    /// name, tutorProvider (a preset id), tutorModel, tutorBase, microphone.
    pub fn set_setting(&self, key: &str, value: &str, now: Moment) -> Result<(), SettingError> {
        match key {
            // The brake follows the new limit at once (/LF714/).
            "costLimit" => {
                self.set_cost_limit(value.parse().ok(), now)?;
            }
            // The pick is the learner's: a placement leaves it (V-02).
            "feedback" => {
                if let Ok(a) = serde_json::from_value(json!(value)) {
                    self.store
                        .set_feedback(a)
                        .map_err(|e| SettingError::NotSaved(e.into()))?;
                }
            }
            _ => self.edit_profile(key, value, now)?,
        }
        Ok(())
    }

    fn edit_profile(&self, key: &str, value: &str, now: Moment) -> Result<(), SettingError> {
        let p = self
            .store
            .update_profile(|p| {
                if !apply_look(p, key, value) {
                    apply_rest(p, key, value);
                }
            })
            .map_err(|e| SettingError::NotSaved(e.into()))?;
        let follow = |r: anyhow::Result<()>| r.map_err(SettingError::NotApplied);

        // A day already begun is judged by the goal it ends with (/LF602/).
        if key == "goal" {
            follow(
                self.store
                    .set_day_goal(now.day(), p.daily_goal)
                    .map_err(Into::into),
            )?;
        }

        // Turning recordings off (or shortening the days) deletes them now (/LF712/).
        if key == "keepRecordings" {
            follow(self.purge_recordings(now).map(|_| ()))?;
        }
        Ok(())
    }

    /// "I know this already" or "too hard" for a skill (/LF202/); an unknown skill changes
    /// nothing.
    pub fn nudge(&self, skill: &str, felt: Felt) -> anyhow::Result<()> {
        let up = felt == Felt::TooEasy;
        let Ok(skill) = serde_json::from_value::<Skill>(json!(skill)) else {
            return Ok(());
        };
        let mut l = self.store.learning()?;

        // "Too easy" for listening lifts the speed one step at once (Selin, /LF202/).
        let (model, regulator) = (&mut l.model, &mut l.regulator);
        kaiwa_core::difficulty::learner_says(model, regulator, skill, up);
        self.store.set_learning(&l)?;
        Ok(())
    }

    /// The profile as stored; the front ends read it here.
    pub fn profile(&self) -> anyhow::Result<Profile> {
        Ok(self.store.profile()?)
    }

    /// The text size the interface starts with: the stored one, never above the maximum.
    pub fn text_scale(&self) -> f32 {
        self.store
            .profile()
            .map(|p| p.text_scale.min(kaiwa_store::TEXT_SCALE_MAX))
            .unwrap_or(1.0)
    }

    /// The learner's pick of "louder" and "clearer" for the tutor's voice.
    pub fn set_sound(&self, louder: bool, clear: bool) -> anyhow::Result<()> {
        self.store.update_profile(|p| {
            p.louder = louder;
            p.clear = clear;
        })?;
        Ok(())
    }

    /// Takes the pause the app offered after a cut-off turn (/LF101/): it only ever makes
    /// the end-of-turn pause longer, and stays within the limits of the setting.
    pub fn take_longer_pause(&self, secs: f32) -> anyhow::Result<()> {
        let secs = kaiwa_core::turn_end::clamp_pause(secs);
        if secs <= self.store.profile()?.end_pause_secs {
            return Ok(());
        }
        self.store.update_profile(|p| p.end_pause_secs = secs)?;
        Ok(())
    }

    /// Takes the levels a placement measured (/LF200/); mistakes and pronunciation stay.
    pub fn set_placement(&self, model: kaiwa_core::learner::LearnerModel) -> anyhow::Result<()> {
        Ok(self.store.set_placement(model)?)
    }

    /// The learner leaves the placement out (/LF200/).
    pub fn leave_out_placement(&self) -> anyhow::Result<()> {
        self.store.update_profile(|p| p.placed = true)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use kaiwa_core::Amount;
    use kaiwa_core::difficulty::Knob;
    use kaiwa_core::progress::DayRecord;
    use kaiwa_store::FeedbackSource;

    use super::*;
    use crate::TempBackend;

    const T0: Moment = Moment {
        unix: 1_791_100_000,
        offset_min: 120,
    };

    fn backend(name: &str) -> TempBackend {
        TempBackend::new(&format!("set-{name}"), false).unwrap()
    }

    fn set(b: &Backend, key: &str, value: &str) -> Profile {
        b.set_setting(key, value, T0).unwrap();
        b.store.profile().unwrap()
    }

    #[test]
    fn the_goal_stays_between_one_and_120_minutes() {
        let b = backend("goal");
        assert_eq!(set(&b, "goal", "0").daily_goal, DailyGoal::Minutes(1));
        assert_eq!(set(&b, "goal", "999").daily_goal, DailyGoal::Minutes(120));
        assert_eq!(set(&b, "goal", "20").daily_goal, DailyGoal::Minutes(20));
        assert_eq!(set(&b, "goal", "x").daily_goal, DailyGoal::Minutes(20));
    }

    #[test]
    fn a_new_goal_applies_to_today_and_later_days_only() {
        let b = backend("goal-today");
        let today = T0.day();
        let yesterday = T0.plus_secs(-86_400).day();
        for d in [yesterday, today] {
            b.store.put_day(d, &DayRecord::default()).unwrap();
        }
        set(&b, "goal", "30");
        let days = b.store.days().unwrap();
        assert_eq!(days[&today].goal, Some(DailyGoal::Minutes(30)));
        assert_ne!(days[&yesterday].goal, Some(DailyGoal::Minutes(30)));
    }

    #[test]
    fn text_scale_and_style_keep_their_limits() {
        let b = backend("limits");
        assert_eq!(set(&b, "textScale", "0.2").text_scale, 1.0);
        assert_eq!(
            set(&b, "textScale", "9").text_scale,
            kaiwa_store::TEXT_SCALE_MAX
        );
        assert_eq!(set(&b, "style", "7").style, 2);
        assert_eq!(set(&b, "style", "1").style, 1);
        assert_eq!(set(&b, "style", "x").style, 0);
    }

    #[test]
    fn the_profile_view_carries_the_steps_and_the_limits_match_them() {
        let b = backend("steps");
        let v = crate::views::profile(&b).unwrap();
        assert_eq!(v["textScaleSteps"], json!([1.0, 1.25, 1.5, 1.6]));
        assert_eq!(v["goalSteps"], json!([3, 5, 10, 15, 20]));
        assert_eq!(v["unloadSteps"], json!([5, 10, 30, null]));

        // The ends of the lists are the limits.
        let scales = v["textScaleSteps"].as_array().unwrap();
        let (first, last) = (&scales[0], &scales[scales.len() - 1]);
        assert_eq!(
            set(&b, "textScale", "0.2").text_scale,
            first.as_f64().unwrap() as f32
        );
        assert_eq!(
            set(&b, "textScale", "9").text_scale,
            last.as_f64().unwrap() as f32
        );
        assert_eq!(set(&b, "goal", "3").daily_goal, DailyGoal::Minutes(3));
        let goals = kaiwa_core::choices::GOAL_STEPS;
        assert!(
            goals
                .iter()
                .all(|g| (GOAL_MIN_MINUTES..=GOAL_MAX_MINUTES).contains(g))
        );

        // A new profile starts on a step, and the view shows it.
        assert!(kaiwa_store::UNLOAD_STEPS.contains(&kaiwa_store::DEFAULT_UNLOAD));
        assert_eq!(v["unloadAfter"], json!(kaiwa_store::DEFAULT_UNLOAD));
        assert!(goals.contains(&5) && v["goal"]["value"] == json!(5));
    }

    #[test]
    fn the_end_pause_is_clamped_to_the_steps() {
        let b = backend("pause");
        let max = kaiwa_core::turn_end::clamp_pause(99.0);
        assert_eq!(set(&b, "endPause", "99").end_pause_secs, max);
        assert_eq!(
            set(&b, "endPause", "0.1").end_pause_secs,
            kaiwa_core::turn_end::clamp_pause(0.0)
        );
    }

    #[test]
    fn script_and_language_map_their_values() {
        let b = backend("map");
        assert_eq!(set(&b, "script", "1").script, Script::Kana);
        assert_eq!(set(&b, "script", "2").script, Script::Hidden);
        assert_eq!(set(&b, "script", "0").script, Script::Furigana);
        assert_eq!(set(&b, "uiLang", "en").ui_lang, "en");
        assert_eq!(set(&b, "uiLang", "fr").ui_lang, "de");
    }

    #[test]
    fn switches_are_on_only_for_true() {
        let b = backend("switch");
        assert!(set(&b, "gamification", "true").gamification);
        assert!(!set(&b, "gamification", "1").gamification);
        assert!(set(&b, "moraHints", "true").mora_hints);
        assert!(!set(&b, "moraHints", "false").mora_hints);
        assert!(set(&b, "handsFree", "true").hands_free);
    }

    #[test]
    fn optional_numbers_clear_on_garbage() {
        let b = backend("optional");
        assert_eq!(set(&b, "reminder", "480").reminder, Some(480));
        assert_eq!(set(&b, "reminder", "").reminder, None);
        assert_eq!(set(&b, "unloadAfter", "5").unload_after_min, Some(5));
        assert_eq!(set(&b, "unloadAfter", "never").unload_after_min, None);
        assert_eq!(
            set(&b, "microphone", "mic-1").microphone.as_deref(),
            Some("mic-1")
        );
        assert_eq!(set(&b, "microphone", "").microphone, None);
        assert_eq!(set(&b, "name", " Lena ").name, " Lena ");
    }

    #[test]
    fn feedback_is_the_learners_pick() {
        let b = backend("feedback");
        let p = set(&b, "feedback", "much");
        assert_eq!(p.feedback, Amount::Much);
        assert_eq!(p.feedback_source, FeedbackSource::Chosen);
        assert_eq!(set(&b, "feedback", "most").feedback, Amount::Much);
    }

    #[test]
    fn a_new_tutor_provider_starts_without_model_and_server() {
        let b = backend("tutor");
        set(&b, "tutorProvider", "custom");
        set(&b, "tutorModel", "  qwen3:14b ");
        let p = set(&b, "tutorBase", " http://localhost:11434/v1  ");
        assert_eq!(p.tutor.provider, "custom");
        assert_eq!(p.tutor.model, "qwen3:14b");
        assert_eq!(p.tutor.base_url, "http://localhost:11434/v1");

        // The same provider again keeps them.
        assert_eq!(set(&b, "tutorProvider", "custom").tutor.model, "qwen3:14b");

        // Another one drops them; an unknown preset means none.
        let p = set(&b, "tutorProvider", "ollama");
        assert_eq!(p.tutor.provider, "ollama");
        assert!(p.tutor.model.is_empty() && p.tutor.base_url.is_empty());
        set(&b, "tutorModel", "m");
        let p = set(&b, "tutorProvider", "no-such-preset");
        assert_eq!(p.tutor, kaiwa_ai::Tutor::default());
    }

    #[test]
    fn an_unknown_key_changes_nothing() {
        let b = backend("unknown");
        let before = b.store.profile().unwrap();
        b.set_setting("colour", "red", T0).unwrap();
        assert_eq!(
            serde_json::to_value(&before).unwrap(),
            serde_json::to_value(b.store.profile().unwrap()).unwrap()
        );
    }

    #[test]
    fn turning_recordings_off_deletes_them_at_once() {
        let b = backend("rec");
        set(&b, "keepRecordings", "30");
        let f = b
            .keep_recording(1, 0, &[0.1; 1600], 16_000, T0)
            .unwrap()
            .unwrap();
        set(&b, "name", "Lena");
        assert!(f.exists(), "other settings leave them");
        set(&b, "keepRecordings", "");
        assert!(!f.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_effect_keeps_the_saved_setting() {
        // The recordings cannot be deleted (their folder is read-only): the setting is
        // saved all the same, and the error says only the effect failed.
        use std::os::unix::fs::PermissionsExt;
        let b = backend("rec-locked");
        set(&b, "keepRecordings", "30");
        let f = b
            .keep_recording(1, 0, &[0.1; 1600], 16_000, T0)
            .unwrap()
            .unwrap();
        let dir = f.parent().unwrap().to_path_buf();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        // Root may delete there anyway: then there is nothing to show.
        if std::fs::write(dir.join("probe"), b"").is_ok() {
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
            return;
        }
        let r = b.set_setting("keepRecordings", "", T0);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(r, Err(SettingError::NotApplied(_))), "{r:?}");
        assert_eq!(b.store.profile().unwrap().keep_recordings_days, None);
    }

    #[test]
    fn the_cost_limit_is_stored() {
        let b = backend("cost");
        assert_eq!(set(&b, "costLimit", "500").cost_limit_cents, Some(500));
        assert_eq!(set(&b, "costLimit", "").cost_limit_cents, None);
    }

    #[test]
    fn too_easy_lifts_the_skill_and_the_speed() {
        let b = backend("nudge");
        let before = b.store.learning().unwrap();
        b.nudge("listening", Felt::TooEasy).unwrap();
        let after = b.store.learning().unwrap();
        assert!(after.model.level(Skill::Listening) > before.model.level(Skill::Listening));
        assert_eq!(
            after.regulator.difficulty.get(Knob::Speed),
            before.regulator.difficulty.get(Knob::Speed) + 1
        );
        assert_eq!(after.model.corrections_by_learner, 1);

        // An unknown skill is ignored.
        b.nudge("juggling", Felt::TooEasy).unwrap();
        assert_eq!(b.store.learning().unwrap().model.corrections_by_learner, 1);
    }

    #[test]
    fn skipping_the_placement_marks_it_done() {
        let b = backend("placed");
        assert!(!b.store.profile().unwrap().placed);
        b.leave_out_placement().unwrap();
        assert!(b.store.profile().unwrap().placed);
    }

    #[test]
    fn a_longer_pause_is_taken_a_shorter_one_is_not() {
        let b = backend("longer-pause");
        let start = b.profile().unwrap().end_pause_secs;

        b.take_longer_pause(start - 0.1).unwrap();
        assert_eq!(b.profile().unwrap().end_pause_secs, start);

        b.take_longer_pause(start + 0.5).unwrap();
        assert_eq!(b.profile().unwrap().end_pause_secs, start + 0.5);

        b.take_longer_pause(start).unwrap();
        assert_eq!(b.profile().unwrap().end_pause_secs, start + 0.5);
    }

    #[test]
    fn a_longer_pause_stays_within_the_limit() {
        let b = backend("longer-pause-limit");
        b.take_longer_pause(99.0).unwrap();
        let top = kaiwa_core::turn_end::clamp_pause(99.0);
        assert_eq!(b.profile().unwrap().end_pause_secs, top);
    }

    #[test]
    fn the_sound_is_stored() {
        let b = backend("sound");
        b.set_sound(true, false).unwrap();
        let p = b.profile().unwrap();
        assert!(p.louder && !p.clear);

        b.set_sound(false, true).unwrap();
        let p = b.profile().unwrap();
        assert!(!p.louder && p.clear);
    }

    #[test]
    fn the_placement_is_stored() {
        let b = backend("placement");
        let mut model = b.store.learning().unwrap().model;
        model.set(Skill::Speaking, 3.0);

        b.set_placement(model).unwrap();
        let stored = b.store.learning().unwrap().model.level(Skill::Speaking);
        assert_eq!(stored, 3.0);
    }

    #[test]
    fn the_profile_is_read_and_the_text_scale_is_capped() {
        let b = backend("profile-read");
        set(&b, "name", "Mika");
        assert_eq!(b.profile().unwrap().name, "Mika");

        b.store
            .update_profile(|p| p.text_scale = kaiwa_store::TEXT_SCALE_MAX + 1.0)
            .unwrap();
        assert_eq!(b.text_scale(), kaiwa_store::TEXT_SCALE_MAX);
    }
}
