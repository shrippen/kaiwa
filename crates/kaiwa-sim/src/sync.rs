//! Sync between devices through a folder (/LF700/, ADR-010), played through the backend.

use anyhow::{Context, Result};
use kaiwa_backend::flow::Talk;
use kaiwa_backend::sync::{Secret, Start};
use kaiwa_backend::{Backend, TempBackend};
use kaiwa_core::Input;
use kaiwa_core::clock::{Day, Moment};
use kaiwa_core::srs::{Item, ItemKind, Source};

use crate::profile::temp_dir;

const PASS: &str = "kirschbluete";
const SCENARIO: &str = "n5-02-jikoshoukai";
const WORDS: [&str; 2] = ["駅", "袋"];
const HOUR: i64 = 3_600;

/// The day Jonas practises, from 7 o'clock.
fn morning() -> Moment {
    Day::from_ymd(2026, 10, 5).start(120).plus_secs(7 * HOUR)
}

/// /LF700/, /LF507/ (`jonas_sync`): the PC syncs first, the phone joins with an empty
/// profile. In the morning the phone reviews a card and leaves a talk open after one turn;
/// at noon the PC (the widget's review) reviews the other card. Both sync through one
/// folder. In the evening the PC has both reviews of the day, the card as the phone left
/// it, the same goal and streak as the phone, and the phone's open talk, which goes on there.
pub fn devices() -> Result<Option<String>> {
    let folder = temp_dir("sync-folder");
    let pc = TempBackend::new("sync-pc", false)?;
    let phone = TempBackend::new("sync-phone", false)?;
    let t = morning();
    for w in WORDS {
        pc.seed()
            .put_item(&Item::new(w, ItemKind::Word, Source::NewWord, t))?;
    }
    // Both devices are set up as the settings' tile does it: the folder, the passphrase.
    for b in [&pc, &phone] {
        b.set_setting("syncFolder", &folder.to_string_lossy(), t)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        b.set_sync_secret(Secret::Passphrase, PASS)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    let sync = |b: &Backend, start: Start| b.sync_now(start, t).map_err(anyhow::Error::from);
    sync(&pc, Start::Here)?;
    sync(&phone, Start::Join)?;

    // Morning on the phone: a review, a talk left after one turn.
    phone.answer_review(WORDS[0], WORDS[0], false, t);
    let mut talk = crate::steps::opened(&phone, SCENARIO, t.plus_secs(60))?;
    let line = talk.session().goals()[0].help.example.clone();
    crate::steps::said(&mut talk, &phone, &line, Input::Spoken, t.plus_secs(120))?;
    drop(talk);
    sync(&phone, Start::Join)?;

    // Noon on the PC; then both sync.
    pc.answer_review(WORDS[1], WORDS[1], false, t.plus_secs(5 * HOUR));
    sync(&pc, Start::Here)?;
    sync(&phone, Start::Join)?;
    evening(&pc, &phone, t.plus_secs(12 * HOUR))
}

/// What the PC shows in the evening, against the phone.
fn evening(pc: &Backend, phone: &Backend, now: Moment) -> Result<Option<String>> {
    let day = pc.seed().day(now.day())?;
    if day.reviews_done != 2 {
        return Ok(Some(format!(
            "{} reviews on the PC, not both devices' 2",
            day.reviews_done
        )));
    }
    let card = |b: &Backend| b.seed().item(WORDS[0]).map(|i| i.map(|i| i.reps));
    if card(pc)? != card(phone)? {
        return Ok(Some("the PC's card differs from the phone's".into()));
    }
    let (a, b) = (
        kaiwa_backend::views::today(pc, now)?,
        kaiwa_backend::views::today(phone, now)?,
    );
    for key in ["goalFraction", "streakDays", "xpToday", "due", "open"] {
        if a[key] != b[key] {
            return Ok(Some(format!("{key}: PC {} but phone {}", a[key], b[key])));
        }
    }

    // The phone's talk goes on on the PC.
    let Some(opened) = Talk::resume(pc, now)? else {
        return Ok(Some("the phone's open talk is not on the PC".into()));
    };
    let mut talk = opened.talk;
    if talk.session().turns().len() != 1 {
        return Ok(Some("the open talk lost the phone's turn".into()));
    }
    let line = talk.session().goals()[1].help.example.clone();
    crate::steps::said(&mut talk, pc, &line, Input::Spoken, now)
        .context("the talk does not go on on the PC")?;
    Ok(None)
}
