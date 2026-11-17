import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante
import "lang.js" as Lang

// Today (Lastenheft Kap. 7): an interrupted session to continue (/LF121/), the next scenario
// on the path, the reviews that are due (/LF230/, /LF231/) and free talk on the left; the daily
// goal, the streak and XP (/LF601/, /LF602/, without them when gamification is off, /LF605/)
// and the week's minutes on the right. In a compact window the columns stack.
QQC2.ScrollView {
    id: page
    required property var kaiwa
    signal talk(string scenario, bool resume)
    signal showTab(int index)
    signal place()
    function t(de, en) { return Lang.pick(kaiwa.lang, de, en) }

    readonly property var today: kaiwa.today.length > 0 ? JSON.parse(kaiwa.today) : ({})
    readonly property bool compact: applicationWindow().compact
    readonly property var week: today.week || ({ labels: [], minutes: [], met: [], today: -1 })
    // One stacked bar per day, its minutes in the part that gives the colour: not met, met, today.
    readonly property var weekBars: week.labels.map((_, i) => {
        const m = week.minutes[i]
        return i === week.today ? [0, 0, m] : week.met[i] ? [0, m, 0] : [m, 0, 0]
    })
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
                onClicked: page.kaiwa.leaveOutPlacement()
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

        GridLayout {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            columns: page.compact ? 1 : 2
            columnSpacing: KanteStyle.unit(24)
            rowSpacing: KanteStyle.unit(14)

            // Left: what to do.
            ColumnLayout {
                Layout.fillWidth: true
                Layout.preferredWidth: KanteStyle.unit(17)
                Layout.alignment: Qt.AlignTop
                spacing: KanteStyle.unit(14)

                // The next scenario.
                KanteCard {
                    Layout.fillWidth: true
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

                // Reviews (/LF230/); after a break a short refresher (/LF231/).
                KanteCard {
                    Layout.fillWidth: true
                    visible: (page.today.due || 0) > 0
                    barColor: KanteStyle.infoColor
                    implicitHeight: review.implicitHeight + KanteStyle.unit(28)
                    RowLayout {
                        id: review
                        anchors.fill: parent
                        anchors.margins: KanteStyle.unit(14)
                        spacing: KanteStyle.unit(16)
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: KanteStyle.unit(2)
                            KanteHeading {
                                level: 3
                                text: page.today.refresher ? page.t("Willkommen zurück", "Welcome back") : page.t("Wiederholen", "Review")
                            }
                            QQC2.Label {
                                Layout.fillWidth: true
                                wrapMode: Text.Wrap
                                font: KanteStyle.defaultFont
                                color: KanteStyle.textColor
                                text: page.today.refresher
                                    ? page.t("Eine kurze Auffrischung mit höchstens zehn Wörtern, dann geht es weiter. Was sonst fällig war, kommt in den nächsten Tagen.", "A short refresher of at most ten words, then on we go. What else was due comes over the next days.")
                                    : page.t(page.today.due + " Wörter und Wendungen sind fällig, etwa " + page.today.reviewMinutes + " Minuten.",
                                             page.today.due + " words and phrases are due, about " + page.today.reviewMinutes + " minutes.")
                            }
                        }
                        KanteButton {
                            text: page.t("Jetzt wiederholen", "Review now")
                            onClicked: page.showTab(2)
                        }
                    }
                }

                // Free talk on a topic (/LF116/), outside the path, at the learner's difficulty.
                ColumnLayout {
                    Layout.fillWidth: true
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
            }

            // Right: where I stand today; the figures and the chart only with gamification.
            ColumnLayout {
                Layout.fillWidth: true
                Layout.preferredWidth: KanteStyle.unit(10)
                Layout.alignment: Qt.AlignTop
                spacing: KanteStyle.unit(14)

                KanteCard {
                    Layout.fillWidth: true
                    implicitHeight: goal.implicitHeight + KanteStyle.unit(32)
                    ColumnLayout {
                        id: goal
                        anchors.fill: parent
                        anchors.margins: KanteStyle.unit(16)
                        spacing: KanteStyle.unit(8)
                        KanteSectionLabel { text: page.t("Tagesziel", "Daily goal") }
                        KanteKpi {
                            value: String(page.today.goalSpoken || 0)
                            label: (page.today.goalRest || "") + (page.today.goalMet ? page.t(" · erreicht", " · reached") : "")
                        }
                        KanteProgressBar {
                            Layout.fillWidth: true
                            visible: page.today.gamification === true
                            value: page.today.goalFraction || 0
                            Accessible.name: page.t("Tagesziel", "Daily goal")
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
                }

                RowLayout {
                    Layout.fillWidth: true
                    visible: page.today.gamification === true
                    spacing: KanteStyle.unit(14)
                    KanteCard {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.preferredWidth: 1
                        implicitHeight: streak.implicitHeight + KanteStyle.unit(28)
                        ColumnLayout {
                            id: streak
                            anchors.fill: parent
                            anchors.margins: KanteStyle.unit(14)
                            spacing: KanteStyle.unit(4)
                            KanteSectionLabel { text: page.t("Serie", "Streak") }
                            // Part of the row that shows only with gamification (/LF605/).
                            KanteKpi {
                                value: String(page.today.streakDays || 0)
                                label: page.t("Tage", "days")
                            }
                        }
                    }
                    KanteCard {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.preferredWidth: 1
                        implicitHeight: xp.implicitHeight + KanteStyle.unit(28)
                        ColumnLayout {
                            id: xp
                            anchors.fill: parent
                            anchors.margins: KanteStyle.unit(14)
                            spacing: KanteStyle.unit(4)
                            KanteSectionLabel { text: "XP" }
                            KanteKpi {
                                value: String(page.today.xpToday || 0)
                                label: page.t("heute", "today")
                            }
                        }
                    }
                }

                // Minutes per day of this week against the goal; the legend names the colours.
                KanteCard {
                    Layout.fillWidth: true
                    visible: page.today.gamification === true
                    implicitHeight: weekBox.implicitHeight + KanteStyle.unit(28)
                    ColumnLayout {
                        id: weekBox
                        anchors.fill: parent
                        anchors.margins: KanteStyle.unit(14)
                        spacing: KanteStyle.unit(8)
                        KanteSectionLabel { text: page.t("Diese Woche · Minuten je Tag", "This week · minutes per day") }
                        KanteBarChart {
                            objectName: "weekChart"
                            Layout.fillWidth: true
                            Layout.preferredHeight: KanteStyle.unit(150)
                            values: page.weekBars
                            labels: page.week.labels
                            stackColors: [KanteStyle.frameColor, KanteStyle.positiveTextColor, KanteStyle.accentColor]
                            goal: page.today.goalValue || -1
                            axis: true
                            wholeSteps: true
                            formatter: v => Math.round(v) + " min"
                            Accessible.name: page.t("Minuten je Tag in dieser Woche", "Minutes per day this week")
                        }
                        KanteLegend {
                            Layout.fillWidth: true
                            model: [
                                { color: KanteStyle.accentColor, text: page.t("heute", "today") },
                                { color: KanteStyle.positiveTextColor, text: page.t("Ziel erreicht", "goal reached") },
                                { color: KanteStyle.frameColor, text: page.t("Ziel nicht erreicht", "goal not reached") },
                                { color: KanteStyle.accentColor, text: page.t("Ziellinie (gestrichelt)", "goal line (dashed)") }
                            ]
                        }
                    }
                }
            }
        }
        Item { implicitHeight: KanteStyle.unit(12) }
    }
}
