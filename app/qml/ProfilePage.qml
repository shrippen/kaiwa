import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Profile and statistics: the learner model in words, no raw numbers, and the learner can
// correct it (Lastenheft /LF202/); what still trips them up; how Kaiwa sets the difficulty
// now (/LF211/); practice over twelve weeks (/LF610/).
QQC2.ScrollView {
    id: page
    required property var kaiwa
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var learner: kaiwa.learner.length > 0 ? JSON.parse(kaiwa.learner) : ({ skills: [], weak: [], knobs: [] })
    readonly property var stats: kaiwa.stats.length > 0 ? JSON.parse(kaiwa.stats) : ({ heat: [] })
    contentWidth: availableWidth

    ColumnLayout {
        width: page.availableWidth
        spacing: KanteStyle.unit(12)

        Item { implicitHeight: KanteStyle.unit(4) }
        KanteHeading {
            Layout.leftMargin: KanteStyle.unit(18)
            text: page.t("Dein Stand", "Where you are")
            pageTitle: true
        }

        Repeater {
            model: page.learner.skills
            delegate: KanteListRow {
                required property var modelData
                Layout.fillWidth: true
                Layout.leftMargin: KanteStyle.unit(18)
                Layout.rightMargin: KanteStyle.unit(18)
                text: modelData.name
                subtitle: modelData.words
                trailing: RowLayout {
                    spacing: KanteStyle.unit(6)
                    KanteButton {
                        size: KanteButton.Size.Small
                        emphasis: KanteButton.Emphasis.Quiet
                        text: page.t("Kann ich schon", "I know this")
                        Accessible.name: text + ": " + modelData.name
                        onClicked: page.kaiwa.nudge(modelData.skill, true)
                    }
                    KanteButton {
                        size: KanteButton.Size.Small
                        emphasis: KanteButton.Emphasis.Quiet
                        text: page.t("Zu schwer", "Too hard")
                        Accessible.name: text + ": " + modelData.name
                        onClicked: page.kaiwa.nudge(modelData.skill, false)
                    }
                }
            }
        }

        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(8)
            visible: page.learner.weak.length > 0
            text: page.t("Daran arbeitest du gerade", "What you are working on")
            rule: true
        }
        QQC2.Label {
            Layout.leftMargin: KanteStyle.unit(18)
            visible: page.learner.weak.length > 0
            text: page.learner.weak.map(function (w) { return w.word }).join(" · ")
            font: KanteStyle.defaultFont
            color: KanteStyle.textColor
        }

        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(8)
            text: page.t("So stellt Kaiwa die Gespräche gerade ein", "How Kaiwa sets up the talks now")
            rule: true
        }
        GridLayout {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            columns: 2
            columnSpacing: KanteStyle.unit(16)
            rowSpacing: KanteStyle.unit(4)
            Repeater {
                model: page.learner.knobs
                delegate: RowLayout {
                    required property var modelData
                    spacing: KanteStyle.unit(8)
                    QQC2.Label {
                        Layout.preferredWidth: KanteStyle.unit(150)
                        text: modelData.word
                        font: KanteStyle.smallFont
                        color: KanteStyle.textColor
                    }
                    KanteProgressBar {
                        Layout.preferredWidth: KanteStyle.unit(120)
                        value: modelData.step / Math.max(1, modelData.steps - 1)
                        Accessible.name: modelData.word
                    }
                    QQC2.Label {
                        text: page.t("Stufe ", "step ") + (modelData.step + 1) + page.t(" von ", " of ") + modelData.steps
                        font: KanteStyle.smallFont
                        color: KanteStyle.mutedTextColor
                    }
                }
            }
        }

        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(8)
            text: page.t("Zwölf Wochen", "Twelve weeks")
            rule: true
        }
        RowLayout {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            spacing: KanteStyle.unit(24)
            KanteKpi { value: String(page.stats.minutes || 0); label: page.t("Minuten gesprochen", "minutes spoken") }
            KanteKpi { value: String(page.stats.days || 0); label: page.t("Übungstage", "days practised") }
            KanteKpi { value: String(page.stats.items || 0); label: page.t("Wörter und Wendungen", "words and phrases") }
            KanteKpi { value: String(page.stats.learnt || 0); label: page.t("sicher (3 Wochen +)", "solid (3 weeks +)") }
        }
        KanteHeatmap {
            Layout.leftMargin: KanteStyle.unit(18)
            levels: page.stats.heat
            columns: 7
            Accessible.role: Accessible.Graphic
            Accessible.name: page.t("Übungstage der letzten zwölf Wochen, eine Zeile pro Woche", "Practice days of the last twelve weeks, one row per week")
        }
        Item { implicitHeight: KanteStyle.unit(12) }
    }
}
