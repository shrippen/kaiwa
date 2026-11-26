//! What the settings' sync tile shows (ADR-010 step 6), and a sync's outcome in words.

use kaiwa_ai::privacy::Flow;
use kaiwa_core::Lang;
use kaiwa_store::SyncPlace;
use serde_json::{Value, json};

use crate::Backend;
use crate::merge::MergeError;
use crate::sync::{Secret, SyncError, Synced};

/// The tile: the place, which secrets are kept, whether the device syncs already, and a line
/// that says what the next sync does.
pub fn sync_view(b: &Backend, lang: Lang) -> Value {
    let de = lang == Lang::De;
    let place = b.store.profile().map(|p| p.sync).unwrap_or_default();
    let (kind, folder, url, user) = match &place {
        SyncPlace::None => ("none", "", "", ""),
        SyncPlace::Folder { path } => ("folder", path.as_str(), "", ""),
        SyncPlace::WebDav { url, user } => ("webdav", "", url.as_str(), user.as_str()),
    };
    let syncs = b.syncs().unwrap_or(false);
    let passphrase = b.has_sync_secret(Secret::Passphrase);
    let password = b.has_sync_secret(Secret::Password);
    let text = match (kind, passphrase, syncs, de) {
        ("none", _, _, true) => "Kein Ort zum Abgleichen gewählt.",
        ("none", _, _, false) => "No place to sync with chosen.",
        (_, false, _, true) => "Noch keine Passphrase: Sie verschlüsselt die Dateien.",
        (_, false, _, false) => "No passphrase yet: it seals the files.",
        (_, _, true, true) => "Dieses Gerät gleicht ab.",
        (_, _, true, false) => "This device syncs.",
        (_, _, false, true) => {
            "Noch nicht abgeglichen: Beginne hier oder tritt mit einem leeren Profil bei."
        }
        (_, _, false, false) => "Not synced yet: begin here or join with an empty profile.",
    };
    json!({
        "kind": kind,
        "folder": folder,
        "url": url,
        "user": user,
        "passphrase": passphrase,
        "password": password,
        "syncs": syncs,
        "keyring": b.keys.keyring_home(),
        "text": text,
    })
}

/// "What leaves the device?" (/LF711/): the tutor, recognition and voice from
/// `kaiwa_ai::privacy`, then the sync when a place is set up.
pub fn privacy(b: &Backend, lang: Lang) -> Value {
    let mut rows = kaiwa_ai::privacy::flows(b.tutor_env(), lang.code());
    rows.extend(sync_flow(b, lang));
    serde_json::to_value(rows).unwrap_or_default()
}

/// The sync's row: where the journal goes, sealed.
fn sync_flow(b: &Backend, lang: Lang) -> Option<Flow> {
    let de = lang == Lang::De;
    let (provider, cloud) = match b.store.profile().ok()?.sync {
        SyncPlace::None => return None,
        SyncPlace::Folder { path } => (
            format!("{} ({path})", if de { "Ordner" } else { "Folder" }),
            false,
        ),
        SyncPlace::WebDav { url, .. } => (format!("WebDAV ({url})"), true),
    };
    let place = match (cloud, de) {
        (true, _) => "Cloud",
        (false, true) => "Lokal",
        (false, false) => "Local",
    };
    Some(Flow {
        part: "Sync".into(),
        provider,
        cloud,
        place: place.into(),
        sends: if de {
            "Alle Lerndaten (Journal), mit deiner Passphrase verschlüsselt (age)"
        } else {
            "All learning data (the journal), sealed with your passphrase (age)"
        }
        .into(),
    })
}

/// A sync's outcome for the learner.
pub fn sync_words(r: &Result<Synced, SyncError>, lang: Lang) -> String {
    let de = lang == Lang::De;
    let (de_text, en_text) = match r {
        Ok(s) => done_words(s),
        Err(e) => error_words(e),
    };
    if de { de_text } else { en_text }
}

/// A sync that went through, in German and English.
fn done_words(s: &Synced) -> (String, String) {
    if s.devices == 0 {
        return (
            "Abgeglichen; noch kein anderes Gerät.".into(),
            "Synced; no other device yet.".into(),
        );
    }
    (
        format!("Abgeglichen mit {} Gerät(en), {} neu.", s.devices, s.taken),
        format!("Synced with {} device(s), {} new.", s.devices, s.taken),
    )
}

/// Why a sync stopped, in German and English.
fn error_words(e: &SyncError) -> (String, String) {
    match e {
        SyncError::NotSetUp => (
            "Erst Ort und Passphrase einrichten.".to_string(),
            "Set up a place and a passphrase first.".to_string(),
        ),
        SyncError::Taken => (
            "Hier gleicht schon ein Gerät ab: Tritt bei, mit einem leeren Profil.".into(),
            "A device syncs here already: join, with an empty profile.".into(),
        ),
        SyncError::Alone => (
            "Hier ist noch kein Gerät zum Beitreten. Beginne hier, oder warte auf die Datei des ersten Geräts.".into(),
            "No device to join here yet. Begin here, or wait for the first device's file.".into(),
        ),
        SyncError::Place(kaiwa_sync::SyncError::Sealed(_)) => (
            "Die Passphrase öffnet die Dateien nicht.".into(),
            "The passphrase does not open the files.".into(),
        ),
        SyncError::Place(e) => (
            format!("Der Ort ist nicht erreichbar: {e}"),
            format!("The place cannot be reached: {e}"),
        ),
        SyncError::Newer(_) => (
            "Ein Gerät nutzt ein neueres Kaiwa; aktualisiere dieses zuerst.".into(),
            "A device uses a newer Kaiwa; update this one first.".into(),
        ),
        SyncError::Broken(name) => (
            format!("{name} ist keine Datei von Kaiwa."),
            format!("{name} is no file of Kaiwa."),
        ),
        SyncError::Merge(MergeError::NotEmpty) => (
            "Beitreten geht nur mit einem leeren Profil. Lege ein neues Profil an und tritt dort bei.".into(),
            "Only an empty profile joins. Add a new profile and join there.".into(),
        ),
        SyncError::Merge(MergeError::Unrelated) => (
            "Die Dateien hier gehören zu einem anderen Profil.".into(),
            "The files here belong to another profile.".into(),
        ),
        SyncError::Merge(e) => (
            format!("Nicht abgeglichen: {e}"),
            format!("Not synced: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TempBackend;

    const T0: kaiwa_core::Moment = kaiwa_core::Moment {
        unix: 1_791_100_000,
        offset_min: 120,
    };

    #[test]
    fn the_tile_says_what_is_missing_then_what_the_next_sync_does() {
        let b = TempBackend::new("sync-view", false).unwrap();
        let text = |b: &Backend| sync_view(b, Lang::De)["text"].as_str().unwrap().to_string();
        assert!(text(&b).contains("Kein Ort"));
        b.set_setting("syncUrl", "https://cloud.example.org/dav/Kaiwa", T0)
            .unwrap();
        b.set_setting("syncUser", "jonas", T0).unwrap();
        let v = sync_view(&b, Lang::De);
        assert_eq!(
            (v["kind"].as_str(), v["user"].as_str()),
            (Some("webdav"), Some("jonas"))
        );
        assert!(text(&b).contains("Passphrase"));
        b.set_sync_secret(Secret::Passphrase, "kirschbluete")
            .unwrap();
        assert!(text(&b).contains("Beginne hier"));

        let rows = privacy(&b, Lang::De);
        let row = rows.as_array().unwrap().last().unwrap().clone();
        assert_eq!(
            (row["part"].as_str(), row["cloud"].as_bool()),
            (Some("Sync"), Some(true))
        );
        assert!(row["sends"].as_str().unwrap().contains("verschlüsselt"));

        // A blank address sets no place, and the row goes.
        b.set_setting("syncUrl", " ", T0).unwrap();
        assert_eq!(sync_view(&b, Lang::En)["kind"], "none");
        assert_eq!(privacy(&b, Lang::De).as_array().unwrap().len(), 3);
    }

    #[test]
    fn every_outcome_has_words_in_both_languages() {
        let outcomes = [
            Ok(Synced {
                taken: 3,
                devices: 1,
            }),
            Err(SyncError::NotSetUp),
            Err(SyncError::Taken),
            Err(SyncError::Alone),
            Err(SyncError::Merge(MergeError::NotEmpty)),
        ];
        for r in &outcomes {
            let (de, en) = (sync_words(r, Lang::De), sync_words(r, Lang::En));
            assert!(!de.is_empty() && de != en, "{r:?}");
        }
        let ok = sync_words(&outcomes[0], Lang::De);
        assert!(ok.contains("1 Gerät") && ok.contains("3 neu"), "{ok}");
    }
}
