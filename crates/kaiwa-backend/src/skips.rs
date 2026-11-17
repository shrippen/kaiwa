//! The skip commands (/LF215/): the offer on the path and the offer after the placement, and
//! taking them. The rule for keigo is `LearnerModel::may_skip`.

use kaiwa_core::Event;
use kaiwa_core::curriculum::NodeState;
use kaiwa_core::event::SkipOffer;

use crate::{Backend, journal};

/// Whether the scenario `id` is in keigo.
fn has_keigo(st: &Backend, id: &str) -> bool {
    st.scenarios
        .get(id)
        .is_some_and(|s| s.register == kaiwa_core::scenario::Register::Keigo)
}

/// The unit the skip offer is about (/LF215/, T-08), if there is an offer: three sessions in a
/// row went almost without a slip since the last skip, and the unit the learner is in now is
/// not one with keigo while the learner's register lacks it (`LearnerModel::may_skip`, as after
/// the placement).
pub fn skip_offer(st: &Backend) -> anyhow::Result<Option<kaiwa_core::curriculum::UnitNode>> {
    let l = st.store.learning()?;
    if !l.regulator.offer_skip() {
        return Ok(None);
    }
    let lang = st.store.profile()?.ui_lang;
    let Some(unit) = st
        .curriculum
        .path(&l.course, lang)
        .into_iter()
        .find(|u| u.state == NodeState::Current)
    else {
        return Ok(None);
    };
    let keigo = || unit.scenarios.iter().any(|s| has_keigo(st, &s.id));
    Ok(l.model.may_skip(keigo).then_some(unit))
}

/// Takes the skip offer for `unit` (the button at the path's hint): the unit counts as done and
/// the offer is used up, the next one needs three new sessions. Nothing happens when the offer
/// is not about `unit` (a second click, an old page).
pub fn take_skip(st: &Backend, unit: &str) -> anyhow::Result<bool> {
    if skip_offer(st)?.is_none_or(|u| u.id != unit) {
        return Ok(false);
    }
    let event = Event::Skipped {
        units: vec![unit.to_string()],
        offer: SkipOffer::Unit,
    };
    journal::record(st, &event, crate::now())?;
    Ok(true)
}

/// What a journaled skip does: its units count as done; the path's offer is used up.
pub(crate) fn apply(st: &Backend, units: &[String], offer: SkipOffer) -> anyhow::Result<()> {
    let mut l = st.store.learning()?;
    l.course.skipped_units.extend(units.iter().cloned());
    if offer == SkipOffer::Unit {
        l.regulator.took_skip();
    }
    st.store.set_learning(&l)?;
    Ok(())
}

/// The skip offered after the placement (/LF215/): the units up to the first keigo unit the
/// learner's register lacks, and the title of the last. No units, no offer.
#[derive(Debug, Default)]
pub struct PlacementOffer {
    pub units: Vec<String>,
    pub title: String,
}

/// What the placement lets the learner skip (`Curriculum::placement_skip`).
pub fn placement_offer(st: &Backend) -> anyhow::Result<PlacementOffer> {
    let l = st.store.learning()?;
    let keigo = |u: &kaiwa_core::curriculum::Unit| u.scenarios.iter().any(|s| has_keigo(st, s));
    let units = st.curriculum.placement_skip(&l.model, &l.course, keigo);
    let Some(last) = units.last().and_then(|id| st.curriculum.unit(id)) else {
        return Ok(PlacementOffer::default());
    };
    let title = last.title.get(st.store.profile()?.ui_lang).to_string();
    Ok(PlacementOffer { units, title })
}

/// Takes the placement offer ending at unit `last`: its units count as done. Nothing happens
/// when the offer ends elsewhere or is gone (a second click, an old page).
pub fn take_placement_skip(st: &Backend, last: &str) -> anyhow::Result<bool> {
    let offer = placement_offer(st)?;
    if offer.units.last().is_none_or(|u| u != last) {
        return Ok(false);
    }
    let event = Event::Skipped {
        units: offer.units,
        offer: SkipOffer::Placement,
    };
    journal::record(st, &event, crate::now())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::{learner, profile};
    use kaiwa_core::Lang;

    #[test]
    fn the_placement_offer_is_taken_once_and_not_for_a_beginner() {
        use kaiwa_core::LearnerModel;
        use kaiwa_core::learner::Skill;
        let b = crate::TempBackend::new("placement-offer", false).unwrap();
        assert!(placement_offer(&b).unwrap().units.is_empty());

        let mut l = b.store.learning().unwrap();
        l.model = LearnerModel::placed(&[
            (Skill::Speaking, "n3"),
            (Skill::Listening, "n3"),
            (Skill::Register, "plain"),
        ]);
        b.store.set_learning(&l).unwrap();
        let offer = placement_offer(&b).unwrap();
        let last = offer.units.last().unwrap().clone();
        assert!(!offer.title.is_empty());
        assert!(!take_placement_skip(&b, &b.curriculum.units[0].id).unwrap());

        assert!(take_placement_skip(&b, &last).unwrap());
        assert!(!take_placement_skip(&b, &last).unwrap());
        let skipped = b.store.learning().unwrap().course.skipped_units;
        assert_eq!(skipped.len(), offer.units.len());
        assert!(placement_offer(&b).unwrap().units.is_empty());
    }

    #[test]
    fn the_skip_offer_names_the_unit_and_the_settings_a_running_pause() {
        let b = crate::TempBackend::new("promises", false).unwrap();
        let first = b.curriculum.units[0].clone();
        // No offer without three easy talks.
        let v = learner(&b).unwrap();
        assert_eq!(v["offerSkip"], false);
        assert!(v["skipUnit"].is_null());
        assert!(!take_skip(&b, &first.id).unwrap());
        let easy = |n: usize| {
            let mut l = b.store.learning().unwrap();
            l.regulator.success.extend(vec![0.97; n]);
            b.store.set_learning(&l).unwrap();
        };
        easy(3);
        let v = learner(&b).unwrap();
        assert_eq!(v["offerSkip"], true);
        assert_eq!(v["skipUnit"]["id"], first.id.as_str());
        assert_eq!(v["skipUnit"]["title"], first.title.get(Lang::De));
        // Not the unit of the offer: nothing happens.
        assert!(!take_skip(&b, &b.curriculum.units[1].id).unwrap());
        // Taken (what the button does): one unit, and the offer is used up; a second click
        // skips nothing.
        assert!(take_skip(&b, &first.id).unwrap());
        assert!(!take_skip(&b, &first.id).unwrap());
        assert!(!take_skip(&b, &b.curriculum.units[1].id).unwrap());
        let v = learner(&b).unwrap();
        assert_eq!(v["offerSkip"], false);
        assert!(v["skipUnit"].is_null());
        let done = b.store.learning().unwrap().course.skipped_units;
        assert_eq!(done.len(), 1);
        assert!(done.contains(&first.id));
        // Three new easy talks, not two, bring the next offer.
        easy(2);
        assert_eq!(learner(&b).unwrap()["offerSkip"], false);
        easy(1);
        let v = learner(&b).unwrap();
        assert_eq!(v["skipUnit"]["id"], b.curriculum.units[1].id.as_str());
        // A unit with keigo is not offered to a learner without the register.
        let keigo = b
            .curriculum
            .units
            .iter()
            .position(|u| {
                u.scenarios
                    .iter()
                    .any(|s| b.scenarios[s].register == kaiwa_core::scenario::Register::Keigo)
            })
            .unwrap();
        let mut l = b.store.learning().unwrap();
        for u in &b.curriculum.units[..keigo] {
            l.course.skipped_units.insert(u.id.clone());
        }
        b.store.set_learning(&l).unwrap();
        assert!(b.store.learning().unwrap().regulator.offer_skip());
        let v = learner(&b).unwrap();
        assert_eq!(v["offerSkip"], false);
        assert!(!take_skip(&b, &b.curriculum.units[keigo].id).unwrap());
        let mut l = b.store.learning().unwrap();
        l.course.skipped_units.clear();
        l.regulator = Default::default();
        b.store.set_learning(&l).unwrap();

        assert!(profile(&b).unwrap()["pausedUntil"].is_null());
        let today = crate::now().day();
        b.store
            .add_pause(kaiwa_core::progress::Pause {
                from: today,
                to: today.plus(2),
            })
            .unwrap();
        assert_eq!(
            profile(&b).unwrap()["pausedUntil"]["iso"],
            today.plus(2).iso()
        );
        b.store.end_pause(today).unwrap();
        assert!(profile(&b).unwrap()["pausedUntil"].is_null());
    }
}
