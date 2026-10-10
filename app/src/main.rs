//! Kaiwa (会話): practise speaking Japanese in conversations. The start page, the learning path,
//! conversations with help and corrections, review, profile, statistics and settings, with the
//! GUI from Kante (System, Kante, Kante Light). The profile is a file (`kaiwa-store`).
//!
//!   cargo run -p kaiwa-app                      # recorded tutor answers (demo mode)
//!   KAIWA_PROVIDER=openai KAIWA_MODEL=… cargo run -p kaiwa-app
//!   cargo run -p kaiwa-app -- --shot out.png --kind 1 --say "コーヒーを好きです。"

mod app;
mod conversation;
mod state;
mod views;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/org/shrippen/kaiwa/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
    kaiwa_speech::shutdown();
}
