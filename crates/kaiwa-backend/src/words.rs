//! What the learner reads when a command or a background step failed (or, for a few, worked),
//! in the UI language `lang` ("de", "en"). The error inside stays English: it is the detail
//! after the sentence. Speech has its own words (`ears::text`), keys `profiles::key_error`.

use std::path::PathBuf;

use kaiwa_ai::AiError;
use kaiwa_ai::secrets::KeyError;
use kaiwa_core::Lang;
use kaiwa_core::clock::Day;

use crate::Trouble;
use crate::profiles::key_error;
use crate::transfer::ImportError;

/// The profile was left as it is: it could not be read or written.
pub fn unreadable(e: &anyhow::Error, lang: Lang) -> String {
    match lang == Lang::De {
        true => format!("Das Profil ließ sich nicht ändern, nichts geändert: {e}"),
        false => format!("The profile could not be changed, nothing changed: {e}"),
    }
}

/// A save or load inside a talk failed.
pub fn unsaved(e: &str, lang: Lang) -> String {
    match lang == Lang::De {
        true => format!("Das Profil ließ sich nicht lesen oder schreiben: {e}"),
        false => format!("The profile could not be read or written: {e}"),
    }
}

/// Why a change is refused while the stand-in profile runs.
pub fn spare(lang: Lang) -> &'static str {
    match lang == Lang::De {
        true => {
            "Das Profil ließ sich nicht öffnen, das geht gerade nicht. Lösche es in den Einstellungen („Profil löschen …“) oder wähle ein anderes Profil."
        }
        false => {
            "The profile could not be opened, so this is not possible now. Delete it in the settings (“Delete profile …”) or pick another profile."
        }
    }
}

/// A setting is saved but what it sets off failed (today's goal, deleting recordings).
pub fn unapplied(e: &anyhow::Error, lang: Lang) -> String {
    match lang == Lang::De {
        true => format!("Gespeichert, aber nicht ganz umgesetzt: {e}"),
        false => format!("Saved, but not fully applied: {e}"),
    }
}

/// A page or the profile could not be loaded; what the page shows is empty or the defaults.
pub fn unloaded(e: &anyhow::Error, lang: Lang) -> String {
    match lang == Lang::De {
        true => format!("Das Profil ließ sich nicht vollständig laden: {e:#}"),
        false => format!("The profile could not be loaded completely: {e:#}"),
    }
}

/// A talk could not be opened because the profile could not be read.
pub fn talk_unopened(e: &anyhow::Error, lang: Lang) -> String {
    match lang == Lang::En {
        true => format!("The talk cannot be opened: {e}"),
        false => format!("Das Gespräch lässt sich nicht öffnen: {e}"),
    }
}

/// What went wrong in the background ([`crate::Backend::take_troubles`]).
pub fn trouble(t: &Trouble, lang: Lang) -> String {
    let de = lang == Lang::De;
    match (t, de) {
        (Trouble::Unopened(e), true) => format!(
            "Das Profil ließ sich nicht öffnen, die Datei bleibt unverändert. Alles, was jetzt geschieht, wird nicht gespeichert: {e:#}"
        ),
        (Trouble::Unopened(e), false) => format!(
            "The profile could not be opened; the file is left as it is. Nothing done now is saved: {e:#}"
        ),
        (Trouble::Unpurged(e), true) => {
            format!("Abgelaufene Aufnahmen ließen sich nicht löschen: {e}")
        }
        (Trouble::Unpurged(e), false) => format!("Expired recordings could not be deleted: {e}"),
        (Trouble::RecordingsKept(e), true) => {
            format!("Die Aufnahmen ließen sich nicht löschen: {e:#}")
        }
        (Trouble::RecordingsKept(e), false) => {
            format!("The recordings could not be deleted: {e:#}")
        }
        (Trouble::KeysKept(e), true) => {
            format!("Der Schlüsselbund behält Schlüssel dieses Profils: {e}")
        }
        (Trouble::KeysKept(e), false) => format!("The keyring kept keys of this profile: {e}"),
        (Trouble::KeysMarkerLost(e), true) => format!(
            "Die alten Schlüssel sind umgezogen, aber das Vermerken ist fehlgeschlagen; der nächste Start versucht es noch einmal: {e}"
        ),
        (Trouble::KeysMarkerLost(e), false) => {
            format!("The old keys moved, but noting it failed; the next start tries again: {e}")
        }
        (Trouble::TutorUnready(e), true) => format!("Der Tutor ist nicht bereit: {e}"),
        (Trouble::TutorUnready(e), false) => format!("The tutor is not ready: {e}"),
        (Trouble::KeysUnmoved(e), true) => {
            format!("Die alten Schlüssel blieben, wo sie waren: {e}")
        }
        (Trouble::KeysUnmoved(e), false) => {
            format!("The old keys stayed where they were: {e}")
        }
    }
}

/// The export's result.
pub fn exported(result: &anyhow::Result<PathBuf>, lang: Lang) -> String {
    match (result, lang == Lang::De) {
        (Ok(file), true) => format!("Exportiert nach {}.", file.display()),
        (Ok(file), false) => format!("Exported to {}.", file.display()),
        (Err(e), true) => format!("Der Export ist fehlgeschlagen: {e}"),
        (Err(e), false) => format!("The export failed: {e}"),
    }
}

/// The import's result; `file` is what the learner named.
pub fn imported(result: &Result<(), ImportError>, file: &str, lang: Lang) -> String {
    match (result, lang == Lang::De) {
        (Ok(()), true) => format!("Importiert aus {file}."),
        (Ok(()), false) => format!("Imported from {file}."),
        (Err(ImportError::Spare), _) => spare(lang).to_string(),
        (Err(ImportError::NotEmpty), true) => "Der Import geht nur in ein leeres Profil. Lege oben ein neues Profil an und importiere dort.".to_string(),
        (Err(ImportError::NotEmpty), false) => "An import needs an empty profile. Add a new profile above and import there.".to_string(),
        (Err(ImportError::NotAnExport), true) => format!("{file} ist kein Export von Kaiwa."),
        (Err(ImportError::NotAnExport), false) => format!("{file} is not an export of Kaiwa."),
        (Err(ImportError::Unreadable(e)), true) => format!("{file} ließ sich nicht lesen: {e}"),
        (Err(ImportError::Unreadable(e)), false) => format!("{file} could not be read: {e}"),
        (Err(ImportError::Broken(e)), true) => {
            format!("Der Import ist fehlgeschlagen, das Profil ist unverändert: {e}")
        }
        (Err(ImportError::Broken(e)), false) => {
            format!("The import failed, the profile is unchanged: {e}")
        }
    }
}

/// The result of deleting all data of the profile.
pub fn wiped(result: &anyhow::Result<()>, lang: Lang) -> String {
    match (result, lang == Lang::De) {
        (Ok(()), true) => "Alle Daten dieses Profils sind gelöscht.".to_string(),
        (Ok(()), false) => "All data of this profile is deleted.".to_string(),
        (Err(e), true) => format!("Das Löschen ist fehlgeschlagen: {e}"),
        (Err(e), false) => format!("Deleting failed: {e}"),
    }
}

/// The result of setting a pause; says until when, as nothing else on the page shows a pause.
/// With `streak`, also that the streak holds.
pub fn pause_set(result: &anyhow::Result<Day>, streak: bool, lang: Lang) -> String {
    let de = lang == Lang::De;
    let until = match result {
        Ok(until) => *until,
        Err(e) if de => return format!("Die Pause ließ sich nicht eintragen: {e}"),
        Err(e) => return format!("The pause could not be set: {e}"),
    };
    let (_, m, d) = until.ymd();
    match (de, streak) {
        (true, true) => {
            format!("Pause bis einschließlich {d}.{m}.: Die Serie bleibt so lange stehen.")
        }
        (true, false) => format!("Pause bis einschließlich {d}.{m}."),
        (false, true) => format!(
            "Paused up to and including {}: the streak holds until then.",
            until.iso()
        ),
        (false, false) => format!("Paused up to and including {}.", until.iso()),
    }
}

/// The result of ending a pause.
pub fn pause_ended(result: &anyhow::Result<()>, lang: Lang) -> String {
    match (result, lang == Lang::De) {
        (Ok(()), true) => "Die Pause ist beendet: Heute zählt wieder.".to_string(),
        (Ok(()), false) => "The pause is over: today counts again.".to_string(),
        (Err(e), true) => format!("Die Pause ließ sich nicht beenden: {e}"),
        (Err(e), false) => format!("The pause could not be ended: {e}"),
    }
}

/// The result of putting the tutor's key into the keyring.
pub fn key_stored(result: &Result<(), KeyError>, lang: Lang) -> String {
    match (result, lang == Lang::De) {
        (Ok(()), true) => "Der Schlüssel liegt im Schlüsselbund.".to_string(),
        (Ok(()), false) => "The key is in the keyring.".to_string(),
        (Err(e), _) => key_error(e, lang),
    }
}

/// The result of taking the tutor's key out of the keyring.
pub fn key_removed(result: &Result<(), KeyError>, lang: Lang) -> String {
    match (result, lang == Lang::De) {
        (Ok(()), true) => "Der Schlüssel ist aus dem Schlüsselbund entfernt.".to_string(),
        (Ok(()), false) => "The key is removed from the keyring.".to_string(),
        (Err(e), _) => key_error(e, lang),
    }
}

/// The tutor did not answer. The budget and a missing tutor have their own sentence and say
/// what to do.
pub fn tutor_failed(e: &AiError, lang: Lang) -> String {
    match (e, lang == Lang::En) {
        (AiError::NoTutor, true) => {
            "No tutor is set up. Choose one in the settings under Tutor.".to_string()
        }
        (AiError::NoTutor, false) => {
            "Es ist kein Tutor eingerichtet. In den Einstellungen unter „Tutor“ einen wählen."
                .to_string()
        }
        (AiError::Budget, true) => {
            "The cost limit for this month is reached. Raise it in the settings or switch to a local tutor.".to_string()
        }
        (AiError::Budget, false) => {
            "Das Kostenlimit für diesen Monat ist erreicht. In den Einstellungen anheben oder auf einen lokalen Tutor wechseln.".to_string()
        }
        (e, true) => format!("The tutor did not answer: {e}"),
        (e, false) => format!("Der Tutor hat nicht geantwortet: {e}"),
    }
}

/// The tutor could not translate a line.
pub fn translation_failed(e: &AiError, lang: Lang) -> String {
    match (e, lang == Lang::En) {
        (AiError::NoFixture, true) => {
            "The recorded answers have no translation for this line.".to_string()
        }
        (AiError::NoFixture, false) => {
            "Für diese Zeile haben die aufgezeichneten Antworten keine Übersetzung.".to_string()
        }
        (e, true) => format!("No translation right now: {e}"),
        (e, false) => format!("Gerade keine Übersetzung: {e}"),
    }
}

/// A recording had no voice in it.
pub fn no_sound(lang: Lang) -> &'static str {
    match lang == Lang::En {
        true => {
            "No sound came in. Is a microphone plugged in, switched on and chosen? Settings › Microphone tests it."
        }
        false => {
            "Es kam kein Ton an. Ist ein Mikrofon angeschlossen, eingeschaltet und gewählt? Einstellungen › Mikrofon testet es."
        }
    }
}

/// The recognition heard nothing in a recording.
pub fn not_understood(lang: Lang) -> &'static str {
    match lang == Lang::En {
        true => "Nothing understood. Once more, a little louder or closer to the microphone.",
        false => "Nichts verstanden. Noch einmal, etwas lauter oder näher am Mikrofon.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure() -> anyhow::Error {
        anyhow::anyhow!("disk full")
    }

    #[test]
    fn a_profile_failure_says_what_happened_in_both_languages() {
        assert_eq!(
            unreadable(&failure(), Lang::De),
            "Das Profil ließ sich nicht ändern, nichts geändert: disk full"
        );
        assert_eq!(
            unreadable(&failure(), Lang::En),
            "The profile could not be changed, nothing changed: disk full"
        );
        assert!(unapplied(&failure(), Lang::De).starts_with("Gespeichert, aber"));
        assert!(unapplied(&failure(), Lang::En).starts_with("Saved, but"));
        assert!(unsaved("x", Lang::De).contains("lesen oder schreiben"));
        assert!(unsaved("x", Lang::En).contains("read or written"));
    }

    #[test]
    fn a_trouble_has_its_sentence_with_the_detail() {
        let t = Trouble::KeysUnmoved("locked".into());
        assert_eq!(
            trouble(&t, Lang::De),
            "Die alten Schlüssel blieben, wo sie waren: locked"
        );
        assert_eq!(
            trouble(&t, Lang::En),
            "The old keys stayed where they were: locked"
        );
    }

    #[test]
    fn an_unready_tutor_says_what_is_missing() {
        let t = Trouble::TutorUnready("needs a key".into());
        assert_eq!(
            trouble(&t, Lang::De),
            "Der Tutor ist nicht bereit: needs a key"
        );
        assert_eq!(trouble(&t, Lang::En), "The tutor is not ready: needs a key");
    }

    #[test]
    fn the_tutor_failures_are_told_by_kind_not_by_text() {
        assert!(tutor_failed(&AiError::Budget, Lang::En).starts_with("The cost limit"));
        assert!(tutor_failed(&AiError::Budget, Lang::De).starts_with("Das Kostenlimit"));
        assert!(tutor_failed(&AiError::NoTutor, Lang::En).starts_with("No tutor"));
        assert!(tutor_failed(&AiError::NoTutor, Lang::De).starts_with("Es ist kein Tutor"));
        let net = AiError::Network("down".into());
        assert_eq!(
            tutor_failed(&net, Lang::En),
            "The tutor did not answer: network: down"
        );
        assert_eq!(
            tutor_failed(&net, Lang::De),
            "Der Tutor hat nicht geantwortet: network: down"
        );
        assert!(translation_failed(&AiError::NoFixture, Lang::En).contains("no translation"));
        assert!(translation_failed(&net, Lang::De).starts_with("Gerade keine Übersetzung"));
    }

    #[test]
    fn an_import_names_the_file_and_the_reason() {
        let bad: Result<(), ImportError> = Err(ImportError::NotAnExport);
        assert_eq!(
            imported(&bad, "a.json", Lang::De),
            "a.json ist kein Export von Kaiwa."
        );
        assert_eq!(
            imported(&bad, "a.json", Lang::En),
            "a.json is not an export of Kaiwa."
        );
        let spare_refused = Err(ImportError::Spare);
        assert_eq!(imported(&spare_refused, "a", Lang::En), spare(Lang::En));
        assert_eq!(
            imported(&Ok(()), "a.json", Lang::En),
            "Imported from a.json."
        );
    }

    #[test]
    fn a_pause_says_until_when_and_whether_the_streak_holds() {
        let until: anyhow::Result<Day> = Ok(Day(20_000));
        let (_, m, d) = Day(20_000).ymd();
        assert_eq!(
            pause_set(&until, true, Lang::De),
            format!("Pause bis einschließlich {d}.{m}.: Die Serie bleibt so lange stehen.")
        );
        assert_eq!(
            pause_set(&until, false, Lang::En),
            format!("Paused up to and including {}.", Day(20_000).iso())
        );
        assert_eq!(
            pause_set(&Err(failure()), true, Lang::En),
            "The pause could not be set: disk full"
        );
    }

    #[test]
    fn a_result_of_an_action_has_both_outcomes() {
        assert_eq!(
            wiped(&Ok(()), Lang::En),
            "All data of this profile is deleted."
        );
        assert_eq!(
            wiped(&Err(failure()), Lang::De),
            "Das Löschen ist fehlgeschlagen: disk full"
        );
        assert_eq!(
            pause_ended(&Ok(()), Lang::De),
            "Die Pause ist beendet: Heute zählt wieder."
        );
        assert_eq!(key_stored(&Ok(()), Lang::En), "The key is in the keyring.");
        assert_eq!(
            key_removed(&Ok(()), Lang::De),
            "Der Schlüssel ist aus dem Schlüsselbund entfernt."
        );
        assert_eq!(
            exported(&Ok(PathBuf::from("/x/a.json")), Lang::En),
            "Exported to /x/a.json."
        );
    }
}
