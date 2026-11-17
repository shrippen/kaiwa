import QtQuick
import QtQuick.Layouts
import Kante

// The talk page's footer without the speech models: one calm line instead of the talk
// button's "No speech" step, with the way to the models. Typing goes on below it.
RowLayout {
    id: line

    // Page language: "de" or "en".
    property string lang: "de"
    // The learner wants the models: the page leaves for the settings.
    signal modelsRequested()
    function t(de, en) { return lang === "en" ? en : de }

    spacing: KanteStyle.unit(10)

    Text {
        Layout.fillWidth: true
        text: line.t("Sprechen braucht das Modell für die Erkennung. Bis dahin tippst du.",
                     "Speaking needs the model for recognition. Until then you type.")
        color: KanteStyle.mutedTextColor
        font: KanteStyle.defaultFont
        wrapMode: Text.Wrap
    }
    KanteButton {
        text: line.t("Modelle laden", "Load models")
        emphasis: KanteButton.Emphasis.Quiet
        onClicked: line.modelsRequested()
    }
}
