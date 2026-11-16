import QtQuick
import Kante

// Under the main tabs for as long as the profile could not be opened and a stand-in runs
// (`profile.spare`): nothing done now is saved. Unlike the notice it stays on every tab.
KanteBanner {
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

    visible: spare
    function t(de, en) { return kaiwa.lang === "en" ? en : de }
    title: t("Ersatzprofil", "Stand-in profile")
    text: t("Das Profil ließ sich nicht öffnen. Nichts, was du jetzt tust, wird gespeichert.",
            "The profile could not be opened. Nothing you do now is saved.")
}
