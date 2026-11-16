//! The one place where the simulation opens a talk on recorded tutor answers (AUDIT.md
//! finding 6): the scenario from `data/scenarios`, the answers from `qa/fixtures`, and the
//! backend's recorded tutor with its fallback reply.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use kaiwa_ai::mock::MockProvider;
use kaiwa_backend::seed::recorded_tutor;
use kaiwa_core::{Amount, Scenario, Session};

/// The scenario `id` from the data directory.
pub fn scenario(root: &Path, id: &str) -> Result<Scenario> {
    let path = root.join(format!("data/scenarios/{id}.json"));
    let json = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    Ok(Scenario::from_json(&json)?)
}

/// The recorded answers in `qa/fixtures/{name}.json`.
pub fn recorded(root: &Path, name: &str) -> Result<MockProvider> {
    let path = root.join(format!("qa/fixtures/{name}.json"));
    let json = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    recorded_tutor(&json)
}

/// A German talk on scenario `id` against its recorded answers.
pub fn open(root: &Path, id: &str, amount: Amount) -> Result<Session> {
    Ok(Session::new(
        scenario(root, id)?,
        Arc::new(recorded(root, id)?),
        amount,
        "de",
    ))
}
