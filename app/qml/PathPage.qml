import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// The learning path (Lastenheft /LF600/, /LF220/): units of the curriculum with their
// scenarios and, where words are due, a review node. Done, now, open and locked differ in
// sign and word, never only in colour; a locked node says what unlocks it.
QQC2.ScrollView {
    id: page
    required property var kaiwa
    signal talk(string scenario)
    signal showTab(int index)
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var nodes: kaiwa.path.length > 0 ? JSON.parse(kaiwa.path) : []
    readonly property var learner: kaiwa.learner.length > 0 ? JSON.parse(kaiwa.learner) : ({})
    contentWidth: availableWidth

    ColumnLayout {
        width: page.availableWidth
        spacing: KanteStyle.unit(14)

        Item { implicitHeight: KanteStyle.unit(4) }
        KanteHeading {
            Layout.leftMargin: KanteStyle.unit(18)
            text: page.t("Lernpfad N5", "Learning path N5")
            pageTitle: true
        }

        // Far ahead of the material: offer to skip (/LF215/).
        KanteCallout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: page.learner.offerSkip === true
            kind: KanteCallout.Kind.Ok
            title: page.t("Das fällt dir leicht", "This comes easy to you")
            text: page.t("Die letzten drei Gespräche liefen fast fehlerfrei. Du kannst die aktuelle Einheit überspringen.",
                         "Your last three talks went almost without a slip. You can skip the current unit.")
        }

        KantePath {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            model: page.nodes
            doneText: page.t("erledigt", "done")
            currentText: page.t("jetzt", "now")
            reviewText: page.t("Wiederholung", "review")
            lockedText: page.t("gesperrt", "locked")
            openText: page.t("offen", "open")
            onActivated: function (i) {
                var n = page.nodes[i]
                if (n.kind === "review") page.showTab(2)
                else page.talk(n.id)
            }
        }
        Item { implicitHeight: KanteStyle.unit(12) }
    }
}
