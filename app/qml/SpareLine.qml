import QtQuick
import QtQuick.Controls as QQC2
import Kante

// Under the main tabs for as long as the profile could not be opened and a stand-in runs
// (`profile.spare`): nothing done now is saved. Unlike the notice it stays on every tab.
QQC2.Label {
    id: line

    // The app object (`kaiwa`).
    property var kaiwa
    readonly property bool spare: {
        try {
            return JSON.parse(kaiwa.profile).spare === true
        } catch (e) {
            return false
        }
    }

    leftPadding: KanteStyle.unit(18)
    rightPadding: KanteStyle.unit(18)
    topPadding: KanteStyle.unit(6)
    bottomPadding: KanteStyle.unit(6)
    visible: spare
    function t(de, en) { return kaiwa.lang === "en" ? en : de }
    text: t("Das Profil ließ sich nicht öffnen. Dies ist ein Ersatz: Nichts, was du jetzt tust, wird gespeichert.",
            "The profile could not be opened. This is a stand-in: nothing you do now is saved.")
    wrapMode: Text.Wrap
    font: KanteStyle.defaultFont
    color: KanteStyle.warningColor
}
