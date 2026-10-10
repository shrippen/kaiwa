import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import "Kante"
import "KantePlasma"

// The expanded plasmoid: today (goal, streak, what comes next), a short talk of two goals
// (/LF501/) with corrections in one line each and "see it in the app" (/LF503/), and five
// reviews (/LF502/). Typed for now; push-to-talk follows with the shared audio path (/LF505/).
ColumnLayout {
    id: full

    required property var kaiwad
    required property var app
    function t(de, en) { return app.t(de, en) }

    Layout.preferredWidth: Kirigami.Units.gridUnit * 22
    Layout.preferredHeight: Kirigami.Units.gridUnit * 26
    spacing: Kirigami.Units.smallSpacing * 2

    /** "home", "talk" or "review". */
    property string mode: "home"
    property var talk: null
    property var lines: []
    property var reviews: []
    property var answer: null
    property bool busy: false
    readonly property var today: app.today
    readonly property var current: reviews.length > 0 ? reviews[0] : null

    KanteScope { target: full }

    function startShort() {
        busy = true
        kaiwad.call("StartShort", "", [], function (v) {
            busy = false
            talk = v
            lines = [{ who: "character", markup: v.last || v.opening }]
            mode = "talk"
        })
    }
    function send() {
        var text = input.text.trim()
        if (text.length === 0 || busy) return
        input.text = ""
        busy = true
        lines = lines.concat([{ who: "learner", text: text, corrections: [] }])
        kaiwad.call("Say", "(sb)", [kaiwad.str(text), kaiwad.bool(true)], function (v) {
            busy = false
            talk = v
            var l = lines.slice()
            l[l.length - 1] = { who: "learner", text: v.said, corrections: v.corrections }
            l.push({ who: "character", markup: v.reply })
            lines = l
            app.refresh()
        })
    }
    function startReviews() {
        answer = null
        kaiwad.call("Reviews", "(u)", [kaiwad.uint(5)], function (v) {
            reviews = v || []
            mode = "review"
        })
    }
    function check() {
        if (!current) return
        kaiwad.call("AnswerReview", "(ssb)", [kaiwad.str(current.text), kaiwad.str(reviewInput.text), kaiwad.bool(false)], function (v) {
            answer = v
            app.refresh()
        })
    }
    function nextReview() {
        reviews = reviews.slice(1)
        answer = null
        reviewInput.text = ""
    }

    KantePlasmaHeading {
        Layout.fillWidth: true
        level: 2
        text: full.mode === "talk" && full.talk ? full.talk.title
            : full.mode === "review" ? full.t("Wiederholen", "Review") : "Kaiwa"
    }

    KanteCallout {
        Layout.fillWidth: true
        visible: !full.kaiwad.reachable
        kind: KanteCallout.Kind.Warn
        title: full.t("Der Kaiwa-Dienst antwortet nicht", "The Kaiwa service does not answer")
        text: full.t("Das Plasmoid braucht kaiwad aus dem Kaiwa-Paket; er startet über D-Bus von selbst. ",
                     "The plasmoid needs kaiwad from the Kaiwa package; it starts by itself over D-Bus. ") + full.kaiwad.lastError
    }

    // ── Today ──
    ColumnLayout {
        visible: full.mode === "home" && full.today !== null
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing * 2
        RowLayout {
            spacing: Kirigami.Units.largeSpacing
            KanteGoalMeter {
                value: full.today ? full.today.goalFraction : 0
                label: full.today && full.today.gamification ? full.today.streakDays + full.t(" T", " d") : ""
                stackHeight: Kirigami.Units.gridUnit * 2
                accessibleName: full.t("Tagesziel", "Daily goal")
            }
            ColumnLayout {
                spacing: 0
                Text { text: full.today ? full.today.goalText : ""; color: KanteStyle.textColor; font: KanteStyle.defaultFont }
                Text {
                    visible: text.length > 0
                    text: full.today ? full.today.streakNote : ""
                    color: KanteStyle.mutedTextColor
                    font: KanteStyle.smallFont
                }
            }
        }
        Text {
            Layout.fillWidth: true
            visible: full.today !== null && !!full.today.next
            text: full.today && full.today.next ? full.t("Als Nächstes: ", "Next: ") + full.today.next.title : ""
            color: KanteStyle.textColor
            font: KanteStyle.defaultFont
            wrapMode: Text.Wrap
        }
        KantePlasmaButton {
            Layout.fillWidth: true
            emphasis: KantePlasmaButton.Emphasis.Primary
            busy: full.busy
            text: full.today && full.today.open ? full.t("Gespräch fortsetzen", "Continue the talk") : full.t("Kurzgespräch, 3–5 min", "Short talk, 3–5 min")
            onClicked: full.startShort()
        }
        KantePlasmaButton {
            Layout.fillWidth: true
            enabled: full.today !== null && full.today.due > 0
            text: full.today && full.today.due > 0
                ? full.t(Math.min(5, full.today.due) + " Wiederholungen", Math.min(5, full.today.due) + " reviews")
                : full.t("Nichts zu wiederholen", "Nothing to review")
            onClicked: full.startReviews()
        }
    }

    // ── Short talk ──
    ColumnLayout {
        visible: full.mode === "talk"
        Layout.fillWidth: true
        Layout.fillHeight: true
        spacing: Kirigami.Units.smallSpacing * 2
        Repeater {
            model: full.talk ? full.talk.goals : []
            delegate: Text {
                required property var modelData
                Layout.fillWidth: true
                text: (modelData.done ? full.t("erledigt · ", "done · ") : "") + modelData.text
                color: modelData.done ? KanteStyle.mutedTextColor : KanteStyle.textColor
                font: KanteStyle.smallFont
                wrapMode: Text.Wrap
            }
        }
        QQC2.ScrollView {
            id: scroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            ColumnLayout {
                width: scroll.availableWidth
                spacing: Kirigami.Units.smallSpacing * 2
                Repeater {
                    model: full.lines
                    delegate: ColumnLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 0
                        Text {
                            text: modelData.who === "learner" ? full.t("Du", "You") : (full.talk && full.talk.character ? full.talk.character : "")
                            color: KanteStyle.mutedTextColor
                            font: KanteStyle.smallFont
                        }
                        KanteRubyText {
                            Layout.fillWidth: true
                            visible: modelData.who === "character"
                            markup: modelData.who === "character" ? modelData.markup : ""
                            density: KanteRubyText.Density.All
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: modelData.who === "learner"
                            text: modelData.text || ""
                            color: KanteStyle.textColor
                            font: KanteStyle.defaultFont
                            wrapMode: Text.Wrap
                        }
                        // One line per correction (/LF503/); the full card is in the app.
                        Repeater {
                            model: modelData.corrections || []
                            delegate: Text {
                                required property var modelData
                                Layout.fillWidth: true
                                text: modelData
                                color: KanteStyle.warningColor
                                font: KanteStyle.smallFont
                                elide: Text.ElideRight
                            }
                        }
                    }
                }
            }
        }
        Text {
            Layout.fillWidth: true
            visible: full.talk !== null && full.talk.done === true
            text: full.talk && full.talk.recorded && full.talk.recorded.xp !== undefined
                ? full.t("Geschafft. +" + full.talk.recorded.xp + " XP, im Profil gespeichert.", "Done. +" + full.talk.recorded.xp + " XP, kept in the profile.")
                : full.t("Geschafft.", "Done.")
            color: KanteStyle.textColor
            font: KanteStyle.defaultFont
            wrapMode: Text.Wrap
        }
        RowLayout {
            Layout.fillWidth: true
            visible: !(full.talk && full.talk.done)
            KanteTextField {
                id: input
                Layout.fillWidth: true
                enabled: !full.busy
                placeholderText: full.t("Auf Japanisch antworten …", "Answer in Japanese …")
                onAccepted: full.send()
            }
            KantePlasmaButton {
                text: full.t("Senden", "Send")
                emphasis: KantePlasmaButton.Emphasis.Primary
                busy: full.busy
                enabled: !full.busy && input.text.length > 0
                onClicked: full.send()
            }
        }
        RowLayout {
            KantePlasmaToolButton {
                text: full.t("In der App ansehen", "See it in the app")
                icon.name: "window-new"
                onClicked: full.kaiwad.call("OpenApp", "", [], null)
            }
            Item { Layout.fillWidth: true }
            KantePlasmaToolButton {
                text: full.talk && full.talk.done ? full.t("Fertig", "Done") : full.t("Später weiter", "Continue later")
                onClicked: { full.mode = "home"; full.app.refresh() }
            }
        }
    }

    // ── Five reviews ──
    ColumnLayout {
        visible: full.mode === "review"
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing * 2
        Text {
            text: full.current ? full.t("Noch " + full.reviews.length + " · zum Nachsprechen oder Tippen", full.reviews.length + " left · say it or type it")
                : full.t("Alles wiederholt.", "All reviewed.")
            color: KanteStyle.mutedTextColor
            font: KanteStyle.smallFont
        }
        KanteRubyText {
            Layout.fillWidth: true
            visible: full.current !== null
            markup: full.current ? full.current.text : ""
            font.pixelSize: Kirigami.Units.gridUnit * 1.4
        }
        RowLayout {
            Layout.fillWidth: true
            visible: full.current !== null && full.answer === null
            KanteTextField {
                id: reviewInput
                Layout.fillWidth: true
                placeholderText: full.t("Tippen (Rōmaji geht auch) …", "Type it (rōmaji works too) …")
                onAccepted: full.check()
            }
            KantePlasmaButton { text: full.t("Prüfen", "Check"); onClicked: full.check() }
        }
        Text {
            Layout.fillWidth: true
            visible: full.answer !== null
            text: full.answer ? (full.answer.right ? full.t("Richtig. Wieder am ", "Right. Again on ") + full.answer.next
                                                    : full.t("Fast: ", "Nearly: ") + full.answer.expected) : ""
            color: full.answer && full.answer.right ? KanteStyle.positiveTextColor : KanteStyle.textColor
            font: KanteStyle.defaultFont
            wrapMode: Text.Wrap
        }
        RowLayout {
            KantePlasmaButton {
                visible: full.answer !== null
                text: full.t("Weiter", "Next")
                emphasis: KantePlasmaButton.Emphasis.Primary
                onClicked: full.nextReview()
            }
            Item { Layout.fillWidth: true }
            KantePlasmaToolButton { text: full.t("Zurück", "Back"); onClicked: full.mode = "home" }
        }
    }

    Item { Layout.fillHeight: full.mode !== "talk" }
}
