import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante
import org.shrippen.kaiwa

// One scenario as a conversation (M1): the character speaks with furigana, the learner
// types (speech follows with spike 0.3), help comes in three steps, corrections sit under
// the learner's turn as hint cards with a diff, and the summary closes the scenario.
Kirigami.ScrollablePage {
    id: page

    property int styleKind: 0
    signal styleKindChangeRequested(int kind)
    /** Screenshot mode: say the lines of `script` (separated by "|"), save `shot`, quit. */
    property string shot: ""
    property string script: ""

    title: conv.title
    background: Rectangle { color: KanteStyle.backgroundColor }
    KanteScope { target: page }

    Conversation {
        id: conv
        Component.onCompleted: start()
    }

    readonly property var turns: conv.turns.length > 0 ? JSON.parse(conv.turns) : []
    readonly property var goals: conv.goals.length > 0 ? JSON.parse(conv.goals) : []
    readonly property var summary: conv.summary.length > 0 ? JSON.parse(conv.summary) : null
    readonly property int currentGoal: {
        for (var i = 0; i < goals.length; i++) if (!goals[i].done) return i
        return goals.length
    }

    header: ColumnLayout {
        spacing: 0
        KanteBanner {
            Layout.fillWidth: true
            visible: conv.demo
            title: "Demo"
            text: "Aufgezeichnete Antworten. Ein echter Tutor kommt über KAIWA_PROVIDER (siehe README)."
        }
    }

    ColumnLayout {
        spacing: KanteStyle.unit(14)

        RowLayout {
            Layout.fillWidth: true
            KanteHeading { text: conv.title; pageTitle: true; Layout.fillWidth: true }
            KanteSegmented {
                model: ["System", "Kante", "Light"]
                tooltips: ["Plattform-Stil", "Kante", "Kante Light"]
                currentIndex: page.styleKind
                onActivated: function (i) { page.styleKindChangeRequested(i) }
            }
        }

        KanteSteps {
            Layout.fillWidth: true
            model: page.goals.map(function (g) { return g.text })
            current: page.currentGoal
        }

        Repeater {
            model: page.turns
            delegate: KanteMessage {
                required property var modelData
                Layout.fillWidth: true
                from: modelData.who === "learner" ? KanteMessage.From.Own : KanteMessage.From.Other
                author: modelData.who === "learner" ? (modelData.typed ? "Du · getippt" : "Du") : "田中 · Tanaka"
                text: modelData.who === "learner" ? modelData.text : ""

                KanteRubyText {
                    visible: modelData.who === "character"
                    Layout.fillWidth: true
                    markup: modelData.who === "character" ? modelData.markup : ""
                    density: KanteRubyText.Density.All
                }
                KanteLoader { visible: modelData.pending === true }
                Repeater {
                    model: modelData.who === "learner" ? modelData.corrections : []
                    delegate: KanteHintCard {
                        required property var modelData
                        Layout.fillWidth: true
                        tier: modelData.tier === "red" ? KanteHintCard.Tier.Red
                            : modelData.tier === "yellow" ? KanteHintCard.Tier.Yellow
                            : modelData.tier === "blue" ? KanteHintCard.Tier.Blue : KanteHintCard.Tier.None
                        tierText: modelData.tierText
                        meta: modelData.meta
                        titleItem: KanteDiffText { parts: modelData.parts }
                        text: modelData.explanation
                    }
                }
            }
        }

        KanteCallout {
            Layout.fillWidth: true
            visible: conv.helpLevel > 0
            title: ["", "Stichwort", "Satzanfang", "Zum Nachsprechen"][conv.helpLevel]
            text: conv.helpText
        }
        KanteCallout {
            Layout.fillWidth: true
            visible: conv.error.length > 0
            kind: KanteCallout.Kind.Danger
            title: "Keine Antwort"
            text: conv.error
        }

        // ── Summary (/LF310/) ──
        ColumnLayout {
            visible: conv.finished && page.summary !== null
            Layout.fillWidth: true
            spacing: KanteStyle.unit(10)
            KanteHeading { text: "Bilanz"; level: 2 }
            RowLayout {
                spacing: KanteStyle.unit(12)
                KanteKpi { label: "Ziele"; value: page.summary ? page.summary.goalsMet + " / " + page.summary.goalsTotal : "" }
                KanteKpi { label: "Gesprochen"; value: page.summary ? String(page.summary.spoken) : "" }
                KanteKpi { label: "Getippt"; value: page.summary ? String(page.summary.typed) : "" }
                KanteKpi { label: "Hilfen"; value: page.summary ? String(page.summary.helps) : "" }
            }
            Repeater {
                model: page.summary ? page.summary.top : []
                delegate: KanteHintCard {
                    required property var modelData
                    Layout.fillWidth: true
                    compact: true
                    tier: modelData.tier === "red" ? KanteHintCard.Tier.Red
                        : modelData.tier === "yellow" ? KanteHintCard.Tier.Yellow
                        : modelData.tier === "blue" ? KanteHintCard.Tier.Blue : KanteHintCard.Tier.None
                    tierText: modelData.tierText
                    meta: modelData.meta
                    titleItem: KanteDiffText { parts: modelData.parts }
                }
            }
            Text {
                Layout.fillWidth: true
                visible: page.summary && page.summary.review.length > 0
                text: "In die Wiederholung: " + (page.summary ? page.summary.review.join(" · ") : "")
                color: KanteStyle.mutedTextColor
                font: KanteStyle.defaultFont
                wrapMode: Text.Wrap
            }
        }
    }

    footer: QQC2.ToolBar {
        padding: KanteStyle.unit(12)
        background: Rectangle { color: KanteStyle.backgroundColor; Rectangle { width: parent.width; height: 1; color: KanteStyle.ruleColor } }
        contentItem: RowLayout {
            spacing: KanteStyle.unit(8)
            KanteTextField {
                id: input
                Layout.fillWidth: true
                enabled: !conv.busy && !conv.finished
                placeholderText: "Auf Japanisch antworten …"
                onAccepted: page.send()
            }
            KanteButton {
                text: "Hilfe"
                enabled: !conv.finished
                onClicked: conv.help()
            }
            KanteButton {
                text: "Senden"
                emphasis: KanteButton.Emphasis.Primary
                busy: conv.busy
                enabled: !conv.busy && !conv.finished && input.text.length > 0
                onClicked: page.send()
            }
        }
    }

    function send() {
        conv.say(input.text, true)
        input.text = ""
    }

    // Screenshot mode.
    property var lines: script.length > 0 ? script.split("|") : []
    Timer {
        id: feeder
        running: page.shot.length > 0
        interval: 300
        repeat: true
        onTriggered: {
            if (conv.busy) return
            if (page.lines.length > 0) {
                var l = page.lines.slice()
                var next = l.shift()
                page.lines = l
                if (next === "?") conv.help(); else conv.say(next, true)
            } else {
                stop()
                grabber.start()
            }
        }
    }
    Timer {
        id: grabber
        interval: 600
        onTriggered: page.grabToImage(function (r) { r.saveToFile(page.shot); Qt.quit() })
    }
}
