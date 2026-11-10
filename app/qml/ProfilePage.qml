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
    /** Do the placement again (/LF200/: repeatable). */
    signal place()
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
        KanteButton {
            Layout.leftMargin: KanteStyle.unit(18)
            size: KanteButton.Size.Small
            emphasis: KanteButton.Emphasis.Quiet
            text: page.t("Neu einstufen", "Place me again")
            Accessible.description: page.t("Ein kurzes Gespräch, höchstens acht Minuten; das Ergebnis ersetzt den Stand oben.", "A short talk, eight minutes at most; the result replaces the levels above.")
            onClicked: page.place()
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

        // Grammar points (/LF201/): how many sit, which are still practised, with explanations.
        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(8)
            visible: !!page.learner.grammar && page.learner.grammar.seen > 0
            text: page.t("Grammatik", "Grammar")
            rule: true
        }
        QQC2.Label {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            Layout.fillWidth: true
            visible: !!page.learner.grammar && page.learner.grammar.seen > 0
            wrapMode: Text.Wrap
            text: page.learner.grammar ? page.t(page.learner.grammar.firm + " von " + page.learner.grammar.seen + " benutzten Punkten sitzen.",
                                                page.learner.grammar.firm + " of " + page.learner.grammar.seen + " points used are firm.") : ""
            font: KanteStyle.defaultFont
            color: KanteStyle.textColor
        }
        Repeater {
            model: page.learner.grammar ? page.learner.grammar.practising : []
            delegate: KanteListRow {
                required property var modelData
                Layout.fillWidth: true
                Layout.leftMargin: KanteStyle.unit(18)
                Layout.rightMargin: KanteStyle.unit(18)
                text: modelData.label
                subtitle: modelData.explain
                meta: page.t(modelData.wrong + "× verbessert", modelData.wrong + "× corrected")
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

        // The weekly review (/LF604/): the last seven days against the seven before.
        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(8)
            text: page.t("Diese Woche", "This week")
            rule: true
        }
        RowLayout {
            readonly property var w: page.stats.week || ({})
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            spacing: KanteStyle.unit(24)
            KanteKpi {
                value: String(parent.w.minutes || 0)
                label: page.t("Minuten gesprochen (Vorwoche ", "minutes spoken (week before ") + (parent.w.minutesBefore || 0) + ")"
            }
            KanteKpi { value: String(parent.w.days || 0); label: page.t("Übungstage", "days practised") }
            KanteKpi { value: String(parent.w.newItems || 0); label: page.t("neue Wörter und Wendungen", "new words and phrases") }
            KanteKpi {
                visible: parent.w.unaided !== undefined && parent.w.unaided !== null
                value: (parent.w.unaided || 0) + " %"
                label: page.t("ohne Hilfe verstanden", "understood without help")
                    + (parent.w.unaidedBefore !== undefined && parent.w.unaidedBefore !== null ? page.t(" (Vorwoche ", " (week before ") + parent.w.unaidedBefore + " %)" : "")
            }
        }
        Text {
            readonly property var examples: (page.stats.week || {}).newExamples || []
            visible: examples.length > 0
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            text: page.t("Neu: ", "New: ") + examples.join("、")
            wrapMode: Text.Wrap
            color: KanteStyle.mutedTextColor
            font: KanteStyle.defaultFont
        }
        // Progress per skill over the week (/LF604/), in words: what moved first.
        Text {
            readonly property var skills: (page.stats.week || {}).skills || []
            readonly property var moved: skills.filter(function (s) { return s.change !== "same" })
            visible: skills.length > 0
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            text: moved.length > 0
                ? moved.map(function (s) { return s.text }).join(" · ")
                : page.t("Deine Fähigkeiten stehen etwa wie vor einer Woche.", "Your skills are about where they were a week ago.")
            wrapMode: Text.Wrap
            color: KanteStyle.textColor
            font: KanteStyle.defaultFont
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
        // Vocabulary per level and the kinds of mistakes over the same weeks (/LF610/).
        RowLayout {
            Layout.leftMargin: KanteStyle.unit(18)
            spacing: KanteStyle.unit(24)
            visible: (page.stats.vocab || []).length > 1
            Repeater {
                model: page.stats.vocab || []
                delegate: KanteKpi {
                    required property var modelData
                    value: String(modelData.count)
                    label: page.t("Wörter und Wendungen ", "words and phrases ") + modelData.level
                }
            }
        }
        Repeater {
            model: page.stats.mistakes || []
            delegate: RowLayout {
                required property var modelData
                Layout.fillWidth: true
                Layout.leftMargin: KanteStyle.unit(18)
                Layout.rightMargin: KanteStyle.unit(18)
                spacing: KanteStyle.unit(14)
                KanteSparkline {
                    values: modelData.weeks
                    implicitWidth: KanteStyle.unit(120)
                    implicitHeight: KanteStyle.unit(24)
                    Accessible.role: Accessible.Graphic
                    Accessible.name: modelData.word + ": " + modelData.weeks.join(", ")
                }
                Text {
                    Layout.fillWidth: true
                    text: modelData.text
                    color: KanteStyle.textColor
                    font: KanteStyle.defaultFont
                    wrapMode: Text.Wrap
                }
            }
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
