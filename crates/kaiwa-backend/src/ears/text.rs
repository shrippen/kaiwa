//! What the learner reads when listening does not work, in the UI language.

use kaiwa_core::Lang;
use kaiwa_speech::SpeechError;
use kaiwa_speech::capture::Verdict;

/// When the pronunciation check failed.
pub enum MoraFailure {
    /// Its model could not be loaded.
    Load,
    /// It failed on a recording.
    Check,
}

/// Why there are no pronunciation hints, in the UI language; speaking goes on either way. The
/// error itself is English and stays in the log.
pub fn mora_text(when: MoraFailure, lang: Lang) -> &'static str {
    match (when, lang == Lang::De) {
        (MoraFailure::Check, true) => "Die Aussprachehinweise fehlen, die Prüfung ist ausgefallen.",
        (MoraFailure::Check, false) => "Pronunciation hints are missing, the check failed.",
        (MoraFailure::Load, true) => {
            "Die Aussprachehinweise fehlen, das Modell ist beschädigt. Lade es in den Einstellungen unter „Modelle“ neu herunter; Sprechen geht weiter."
        }
        (MoraFailure::Load, false) => {
            "Pronunciation hints are missing, the model is damaged. Download it again in the settings under “Models”; speaking still works."
        }
    }
}

/// What the learner reads when speaking does not work, in the UI language.
pub fn speech_text(e: &SpeechError, lang: Lang) -> String {
    let de = lang == Lang::De;
    match (e, de) {
        (SpeechError::NoMicrophone, true) => {
            "Kein Mikrofon gefunden. Bis dahin geht Tippen.".into()
        }
        (SpeechError::NoMicrophone, false) => "No microphone found. You can type meanwhile.".into(),
        (SpeechError::Microphone(why), true) => {
            format!("Das Mikrofon lässt sich nicht öffnen ({why}). Bis dahin geht Tippen.")
        }
        (SpeechError::Microphone(why), false) => {
            format!("The microphone cannot be opened ({why}). You can type meanwhile.")
        }
        (SpeechError::ModelMissing(dir), true) => {
            format!("Das Modell für die Spracherkennung fehlt in {dir}.")
        }
        (SpeechError::ModelMissing(dir), false) => format!("The speech model is missing in {dir}."),
        (SpeechError::ModelLoad(dir), true) => format!(
            "Das Modell in {dir} lässt sich nicht laden. Neu entpacken; bis dahin geht Tippen."
        ),
        (SpeechError::ModelLoad(dir), false) => {
            format!("The model in {dir} cannot be loaded. Unpack it again; you can type meanwhile.")
        }
        (SpeechError::NoSpeaker, true) => "Kein Lautsprecher gefunden.".into(),
        (SpeechError::NoSpeaker, false) => "No speaker found.".into(),
        (SpeechError::Speaker(why), true) => {
            format!("Der Lautsprecher lässt sich nicht öffnen ({why}).")
        }
        (SpeechError::Speaker(why), false) => format!("The speaker cannot be opened ({why})."),
        (SpeechError::Voice(why), true) => format!("Die Stimme ist ausgefallen ({why})."),
        (SpeechError::Voice(why), false) => format!("The voice failed ({why})."),
    }
}

/// Why the recognition model did not load for a talk; a missing model says where to get it.
pub fn recognizer_text(e: &SpeechError, lang: Lang) -> String {
    match (e, lang == Lang::De) {
        (SpeechError::ModelMissing(_), true) => "Das Modell für die Spracherkennung fehlt. ReazonSpeech lässt sich in den Einstellungen unter „Modelle“ laden. Bis dahin geht Tippen.".to_string(),
        (SpeechError::ModelMissing(_), false) => "The speech recognition model is missing. Download ReazonSpeech in the settings under “Models”. You can type meanwhile.".to_string(),
        (e, _) => speech_text(e, lang),
    }
}

/// Why the recognition could not check the microphone test's recording, and where to fix it
/// (Settings › Models).
pub fn test_text(e: &SpeechError, lang: Lang) -> String {
    match (e, lang == Lang::De) {
        (SpeechError::ModelLoad(dir), true) => format!(
            "ReazonSpeech lässt sich nicht laden. Den Ordner {dir} löschen, dann unter „Modelle“ neu laden."
        ),
        (SpeechError::ModelLoad(dir), false) => format!(
            "ReazonSpeech cannot be loaded. Delete the folder {dir}, then download it again under “Models”."
        ),
        (e, true) => format!("Die Spracherkennung lässt sich nicht starten ({e})."),
        (e, false) => format!("The recognition cannot start ({e})."),
    }
}

/// The verdict in words: what it means and what to do.
pub fn verdict_text(v: Verdict, lang: Lang) -> &'static str {
    match (v, lang == Lang::De) {
        (Verdict::Silent, true) => {
            "Es kommt nichts an. Ist das Mikrofon stummgeschaltet, das falsche gewählt oder hat Kaiwa keinen Zugriff darauf?"
        }
        (Verdict::Silent, false) => {
            "Nothing comes in. Is the microphone muted, the wrong one chosen, or does Kaiwa have no access to it?"
        }
        (Verdict::Quiet, true) => {
            "Zu leise für die Spracherkennung. Näher ans Mikrofon oder die Eingangslautstärke im System höher stellen."
        }
        (Verdict::Quiet, false) => {
            "Too quiet for the recognition. Move closer to the microphone or raise the input volume in the system."
        }
        (Verdict::Noisy, true) => {
            "Deine Stimme hebt sich kaum vom Hintergrund ab. Ein ruhigerer Ort oder ein Headset hilft."
        }
        (Verdict::Noisy, false) => {
            "Your voice hardly stands out from the background. A quieter place or a headset helps."
        }
        (Verdict::Clipping, true) => {
            "Zu laut, die Spitzen werden abgeschnitten. Etwas Abstand halten oder die Eingangslautstärke senken."
        }
        (Verdict::Clipping, false) => {
            "Too loud, the peaks are cut off. Keep a little distance or lower the input volume."
        }
        (Verdict::Good, true) => "Gut: laut genug und klar.",
        (Verdict::Good, false) => "Good: loud enough and clear.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_microphone_is_said_in_both_languages() {
        let e = SpeechError::NoMicrophone;
        assert_eq!(
            speech_text(&e, Lang::De),
            "Kein Mikrofon gefunden. Bis dahin geht Tippen."
        );
        assert_eq!(
            speech_text(&e, Lang::En),
            "No microphone found. You can type meanwhile."
        );
    }

    #[test]
    fn the_reason_comes_with_the_error() {
        let e = SpeechError::Microphone("busy".into());
        assert!(speech_text(&e, Lang::De).contains("(busy)"));
        assert!(speech_text(&e, Lang::En).contains("(busy)"));
    }

    #[test]
    fn a_missing_recognition_model_says_where_to_get_it() {
        let e = SpeechError::ModelMissing("/m".into());
        assert!(recognizer_text(&e, Lang::De).contains("„Modelle“"));
        assert!(recognizer_text(&e, Lang::En).contains("“Models”"));
        let other = SpeechError::NoMicrophone;
        assert_eq!(
            recognizer_text(&other, Lang::En),
            speech_text(&other, Lang::En)
        );
    }

    #[test]
    fn a_broken_model_names_its_folder_in_the_test() {
        let e = SpeechError::ModelLoad("/m".into());
        assert!(test_text(&e, Lang::De).contains("/m"));
        assert!(test_text(&e, Lang::En).contains("/m"));
    }

    #[test]
    fn every_verdict_has_a_text_in_both_languages() {
        for v in [
            Verdict::Silent,
            Verdict::Quiet,
            Verdict::Noisy,
            Verdict::Clipping,
            Verdict::Good,
        ] {
            assert_ne!(verdict_text(v, Lang::De), verdict_text(v, Lang::En));
        }
    }
}
