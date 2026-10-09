use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("org.shrippen.kaiwa.spike").qml_file("qml/Main.qml"),
    )
    .qt_module("Network")
    .file("src/session.rs")
    .build();
}
