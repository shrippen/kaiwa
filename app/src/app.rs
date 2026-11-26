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
        /// The unit the review is narrowed to (a review node of the path); empty: all due.
        #[qproperty(QString, review_unit, cxx_name = "reviewUnit")]
        /// That unit as {"id", "title", "due"}; empty when the review is not narrowed.
        #[qproperty(QString, review_scope, cxx_name = "reviewScope")]
        #[qproperty(QString, learner)]
        #[qproperty(QString, stats)]
        #[qproperty(QString, profile)]
        #[qproperty(QString, privacy)]
        /// UI language, "de" or "en".
        #[qproperty(QString, lang)]
        #[qproperty(bool, gamification)]
        /// The last review answer: {"right", "expected", "next"}, or {"error"} if not saved.
        #[qproperty(QString, answer)]
        /// A short message after an action (export, deletion).
        #[qproperty(QString, notice)]
        /// The speech models: what, how big, which licence, from where, there or not (F-08).
        #[qproperty(QString, models)]
        /// The running download: {"dir", "done", "total"}; empty when none runs.
        #[qproperty(QString, download)]
        /// A build with the demo (screenshots, recorded answers); never a package.
        #[qproperty(bool, demo_build, cxx_name = "demoBuild")]
        /// A sync runs (/LF700/); the tile waits for it.
        #[qproperty(bool, syncing)]
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

        /// Narrows the review to `unit` (empty: all due again).
        #[qinvokable]
        #[cxx_name = "openReview"]
        fn open_review(self: Pin<&mut KaiwaApp>, unit: &QString);

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

        /// Keeps the tutor's API key in the system keyring; a blank key is refused (/LF713/).
        #[qinvokable]
        #[cxx_name = "storeKey"]
        fn store_key(self: Pin<&mut KaiwaApp>, key: &QString);

        /// Removes the tutor's API key from the keyring (/LF713/).
        #[qinvokable]
        #[cxx_name = "removeKey"]
        fn remove_key(self: Pin<&mut KaiwaApp>);

        /// Starts at zero without the placement (/LF200/).
        #[qinvokable]
        #[cxx_name = "leaveOutPlacement"]
        fn leave_out_placement(self: Pin<&mut KaiwaApp>);

        /// Downloads the model in directory `dir`, after the learner asked for it (F-08,
        /// /LL401/).
        #[qinvokable]
        #[cxx_name = "downloadModel"]
        fn download_model(self: Pin<&mut KaiwaApp>, dir: &QString);

        /// Stops the running download.
        #[qinvokable]
        #[cxx_name = "stopDownload"]
        fn stop_download(self: Pin<&mut KaiwaApp>);

        /// Syncs with the place of the settings (/LF700/); `start` is "here" or "join" and
        /// counts only on the first sync.
        #[qinvokable]
        #[cxx_name = "syncNow"]
        fn sync_now(self: Pin<&mut KaiwaApp>, start: &QString);

        /// Keeps the sync's "passphrase" or the WebDAV "password" in the keyring (/LF713/).
        #[qinvokable]
        #[cxx_name = "setSyncSecret"]
        fn set_sync_secret(self: Pin<&mut KaiwaApp>, kind: &QString, value: &QString);
    }

    impl cxx_qt::Threading for KaiwaApp {}
}

use core::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::Threading;

use cxx_qt_lib::QString;
use kaiwa_backend::Lang;
use kaiwa_backend::Trouble;
use kaiwa_backend::journal::JournalError;
use kaiwa_backend::models::{self, Fetch};
use kaiwa_backend::settings::{Felt, SettingError};
use kaiwa_backend::sync::{self, Secret, Start};
use kaiwa_backend::sync_view;
use kaiwa_backend::words;
use serde_json::json;

use crate::state::views;
use crate::state::{now, state};
use crate::util::append_notice;

pub struct KaiwaAppRust {
    today: QString,
    path: QString,
    review: QString,
    review_unit: QString,
    review_scope: QString,
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
    syncing: bool,
    stop: Arc<AtomicBool>,
}

impl Default for KaiwaAppRust {
    fn default() -> Self {
        Self {
            today: Default::default(),
            path: Default::default(),
            review: Default::default(),
            review_unit: Default::default(),
            review_scope: Default::default(),
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
            syncing: false,
            stop: Default::default(),
        }
    }
}

fn q(v: &serde_json::Value) -> QString {
    QString::from(&v.to_string())
}

impl qobject::KaiwaApp {
    fn ui_lang(&self) -> Lang {
        Lang::of(&self.lang().to_string())
    }

    /// Adds `msg` below what the notice already says, once: a second failure does not wipe
    /// the first.
    fn add_notice(mut self: Pin<&mut Self>, msg: &str) {
        let new = append_notice(&self.notice().to_string(), msg);
        self.as_mut().set_notice(QString::from(&new));
    }

    /// Says what an action the learner just asked for came to. That replaces the notice: what
    /// it said before is not about this action.
    fn say_result(mut self: Pin<&mut Self>, msg: &str) {
        self.as_mut().set_notice(QString::default());
        self.add_notice(msg);
    }

    /// Tells the learner the profile was left as it is (it could not be read or written).
    fn say_unreadable(self: Pin<&mut Self>, e: &anyhow::Error) {
        let msg = words::unreadable(e, self.ui_lang());
        self.add_notice(&msg);
    }

    /// Tells the learner why a change is refused while the stand-in profile runs.
    fn say_spare(self: Pin<&mut Self>) {
        let msg = words::spare(self.ui_lang());
        self.add_notice(msg);
    }

    /// Tells the learner the setting is saved but its effect failed (today's goal, deleting
    /// recordings).
    fn say_unapplied(self: Pin<&mut Self>, e: &anyhow::Error) {
        let msg = words::unapplied(e, self.ui_lang());
        self.add_notice(&msg);
    }

    /// Tells the learner a page or the profile could not be loaded; what the page shows is
    /// then empty or the defaults.
    fn say_unloaded(self: Pin<&mut Self>, e: &anyhow::Error) {
        let msg = words::unloaded(e, self.ui_lang());
        self.add_notice(&msg);
    }

    /// Tells the learner what went wrong in the background (`Backend::take_troubles`).
    fn say_trouble(self: Pin<&mut Self>, t: &Trouble) {
        let msg = words::trouble(t, self.ui_lang());
        self.add_notice(&msg);
    }

    fn refresh(mut self: Pin<&mut Self>) {
        let now = now();
        let unit = self.review_unit().to_string();
        let mut failed = None;
        let troubles;
        let (today, path, review, scope, learner, stats, profile, lang, gam, privacy) = {
            let st = state();
            troubles = st.take_troubles();
            let p = st.profile().unwrap_or_else(|e| {
                failed.get_or_insert(e);
                Default::default()
            });
            let privacy = views::privacy(&st, p.ui_lang);
            (
                views::today(&st, now),
                views::path(&st),
                match unit.is_empty() {
                    true => views::review(&st, now, 10),
                    false => views::review_unit(&st, now, 10, &unit),
                },
                match unit.is_empty() {
                    true => Ok(serde_json::Value::Null),
                    false => views::review_scope(&st, now, &unit),
                },
                views::learner(&st),
                views::stats(&st, now),
                views::profile(&st),
                p.ui_lang,
                p.gamification,
                privacy,
            )
        };
        let ok = |r: anyhow::Result<serde_json::Value>, failed: &mut Option<anyhow::Error>| {
            QString::from(&views::text(r, failed))
        };
        self.as_mut().set_today(ok(today, &mut failed));
        self.as_mut().set_path(ok(path, &mut failed));
        self.as_mut().set_review(ok(review, &mut failed));
        // The unit is done: back to all due items.
        let scope = scope.unwrap_or_else(|e| {
            failed.get_or_insert(e);
            serde_json::Value::Null
        });
        if scope.is_null() {
            self.as_mut().set_review_unit(QString::default());
        }
        let scope = if scope.is_null() {
            QString::default()
        } else {
            q(&scope)
        };
        self.as_mut().set_review_scope(scope);
        self.as_mut().set_learner(ok(learner, &mut failed));
        self.as_mut().set_stats(ok(stats, &mut failed));
        // Where the mora check's model belongs and whether it is there (/LF320/, F-08).
        let profile = profile.map(|mut v| {
            v["moraModel"] = models::mora_model();
            v
        });
        self.as_mut().set_profile(ok(profile, &mut failed));
        self.as_mut().set_privacy(q(&privacy));
        self.as_mut().set_models(q(&models::list(lang)));
        self.as_mut().set_lang(QString::from(lang.code()));
        self.as_mut().set_gamification(gam);
        if let Some(e) = failed {
            self.as_mut().say_unloaded(&e);
        }
        for t in &troubles {
            self.as_mut().say_trouble(t);
        }
    }

    fn download_model(mut self: Pin<&mut Self>, dir: &QString) {
        let dir = dir.to_string();
        let Some(fetch) = Fetch::of(&dir) else {
            return;
        };
        if !self.download().is_empty() {
            return;
        }
        let total = fetch.total;
        self.as_mut()
            .set_download(q(&json!({ "dir": dir, "done": 0, "total": total })));
        self.stop.store(false, Ordering::Relaxed);
        let stop = self.stop.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let progress = |done: u64| {
                let dir = dir.clone();
                let _ = thread.queue(move |mut app| {
                    if !app.download().is_empty() {
                        app.as_mut()
                            .set_download(q(&json!({ "dir": dir, "done": done, "total": total })));
                    }
                });
            };
            let outcome = fetch.run(&stop, &progress);
            let _ = thread.queue(move |mut app| {
                app.as_mut().set_download(QString::default());
                let notice = outcome.text(app.ui_lang());
                app.as_mut().say_result(&notice);
                app.as_mut().refresh();
            });
        });
    }

    fn stop_download(self: Pin<&mut Self>) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn sync_now(mut self: Pin<&mut Self>, start: &QString) {
        if *self.syncing() {
            return;
        }
        let start = match start.to_string().as_str() {
            "here" => Start::Here,
            _ => Start::Join,
        };
        let lang = self.ui_lang();
        self.as_mut().set_syncing(true);
        // Sealing and the network take seconds: off the GUI thread, and the profile is held
        // only while the sync reads or merges it.
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = sync::sync_locked(state, start, now());
            let msg = sync_view::sync_words(&result, lang);
            let _ = thread.queue(move |mut app| {
                app.as_mut().set_syncing(false);
                app.as_mut().say_result(&msg);
                app.as_mut().refresh();
            });
        });
    }

    fn set_sync_secret(mut self: Pin<&mut Self>, kind: &QString, value: &QString) {
        let secret = match kind.to_string().as_str() {
            "password" => Secret::Password,
            _ => Secret::Passphrase,
        };
        let lang = self.ui_lang();
        let msg = {
            let st = state();
            let result = st.set_sync_secret(secret, &value.to_string());
            sync_view::secret_words(&st, secret, &result, lang)
        };
        self.as_mut().say_result(&msg);
        self.as_mut().refresh();
    }

    fn set_setting(mut self: Pin<&mut Self>, key: &QString, value: &QString) {
        let edited = state().set_setting(&key.to_string(), &value.to_string(), now());
        match edited {
            Err(SettingError::Journal(JournalError::NotSaved(e))) => {
                self.as_mut().say_unreadable(&e)
            }
            // A key the page passes is always known: nothing changed, so say nothing.
            Err(SettingError::Unknown) => {}
            Err(SettingError::Journal(JournalError::NotApplied(e))) => {
                self.as_mut().say_unapplied(&e)
            }
            Err(SettingError::Journal(JournalError::Spare)) => self.as_mut().say_spare(),
            Ok(()) => {}
        }
        // Also after a failure: the page shows what the profile now holds.
        self.as_mut().refresh();
    }

    fn nudge(mut self: Pin<&mut Self>, skill: &QString, up: bool) {
        self.as_mut().set_notice(QString::default());
        let felt = if up { Felt::TooEasy } else { Felt::TooHard };
        let saved = state().nudge(&skill.to_string(), felt);
        if let Err(e) = saved {
            self.as_mut().say_unreadable(&e);
        }
        self.as_mut().refresh();
    }

    fn open_review(mut self: Pin<&mut Self>, unit: &QString) {
        self.as_mut().set_review_unit(unit.clone());
        self.as_mut().refresh();
    }

    fn answer_review(mut self: Pin<&mut Self>, text: &QString, said: &QString, helped: bool) {
        let answer = state().answer_review(&text.to_string(), &said.to_string(), helped, now());
        self.as_mut().set_answer(q(&answer));
        self.as_mut().refresh();
    }

    fn export_profile(self: Pin<&mut Self>, file: &QString) {
        let result = state().export_to(&file.to_string(), now());
        let msg = words::exported(&result, self.ui_lang());
        self.say_result(&msg);
    }

    fn delete_profile(mut self: Pin<&mut Self>) {
        self.as_mut().set_answer(QString::default());
        let result = state().wipe();
        let msg = words::wiped(&result, self.ui_lang());
        self.as_mut().say_result(&msg);
        self.as_mut().refresh();
    }

    fn pause(mut self: Pin<&mut Self>, days: i32) {
        if days > 0 {
            let today = now().day();
            let (added, streak) = {
                let st = state();
                let added = st.pause(today, days.unsigned_abs());
                let streak = st.profile().map(|p| p.gamification).unwrap_or(true);
                (added, streak)
            };
            let msg = words::pause_set(&added, streak, self.ui_lang());
            self.as_mut().say_result(&msg);
        }
        self.as_mut().refresh();
    }

    fn stop_pause(mut self: Pin<&mut Self>) {
        let result = state().end_pause(now().day());
        let msg = words::pause_ended(&result, self.ui_lang());
        self.as_mut().say_result(&msg);
        self.as_mut().refresh();
    }

    fn import_profile(mut self: Pin<&mut Self>, file: &QString) {
        let result = state().import_from(&file.to_string());
        let file = match &result {
            Ok(file) => file.display().to_string(),
            Err(_) => file.to_string(),
        };
        let result = result.map(|_| ());
        // The import brings its own UI language: say it in the one the profile has now.
        self.as_mut().set_notice(QString::default());
        self.as_mut().refresh();
        let msg = words::imported(&result, &file, self.ui_lang());
        self.add_notice(&msg);
    }

    fn skip_unit(mut self: Pin<&mut Self>, unit: &QString) {
        // Only the unit the offer is about, once per offer (review #86).
        let taken = kaiwa_backend::skips::take_skip(&state(), &unit.to_string());
        if let Err(e) = taken {
            self.as_mut().say_unreadable(&e);
        }
        self.as_mut().refresh();
    }

    fn switch_profile(mut self: Pin<&mut Self>, file: &QString) {
        // What the page said to the last person is not for the next.
        self.as_mut().set_answer(QString::default());
        self.as_mut().set_notice(QString::default());
        let result = {
            let mut st = state();
            st.open_profile(&file.to_string()).map(|b| *st = b)
        };
        if let Err(e) = result {
            let msg = format!("{e:#}");
            self.as_mut().add_notice(&msg);
        }
        self.as_mut().refresh();
    }

    fn new_profile(mut self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        self.as_mut().set_answer(QString::default());
        self.as_mut().set_notice(QString::default());
        let result = {
            let mut st = state();
            st.add_profile(&name).map(|b| {
                if let Some(b) = b {
                    *st = b;
                }
            })
        };
        if let Err(e) = result {
            let msg = format!("{e:#}");
            self.as_mut().add_notice(&msg);
        }
        self.as_mut().refresh();
    }

    fn store_key(mut self: Pin<&mut Self>, key: &QString) {
        // Only a provider that takes a key has a keyring entry.
        let Some(result) = state().store_key(&key.to_string()) else {
            return;
        };
        let msg = words::key_stored(&result, self.ui_lang());
        self.as_mut().say_result(&msg);
        self.as_mut().refresh();
    }

    fn remove_key(mut self: Pin<&mut Self>) {
        let Some(result) = state().remove_key() else {
            return;
        };
        let msg = words::key_removed(&result, self.ui_lang());
        self.as_mut().say_result(&msg);
        self.as_mut().refresh();
    }

    fn leave_out_placement(mut self: Pin<&mut Self>) {
        let edited = state().leave_out_placement();
        if let Err(e) = edited {
            self.as_mut().say_unreadable(&e);
            return;
        }
        self.as_mut().refresh();
    }
}
