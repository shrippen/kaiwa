import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// The skills (/LF202/): one row each, the level as a bar with the levels it is measured in
// named over it, the level in words at its end. The view gives each skill its own marks (the
// JLPT levels; the politeness its own steps): the head names those of the first skill, a row
// with other marks (the skills of one scale are listed together) names its own under its bar. A click opens the row: its sentence and the
// two corrections, "I know this" and "too hard". The view says where the bar ends and where
// the heads sit; this holds no rule.
ColumnLayout {
    id: rows

    // The app object (`kaiwa`), the learner view's `skills`, each with its `marks`.
    required property var kaiwa
    property var skills: []
    readonly property var headMarks: skills.length > 0 ? skills[0].marks : []
    property bool compact: false
    /** The skill whose row is open, "" for none; it survives a refresh of the view. */
    property string opened: ""

    function t(de, en) { return kaiwa.lang === "en" ? en : de }
    readonly property real barWidth: KanteStyle.unit(compact ? 110 : 240)
    readonly property real wordsWidth: KanteStyle.unit(compact ? 110 : 150)
    readonly property real gap: KanteStyle.unit(12)
    // Row margin of a KanteListRow, so the heads sit over the bars.
    readonly property real margin: KanteStyle.unit(12)

    spacing: 0

    function namesOf(marks) { return marks.map(m => m.name).join("|") }
    // Where a mark sits on a bar `width` long, centred on its level, kept inside the bar.
    function markX(mark, width, labelWidth) {
        return Math.max(0, Math.min(width - labelWidth, mark.at * width - labelWidth / 2))
    }
    // Column heads: "Skill · Start N5 N4 N3 · Level"; the levels at the share of the bar
    // where the view puts them.
    RowLayout {
        Layout.fillWidth: true
        Layout.leftMargin: rows.margin
        Layout.rightMargin: rows.margin
        spacing: rows.gap
        KanteSectionLabel {
            Layout.fillWidth: true
            text: rows.t("Fähigkeit", "Skill")
        }
        Item {
            Layout.preferredWidth: rows.barWidth
            Layout.preferredHeight: startHead.implicitHeight
            KanteSectionLabel { id: startHead; x: 0; visible: !rows.compact; text: rows.t("Start", "Start") }
            Repeater {
                model: rows.headMarks
                delegate: KanteSectionLabel {
                    required property int index
                    required property var modelData
                    // Not over the mark before it where the bar is too short for both.
                    visible: index === 0 || (modelData.at - rows.headMarks[index - 1].at) * rows.barWidth > width + KanteStyle.unit(6)
                    x: rows.markX(modelData, rows.barWidth, width)
                    text: modelData.name
                }
            }
        }
        KanteSectionLabel {
            Layout.preferredWidth: rows.wordsWidth
            text: rows.t("Stufe", "Level")
        }
    }

    Repeater {
        model: rows.skills
        delegate: ColumnLayout {
            id: line
            required property var modelData
            readonly property bool open: rows.opened === modelData.skill
            Layout.fillWidth: true
            spacing: 0

            KanteListRow {
                Layout.fillWidth: true
                text: modelData.name
                selected: line.open
                minTextWidth: KanteStyle.unit(compact ? 70 : 110)
                Accessible.description: modelData.short + ". " + rows.t("Anklicken, um die Stufe zu korrigieren.", "Click to correct the level.")
                onClicked: rows.opened = line.open ? "" : modelData.skill
                trailing: RowLayout {
                    spacing: rows.gap
                    ColumnLayout {
                        Layout.preferredWidth: rows.barWidth
                        spacing: KanteStyle.unit(2)
                        KanteProgressBar {
                            Layout.fillWidth: true
                            value: modelData.share
                            Accessible.name: modelData.name
                        }
                        // Marks other than the head's: this skill's own.
                        Item {
                            readonly property bool own: rows.namesOf(modelData.marks) !== rows.namesOf(rows.headMarks)
                            visible: own && modelData.marks.length > 0
                            Layout.fillWidth: true
                            Layout.preferredHeight: visible ? ownProbe.implicitHeight : 0
                            KanteSectionLabel { id: ownProbe; visible: false; text: "0" }
                            Repeater {
                                model: parent.own ? modelData.marks : []
                                delegate: KanteSectionLabel {
                                    required property var modelData
                                    x: rows.markX(modelData, rows.barWidth, width)
                                    text: modelData.name
                                }
                            }
                        }
                    }
                    QQC2.Label {
                        Layout.preferredWidth: rows.wordsWidth
                        text: modelData.short
                        wrapMode: Text.Wrap
                        font: KanteStyle.smallFont
                        color: KanteStyle.textColor
                    }
                }
            }
            // The opened row: the level as a sentence and the corrections.
            Flow {
                Layout.fillWidth: true
                Layout.leftMargin: rows.margin
                Layout.rightMargin: rows.margin
                Layout.topMargin: KanteStyle.unit(6)
                Layout.bottomMargin: KanteStyle.unit(6)
                visible: line.open
                spacing: rows.gap
                QQC2.Label {
                    width: Math.min(implicitWidth, parent.width)
                    text: modelData.words
                    wrapMode: Text.Wrap
                    font: KanteStyle.defaultFont
                    color: KanteStyle.textColor
                }
                KanteButton {
                    size: KanteButton.Size.Small
                    emphasis: KanteButton.Emphasis.Quiet
                    text: rows.t("Kann ich schon", "I know this")
                    Accessible.name: text + ": " + modelData.name
                    onClicked: rows.kaiwa.nudge(modelData.skill, true)
                }
                KanteButton {
                    size: KanteButton.Size.Small
                    emphasis: KanteButton.Emphasis.Quiet
                    text: rows.t("Zu schwer", "Too hard")
                    Accessible.name: text + ": " + modelData.name
                    onClicked: rows.kaiwa.nudge(modelData.skill, false)
                }
            }
        }
    }

    QQC2.Label {
        Layout.fillWidth: true
        Layout.leftMargin: rows.margin
        Layout.rightMargin: rows.margin
        Layout.topMargin: KanteStyle.unit(6)
        text: rows.t("Eine Zeile anklicken, um die Stufe zu korrigieren.", "Click a row to correct its level.")
        wrapMode: Text.Wrap
        font: KanteStyle.smallFont
        color: KanteStyle.mutedTextColor
    }
}
