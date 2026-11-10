import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// The learning path (Lastenheft /LF600/, /LF220/): units of the curriculum with their
// scenarios and, where words are due, a review node. Done, now, open and locked differ in
// sign and word, never only in colour; a locked node says what unlocks it. One section per
// level, named „N5 · A1“ and oriented on JLPT and the JF Standard, never a JLPT course (Q-03).
QQC2.ScrollView {
    id: page
    required property var kaiwa
    signal talk(string scenario)
    signal showTab(int index)
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var nodes: kaiwa.path.length > 0 ? JSON.parse(kaiwa.path) : []
    // The levels in path order, each with its nodes.
    readonly property var levels: {
        var out = []
        for (var i = 0; i < nodes.length; i++) {
            var l = nodes[i].level || ""
            if (out.length === 0 || out[out.length - 1].level !== l)
                out.push({ level: l, nodes: [] })
            out[out.length - 1].nodes.push(nodes[i])
        }
        return out
    }
    readonly property var learner: kaiwa.learner.length > 0 ? JSON.parse(kaiwa.learner) : ({})
    contentWidth: availableWidth

    ColumnLayout {
        width: page.availableWidth
        spacing: KanteStyle.unit(14)

        Item { implicitHeight: KanteStyle.unit(4) }
        KanteHeading {
            Layout.leftMargin: KanteStyle.unit(18)
            text: page.t("Lernpfad", "Learning path")
            pageTitle: true
        }
        QQC2.Label {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            wrapMode: Text.Wrap
            color: KanteStyle.mutedTextColor
            text: page.t("Die Stufen orientieren sich an JLPT und JF-Standard.",
                         "The levels are oriented on the JLPT and the JF Standard.")
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
            // The offer can be taken (T-08): the unit counts as done, the path goes on.
            KanteButton {
                visible: !!page.learner.skipUnit
                text: page.learner.skipUnit
                    ? page.t("„" + page.learner.skipUnit.title + "“ überspringen", "Skip “" + page.learner.skipUnit.title + "”")
                    : ""
                onClicked: page.kaiwa.skipUnit(page.learner.skipUnit.id)
            }
        }

        Repeater {
            model: page.levels
            delegate: ColumnLayout {
                id: level
                required property var modelData
                Layout.fillWidth: true
                Layout.leftMargin: KanteStyle.unit(18)
                Layout.rightMargin: KanteStyle.unit(18)
                spacing: KanteStyle.unit(8)

                KanteSectionLabel {
                    Layout.fillWidth: true
                    visible: level.modelData.level.length > 0
                    text: level.modelData.level
                    rule: true
                }
                KantePath {
                    Layout.fillWidth: true
                    model: level.modelData.nodes
                    doneText: page.t("erledigt", "done")
                    currentText: page.t("jetzt", "now")
                    reviewText: page.t("Wiederholung", "review")
                    lockedText: page.t("gesperrt", "locked")
                    openText: page.t("offen", "open")
                    onActivated: function (i) {
                        var n = level.modelData.nodes[i]
                        if (n.kind === "review") page.showTab(2)
                        else page.talk(n.id)
                    }
                }
            }
        }
        Item { implicitHeight: KanteStyle.unit(12) }
    }
}
