//! The app as a QObject for QML: start page, learning path, review, profile, statistics,
//! settings and the privacy view, each a JSON property (built in `views`), and the actions on
//! the profile. The conversation is its own object (`conversation`).

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, today)]
        #[qproperty(QString, path)]
        #[qproperty(QString, review)]
        #[qproperty(QString, learner)]
        #[qproperty(QString, stats)]
        #[qproperty(QString, profile)]
        #[qproperty(QString, privacy)]
        /// UI language, "de" or "en".
        #[qproperty(QString, lang)]
        #[qproperty(bool, gamification)]
        /// The last review answer: {"right", "expected", "next"}.
        #[qproperty(QString, answer)]
        /// A short message after an action (export, deletion).
        #[qproperty(QString, notice)]
        /// The speech models: what, how big, which licence, from where, there or not (F-08).
        #[qproperty(QString, models)]
        /// The running download: {"dir", "done", "total"}; empty when none runs.
        #[qproperty(QString, download)]
        /// A build with the demo (screenshots, recorded answers); never a package.
        #[qproperty(bool, demo_build, cxx_name = "demoBuild")]
        type KaiwaApp = super::KaiwaAppRust;

        /// Reads everything again from the profile.
        #[qinvokable]
        fn refresh(self: Pin<&mut KaiwaApp>);

        /// Changes one setting: goal, gamification, feedback, script, endPause, style, uiLang,
        /// reminder, costLimit, moraHints, keepRecordings, name, tutorProvider (a preset id),
        /// tutorModel, tutorBase, microphone.
        #[qinvokable]
        #[cxx_name = "setSetting"]
        fn set_setting(self: Pin<&mut KaiwaApp>, key: &QString, value: &QString);

        /// "I know this already" (up) or "too hard" for a skill (/LF202/).
        #[qinvokable]
        fn nudge(self: Pin<&mut KaiwaApp>, skill: &QString, up: bool);

        /// A review answer: what the learner said or typed for `text` (/LF230/).
        #[qinvokable]
        #[cxx_name = "answerReview"]
        fn answer_review(self: Pin<&mut KaiwaApp>, text: &QString, said: &QString, helped: bool);

        /// Writes the export (JSON) to `file` (/LF710/).
        #[qinvokable]
        #[cxx_name = "exportProfile"]
        fn export_profile(self: Pin<&mut KaiwaApp>, file: &QString);

        /// Deletes the profile completely and starts a new, empty one (/LF710/).
        #[qinvokable]
        #[cxx_name = "deleteProfile"]
        fn delete_profile(self: Pin<&mut KaiwaApp>);

        /// Pause mode for `days` days from today (/LF602/).
        #[qinvokable]
        fn pause(self: Pin<&mut KaiwaApp>, days: i32);

        /// Ends the pause that runs today (T-08).
        #[qinvokable]
        #[cxx_name = "stopPause"]
        fn stop_pause(self: Pin<&mut KaiwaApp>);

        /// Reads an export (JSON) into this profile, which must be empty (/LF710/, T-08).
        #[qinvokable]
        #[cxx_name = "importProfile"]
        fn import_profile(self: Pin<&mut KaiwaApp>, file: &QString);

        /// Skips a unit after the offer (/LF215/).
        #[qinvokable]
        #[cxx_name = "skipUnit"]
        fn skip_unit(self: Pin<&mut KaiwaApp>, unit: &QString);

        /// Uses another profile of this device (/LF720/).
        #[qinvokable]
        #[cxx_name = "switchProfile"]
        fn switch_profile(self: Pin<&mut KaiwaApp>, file: &QString);

        /// Creates a profile called `name` and uses it (/LF720/).
        #[qinvokable]
        #[cxx_name = "newProfile"]
        fn new_profile(self: Pin<&mut KaiwaApp>, name: &QString);

        /// Keeps the tutor's API key in the system keyring; empty removes it (/LF713/).
        #[qinvokable]
        #[cxx_name = "storeKey"]
        fn store_key(self: Pin<&mut KaiwaApp>, key: &QString);

        /// Removes the tutor's API key from the keyring.
        #[qinvokable]
        #[cxx_name = "removeKey"]
        fn remove_key(self: Pin<&mut KaiwaApp>);

        /// Starts at zero without the placement (/LF200/).
        #[qinvokable]
        #[cxx_name = "skipPlacement"]
        fn skip_placement(self: Pin<&mut KaiwaApp>);

        /// Downloads the model in directory `dir`, after the learner asked for it (F-08,
        /// /LL401/).
        #[qinvokable]
        #[cxx_name = "downloadModel"]
        fn download_model(self: Pin<&mut KaiwaApp>, dir: &QString);

        /// Stops the running download.
        #[qinvokable]
        #[cxx_name = "stopDownload"]
        fn stop_download(self: Pin<&mut KaiwaApp>);
    }

    impl cxx_qt::Threading for KaiwaApp {}
}

use core::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::Threading;

use cxx_qt_lib::QString;
use kaiwa_core::learner::Skill;
use kaiwa_core::progress::DailyGoal;
use kaiwa_store::{Script, Store};
use serde_json::json;

use crate::state::views;
use crate::state::{AppState, now, state};

pub struct KaiwaAppRust {
    today: QString,
    path: QString,
    review: QString,
    learner: QString,
    stats: QString,
    profile: QString,
    privacy: QString,
    lang: QString,
    gamification: bool,
    answer: QString,
    notice: QString,
    models: QString,
    download: QString,
    demo_build: bool,
    stop: Arc<AtomicBool>,
}

impl Default for KaiwaAppRust {
    fn default() -> Self {
        Self {
            today: Default::default(),
            path: Default::default(),
            review: Default::default(),
            learner: Default::default(),
            stats: Default::default(),
            profile: Default::default(),
            privacy: Default::default(),
            lang: Default::default(),
            gamification: Default::default(),
            answer: Default::default(),
            notice: Default::default(),
            models: Default::default(),
            download: Default::default(),
            demo_build: kaiwa_backend::DEMO,
            stop: Default::default(),
        }
    }
}

/// The speech models for the settings (F-08: source, size and licence before the download).
fn models_json(lang: &str) -> serde_json::Value {
    use kaiwa_speech::download::MODELS;
    let de = lang == "de";
    let mb = |b: u64| (b as f64 / 1e6).round() as u64;
    let rows: Vec<serde_json::Value> = MODELS
        .iter()
        .map(|m| {
            let purpose = match (m.name, de) {
                ("ReazonSpeech", true) => "Spracherkennung: versteht, was du sagst",
                ("ReazonSpeech", false) => "Speech recognition: understands what you say",
                ("Kokoro", true) => "Übergangsstimme der Figur",
                ("Kokoro", false) => "The character's interim voice",
                ("kaiwa-data", true) => {
                    "Wörterbuch: Lesung, Bedeutung und Akzent beim Klick auf ein Wort"
                }
                ("kaiwa-data", false) => {
                    "Dictionary: reading, meaning and accent when you click a word"
                }
                ("UniDic", true) => {
                    "Lesungen: Furigana über deinen eigenen Sätzen und wo der Tutor keine schreibt"
                }
                ("UniDic", false) => {
                    "Readings: furigana over your own sentences and where the tutor writes none"
                }
                (_, true) => "Aussprache-Hinweise zu langen Vokalen und っ",
                (_, false) => "Pronunciation hints on long vowels and っ",
            };
            json!({
                "dir": m.dir,
                "name": m.name,
                "purpose": purpose,
                "licence": m.licence,
                "origin": m.origin,
                "downloadMB": mb(m.download_bytes()),
                "diskMB": mb(m.disk_bytes),
                "present": m.present(),
            })
        })
        .collect();
    json!(rows)
}

fn q(v: &serde_json::Value) -> QString {
    QString::from(&v.to_string())
}

impl qobject::KaiwaApp {
    fn refresh(mut self: Pin<&mut Self>) {
        let now = now();
        let (today, path, review, learner, stats, profile, lang, gam, privacy) = {
            let st = state();
            let p = st.store.profile().unwrap_or_default();
            let privacy = views::privacy(&st, &p.ui_lang);
            (
                views::today(&st, now),
                views::path(&st),
                views::review(&st, now, 10),
                views::learner(&st),
                views::stats(&st, now),
                views::profile(&st),
                p.ui_lang.clone(),
                p.gamification,
                privacy,
            )
        };
        let ok = |r: anyhow::Result<serde_json::Value>| QString::from(&views::text(r));
        self.as_mut().set_today(ok(today));
        self.as_mut().set_path(ok(path));
        self.as_mut().set_review(ok(review));
        self.as_mut().set_learner(ok(learner));
        self.as_mut().set_stats(ok(stats));
        // Where the mora check's model belongs and whether it is there (/LF320/, F-08).
        let profile = profile.map(|mut v| {
            v["moraModel"] = crate::conversation::mora_model_json();
            v
        });
        self.as_mut().set_profile(ok(profile));
        self.as_mut().set_privacy(q(&privacy));
        self.as_mut().set_models(q(&models_json(&lang)));
        self.as_mut().set_lang(QString::from(&lang));
        self.as_mut().set_gamification(gam);
    }

    fn download_model(mut self: Pin<&mut Self>, dir: &QString) {
        let dir = dir.to_string();
        let Some(model) = kaiwa_speech::download::MODELS
            .iter()
            .find(|m| m.dir == dir)
            .copied()
        else {
            return;
        };
        if !self.download().is_empty() {
            return;
        }
        let total = model.download_bytes();
        self.as_mut()
            .set_download(q(&json!({ "dir": dir, "done": 0, "total": total })));
        self.stop.store(false, Ordering::Relaxed);
        let stop = self.stop.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            // At most one progress update per megabyte, so the UI thread keeps up.
            let last = std::sync::atomic::AtomicU64::new(0);
            let progress = |done: u64| {
                if done / 1_000_000 == last.load(Ordering::Relaxed) / 1_000_000 && done < total {
                    return;
                }
                last.store(done, Ordering::Relaxed);
                let dir = dir.clone();
                let _ = thread.queue(move |mut app| {
                    if !app.download().is_empty() {
                        app.as_mut()
                            .set_download(q(&json!({ "dir": dir, "done": done, "total": total })));
                    }
                });
            };
            let result = kaiwa_speech::download::download(&model, &progress, &stop);
            let _ = thread.queue(move |mut app| {
                app.as_mut().set_download(QString::default());
                let de = app.lang().to_string() == "de";
                let notice = match (result, de) {
                    (Ok(()), true) => format!("{} ist geladen.", model.name),
                    (Ok(()), false) => format!("{} is downloaded.", model.name),
                    (Err(kaiwa_speech::download::DownloadError::Stopped), true) => {
                        "Download abgebrochen.".to_string()
                    }
                    (Err(kaiwa_speech::download::DownloadError::Stopped), false) => {
                        "Download stopped.".to_string()
                    }
                    (Err(kaiwa_speech::download::DownloadError::Stalled(s)), true) => format!(
                        "{} ließ sich nicht laden: Die Verbindung hat {s} s lang keine Daten geliefert. Versuch es noch einmal.",
                        model.name
                    ),
                    (Err(kaiwa_speech::download::DownloadError::Stalled(s)), false) => format!(
                        "{} could not be downloaded: the connection gave no data for {s} s. Please try again.",
                        model.name
                    ),
                    (Err(e), true) => format!("{} ließ sich nicht laden: {e}", model.name),
                    (Err(e), false) => format!("{} could not be downloaded: {e}", model.name),
                };
                app.as_mut().set_notice(QString::from(&notice));
                app.as_mut().refresh();
            });
        });
    }

    fn stop_download(self: Pin<&mut Self>) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn set_setting(mut self: Pin<&mut Self>, key: &QString, value: &QString) {
        let (key, value) = (key.to_string(), value.to_string());
        {
            let st = state();
            let mut p = st.store.profile().unwrap_or_default();
            match key.as_str() {
                "goal" => {
                    if let Ok(m) = value.parse::<u32>() {
                        p.daily_goal = DailyGoal::Minutes(m.clamp(1, 120));
                    }
                }
                "gamification" => p.gamification = value == "true",
                "feedback" => {
                    if let Ok(a) = serde_json::from_value(json!(value)) {
                        p.feedback = a;
                    }
                }
                "script" => {
                    p.script = match value.as_str() {
                        "1" => Script::Kana,
                        "2" => Script::Hidden,
                        _ => Script::Furigana,
                    }
                }
                "endPause" => {
                    if let Ok(s) = value.parse::<f32>() {
                        p.end_pause_secs = s.clamp(1.2, 5.0);
                    }
                }
                "style" => p.style = value.parse().unwrap_or(0).min(2),
                "uiLang" => {
                    p.ui_lang = if value == "en" {
                        "en".into()
                    } else {
                        "de".into()
                    }
                }
                "reminder" => p.reminder = value.parse().ok(),
                "costLimit" => p.cost_limit_cents = value.parse().ok(),
                "moraHints" => p.mora_hints = value == "true",
                "unloadAfter" => p.unload_after_min = value.parse().ok(),
                "handsFree" => p.hands_free = value == "true",
                "textScale" => {
                    if let Ok(s) = value.parse::<f32>() {
                        p.text_scale = s.clamp(1.0, kaiwa_store::TEXT_SCALE_MAX);
                    }
                }
                "keepRecordings" => p.keep_recordings_days = value.parse().ok(),
                "name" => p.name = value,
                // The tutor (Q-15): a preset id or "" for none; model and server for an
                // OpenAI-compatible one.
                // Model and server belong to the provider: a new one starts without them, so
                // no server of the old one is used by mistake.
                "tutorProvider" => {
                    let id = kaiwa_ai::presets::preset(&value)
                        .map(|p| p.id.to_string())
                        .unwrap_or_default();
                    if id != p.tutor.provider {
                        p.tutor = kaiwa_ai::Tutor {
                            provider: id,
                            ..Default::default()
                        };
                    }
                }
                // The microphone (a device id from MicTest.devices); "" is the default.
                "microphone" => p.microphone = (!value.is_empty()).then_some(value),
                "tutorModel" => p.tutor.model = value.trim().to_string(),
                "tutorBase" => p.tutor.base_url = value.trim().to_string(),
                _ => return,
            }
            if let Err(e) = st.store.set_profile(&p) {
                eprintln!("kaiwa: {e}");
            }
            // A day already begun is judged by the goal it ends with (/LF602/).
            if key == "goal"
                && let Err(e) = st.store.set_day_goal(now().day(), p.daily_goal)
            {
                eprintln!("kaiwa: {e}");
            }
            // Turning recordings off (or shortening the days) deletes them now.
            if key == "keepRecordings"
                && let Err(e) = st.purge_recordings(now())
            {
                eprintln!("kaiwa: {e:#}");
            }
        }
        self.as_mut().refresh();
    }

    fn nudge(mut self: Pin<&mut Self>, skill: &QString, up: bool) {
        let Ok(skill) = serde_json::from_value::<Skill>(json!(skill.to_string())) else {
            return;
        };
        {
            let st = state();
            if let Ok(mut l) = st.store.learning() {
                // "Too easy" for listening lifts the speed one step at once (Selin, /LF202/).
                let (model, regulator) = (&mut l.model, &mut l.regulator);
                kaiwa_core::difficulty::learner_says(model, regulator, skill, up);
                let _ = st.store.set_learning(&l);
            }
        }
        self.as_mut().refresh();
    }

    fn answer_review(mut self: Pin<&mut Self>, text: &QString, said: &QString, helped: bool) {
        let answer = state().answer_review(&text.to_string(), &said.to_string(), helped, now());
        self.as_mut().set_answer(q(&answer));
        self.as_mut().refresh();
    }

    fn export_profile(mut self: Pin<&mut Self>, file: &QString) {
        let file = file.to_string();
        let file = file.strip_prefix("file://").unwrap_or(&file).to_string();
        let result = {
            let st = state();
            st.store
                .export(now())
                .map_err(anyhow::Error::from)
                .and_then(|v| {
                    std::fs::write(&file, serde_json::to_string_pretty(&v)?)?;
                    Ok(())
                })
        };
        let de = self.lang().to_string() == "de";
        self.as_mut().set_notice(QString::from(&match result {
            Ok(()) if de => format!("Exportiert nach {file}."),
            Ok(()) => format!("Exported to {file}."),
            Err(e) if de => format!("Der Export ist fehlgeschlagen: {e}"),
            Err(e) => format!("The export failed: {e}"),
        }));
    }

    fn delete_profile(mut self: Pin<&mut Self>) {
        self.as_mut().set_answer(QString::default());
        let result = {
            let mut st = state();
            let path = st.path.clone();
            // The emptied profile keeps the language the page is in.
            let lang = st.store.profile().map(|p| p.ui_lang).ok();
            if let Err(e) = st.delete_recordings() {
                eprintln!("kaiwa: {e:#}");
            }
            match Store::in_memory() {
                Ok(mem) => {
                    // Close the file before removing it.
                    st.store = mem;
                    kaiwa_store::delete_profile(&path)
                        .map_err(anyhow::Error::from)
                        .and_then(|_| {
                            st.store = Store::open(&path)?;
                            if let Some(lang) = lang {
                                let mut p = st.store.profile()?;
                                p.ui_lang = lang;
                                st.store.set_profile(&p)?;
                            }
                            Ok(())
                        })
                }
                Err(e) => Err(e.into()),
            }
        };
        let de = self.lang().to_string() == "de";
        self.as_mut().set_notice(QString::from(match (result, de) {
            (Ok(()), true) => "Alle Daten dieses Profils sind gelöscht.".to_string(),
            (Ok(()), false) => "All data of this profile is deleted.".to_string(),
            (Err(e), true) => format!("Das Löschen ist fehlgeschlagen: {e}"),
            (Err(e), false) => format!("Deleting failed: {e}"),
        }));
        self.as_mut().refresh();
    }

    fn pause(mut self: Pin<&mut Self>, days: i32) {
        if days > 0 {
            let today = now().day();
            let (added, streak) = {
                let st = state();
                let added = st.pause(today, days.unsigned_abs());
                let streak = st.store.profile().map(|p| p.gamification).unwrap_or(true);
                (added, streak)
            };
            // Say that it took, and until when: nothing else on the page shows a pause.
            let de = self.lang().to_string() == "de";
            self.as_mut().set_notice(QString::from(&match (added, de) {
                (Ok(until), true) if streak => {
                    let (_, m, d) = until.ymd();
                    format!("Pause bis einschließlich {d}.{m}.: Die Serie bleibt so lange stehen.")
                }
                (Ok(until), true) => {
                    let (_, m, d) = until.ymd();
                    format!("Pause bis einschließlich {d}.{m}.")
                }
                (Ok(until), false) if streak => format!(
                    "Paused up to and including {}: the streak holds until then.",
                    until.iso()
                ),
                (Ok(until), false) => format!("Paused up to and including {}.", until.iso()),
                (Err(e), true) => format!("Die Pause ließ sich nicht eintragen: {e}"),
                (Err(e), false) => format!("The pause could not be set: {e}"),
            }));
        }
        self.as_mut().refresh();
    }

    fn stop_pause(mut self: Pin<&mut Self>) {
        let result = state().end_pause(now().day());
        let de = self.lang().to_string() == "de";
        self.as_mut().set_notice(QString::from(&match (result, de) {
            (Ok(()), true) => "Die Pause ist beendet: Heute zählt wieder.".to_string(),
            (Ok(()), false) => "The pause is over: today counts again.".to_string(),
            (Err(e), true) => format!("Die Pause ließ sich nicht beenden: {e}"),
            (Err(e), false) => format!("The pause could not be ended: {e}"),
        }));
        self.as_mut().refresh();
    }

    fn import_profile(mut self: Pin<&mut Self>, file: &QString) {
        let file = file.to_string();
        let file = file.strip_prefix("file://").unwrap_or(&file).to_string();
        let result = std::fs::read_to_string(&file)
            .map_err(|e| ImportError::Unreadable(e.to_string()))
            .and_then(|text| {
                serde_json::from_str::<serde_json::Value>(&text)
                    .map_err(|_| ImportError::NotAnExport)
            })
            .and_then(|v| {
                state().store.import(&v).map_err(|e| match e {
                    kaiwa_store::StoreError::NotEmpty => ImportError::NotEmpty,
                    kaiwa_store::StoreError::WrongFormat(_) => ImportError::NotAnExport,
                    e => ImportError::Broken(e.to_string()),
                })
            });
        // The import brings its own UI language: say it in the one the profile has now.
        self.as_mut().refresh();
        let de = self.lang().to_string() == "de";
        self.as_mut().set_notice(QString::from(&match (result, de) {
            (Ok(()), true) => format!("Importiert aus {file}."),
            (Ok(()), false) => format!("Imported from {file}."),
            (Err(ImportError::NotEmpty), true) => "Der Import geht nur in ein leeres Profil. Lege oben ein neues Profil an und importiere dort.".to_string(),
            (Err(ImportError::NotEmpty), false) => "An import needs an empty profile. Add a new profile above and import there.".to_string(),
            (Err(ImportError::NotAnExport), true) => format!("{file} ist kein Export von Kaiwa."),
            (Err(ImportError::NotAnExport), false) => format!("{file} is not an export of Kaiwa."),
            (Err(ImportError::Unreadable(e)), true) => format!("{file} ließ sich nicht lesen: {e}"),
            (Err(ImportError::Unreadable(e)), false) => format!("{file} could not be read: {e}"),
            (Err(ImportError::Broken(e)), true) => format!("Der Import ist fehlgeschlagen, das Profil ist unverändert: {e}"),
            (Err(ImportError::Broken(e)), false) => format!("The import failed, the profile is unchanged: {e}"),
        }));
    }

    fn skip_unit(mut self: Pin<&mut Self>, unit: &QString) {
        // Only the unit the offer is about, once per offer (review #86).
        let _ = views::take_skip(&state(), &unit.to_string());
        self.as_mut().refresh();
    }

    fn switch_profile(mut self: Pin<&mut Self>, file: &QString) {
        // What the page said to the last person is not for the next.
        self.as_mut().set_answer(QString::default());
        self.as_mut().set_notice(QString::default());
        let result = {
            let mut st = state();
            let dir = st.dir();
            kaiwa_backend::profiles::set_current(&dir, &file.to_string())
                .and_then(|path| AppState::open_at(path, false))
                .map(|b| *st = b)
        };
        if let Err(e) = result {
            self.as_mut().set_notice(QString::from(&format!("{e:#}")));
        }
        self.as_mut().refresh();
    }

    fn new_profile(mut self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        if name.trim().is_empty() {
            return;
        }
        self.as_mut().set_answer(QString::default());
        self.as_mut().set_notice(QString::default());
        let result = {
            let mut st = state();
            let dir = st.dir();
            let lang = st
                .store
                .profile()
                .map(|p| p.ui_lang)
                .unwrap_or_else(|_| "de".into());
            kaiwa_backend::profiles::create(&dir, &name, &lang)
                .and_then(|path| AppState::open_at(path, false))
                .map(|b| *st = b)
        };
        if let Err(e) = result {
            self.as_mut().set_notice(QString::from(&format!("{e:#}")));
        }
        self.as_mut().refresh();
    }

    fn store_key(mut self: Pin<&mut Self>, key: &QString) {
        let provider = state().tutor_env()("KAIWA_PROVIDER").unwrap_or_default();
        let de = self.lang().to_string() == "de";
        // Only a provider that takes a key has a keyring entry.
        if kaiwa_ai::presets::preset(&provider).is_none_or(|p| p.key_env.is_none()) {
            return;
        }
        let msg = match (kaiwa_ai::secrets::store(&provider, &key.to_string()), de) {
            (Ok(()), true) => "Der Schlüssel liegt im Schlüsselbund.".to_string(),
            (Ok(()), false) => "The key is in the keyring.".to_string(),
            (Err(e), true) => format!("Der Schlüsselbund hat abgelehnt: {e}"),
            (Err(e), false) => format!("The keyring refused: {e}"),
        };
        self.as_mut().set_notice(QString::from(&msg));
        self.as_mut().refresh();
    }

    fn remove_key(mut self: Pin<&mut Self>) {
        let provider = state().tutor_env()("KAIWA_PROVIDER").unwrap_or_default();
        let de = self.lang().to_string() == "de";
        let msg = match (kaiwa_ai::secrets::remove(&provider), de) {
            (Ok(()), true) => "Der Schlüssel ist aus dem Schlüsselbund entfernt.".to_string(),
            (Ok(()), false) => "The key is removed from the keyring.".to_string(),
            (Err(e), true) => format!("Der Schlüsselbund hat abgelehnt: {e}"),
            (Err(e), false) => format!("The keyring refused: {e}"),
        };
        self.as_mut().set_notice(QString::from(&msg));
        self.as_mut().refresh();
    }

    fn skip_placement(mut self: Pin<&mut Self>) {
        {
            let st = state();
            let mut p = st.store.profile().unwrap_or_default();
            p.placed = true;
            let _ = st.store.set_profile(&p);
        }
        self.as_mut().refresh();
    }
}

/// Why an import did not happen, for the words the page says.
enum ImportError {
    Unreadable(String),
    NotAnExport,
    NotEmpty,
    Broken(String),
}
