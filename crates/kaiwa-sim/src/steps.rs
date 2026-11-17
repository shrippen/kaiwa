//! The steps of a journey, played through the backend's talk flow: help,
//! a turn and a translation run as in the app, on a talk the profile does not keep.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use kaiwa_backend::flow::Talk;
use kaiwa_core::Input;

use crate::run::{Event, Script, Step};

/// Fails when the flow could not keep or book something.
pub fn refuse(failed: Vec<anyhow::Error>) -> Result<()> {
    match failed.into_iter().next() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// The translation of the character's line `line`, counted as an aid on the current line.
pub fn translate(
    b: &kaiwa_backend::Backend,
    talk: &mut Talk,
    line: &str,
    now: kaiwa_core::Moment,
) -> Result<Result<String, String>> {
    let line_no = talk.session().line();
    let asked = talk
        .ask_translation(line, line_no)
        .context("translation asked while a turn is open")?;
    let translated = talk.take_translation(b, now, asked.run());
    refuse(translated.failed)?;
    Ok(translated.result.map_err(|e| e.to_string()))
}

/// What a step of a script led to.
pub enum Played {
    Event(Event),
    /// A live tutor failed on a turn: the run ends there.
    TutorFailed(String),
}

/// Plays one step through the backend's flow.
pub fn play_step(
    b: &kaiwa_backend::Backend,
    talk: &mut Talk,
    step: &Step,
    script: &Script,
    live: bool,
) -> Result<Played> {
    let now = kaiwa_backend::now();
    let event = match step {
        Step::Silence { silence } => Event::Silence {
            secs: *silence,
            help_offered: talk.session().offer_help(Duration::from_secs_f64(*silence)),
        },
        Step::Help { help } => {
            let helped = talk.help(b);
            refuse(helped.failed)?;
            let h = helped.step.context("help asked with every goal reached")?;
            if h.level != *help {
                bail!(
                    "{}: help reached level {}, the script expects {}",
                    script.journey,
                    h.level,
                    help
                );
            }
            Event::Help(h)
        }
        Step::Say { say, input, plants } => {
            let input = if input == "typed" {
                Input::Typed
            } else {
                Input::Spoken
            };
            let turned = match talk.say(b, say, input, now) {
                Ok(t) => t.with_context(|| format!("no turn for: {say}"))?,
                Err(e) if live => return Ok(Played::TutorFailed(format!("{e:#}"))),
                Err(e) => return Err(e),
            };
            refuse(turned.failed)?;
            Event::Turn {
                say: say.clone(),
                input,
                plants: plants.clone(),
                outcome: Box::new(turned.outcome),
            }
        }
    };
    Ok(Played::Event(event))
}
