import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante
import org.shrippen.kaiwa

// Kaiwa's window (Lastenheft Kap. 7): tabs for today, the learning path, review, profile and
// settings; a conversation opens as its own page on top. Style follows the Kante rule:
// System by default, Kante and Kante Light from the settings.
//
// Screenshot mode: --shot out.png [--view start|path|review|profile|settings|talk|place]
// [--scenario id] [--resume] [--say "a|b|wav:file|?|!|~|read:KK|translate:N"] [--kind 0|1|2]
// [--light 0|1].
Kirigami.ApplicationWindow {
    id: root
    width: Number(arg("--width", "900"))
    height: Number(arg("--height", "760"))
    visible: true
    title: "Kaiwa"

    readonly property var args: Qt.application.arguments
    function arg(name, fallback) {
        var i = args.indexOf(name)
        return i >= 0 && i + 1 < args.length ? args[i + 1] : fallback
    }
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    KaiwaApp {
        id: kaiwa
        Component.onCompleted: refresh()
    }
    readonly property var profile: kaiwa.profile.length > 0 ? JSON.parse(kaiwa.profile) : ({})

    readonly property int styleKind: arg("--kind", "") !== "" ? Number(arg("--kind", "0")) : (profile.style || 0)
    Binding { target: KanteStyle; property: "kind"; value: root.styleKind }
    Binding { target: KanteStyle; property: "preferDark"; value: root.arg("--light", "0") !== "1" }

    readonly property string shot: arg("--shot", "")
    readonly property var views: ["start", "path", "review", "profile", "settings"]

    function talk(scenario, resume) {
        pageStack.push(talkPage, { scenarioId: scenario, resume: resume, lang: kaiwa.lang, script: shot.length > 0 ? arg("--say", "") : "", shot: shot })
    }

    function place() {
        pageStack.push(talkPage, { placement: true, lang: kaiwa.lang, script: shot.length > 0 ? arg("--say", "") : "", shot: shot })
    }

    Component {
        id: talkPage
        ConversationPage {
            onDone: {
                kaiwa.refresh()
                root.pageStack.pop()
            }
        }
    }

    // One page at a time: a conversation takes the whole window.
    Component.onCompleted: pageStack.columnView.columnResizeMode = Kirigami.ColumnView.SingleColumn

    pageStack.initialPage: Kirigami.Page {
        id: home
        title: "Kaiwa"
        padding: 0
        background: Rectangle { color: KanteStyle.backgroundColor }
        KanteScope { target: home }
        globalToolBarStyle: Kirigami.ApplicationHeaderStyle.None

        header: Rectangle {
            color: KanteStyle.backgroundColor
            implicitHeight: tabs.implicitHeight + KanteStyle.unit(16)
            KanteTabBar {
                id: tabs
                anchors.fill: parent
                anchors.margins: KanteStyle.unit(8)
                model: [root.t("Heute", "Today"), root.t("Lernpfad", "Path"), root.t("Wiederholen", "Review"), root.t("Profil", "Profile"), root.t("Einstellungen", "Settings")]
                counts: ["", "", reviewCount > 0 ? String(reviewCount) : "", "", ""]
                readonly property int reviewCount: kaiwa.review.length > 0 ? JSON.parse(kaiwa.review).length : 0
                currentIndex: Math.max(0, root.views.indexOf(root.arg("--view", "start")))
                onActivated: function (i) { currentIndex = i; kaiwa.refresh() }
            }
        }

        StackLayout {
            anchors.fill: parent
            currentIndex: tabs.currentIndex
            StartPage {
                kaiwa: kaiwa
                onTalk: function (scenario, resume) { root.talk(scenario, resume) }
                onShowTab: function (i) { tabs.currentIndex = i }
                onPlace: root.place()
            }
            PathPage {
                kaiwa: kaiwa
                onTalk: function (scenario) { root.talk(scenario, false) }
                onShowTab: function (i) { tabs.currentIndex = i }
            }
            ReviewPage { kaiwa: kaiwa }
            ProfilePage { kaiwa: kaiwa; onPlace: root.place() }
            SettingsPage { kaiwa: kaiwa }
        }
    }

    // Screenshot mode for the tab pages; the conversation takes its own shot.
    Timer {
        running: root.shot.length > 0
        interval: 900
        onTriggered: {
            var v = root.arg("--view", "start")
            if (v === "place") {
                root.place()
            } else if (v === "talk") {
                root.talk(root.arg("--scenario", ""), root.args.indexOf("--resume") >= 0)
            } else {
                home.grabToImage(function (r) { r.saveToFile(root.shot); Qt.quit() })
            }
        }
    }
}
