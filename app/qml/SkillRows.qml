import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// The skills (/LF202/): one row each, the level as a bar on a common scale with the levels
// named over it, the level in words at its end. A click opens the row: its sentence and the
// two corrections, "I know this" and "too hard". The view says where the bar ends and where
// the heads sit; this holds no rule.
ColumnLayout {
    id: rows

    // The app object (`kaiwa`), the learner view's `skills` and `scale` (as `marks`).
    required property var kaiwa
    property var skills: []
    property var marks: []
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
                model: rows.marks
                delegate: KanteSectionLabel {
                    required property int index
                    required property var modelData
                    // Not over the head before it where the bar is too short for both.
                    visible: index === 0 || (modelData.at - rows.marks[index - 1].at) * rows.barWidth > width + KanteStyle.unit(6)
                    // Centred on its level, kept inside the bar's end.
                    x: Math.min(rows.barWidth - width, modelData.at * rows.barWidth - width / 2)
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
                    KanteProgressBar {
                        Layout.preferredWidth: rows.barWidth
                        value: modelData.share
                        Accessible.name: modelData.name
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
