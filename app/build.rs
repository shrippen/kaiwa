use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("org.shrippen.kaiwa").qml_files([
        "qml/Main.qml",
        "qml/ConversationPage.qml",
        "qml/StartPage.qml",
        "qml/PathPage.qml",
        "qml/ReviewPage.qml",
        "qml/ProfilePage.qml",
        "qml/SkillRows.qml",
        "qml/MistakeCharts.qml",
        "qml/SettingsPage.qml",
        "qml/SettingsSpeech.qml",
        "qml/SettingsTutor.qml",
        "qml/SettingsModels.qml",
        "qml/SettingRow.qml",
        "qml/Choice.qml",
        "qml/MainTabs.qml",
        "qml/NoticeLine.qml",
        "qml/SpareLine.qml",
        "qml/leave.js",
        "qml/record.js",
    ]))
    // Kante, copied unchanged by tools/sync-kante.sh, compiled in under qrc:/qt/qml/Kante.
    .qrc("qml/kante.qrc")
    .file("src/app.rs")
    .file("src/conversation.rs")
    .file("src/mic.rs")
    .build();
}
