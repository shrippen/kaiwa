import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante
import "lang.js" as Lang

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
    // This unit's due items, not all of them.
    function review(unit) {
        kaiwa.openReview(unit)
        showTab(2)
    }
    function t(de, en) { return Lang.pick(kaiwa.lang, de, en) }

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
                        Layout.fillWidth: true
                        spacing: KanteStyle.unit(4)

                        KantePath {
                            Layout.fillWidth: true
                            model: [{
                                title: unit.modelData.number + " · " + unit.modelData.title,
                                meta: unit.modelData.note,
                                text: unit.modelData.showCanDo ? unit.modelData.canDo : "",
                                state: unit.modelData.state
                            }]
                            doneText: page.t("erledigt", "done")
                            currentText: page.t("aktuell", "current")
                            lockedText: page.t("gesperrt", "locked")
                            openText: page.t("offen", "open")
                            onActivated: function () {
                                // What a click does is the view's: a scenario or a review.
                                var open = unit.modelData.open
                                if (!open) { return }
                                if (open.review) { page.review(open.review) } else { page.talk(open.scenario) }
                            }
                        }
                        Repeater {
                            model: unit.modelData.showScenarios ? unit.modelData.scenarios : []
                            delegate: KanteButton {
                                required property var modelData
                                Layout.leftMargin: KanteStyle.unit(40)
                                Layout.fillWidth: true
                                text: modelData.label
                                emphasis: modelData.emphasis === "primary" ? KanteButton.Emphasis.Primary
                                    : modelData.emphasis === "quiet" ? KanteButton.Emphasis.Quiet
                                    : KanteButton.Emphasis.Normal
                                onClicked: page.talk(modelData.id)
                            }
                        }
                        // The unit's due words: a review of its own, named with the unit.
                        KanteButton {
                            Layout.leftMargin: KanteStyle.unit(40)
                            Layout.fillWidth: true
                            visible: !!unit.modelData.review
                            emphasis: KanteButton.Emphasis.Quiet
                            text: unit.modelData.review ? unit.modelData.review.label : ""
                            Accessible.name: text
                            onClicked: page.review(unit.modelData.id)
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
