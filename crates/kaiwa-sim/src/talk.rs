//! Where the simulation starts a talk on recorded tutor answers (AUDIT.md finding 6): the
//! scenario and the recorded tutor come from `kaiwa-backend`, the same ones the demo build
//! uses, and every such talk is made by [`start`]. Talks with another tutor (`days.rs`, the
//! free talk in `engine.rs`) do not come through here.

use std::sync::Arc;

use anyhow::{Context, Result};
use kaiwa_ai::ChatProvider;
use kaiwa_ai::mock::MockProvider;
use kaiwa_core::Lang;
use kaiwa_core::{Amount, Scenario, Session};

/// The scenario `id`, as the backend ships it.
pub fn scenario(id: &str) -> Result<Scenario> {
    kaiwa_backend::bundled_scenario(id).with_context(|| format!("no scenario {id}"))
}

/// The recorded answers of scenario `id`, with the backend's fallback reply.
pub fn recorded(id: &str) -> MockProvider {
    kaiwa_backend::recorded_tutor(id)
}

/// A talk on `scenario` with `provider`.
pub fn start(
    scenario: Scenario,
    provider: Arc<dyn ChatProvider>,
    amount: Amount,
    lang: Lang,
) -> Session {
    let unit_jlpt = kaiwa_backend::bundled_unit_jlpt(&scenario.id);
    Session::new(scenario, provider, amount, lang).with_unit_jlpt(unit_jlpt)
}

/// A talk on `scenario` as the backend starts it, on the profile of `persona`: the amount of
/// corrections is her choice (the placement keeps it), then come her levels. The pair is the
/// amount and the language she chose.
pub fn for_persona(
    b: &kaiwa_backend::Backend,
    persona: &serde_json::Value,
    scenario: Scenario,
    provider: Arc<dyn ChatProvider>,
    (amount, lang): (Amount, Lang),
) -> Result<Session> {
    let profile = kaiwa_store::Profile {
        ui_lang: lang,
        ..b.seed().set_feedback(amount)?
    };
    b.seed().set_profile(&profile)?;
    b.set_placement(crate::engine::simulate_placement(persona).result())?;
    b.seed().talk_with(scenario, provider)
}

/// A German talk on scenario `id` against its recorded answers.
pub fn open(id: &str, amount: Amount) -> Result<Session> {
    Ok(start(
        scenario(id)?,
        Arc::new(recorded(id)),
        amount,
        Lang::De,
    ))
}
