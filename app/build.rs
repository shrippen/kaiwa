use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("org.shrippen.kaiwa")
            .qml_files(["qml/Main.qml", "qml/ConversationPage.qml"]),
    )
    // Kante, copied unchanged by tools/sync-kante.sh, compiled in under qrc:/qt/qml/Kante.
    .qrc("qml/kante.qrc")
    .file("src/conversation.rs")
    .build();
}
