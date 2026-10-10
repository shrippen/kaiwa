import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Review (Lastenheft /LF230/): the due words and phrases from the learner's own talks, one at a
// time, to say after reading (or type). Speaking counts: the learner says it and marks how it
// went, or types it and Kaiwa compares. The listen-and-answer form follows with a voice that
// keeps the pitch accent (H-01).
QQC2.ScrollView {
    id: page
    required property var kaiwa
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var items: kaiwa.review.length > 0 ? JSON.parse(kaiwa.review) : []
    readonly property var current: items.length > 0 ? items[0] : null
    readonly property var answer: kaiwa.answer.length > 0 ? JSON.parse(kaiwa.answer) : null
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
            visible: page.current !== null
            barColor: KanteStyle.accentColor
            implicitHeight: card.implicitHeight + KanteStyle.unit(36)
            ColumnLayout {
                id: card
                anchors.fill: parent
                anchors.margins: KanteStyle.unit(18)
                spacing: KanteStyle.unit(10)
                KanteSectionLabel {
                    text: page.current
                        ? (page.current.fromMistake ? page.t("Aus deinen Korrekturen", "From your corrections") : page.t("Neues Wort", "New word"))
                          + " · " + page.t("noch ", "") + page.items.length + page.t("", " left")
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
