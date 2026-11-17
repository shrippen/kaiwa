import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// The learning path (Lastenheft /LF600/, /LF220/): one group per unit of the curriculum, its
// scenarios inside and, where words are due, the review in the same group. Done, next, open
// and locked differ in sign and word, never only in colour; a locked unit says what unlocks
// it, and the locked ones beyond the first three of a level fold into one button. One
// section per level, named „N5 · A1“ and oriented on JLPT and the JF Standard, never a JLPT
// course (Q-03). The view (kaiwa-backend `path`) does the grouping; this page only shows it.
QQC2.ScrollView {
    id: page
    required property var kaiwa
    signal talk(string scenario)
    signal showTab(int index)
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var levels: kaiwa.path.length > 0 ? JSON.parse(kaiwa.path) : []
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
                property bool expanded: false
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
                Repeater {
                    model: level.modelData.units.filter(u => !u.folded || level.expanded)
                    delegate: ColumnLayout {
                        id: unit
                        required property var modelData
                        readonly property bool inReach: modelData.state === "current" || modelData.state === "open"
                        // The scenario a click on the unit starts: the first not done, else the first.
                        readonly property var target: modelData.scenarios.find(s => s.state !== "done") || modelData.scenarios[0]
                        Layout.fillWidth: true
                        spacing: KanteStyle.unit(4)

                        KantePath {
                            Layout.fillWidth: true
                            model: [{
                                title: unit.modelData.number + " · " + unit.modelData.title,
                                meta: unit.modelData.note,
                                text: unit.inReach ? unit.modelData.canDo : "",
                                state: unit.modelData.state
                            }]
                            doneText: page.t("erledigt", "done")
                            currentText: page.t("als nächstes", "up next")
                            lockedText: page.t("gesperrt", "locked")
                            openText: page.t("offen", "open")
                            onActivated: function () {
                                // A done unit with words due opens that unit's review.
                                if (unit.modelData.state === "done" && unit.modelData.due > 0) {
                                    page.kaiwa.openReview(unit.modelData.id)
                                    page.showTab(2)
                                    return
                                }
                                page.talk(unit.target.id)
                            }
                        }
                        Repeater {
                            model: unit.inReach ? unit.modelData.scenarios : []
                            delegate: KanteButton {
                                required property var modelData
                                Layout.leftMargin: KanteStyle.unit(40)
                                Layout.fillWidth: true
                                text: modelData.state === "done" ? modelData.title + " (" + page.t("erledigt", "done") + ")" : modelData.title
                                emphasis: modelData.state === "current" ? KanteButton.Emphasis.Primary : KanteButton.Emphasis.Normal
                                onClicked: page.talk(modelData.id)
                            }
                        }
                        // Words due in a unit still to do: its review is one more button.
                        KanteButton {
                            Layout.leftMargin: KanteStyle.unit(40)
                            Layout.fillWidth: true
                            visible: unit.inReach && unit.modelData.due > 0
                            emphasis: KanteButton.Emphasis.Quiet
                            text: unit.modelData.dueText
                            onClicked: {
                                page.kaiwa.openReview(unit.modelData.id)
                                page.showTab(2)
                            }
                        }
                    }
                }
                // The names wrap under the count, so a long list is read in full.
                KanteListRow {
                    Layout.fillWidth: true
                    visible: level.modelData.foldedCount > 0 && !level.expanded
                    overflow: KanteListRow.Overflow.Wrap
                    text: level.modelData.foldedCount === 1
                          ? page.t("+ 1 weitere gesperrte Einheit", "+ 1 more locked unit")
                          : page.t("+ " + level.modelData.foldedCount + " weitere gesperrte Einheiten",
                                   "+ " + level.modelData.foldedCount + " more locked units")
                    subtitle: level.modelData.foldedNames
                    onClicked: level.expanded = true
                }
            }
        }
        Item { implicitHeight: KanteStyle.unit(12) }
    }
}
