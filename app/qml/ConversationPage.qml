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

    /** The scenario to play ("" = the next on the path), or continue the interrupted one. */
    property string scenarioId: ""
    property bool resume: false
    /** Back to the start page (finished, or to continue later). */
    signal done()
    /** Screenshot mode: say the lines of `script` (separated by "|"), save `shot`, quit. */
    property string shot: ""
    property string script: ""
    property bool showSpeechHelp: false

    title: conv.title
    background: Rectangle { color: KanteStyle.backgroundColor }
    KanteScope { target: page }

    Conversation {
        id: conv
        Component.onCompleted: page.resume ? resume() : start(page.scenarioId)
    }

    readonly property var turns: conv.turns.length > 0 ? JSON.parse(conv.turns) : []
    readonly property var goals: conv.goals.length > 0 ? JSON.parse(conv.goals) : []
    readonly property var summary: conv.summary.length > 0 ? JSON.parse(conv.summary) : null
    readonly property var recorded: conv.recorded.length > 0 ? JSON.parse(conv.recorded) : null
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
            // H-02: Kokoro speaks without pitch accent, so the page says it is a stand-in.
            KantePill {
                text: conv.voice.length > 0 ? conv.voice : "Ohne Stimme"
                status: conv.talkState === KanteTalkButton.State.Speaking ? KantePill.State.Running : KantePill.State.Off
                QQC2.ToolTip.visible: pillHover.hovered
                QQC2.ToolTip.text: conv.voice.length > 0
                    ? "Kokoro spricht die Antworten der Figur, aber ohne Tonhöhenakzent. Zum Nachsprechen kommt eine eigene Stimme."
                    : conv.voiceNote
                HoverHandler { id: pillHover }
            }
            KanteSegmented {
                visible: conv.voice.length > 0
                model: ["Laut", "Stumm"]
                tooltips: ["Die Figur spricht ihre Antworten", "Nur lesen"]
                currentIndex: conv.voiceOn ? 0 : 1
                onActivated: function (i) {
                    conv.voiceOn = i === 0
                    if (i === 1) conv.interrupt()
                }
            }
            KanteButton {
                visible: !conv.finished
                text: "Später weiter"
                emphasis: KanteButton.Emphasis.Quiet
                Accessible.description: "Das Gespräch bleibt offen und lässt sich auf der Startseite fortsetzen."
                onClicked: page.done()
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
                id: msg
                required property var modelData
                required property int index
                Layout.fillWidth: true
                from: modelData.who === "learner" ? KanteMessage.From.Own : KanteMessage.From.Other
                author: modelData.who === "learner" ? (modelData.typed ? "Du · getippt" : "Du") : conv.character
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
                        titleItem: KanteDiffText { parts: modelData.parts }
                        text: modelData.explanation
                        meta: modelData.disputed ? modelData.meta + " · umstritten" : modelData.meta
                        // /LF307/: the learner may say it was right; kept for the QA, no longer
                        // counted as a mistake.
                        KanteButton {
                            size: KanteButton.Size.Small
                            emphasis: KanteButton.Emphasis.Quiet
                            text: modelData.disputed ? "Widersprochen" : "Widersprechen"
                            enabled: !modelData.disputed
                            Accessible.description: "Der Satz war richtig: Kaiwa merkt sich den Widerspruch und zählt es nicht als Fehler."
                            onClicked: conv.dispute(msg.index, modelData.better)
                        }
                    }
                }
                // /LF113/: what Kaiwa heard; a misrecognition is no learning mistake.
                KanteButton {
                    visible: msg.modelData.who === "learner" && !msg.modelData.typed && !msg.modelData.pending && !msg.modelData.misrecognized
                    size: KanteButton.Size.Small
                    emphasis: KanteButton.Emphasis.Quiet
                    text: "Falsch erkannt"
                    Accessible.description: "So habe ich das nicht gesagt: Die Korrekturen zu diesem Zug zählen nicht."
                    onClicked: conv.markMisrecognized(msg.index)
                }
                Text {
                    visible: msg.modelData.misrecognized === true
                    text: "Als falsch erkannt markiert. Das zählt nicht als Fehler."
                    color: KanteStyle.mutedTextColor
                    font: KanteStyle.smallFont
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
            visible: page.showSpeechHelp && conv.speechError.length > 0
            kind: KanteCallout.Kind.Warn
            title: "Sprechen geht noch nicht"
            text: conv.speechError
        }
        KanteCallout {
            Layout.fillWidth: true
            visible: page.showSpeechHelp && conv.voiceNote.length > 0
            title: "Ohne Stimme"
            text: conv.voiceNote
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
                spacing: KanteStyle.unit(28)
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
            // What the session did for the learning model (/LF310/, /LF603/; XP only with
            // gamification, /LF605/).
            RowLayout {
                visible: page.recorded !== null
                spacing: KanteStyle.unit(28)
                KanteKpi { label: "Minuten"; value: page.recorded ? String(page.recorded.minutes) : "" }
                KanteKpi { label: "Neu zum Wiederholen"; value: page.recorded ? String(page.recorded.newItems) : "" }
                KanteKpi {
                    visible: page.recorded !== null && page.recorded.xp !== null
                    label: "XP"
                    value: page.recorded && page.recorded.xp !== null ? String(page.recorded.xp) : ""
                }
            }
            Repeater {
                model: page.recorded ? page.recorded.xpWhy : []
                delegate: Text {
                    required property var modelData
                    Layout.fillWidth: true
                    text: modelData
                    color: KanteStyle.mutedTextColor
                    font: KanteStyle.smallFont
                    wrapMode: Text.Wrap
                }
            }
            Text {
                Layout.fillWidth: true
                visible: page.recorded !== null && !!page.recorded.change
                text: page.recorded && page.recorded.change ? page.recorded.change : ""
                color: KanteStyle.textColor
                font: KanteStyle.defaultFont
                wrapMode: Text.Wrap
            }
            Text {
                Layout.fillWidth: true
                visible: page.recorded !== null && page.recorded.scenarioDone
                text: "Dieses Gespräch ist auf dem Lernpfad erledigt."
                color: KanteStyle.textColor
                font: KanteStyle.defaultFont
                wrapMode: Text.Wrap
            }
            KanteButton {
                text: "Fertig"
                emphasis: KanteButton.Emphasis.Primary
                onClicked: page.done()
            }
        }
    }

    footer: QQC2.ToolBar {
        padding: KanteStyle.unit(12)
        background: Rectangle { color: KanteStyle.backgroundColor; Rectangle { width: parent.width; height: 1; color: KanteStyle.ruleColor } }
        contentItem: RowLayout {
            spacing: KanteStyle.unit(8)
            KanteTalkButton {
                id: talk
                talkState: conv.busy && conv.talkState === KanteTalkButton.State.Ready ? KanteTalkButton.State.Thinking : conv.talkState
                level: conv.level
                titles: ["Sprechen", "Hört zu", "Denkt nach", "Spricht", "Kein Sprechen"]
                hints: ["Leertaste halten", "Loslassen zum Senden", "", "Tippen zum Unterbrechen", "Warum? Klicken"]
                enabled: !conv.finished
                onTalkStarted: conv.startTalk()
                onTalkEnded: conv.stopTalk()
                onInterruptRequested: conv.interrupt()
                onErrorActionRequested: page.showSpeechHelp = !page.showSpeechHelp
                Timer {
                    interval: 80
                    repeat: true
                    running: conv.talkState === KanteTalkButton.State.Listening
                    onTriggered: conv.pollLevel()
                }
            }
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
            if (!conv.speechReady || conv.busy || conv.talkState === KanteTalkButton.State.Thinking) return
            // "~" waits until the character speaks (for a shot of that state)
            if (page.lines.length > 0 && page.lines[0] === "~" && conv.talkState !== KanteTalkButton.State.Speaking && conv.voice.length > 0) return
            if (page.lines.length > 0) {
                var l = page.lines.slice()
                var next = l.shift()
                page.lines = l
                if (next === "?") conv.help()
                else if (next.indexOf("wav:") === 0) conv.sayWav(next.substring(4))
                else if (next === "!") page.showSpeechHelp = true
                else if (next === "~") {}
                else conv.say(next, true)
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
