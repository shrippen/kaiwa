import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Review (Lastenheft /LF230/): the due words and phrases from the learner's own talks, one at a
// time, never as a flash card. A correction comes back as a mini conversation: the character
// says again the line the learner answered when the mistake happened, and the learner answers
// it with the better form, typed (Kaiwa looks for the form in the answer) or spoken (the form is
// shown after, the learner marks how it went). A hint shows the form first and counts as help
// (/LF212/). Words without such a line are said after reading, as before.
QQC2.ScrollView {
    id: page
    required property var kaiwa
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var items: kaiwa.review.length > 0 ? JSON.parse(kaiwa.review) : []
    readonly property var current: items.length > 0 ? items[0] : null
    // Everything due, as the start page and the tab count it; the list holds ten at a time.
    // Narrowed to one unit (its path node): the unit's own count.
    readonly property var scope: kaiwa.reviewScope.length > 0 ? JSON.parse(kaiwa.reviewScope) : null
    readonly property int remaining: scope ? scope.due : Math.max(items.length, kaiwa.today.length > 0 ? (JSON.parse(kaiwa.today).due || 0) : 0)
    // The item on screen by its text: every refresh parses a new object for the same item.
    readonly property string currentText: current ? current.text : ""
    readonly property var answer: kaiwa.answer.length > 0 ? JSON.parse(kaiwa.answer) : null
    // A mini conversation: the item has the character's line to answer.
    readonly property bool talk: current !== null && !!current.cue
    // The form is on screen: asked for as a hint (help) or shown after a spoken answer.
    property bool hinted: false
    property bool spoken: false
    onCurrentTextChanged: { hinted = false; spoken = false }
    contentWidth: availableWidth

    ColumnLayout {
        width: page.availableWidth
        spacing: KanteStyle.unit(14)

        Item { implicitHeight: KanteStyle.unit(4) }
        KanteHeading {
            Layout.leftMargin: KanteStyle.unit(18)
            text: page.t("Wiederholen", "Review")
            pageTitle: true
        }

        // Which unit this review is about, and the way back to everything due.
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: page.scope !== null
            spacing: KanteStyle.unit(8)
            KanteSectionLabel {
                Layout.fillWidth: true
                text: page.scope ? page.t("Wiederholung: ", "Review: ") + page.scope.title + " · " + page.t("noch ", "") + page.scope.due + page.t("", " left") : ""
            }
            KanteButton {
                text: page.t("Alles Fällige", "All due")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: page.kaiwa.openReview("")
            }
        }

        KanteEmptyState {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: page.current === null
            title: page.t("Nichts fällig", "Nothing due")
            text: page.t("Neue Wörter und deine Korrekturen kommen nach jedem Gespräch hierher, wenn es Zeit ist.",
                         "New words and your corrections come here after each talk, when it is time.")
        }

        KanteCard {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: page.current !== null && !page.talk
            barColor: KanteStyle.accentColor
            implicitHeight: card.implicitHeight + KanteStyle.unit(36)
            ColumnLayout {
                id: card
                anchors.fill: parent
                anchors.margins: KanteStyle.unit(18)
                spacing: KanteStyle.unit(10)
                KanteSectionLabel {
                    text: page.current
                        ? (page.current.fromMistake ? page.t("Aus deinen Korrekturen", "From your corrections")
                           : page.current.fromLookup ? page.t("Nachgeschlagen", "Looked up")
                           : page.t("Neues Wort", "New word"))
                          + " · " + page.t("noch ", "") + page.remaining + page.t("", " left")
                        : ""
                }
                KanteRubyText {
                    Layout.fillWidth: true
                    markup: page.current ? page.current.text : ""
                    density: KanteRubyText.Density.All
                }
                QQC2.Label {
                    Layout.fillWidth: true
                    text: page.t("Sprich es laut nach und sag, wie es ging. Oder tipp es.", "Say it out loud and tell how it went. Or type it.")
                    wrapMode: Text.Wrap
                    font: KanteStyle.smallFont
                    color: KanteStyle.mutedTextColor
                }
                RowLayout {
                    spacing: KanteStyle.unit(8)
                    KanteButton {
                        text: page.t("Ging gut", "Went well")
                        emphasis: KanteButton.Emphasis.Primary
                        onClicked: page.kaiwa.answerReview(page.current.text, page.current.text, false)
                    }
                    KanteButton {
                        text: page.t("Mit Mühe", "With effort")
                        onClicked: page.kaiwa.answerReview(page.current.text, page.current.text, true)
                    }
                    KanteButton {
                        text: page.t("Noch nicht", "Not yet")
                        emphasis: KanteButton.Emphasis.Quiet
                        onClicked: page.kaiwa.answerReview(page.current.text, "", false)
                    }
                }
                RowLayout {
                    spacing: KanteStyle.unit(8)
                    KanteTextField {
                        id: typed
                        Layout.fillWidth: true
                        placeholderText: page.t("Auf Japanisch tippen …", "Type in Japanese …")
                        Accessible.name: page.t("Deine Antwort", "Your answer")
                        onAccepted: check()
                        function check() {
                            if (text.length === 0) return
                            page.kaiwa.answerReview(page.current.text, text, false)
                            text = ""
                        }
                    }
                    KanteButton {
                        text: page.t("Prüfen", "Check")
                        enabled: typed.text.length > 0
                        onClicked: typed.check()
                    }
                }
            }
        }

        // The mini conversation (/LF230/).
        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: page.talk
            spacing: KanteStyle.unit(10)
            KanteSectionLabel {
                text: page.t("Mini-Gespräch", "Mini conversation")
                      + (page.current && page.current.practises ? " · " + page.current.practises : "")
                      + " · " + page.t("noch ", "") + page.remaining + page.t("", " left")
            }
            KanteMessage {
                Layout.fillWidth: true
                from: KanteMessage.From.Other
                author: page.t("Die Figur", "The character")
                KanteRubyText {
                    Layout.fillWidth: true
                    markup: page.talk ? page.current.cue : ""
                    density: KanteRubyText.Density.All
                }
            }
            QQC2.Label {
                Layout.fillWidth: true
                text: page.t("Antworte wie im Gespräch, mit der besseren Form aus deiner Korrektur. Sprich laut oder tipp.",
                             "Answer as in the talk, with the better form from your correction. Say it out loud or type.")
                wrapMode: Text.Wrap
                font: KanteStyle.smallFont
                color: KanteStyle.mutedTextColor
            }
            KanteCallout {
                Layout.fillWidth: true
                visible: page.hinted || page.spoken
                kind: KanteCallout.Kind.Info
                title: page.t("Die Form", "The form")
                text: page.current ? page.current.text : ""
            }
            RowLayout {
                spacing: KanteStyle.unit(8)
                visible: !page.spoken
                KanteTextField {
                    id: reply
                    Layout.fillWidth: true
                    placeholderText: page.t("Auf Japanisch antworten …", "Answer in Japanese …")
                    Accessible.name: page.t("Deine Antwort", "Your answer")
                    onAccepted: check()
                    function check() {
                        if (text.length === 0) return
                        page.kaiwa.answerReview(page.current.text, text, page.hinted)
                        text = ""
                    }
                }
                KanteButton {
                    text: page.t("Prüfen", "Check")
                    enabled: reply.text.length > 0
                    onClicked: reply.check()
                }
            }
            RowLayout {
                spacing: KanteStyle.unit(8)
                visible: !page.spoken
                KanteButton {
                    text: page.t("Gesagt", "Said it")
                    emphasis: KanteButton.Emphasis.Primary
                    Accessible.description: page.t("Zeigt die Form, dann sagst du, wie es ging.", "Shows the form, then you tell how it went.")
                    onClicked: page.spoken = true
                }
                KanteButton {
                    text: page.t("Tipp", "Hint")
                    emphasis: KanteButton.Emphasis.Quiet
                    enabled: !page.hinted
                    Accessible.description: page.t("Zeigt die Form vorher; zählt als Hilfe.", "Shows the form first; counts as help.")
                    onClicked: page.hinted = true
                }
            }
            RowLayout {
                spacing: KanteStyle.unit(8)
                visible: page.spoken
                KanteButton {
                    text: page.t("Ging gut", "Went well")
                    emphasis: KanteButton.Emphasis.Primary
                    onClicked: page.kaiwa.answerReview(page.current.text, page.current.text, page.hinted)
                }
                KanteButton {
                    text: page.t("Mit Mühe", "With effort")
                    onClicked: page.kaiwa.answerReview(page.current.text, page.current.text, true)
                }
                KanteButton {
                    text: page.t("Noch nicht", "Not yet")
                    emphasis: KanteButton.Emphasis.Quiet
                    onClicked: page.kaiwa.answerReview(page.current.text, "", false)
                }
            }
        }

        KanteCallout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: page.answer !== null
            kind: page.answer && page.answer.right ? KanteCallout.Kind.Ok : KanteCallout.Kind.Info
            title: page.answer && page.answer.right ? page.t("Gut", "Good") : page.t("Noch einmal später", "Again later")
            text: page.answer
                ? (page.answer.right ? page.t("Wieder dran am ", "Back on ") + page.answer.next
                                     : page.t("So heißt es: ", "It goes: ") + page.answer.expected + page.t(". Morgen kommt es wieder.", ". It comes back tomorrow."))
                : ""
        }
        Item { implicitHeight: KanteStyle.unit(12) }
    }
}
