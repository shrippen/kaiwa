//! Kaiwa (会話): practise speaking Japanese in conversations. M1: one scenario, typed turns,
//! help, corrections and the summary, with the GUI from Kante (System, Kante, Kante Light).
//!
//!   cargo run -p kaiwa-app                      # recorded tutor answers (demo mode)
//!   KAIWA_PROVIDER=openai KAIWA_MODEL=… cargo run -p kaiwa-app
//!   cargo run -p kaiwa-app -- --shot out.png --kind 1 --say "コーヒーを好きです。"

mod conversation;

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
