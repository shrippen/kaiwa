import QtQuick
import QtQuick.Controls as QQC2
import Kante

// Leaving a running talk: a placement starts again next time, a conversation stays open
// and continues from the start page.
KanteDialog {
    id: dialog

    // The conversation page and its `Conversation` object.
    required property var page
    required property var conv

    modal: true
    anchors.centerIn: QQC2.Overlay.overlay
    title: conv.placing ? page.t("Einstufung abbrechen?", "Stop the placement?") : page.t("Gespräch verlassen?", "Leave the talk?")
    kanteBarColor: conv.placing ? KanteStyle.negativeTextColor : KanteStyle.accentColor
    // Own buttons, so both say what they do in the app's language (the platform's
    // Cancel would read "Abbrechen" next to "Einstufung abbrechen").
    standardButtons: QQC2.Dialog.NoButton
    // Staying drops where the leaving would have gone (e.g. to the settings).
    property bool leaving: false
    onClosed: { if (!leaving) page.settingsNext = false; leaving = false }
    // The answer it waited for ended the talk or the placement's stages: the page changes
    // under the dialog (at the reading questions, Back asks again).
    Connections {
        target: page
        function onPlacementOverChanged() { if (page.placementOver) dialog.close() }
    }
    footerStart: [
        KanteButton {
            text: conv.placing ? page.t("Weiter einstufen", "Keep going") : page.t("Weiterreden", "Keep talking")
            // Placement: the way out of the question stays plain beside the red button,
            // as in Kante's destructive dialog.
            emphasis: conv.placing ? KanteButton.Emphasis.Normal : KanteButton.Emphasis.Primary
            onClicked: dialog.close()
        },
        KanteButton {
            text: conv.placing ? page.t("Einstufung beenden", "End the placement") : page.t("Verlassen", "Leave")
            emphasis: conv.placing ? KanteButton.Emphasis.Destructive : KanteButton.Emphasis.Normal
            // A turn still running would be lost: its answer saves the talk (a placement
            // is not saved, it can go at any time).
            enabled: conv.placing || !conv.busy
            onClicked: { dialog.leaving = true; dialog.close(); page.done() }
        }
    ]
    QQC2.Label {
        width: Math.min(page.width - KanteStyle.unit(64), KanteStyle.unit(380))
        text: conv.busy && !conv.placing
            ? page.t("Die Antwort kommt gleich; danach kannst du gehen, ohne dass etwas fehlt.", "The answer is on its way; then you can leave without losing anything.")
            : conv.placing
            ? page.t("Die Einstufung beginnt beim nächsten Mal von vorn.", "The placement starts again from the beginning next time.")
            : page.t("Das Gespräch bleibt offen: Auf der Startseite geht es dort weiter, wo du aufgehört hast.", "The talk stays open: continue it from the start page where you left off.")
        wrapMode: Text.Wrap
        font: KanteStyle.defaultFont
        color: KanteStyle.textColor
    }
}
