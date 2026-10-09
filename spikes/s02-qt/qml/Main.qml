import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante
import org.shrippen.kaiwa.spike

// Spike 0.2: Kirigami + Kante + a Rust QObject (Session). Press the talk button (or hold
// Space): Rust switches to Listening, then Thinking; a worker thread answers and the
// character speaks. The segmented control switches System / Kante / Kante Light.
Kirigami.ApplicationWindow {
    id: root
    width: 760
    height: 620
    visible: true
    title: "Kaiwa · Spike 0.2"

    readonly property var args: Qt.application.arguments
    function arg(name, fallback) {
        var i = args.indexOf(name)
        return i >= 0 && i + 1 < args.length ? args[i + 1] : fallback
    }
    property int styleKind: Number(arg("--kind", "1"))
    Binding { target: KanteStyle; property: "kind"; value: root.styleKind }
    Binding { target: KanteStyle; property: "preferDark"; value: root.arg("--light", "0") !== "1" }

    Session { id: session }

    pageStack.initialPage: Kirigami.Page {
        id: page
        title: "An der Konbini"
        background: Rectangle { color: KanteStyle.backgroundColor }
        KanteScope { target: page }

        ColumnLayout {
            anchors.fill: parent
            spacing: KanteStyle.unit(12)

            RowLayout {
                Layout.fillWidth: true
                KanteHeading { text: "An der Konbini"; Layout.fillWidth: true }
                KanteSegmented {
                    model: ["System", "Kante", "Light"]
                    currentIndex: root.styleKind
                    onActivated: function (i) { root.styleKind = i }
                }
            }
            KanteMessage {
                Layout.fillWidth: true
                author: "店員 · Kassiererin"; time: "12:04"
                KanteRubyText { Layout.fillWidth: true; markup: "いらっしゃいませ。ふくろはいりますか。" }
            }
            KanteMessage {
                Layout.fillWidth: true
                visible: session.heard.length > 0
                from: KanteMessage.From.Own; time: "12:05"; text: session.heard + "。"
                KanteHeardLine { Layout.fillWidth: true; text: session.heard }
            }
            KanteMessage {
                Layout.fillWidth: true
                visible: session.reply.length > 0
                author: "店員 · Kassiererin"; time: "12:05"
                KanteRubyText { Layout.fillWidth: true; markup: session.reply; density: KanteRubyText.Density.New }
            }
            Item { Layout.fillHeight: true }
            RowLayout {
                Layout.fillWidth: true
                spacing: KanteStyle.unit(12)
                KanteTalkButton {
                    id: talk
                    talkState: session.talkState
                    level: session.level
                    onTalkStarted: session.start()
                    onTalkEnded: session.stop()
                    onInterruptRequested: session.interrupt()
                    Component.onCompleted: forceActiveFocus()
                }
                Item { Layout.fillWidth: true }
                KanteGoalMeter { value: 0.6; label: "12 T" }
            }
        }
    }

    // --shot <file>: one conversation turn, then a screenshot of the window, then quit.
    readonly property string shot: arg("--shot", "")
    Timer {
        running: root.shot.length > 0
        interval: 400
        onTriggered: { session.start(); session.stop() }
    }
    Timer {
        running: root.shot.length > 0
        interval: 1700
        onTriggered: page.grabToImage(function (r) { r.saveToFile(root.shot); Qt.quit() })
    }
}
