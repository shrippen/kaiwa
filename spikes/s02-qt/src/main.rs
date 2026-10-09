//! Spike 0.2 (ADR-001): a Kirigami window with Kante components, driven by a Rust QObject.
//!
//!   KANTE_QML=../../../Kante/qml cargo run          # import path of the Kante checkout
//!   cargo run -- --shot out.png --kind 1             # screenshot and quit (kind 0 System, 1 Kante, 2 Kante Light)

mod session;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    if let Some(mut engine) = engine.as_mut() {
        if let Ok(path) = std::env::var("KANTE_QML") {
            engine.as_mut().add_import_path(&QString::from(&path));
        }
        engine.load(&QUrl::from(
            "qrc:/qt/qml/org/shrippen/kaiwa/spike/qml/Main.qml",
        ));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
