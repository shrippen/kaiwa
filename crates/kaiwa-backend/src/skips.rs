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
        .path(&l.course, &lang)
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
    let title = last.title.get(&st.store.profile()?.ui_lang).to_string();
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
}
