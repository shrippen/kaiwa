import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Today (Lastenheft Kap. 7): an interrupted session to continue (/LF121/), the daily goal and
// the streak (/LF601/, /LF602/, without either when gamification is off, /LF605/), the next
// scenario on the path and the reviews that are due (/LF230/, /LF231/).
QQC2.ScrollView {
    id: page
    required property var kaiwa
    signal talk(string scenario, bool resume)
    signal showTab(int index)
    signal place()
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var today: kaiwa.today.length > 0 ? JSON.parse(kaiwa.today) : ({})
    contentWidth: availableWidth

    ColumnLayout {
        width: page.availableWidth
        spacing: KanteStyle.unit(14)

        Item { implicitHeight: KanteStyle.unit(4) }

        KanteHeading {
            Layout.leftMargin: KanteStyle.unit(18)
            text: page.today.name ? page.t("Hallo, " + page.today.name, "Hello, " + page.today.name) : page.t("Heute", "Today")
            pageTitle: true
        }

        // Without a tutor there are no conversations (Q-15): the way to set one up comes first.
        KanteCallout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: page.today.needsTutor === true
            kind: KanteCallout.Kind.Warn
            title: page.t("Noch kein Tutor", "No tutor yet")
            text: page.t("Die Figuren spielt ein Sprachmodell: lokal auf deinem Rechner (Ollama, LM Studio) oder in der Cloud (Anthropic, OpenRouter und andere). Richte es in den Einstellungen unter „Tutor“ ein.",
                         "A language model plays the characters: locally on your computer (Ollama, LM Studio) or in the cloud (Anthropic, OpenRouter and others). Set it up in the settings under Tutor.")
        }
        KanteButton {
            Layout.leftMargin: KanteStyle.unit(18)
            visible: page.today.needsTutor === true
            text: page.t("Tutor einrichten", "Set up a tutor")
            emphasis: KanteButton.Emphasis.Primary
            onClicked: page.showTab(4)
        }

        // Placement (/LF200/): a short talk that gets harder while it goes well, or start at
        // zero; both can be corrected later in the profile (/LF202/).
        KanteCallout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: page.today.placed === false
            title: page.t("Erst einmal ankommen", "Getting started")
            text: page.t("Kannst du schon etwas Japanisch? Ein kurzes Gespräch (höchstens 8 Minuten) zeigt, wo du einsteigst. Kein Test, keine Note. Wer bei null anfängt, überspringt es.",
                         "Do you know some Japanese already? A short talk (8 minutes at most) shows where you start. No test, no grade. Starting at zero skips it.")
        }
        RowLayout {
            Layout.leftMargin: KanteStyle.unit(18)
            visible: page.today.placed === false
            spacing: KanteStyle.unit(8)
            KanteButton {
                text: page.t("Einstufen", "Find my level")
                emphasis: KanteButton.Emphasis.Primary
                onClicked: page.place()
            }
            KanteButton {
                text: page.t("Bei null anfangen", "Start at zero")
                onClicked: page.kaiwa.skipPlacement()
            }
        }

        // Continue where it stopped (/LF121/).
        KanteCallout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: !!page.today.open
            kind: KanteCallout.Kind.Info
            title: page.t("Weiter, wo du warst", "Continue where you were")
            text: page.today.open ? page.today.open.title : ""
        }
        KanteButton {
            Layout.leftMargin: KanteStyle.unit(18)
            visible: !!page.today.open
            text: page.t("Fortsetzen", "Continue")
            emphasis: KanteButton.Emphasis.Primary
            onClicked: page.talk("", true)
        }

        // The daily goal; the meter and the streak only with gamification.
        RowLayout {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            spacing: KanteStyle.unit(16)
            KanteGoalMeter {
                visible: page.today.gamification === true
                value: page.today.goalFraction || 0
                label: page.today.gamification ? page.t(page.today.streakDays + " T", page.today.streakDays + " d") : ""
                accessibleName: page.t("Tagesziel", "Daily goal")
                stackHeight: KanteStyle.unit(44)
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: KanteStyle.unit(2)
                KanteSectionLabel { text: page.t("Tagesziel", "Daily goal") }
                QQC2.Label {
                    text: (page.today.goalText || "") + (page.today.goalMet ? page.t(" · erreicht", " · reached") : "")
                    font: KanteStyle.defaultFont
                    color: KanteStyle.strongTextColor
                }
                QQC2.Label {
                    visible: text.length > 0
                    Layout.fillWidth: true
                    text: page.today.streakNote || ""
                    wrapMode: Text.Wrap
                    font: KanteStyle.smallFont
                    color: KanteStyle.mutedTextColor
                }
            }
            KanteKpi {
                visible: page.today.gamification === true
                value: String(page.today.xpToday || 0)
                label: "XP"
            }
        }

        // The next scenario.
        KanteCard {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: !!page.today.next
            barColor: KanteStyle.accentColor
            implicitHeight: next.implicitHeight + KanteStyle.unit(36)
            ColumnLayout {
                id: next
                anchors.fill: parent
                anchors.margins: KanteStyle.unit(18)
                spacing: KanteStyle.unit(6)
                KanteSectionLabel { text: page.t("Als Nächstes", "Next") + (page.today.next ? " · " + page.today.next.unit : "") }
                KanteHeading { text: page.today.next ? page.today.next.title : "" }
                QQC2.Label {
                    Layout.fillWidth: true
                    text: page.today.next ? page.today.next.canDo : ""
                    wrapMode: Text.Wrap
                    font: KanteStyle.defaultFont
                    color: KanteStyle.textColor
                }
                KanteButton {
                    text: page.t("Gespräch starten", "Start talking")
                    emphasis: KanteButton.Emphasis.Primary
                    onClicked: page.talk(page.today.next.id, false)
                }
            }
        }

        // Free talk on a topic (/LF116/), outside the path, at the learner's difficulty.
        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: (page.today.free || []).length > 0
            spacing: KanteStyle.unit(8)
            KanteSectionLabel { text: page.t("Freies Gespräch", "Free talk") }
            Flow {
                Layout.fillWidth: true
                spacing: KanteStyle.unit(8)
                Repeater {
                    model: page.today.free || []
                    delegate: KanteButton {
                        required property var modelData
                        text: modelData.topic
                        Accessible.description: page.t("Freies Gespräch über ", "Free talk about ") + modelData.topic
                        onClicked: page.talk(modelData.id, false)
                    }
                }
            }
        }

        // Reviews (/LF230/); after a break a short refresher (/LF231/).
        KanteCallout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            visible: (page.today.due || 0) > 0
            kind: KanteCallout.Kind.Info
            title: page.today.refresher ? page.t("Willkommen zurück", "Welcome back") : page.t("Wiederholen", "Review")
            text: page.today.refresher
                ? page.t("Eine kurze Auffrischung mit höchstens zehn Wörtern, dann geht es weiter.", "A short refresher of at most ten words, then on we go.")
                : page.t(page.today.due + " Wörter und Wendungen sind fällig.", page.today.due + " words and phrases are due.")
        }
        KanteButton {
            Layout.leftMargin: KanteStyle.unit(18)
            visible: (page.today.due || 0) > 0
            text: page.t("Wiederholen", "Review")
            onClicked: page.showTab(2)
        }
        Item { implicitHeight: KanteStyle.unit(12) }
    }
}
