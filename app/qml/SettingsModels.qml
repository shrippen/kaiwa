import QtQuick
import QtQuick.Layouts
import Kante

// The speech models tile of the settings (F-08, /LL401/): source, size and licence first;
// Kaiwa connects only when the learner asks for the download.
KanteSettingGroup {
    id: group
    // The settings page: its profile, texts and `kaiwa`.
    required property var host
    readonly property int present: host.models.filter(function (m) { return m.present }).length
    Layout.fillWidth: true
    countText: host.models.length === 0 ? ""
        : host.t(present + " von " + host.models.length + " vorhanden", present + " of " + host.models.length + " present")

    Text {
        Layout.fillWidth: true
        Layout.topMargin: KanteStyle.unit(10)
        Layout.bottomMargin: KanteStyle.unit(2)
        text: host.t("Modelle sind die Dateien, mit denen Kaiwa auf deinem Rechner Sprache erkennt, spricht, die Aussprache prüft, Wörter nachschlägt und Furigana liest. Kaiwa lädt eines nur, wenn du hier auf „Herunterladen“ tippst. Dabei geht nur die Anfrage an die genannte Quelle hinaus, keine Daten von dir. Jede Datei wird vor der Benutzung geprüft.",
                     "Models are the files Kaiwa uses on your computer to recognise speech, to speak, to check pronunciation, to look words up and to read furigana. Kaiwa downloads one only when you tap “Download” here. Only the request goes to the named source, none of your data. Every file is checked before it is used.")
        wrapMode: Text.Wrap
        color: KanteStyle.mutedTextColor
        font: KanteStyle.smallFont
    }
    Repeater {
        model: host.models
        delegate: SettingRow {
            id: modelRow
            required property var modelData
            readonly property bool running: host.download.dir === modelData.dir
            title: modelData.name
            hint: modelData.purpose + " · " + modelData.licence + " · " + modelData.origin + " · "
                + host.t("Download " + modelData.downloadMB + " MB, danach " + modelData.diskMB + " MB auf der Platte",
                         "download " + modelData.downloadMB + " MB, then " + modelData.diskMB + " MB on disk")
            RowLayout {
                spacing: KanteStyle.unit(8)
                Text {
                    visible: modelRow.modelData.present
                    text: host.t("Vorhanden", "Present")
                    color: KanteStyle.textColor
                    font: KanteStyle.defaultFont
                }
                KanteButton {
                    visible: !modelRow.modelData.present && !modelRow.running
                    enabled: host.download.dir === undefined
                    text: host.t("Herunterladen", "Download")
                    Accessible.description: host.t("Lädt " + modelRow.modelData.name + " (" + modelRow.modelData.downloadMB + " MB) von " + modelRow.modelData.origin + ".",
                                                    "Downloads " + modelRow.modelData.name + " (" + modelRow.modelData.downloadMB + " MB) from " + modelRow.modelData.origin + ".")
                    onClicked: host.kaiwa.downloadModel(modelRow.modelData.dir)
                }
                KanteProgressBar {
                    visible: modelRow.running
                    Layout.preferredWidth: KanteStyle.unit(160)
                    value: modelRow.running && host.download.total > 0 ? host.download.done / host.download.total : 0
                    Accessible.name: host.t("Download von ", "Download of ") + modelRow.modelData.name
                }
                Text {
                    visible: modelRow.running
                    text: modelRow.running ? Math.round(host.download.done / 1e6) + " / " + Math.round(host.download.total / 1e6) + " MB" : ""
                    color: KanteStyle.mutedTextColor
                    font: KanteStyle.smallFont
                }
                KanteButton {
                    visible: modelRow.running
                    emphasis: KanteButton.Emphasis.Quiet
                    text: host.t("Abbrechen", "Stop")
                    onClicked: host.kaiwa.stopDownload()
                }
            }
        }
    }
}
