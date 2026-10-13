import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante
import org.shrippen.kaiwa

// One scenario as a conversation (M1): the character speaks with furigana, the learner
// types (speech follows with spike 0.3), help comes in three steps, corrections sit under
// the learner's turn as hint cards with a diff, and the summary closes the scenario. Each
// line of the character can be heard again, slower or translated (/LF111/); a spoken turn
// may get a mora hint (/LF320/).
Kirigami.ScrollablePage {
    id: page

    /** The scenario to play ("" = the next on the path), or continue the interrupted one. */
    property string scenarioId: ""
    property bool resume: false
    /** Play the placement (/LF200/) instead of a scenario. */
    property bool placement: false
    /** UI language of the profile, "de" or "en". */
    property string lang: "de"
    /** The app object (models and their download, for the dictionary). */
    property var kaiwa: null
    function t(de, en) { return lang === "en" ? en : de }
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
        Component.onCompleted: page.placement ? startPlacement() : page.resume ? resume() : start(page.scenarioId)
    }

    readonly property var turns: conv.turns.length > 0 ? JSON.parse(conv.turns) : []
    readonly property var goals: conv.goals.length > 0 ? JSON.parse(conv.goals) : []
    readonly property var summary: conv.summary.length > 0 ? JSON.parse(conv.summary) : null
    readonly property var recorded: conv.recorded.length > 0 ? JSON.parse(conv.recorded) : null
    /** The placement's stages are over: the questions or the result take the page. */
    readonly property bool placementOver: conv.placementAsk || (recorded !== null && recorded.placement !== undefined)
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
            text: page.t("Aufgezeichnete Antworten. Ein echter Tutor kommt über KAIWA_PROVIDER (siehe README).", "Recorded answers. A real tutor comes through KAIWA_PROVIDER (see the README).")
        }
    }

    ColumnLayout {
        spacing: KanteStyle.unit(14)

        RowLayout {
            Layout.fillWidth: true
            KanteHeading { text: conv.title; pageTitle: true; Layout.fillWidth: true }
            // H-02: Kokoro speaks without pitch accent, so the page says it is a stand-in.
            KantePill {
                text: conv.voice.length > 0 ? page.t(conv.voice, "Interim voice") : page.t("Ohne Stimme", "No voice")
                status: conv.talkState === KanteTalkButton.State.Speaking ? KantePill.State.Running : KantePill.State.Off
                QQC2.ToolTip.visible: pillHover.hovered
                QQC2.ToolTip.text: conv.voice.length > 0
                    ? page.t("Kokoro spricht die Antworten der Figur, aber ohne Tonhöhenakzent. Zum Nachsprechen kommt eine eigene Stimme.",
                             "Kokoro speaks the character's replies, but without pitch accent. A voice of its own follows for repeating after it.")
                    : conv.voiceNote
                HoverHandler { id: pillHover }
            }
            KanteSegmented {
                visible: conv.voice.length > 0
                model: [page.t("Laut", "Sound"), page.t("Stumm", "Mute")]
                tooltips: [page.t("Die Figur spricht ihre Antworten", "The character speaks its replies"), page.t("Nur lesen", "Read only")]
                currentIndex: conv.voiceOn ? 0 : 1
                onActivated: function (i) {
                    conv.voiceOn = i === 0
                    if (i === 1) conv.interrupt()
                }
            }
            // /LL204/: easier listening in one step, kept in the profile.
            KanteChip {
                visible: conv.voice.length > 0
                text: page.t("Lauter", "Louder")
                checkable: true
                checked: conv.louder
                toolTip: page.t("Die Figur spricht lauter", "The character speaks louder")
                onClicked: conv.setSound(checked, conv.clear)
            }
            KanteChip {
                visible: conv.voice.length > 0
                text: page.t("Klar", "Clear")
                checkable: true
                checked: conv.clear
                toolTip: page.t("Hebt die Konsonanten an, für leichteres Verstehen", "Lifts the consonants, for easier understanding")
                onClicked: conv.setSound(conv.louder, checked)
            }
            KanteButton {
                visible: !conv.finished && !conv.placing
                text: page.t("Später weiter", "Continue later")
                emphasis: KanteButton.Emphasis.Quiet
                Accessible.description: page.t("Das Gespräch bleibt offen und lässt sich auf der Startseite fortsetzen.", "The talk stays open; continue it from the start page.")
                onClicked: page.done()
            }
        }

        // Placement (/LF200/): no test, a talk that gets harder while it goes well.
        KanteCallout {
            Layout.fillWidth: true
            visible: conv.placing && !conv.placementAsk
            kind: KanteCallout.Kind.Info
            title: page.t("Einstufung, Stufe ", "Placement, stage ") + conv.stage
            text: page.t("Kein Test: Rede einfach, so gut es geht. Nach höchstens vier Antworten wird es eine Stufe schwerer, solange es gut läuft. Hilfe gibt es nur, wenn du sie holst. Alles zusammen dauert höchstens acht Minuten.",
                         "No test: just talk as well as you can. After at most four answers it gets one stage harder, as long as it goes well. Help comes only when you ask for it. All of it takes at most eight minutes.")
        }

        KanteSteps {
            visible: !page.placementOver
            Layout.fillWidth: true
            model: page.goals.map(function (g) { return g.text })
            current: page.currentGoal
        }

        Repeater {
            id: turnRepeater
            model: page.placementOver ? [] : page.turns
            delegate: KanteMessage {
                id: msg
                required property var modelData
                required property int index
                readonly property Item rubyLine: line
                Layout.fillWidth: true
                from: modelData.who === "learner" ? KanteMessage.From.Own : KanteMessage.From.Other
                author: modelData.who === "learner" ? (modelData.typed ? page.t("Du · getippt", "You · typed") : page.t("Du", "You")) : conv.character
                text: modelData.who === "learner" ? modelData.text : ""

                // /LF112/: a click on a word (or Tab, arrows, Enter) looks it up.
                KanteRubyText {
                    id: line
                    visible: modelData.who === "character"
                    Layout.fillWidth: true
                    segments: modelData.who === "character" && modelData.segments ? modelData.segments : []
                    markup: modelData.who === "character" && !modelData.segments ? modelData.markup : ""
                    density: KanteRubyText.Density.All
                    interactive: true
                    wordHint: page.t("Eingabe: nachschlagen", "Enter: look up")
                    onWordActivated: function (word) { wordPopup.show(line, msg.index, word) }
                }
                // /LF111/: again, slower, translated; each counts as help (/LF212/).
                RowLayout {
                    visible: msg.modelData.who === "character"
                    spacing: KanteStyle.unit(6)
                    KanteButton {
                        size: KanteButton.Size.Small
                        emphasis: KanteButton.Emphasis.Quiet
                        text: page.t("Nochmal", "Again")
                        enabled: conv.voice.length > 0 && conv.voiceOn
                        Accessible.description: page.t("Die Figur sagt den Satz noch einmal.", "The character says the line again.")
                        onClicked: conv.listen(msg.index, 0)
                    }
                    KanteButton {
                        size: KanteButton.Size.Small
                        emphasis: KanteButton.Emphasis.Quiet
                        text: page.t("Langsamer", "Slower")
                        enabled: conv.voice.length > 0 && conv.voiceOn
                        Accessible.description: page.t("Die Figur sagt den Satz langsamer, mit 0,7-facher Geschwindigkeit.", "The character says the line slower, at 0.7 times the speed.")
                        onClicked: conv.listen(msg.index, 1)
                    }
                    KanteButton {
                        size: KanteButton.Size.Small
                        emphasis: KanteButton.Emphasis.Quiet
                        text: page.t("Übersetzen", "Translate")
                        enabled: msg.modelData.translation === undefined
                        Accessible.description: page.t("Zeigt den Satz auf Deutsch.", "Shows the line in English.")
                        onClicked: conv.listen(msg.index, 2)
                    }
                }
                Text {
                    visible: msg.modelData.who === "character" && msg.modelData.translation !== undefined
                    Layout.fillWidth: true
                    text: msg.modelData.translation || ""
                    wrapMode: Text.Wrap
                    color: KanteStyle.mutedTextColor
                    font: KanteStyle.defaultFont
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
                        meta: modelData.disputed ? modelData.meta + page.t(" · umstritten", " · disputed") : modelData.meta
                        RowLayout {
                            spacing: KanteStyle.unit(6)
                            // /LF301/: the better form to hear.
                            KanteButton {
                                size: KanteButton.Size.Small
                                emphasis: KanteButton.Emphasis.Quiet
                                text: page.t("Anhören", "Listen")
                                enabled: conv.voice.length > 0 && conv.voiceOn
                                Accessible.description: page.t("Die Figur sagt die bessere Form: ", "The character says the better form: ") + modelData.better
                                onClicked: conv.sayText(modelData.better)
                            }
                            // /LF307/: the learner may say it was right; kept for the QA, no longer
                            // counted as a mistake.
                            KanteButton {
                                size: KanteButton.Size.Small
                                emphasis: KanteButton.Emphasis.Quiet
                                text: modelData.disputed ? page.t("Widersprochen", "Disputed") : page.t("Widersprechen", "Dispute")
                                enabled: !modelData.disputed
                                Accessible.description: page.t("Der Satz war richtig: Kaiwa merkt sich den Widerspruch und zählt es nicht als Fehler.", "The sentence was right: Kaiwa keeps the dispute and does not count it as a mistake.")
                                onClicked: conv.dispute(msg.index, modelData.better)
                            }
                        }
                    }
                }
                // /LF320/: a mora hint, cautious and without a severity; it is no mistake (K-02).
                Repeater {
                    model: modelData.who === "learner" && !modelData.misrecognized && modelData.mora ? modelData.mora : []
                    delegate: KanteHintCard {
                        required property var modelData
                        Layout.fillWidth: true
                        tier: KanteHintCard.Tier.None
                        tierText: modelData.tierText
                        titleText: modelData.title
                        text: modelData.text
                        meta: modelData.meta
                    }
                }
                // /LF113/: what Kaiwa heard; a misrecognition is no learning mistake.
                KanteButton {
                    visible: msg.modelData.who === "learner" && !msg.modelData.typed && !msg.modelData.pending && !msg.modelData.misrecognized
                    size: KanteButton.Size.Small
                    emphasis: KanteButton.Emphasis.Quiet
                    text: page.t("Falsch erkannt", "Misheard")
                    Accessible.description: page.t("So habe ich das nicht gesagt: Die Korrekturen zu diesem Zug zählen nicht.", "That is not what I said: the corrections of this turn do not count.")
                    onClicked: conv.markMisrecognized(msg.index)
                }
                Text {
                    visible: msg.modelData.misrecognized === true
                    text: page.t("Als falsch erkannt markiert. Das zählt nicht als Fehler.", "Marked as misheard. It does not count as a mistake.")
                    color: KanteStyle.mutedTextColor
                    font: KanteStyle.smallFont
                }
            }
        }

        // /LF101/: two turns sounded cut off by the pause; offer the next longer one.
        KanteCallout {
            Layout.fillWidth: true
            visible: conv.longerPause > 0
            kind: KanteCallout.Kind.Info
            title: page.t("Zu früh unterbrochen?", "Cut off too early?")
            text: page.t("Zweimal klang dein Satz, als wäre er noch nicht fertig. Mit einer längeren Pause hast du mehr Zeit zum Nachdenken.",
                         "Twice your sentence sounded unfinished. A longer pause gives you more time to think.")
            KanteButton {
                text: page.t("Pause auf ", "Pause to ") + String(conv.longerPause).replace(".", page.t(",", ".")) + " s"
                onClicked: conv.takeLongerPause()
            }
        }
        KanteCallout {
            Layout.fillWidth: true
            visible: conv.helpLevel > 0
            title: ["", page.t("Stichwort", "Keyword"), page.t("Satzanfang", "Sentence start"), page.t("Zum Nachsprechen", "To say after me")][conv.helpLevel]
            text: conv.helpText
        }
        KanteCallout {
            Layout.fillWidth: true
            visible: page.showSpeechHelp && conv.speechError.length > 0
            kind: KanteCallout.Kind.Warn
            title: page.t("Sprechen geht noch nicht", "Speaking does not work yet")
            text: conv.speechError
        }
        KanteCallout {
            Layout.fillWidth: true
            visible: page.showSpeechHelp && conv.voiceNote.length > 0
            title: page.t("Ohne Stimme", "No voice")
            text: conv.voiceNote
        }
        KanteCallout {
            Layout.fillWidth: true
            visible: conv.error.length > 0
            kind: KanteCallout.Kind.Danger
            title: page.t("Keine Antwort", "No answer")
            text: conv.error
        }

        // ── Placement: reading, then the result (/LF200/, /LF201/) ──
        ColumnLayout {
            visible: conv.placementAsk
            Layout.fillWidth: true
            spacing: KanteStyle.unit(10)
            KanteHeading { text: page.t("Noch zwei Fragen zum Lesen", "Two questions about reading"); level: 2 }
            Text {
                Layout.fillWidth: true
                text: page.t("Lesen kommt im Gespräch nicht vor. Sag selbst, wie es geht; das lässt sich im Profil jederzeit ändern.",
                             "Reading does not come up in a talk. Say how it goes; you can change it in the profile at any time.")
                color: KanteStyle.mutedTextColor
                font: KanteStyle.defaultFont
                wrapMode: Text.Wrap
            }
            Text { text: page.t("Hiragana und Katakana", "Hiragana and katakana"); color: KanteStyle.textColor; font: KanteStyle.defaultFont }
            KanteSegmented {
                id: kana
                model: [page.t("Noch nicht", "Not yet"), page.t("Einige, langsam", "Some, slowly"), page.t("Flüssig", "Fluently")]
                currentIndex: 0
                onActivated: function (i) { currentIndex = i }
            }
            Text { text: page.t("Kanji", "Kanji"); color: KanteStyle.textColor; font: KanteStyle.defaultFont }
            KanteSegmented {
                id: kanji
                model: [page.t("Noch nicht", "Not yet"), page.t("Einige, langsam", "Some, slowly"), page.t("Die von N5", "Those of N5")]
                currentIndex: 0
                onActivated: function (i) { currentIndex = i }
            }
            KanteButton {
                text: page.t("Ergebnis zeigen", "Show the result")
                emphasis: KanteButton.Emphasis.Primary
                onClicked: conv.finishPlacement(kana.currentIndex, kanji.currentIndex)
            }
        }
        ColumnLayout {
            visible: page.recorded !== null && page.recorded.placement !== undefined
            Layout.fillWidth: true
            spacing: KanteStyle.unit(10)
            KanteHeading { text: page.t("So steigst du ein", "Where you start"); level: 2 }
            Text {
                Layout.fillWidth: true
                text: page.recorded && page.recorded.stages
                    ? page.t(page.recorded.stages + " von 4 Stufen gespielt. Kaiwa stellt Tempo, Länge und Hilfen danach ein und passt sie nach jedem Gespräch weiter an.",
                             page.recorded.stages + " of 4 stages played. Kaiwa sets speed, length and help from this and keeps adjusting them after every talk.")
                    : ""
                color: KanteStyle.mutedTextColor
                font: KanteStyle.defaultFont
                wrapMode: Text.Wrap
            }
            Repeater {
                model: page.recorded && page.recorded.placement ? page.recorded.placement : []
                delegate: Text {
                    required property var modelData
                    Layout.fillWidth: true
                    text: "· " + modelData
                    color: KanteStyle.textColor
                    font: KanteStyle.defaultFont
                    wrapMode: Text.Wrap
                }
            }
            KanteButton {
                text: page.t("Los geht's", "Let's go")
                emphasis: KanteButton.Emphasis.Primary
                onClicked: page.done()
            }
        }

        // ── Summary (/LF310/) ──
        ColumnLayout {
            visible: conv.finished && page.summary !== null
            Layout.fillWidth: true
            spacing: KanteStyle.unit(10)
            KanteHeading { text: page.t("Bilanz", "Summary"); level: 2 }
            RowLayout {
                spacing: KanteStyle.unit(28)
                KanteKpi { label: page.t("Ziele", "Goals"); value: page.summary ? page.summary.goalsMet + " / " + page.summary.goalsTotal : "" }
                KanteKpi { label: page.t("Gesprochen", "Spoken"); value: page.summary ? String(page.summary.spoken) : "" }
                KanteKpi { label: page.t("Getippt", "Typed"); value: page.summary ? String(page.summary.typed) : "" }
                KanteKpi { label: page.t("Hilfen", "Helps"); value: page.summary ? String(page.summary.helps) : "" }
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
                text: page.t("In die Wiederholung: ", "Into the review: ") + (page.summary ? page.summary.review.join(" · ") : "")
                color: KanteStyle.mutedTextColor
                font: KanteStyle.defaultFont
                wrapMode: Text.Wrap
            }
            // What the session did for the learning model (/LF310/, /LF603/; XP only with
            // gamification, /LF605/).
            RowLayout {
                visible: page.recorded !== null
                spacing: KanteStyle.unit(28)
                KanteKpi { label: page.t("Minuten", "Minutes"); value: page.recorded ? String(page.recorded.minutes) : "" }
                KanteKpi { label: page.t("Neu zum Wiederholen", "New to review"); value: page.recorded ? String(page.recorded.newItems) : "" }
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
                visible: page.recorded !== null && page.recorded.scenarioDone === true
                text: page.t("Dieses Gespräch ist auf dem Lernpfad erledigt.", "This talk is done on the learning path.")
                color: KanteStyle.textColor
                font: KanteStyle.defaultFont
                wrapMode: Text.Wrap
            }
            KanteButton {
                text: page.t("Fertig", "Done")
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
                titles: [page.t("Sprechen", "Speak"), page.t("Hört zu", "Listening"), page.t("Denkt nach", "Thinking"), page.t("Spricht", "Speaking"), page.t("Kein Sprechen", "No speech")]
                // Hands-free (/LF100/): a click starts and ends, the pause ends the turn too.
                mode: conv.handsFree ? KanteTalkButton.Mode.Toggle : KanteTalkButton.Mode.Hold
                hints: conv.handsFree
                    ? [page.t("Klicken oder Leertaste", "Click or space"), page.t("Eine Pause beendet den Satz", "A pause ends the sentence"), "", page.t("Klicken zum Unterbrechen", "Click to interrupt"), page.t("Warum? Klicken", "Why? Click")]
                    : [page.t("Leertaste halten", "Hold space"), page.t("Loslassen zum Senden", "Release to send"), "", page.t("Tippen zum Unterbrechen", "Tap to interrupt"), page.t("Warum? Klicken", "Why? Click")]
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
                placeholderText: page.t("Auf Japanisch antworten …", "Answer in Japanese …")
                Accessible.name: page.t("Deine Antwort", "Your answer")
                onAccepted: page.send()
            }
            KanteButton {
                text: page.t("Hilfe", "Help")
                enabled: !conv.finished
                onClicked: conv.help()
            }
            KanteButton {
                text: page.t("Senden", "Send")
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
                else if (next.indexOf("read:") === 0) conv.finishPlacement(Number(next.charAt(5)), Number(next.charAt(6)))
                else if (next.indexOf("translate:") === 0) conv.listen(Number(next.substring(10)), 2)
                // "lookup:L:W" opens the lookup of word W in line L
                else if (next.indexOf("lookup:") === 0) {
                    var lw = next.substring(7).split(":")
                    var m = turnRepeater.itemAt(Number(lw[0]))
                    if (m) wordPopup.show(m.rubyLine, Number(lw[0]), Number(lw[1]))
                }
                else conv.say(next, true)
            } else {
                stop()
                grabber.start()
            }
        }
    }
    // /LL012/: the models go after the idle time set in the profile and come back when needed.
    Timer {
        interval: 60000
        repeat: true
        running: true
        onTriggered: conv.idle()
    }
    Timer {
        id: grabber
        interval: 600
        // An open lookup lives in the window's overlay, which cannot be grabbed: then its content.
        onTriggered: (wordPopup.opened ? wordPopup.contentItem : page).grabToImage(function (r) { r.saveToFile(page.shot); Qt.quit() })
    }

    // The word lookup (/LF112/): reading, meaning, part of speech and the target accent
    // (M-04, without a judgement), say it, take it into the review. Without the dictionary
    // the download is offered here, only after a click (M-03).
    QQC2.Popup {
        id: wordPopup
        property int line: -1
        property int word: -1
        property var info: null
        property bool added: false
        readonly property bool missing: info !== null && info.missing === true
        readonly property bool found: info !== null && info.found === true
        readonly property bool loading: page.kaiwa !== null && info !== null && info.download !== undefined
            && page.kaiwa.download.length > 0 && JSON.parse(page.kaiwa.download).dir === info.download.dir

        function reload() {
            info = JSON.parse(conv.lookup(line, word))
            added = false
        }
        function show(anchor, lineIndex, wordIndex) {
            line = lineIndex
            word = wordIndex
            reload()
            var r = anchor.wordRect(wordIndex)
            var p = anchor.mapToItem(parent, r.x, r.y + r.height)
            x = Math.max(0, Math.min(p.x, parent.width - width))
            y = p.y + KanteStyle.unit(4)
            open()
        }

        parent: QQC2.Overlay.overlay
        width: Math.min(KanteStyle.unit(420), parent ? parent.width - KanteStyle.unit(16) : KanteStyle.unit(420))
        padding: KanteStyle.unit(14)
        modal: false
        focus: true
        closePolicy: QQC2.Popup.CloseOnEscape | QQC2.Popup.CloseOnPressOutside

        Connections {
            target: page.kaiwa
            // The dictionary arrived: show the word now.
            function onDownloadChanged() {
                if (wordPopup.opened && wordPopup.missing && page.kaiwa.download.length === 0) wordPopup.reload()
            }
        }

        contentItem: ColumnLayout {
            spacing: KanteStyle.unit(8)
            KantePopupSkin { popup: wordPopup }

            KanteRubyText {
                Layout.fillWidth: true
                markup: !wordPopup.info ? "" : wordPopup.found && wordPopup.info.reading !== wordPopup.info.word
                    ? "{" + wordPopup.info.word + "|" + wordPopup.info.reading + "}" : (wordPopup.found ? wordPopup.info.word : wordPopup.info.text)
            }
            Text {
                visible: wordPopup.found && wordPopup.info.pos.length > 0
                Layout.fillWidth: true
                text: wordPopup.found ? wordPopup.info.pos.join(" · ") : ""
                color: KanteStyle.mutedTextColor
                font: KanteStyle.smallFont
                wrapMode: Text.Wrap
            }
            Repeater {
                model: wordPopup.found ? wordPopup.info.senses : []
                delegate: Text {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    text: (index + 1) + ". " + modelData.join(", ")
                    color: KanteStyle.textColor
                    font: KanteStyle.defaultFont
                    wrapMode: Text.Wrap
                }
            }
            Text {
                visible: wordPopup.found && wordPopup.info.fallback === true
                Layout.fillWidth: true
                text: page.t("Auf Englisch: JMdict hat für dieses Wort keine deutsche Bedeutung.", "")
                color: KanteStyle.mutedTextColor
                font: KanteStyle.smallFont
                wrapMode: Text.Wrap
            }
            KantePitchCurve {
                visible: wordPopup.found && wordPopup.info.accent !== null && wordPopup.info.accent !== undefined
                Layout.fillWidth: true
                Layout.preferredHeight: KanteStyle.unit(130)
                morae: visible ? wordPopup.info.accent.morae : []
                target: visible ? wordPopup.info.accent.target : []
                kernel: visible ? wordPopup.info.accent.kernel : -1
                targetLabel: page.t("Akzent", "Accent")
                kernelLabel: page.t("Akzentkern", "Accent kernel")
                flatText: page.t("flach, ohne Abfall", "flat, no fall")
                fallText: page.t("Abfall nach", "falls after")
            }
            Text {
                visible: wordPopup.info !== null && wordPopup.info.found === false
                Layout.fillWidth: true
                text: page.t("Dieses Wort steht nicht im Wörterbuch.", "This word is not in the dictionary.")
                color: KanteStyle.mutedTextColor
                font: KanteStyle.defaultFont
                wrapMode: Text.Wrap
            }
            Text {
                visible: wordPopup.missing
                Layout.fillWidth: true
                text: wordPopup.missing && wordPopup.info.download
                    ? page.t("Zum Nachschlagen braucht Kaiwa das Wörterbuch (" + wordPopup.info.download.name + ", " + wordPopup.info.download.licence + ", von " + wordPopup.info.download.origin + "): Download " + wordPopup.info.download.downloadMB + " MB, danach " + wordPopup.info.download.diskMB + " MB auf der Platte. Kaiwa lädt es nur, wenn du hier tippst; die Datei wird vor der Benutzung geprüft.",
                             "To look words up, Kaiwa needs the dictionary (" + wordPopup.info.download.name + ", " + wordPopup.info.download.licence + ", from " + wordPopup.info.download.origin + "): download " + wordPopup.info.download.downloadMB + " MB, then " + wordPopup.info.download.diskMB + " MB on disk. Kaiwa downloads it only when you tap here; the file is checked before it is used.")
                    : ""
                color: KanteStyle.textColor
                font: KanteStyle.defaultFont
                wrapMode: Text.Wrap
            }
            KanteProgressBar {
                visible: wordPopup.loading
                Layout.fillWidth: true
                value: {
                    if (!wordPopup.loading) return 0
                    var d = JSON.parse(page.kaiwa.download)
                    return d.total > 0 ? d.done / d.total : 0
                }
                Accessible.name: page.t("Download des Wörterbuchs", "Download of the dictionary")
            }
            RowLayout {
                spacing: KanteStyle.unit(8)
                KanteButton {
                    visible: wordPopup.found
                    size: KanteButton.Size.Small
                    text: page.t("Anhören", "Listen")
                    enabled: conv.voice.length > 0 && conv.voiceOn
                    Accessible.description: page.t("Die Figur sagt das Wort.", "The character says the word.")
                    onClicked: conv.sayWord(wordPopup.line, wordPopup.word)
                }
                KanteButton {
                    visible: wordPopup.found
                    size: KanteButton.Size.Small
                    emphasis: KanteButton.Emphasis.Primary
                    enabled: wordPopup.found && !wordPopup.info.inReview && !wordPopup.added
                    text: wordPopup.found && (wordPopup.info.inReview || wordPopup.added)
                        ? page.t("In der Wiederholung", "In the review") : page.t("In die Wiederholung", "Add to review")
                    onClicked: wordPopup.added = conv.remember(wordPopup.line, wordPopup.word) || wordPopup.added
                }
                KanteButton {
                    visible: wordPopup.missing && !wordPopup.loading && page.kaiwa !== null
                    size: KanteButton.Size.Small
                    emphasis: KanteButton.Emphasis.Primary
                    text: page.t("Wörterbuch herunterladen", "Download the dictionary")
                    onClicked: page.kaiwa.downloadModel(wordPopup.info.download.dir)
                }
                KanteButton {
                    visible: wordPopup.loading
                    size: KanteButton.Size.Small
                    emphasis: KanteButton.Emphasis.Quiet
                    text: page.t("Abbrechen", "Stop")
                    onClicked: page.kaiwa.stopDownload()
                }
            }
            Text {
                visible: wordPopup.found
                Layout.fillWidth: true
                text: "JMdict (EDRDG), Kanjium · CC BY-SA 4.0"
                color: KanteStyle.mutedTextColor
                font: KanteStyle.smallFont
                wrapMode: Text.Wrap
            }
        }
    }
}
