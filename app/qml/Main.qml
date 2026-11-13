import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante
import org.shrippen.kaiwa
import "tabs.js" as Tabs

// Kaiwa's window (Lastenheft Kap. 7): tabs for today, the learning path, review, profile and
// settings; a conversation opens as its own page on top. The GUI comes from Kante (/LL301/;
// the CI checks that the copy matches Kante main). Style follows the Kante rule: System by
// default, Kante and Kante Light from the settings.
//
// Screenshot mode: --shot out.png [--view start|path|review|profile|settings|talk|place]
// [--scenario id] [--resume] [--say "a|b|wav:file|?|!|~|read:KK|translate:N"] [--kind 0|1|2]
// [--light 0|1] [--check-text]. Only in a build with the feature `demo` (kaiwa.demoBuild); a
// package has none of these switches.
Kirigami.ApplicationWindow {
    id: root
    width: Number(arg("--width", "900"))
    height: Number(arg("--height", "760"))
    minimumWidth: narrowest
    visible: true
    title: "Kaiwa"

    // Narrower, the views lose their layout; every view is checked at this width (/LL303/:
    // 800 px on a screen scaled by 150 %).
    readonly property int narrowest: 533
    // Below this, settings put the control under its title and the conversation puts the
    // talk button on a line of its own.
    readonly property int compactBelow: 700
    readonly property bool compact: width < compactBelow

    readonly property var args: Qt.application.arguments
    function arg(name, fallback) {
        if (!kaiwa.demoBuild)
            return fallback
        var i = args.indexOf(name)
        return i >= 0 && i + 1 < args.length ? args[i + 1] : fallback
    }
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    // With --check-text the screenshot mode says which texts of the page do not fit: elided,
    // wider than their place, or beyond the window's right edge (hinnerk_font, /LL202/;
    // tools/font-160.sh runs it at 160 % and reads the lines). Text fields are left out:
    // their text scrolls.
    function unfitTexts(item, found) {
        if (!item || item.visible === false)
            return found
        if (typeof item.text === "string" && item.text.length > 0 && item.cursorPosition === undefined
                && item.contentWidth !== undefined) {
            var what = item.truncated === true ? "elided"
                : item.contentWidth > item.width + 1 ? "wider than its place"
                : item.mapToItem(null, item.width, 0).x > root.width + 1 ? "beyond the window"
                : ""
            if (what.length > 0)
                found.push(what + ": " + item.text.replace(/\s+/g, " ").substring(0, 60))
        }
        var kids = item.children || []
        for (var i = 0; i < kids.length; i++)
            unfitTexts(kids[i], found)
        return found
    }
    function checkText(item) {
        if (!kaiwa.demoBuild || args.indexOf("--check-text") < 0)
            return
        var found = unfitTexts(item, [])
        for (var i = 0; i < found.length; i++)
            console.log("kaiwa-text: " + found[i])
        console.log("kaiwa-text-checked: " + found.length)
    }

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
        pageStack.push(talkPage, { scenarioId: scenario, resume: resume, lang: kaiwa.lang, kaiwa: kaiwa, script: shot.length > 0 ? arg("--say", "") : "", shot: shot })
    }

    function place() {
        pageStack.push(talkPage, { placement: true, lang: kaiwa.lang, kaiwa: kaiwa, script: shot.length > 0 ? arg("--say", "") : "", shot: shot })
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
    // No swiping between pages: a conversation is left through back, which asks first.
    Component.onCompleted: {
        pageStack.columnView.columnResizeMode = Kirigami.ColumnView.SingleColumn
        pageStack.interactive = false
    }

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
                // Everything due, as the start page counts it (the review page holds ten at a time).
                readonly property int reviewCount: kaiwa.today.length > 0 ? (JSON.parse(kaiwa.today).due || 0) : 0
                currentIndex: Math.max(0, root.views.indexOf(root.arg("--view", "start")))
                onActivated: function (i) {
                    if (!Tabs.changes(currentIndex, i)) {
                        return
                    }
                    currentIndex = i
                    kaiwa.openReview("")
                }
            }
        }

        StackLayout {
            anchors.fill: parent
            currentIndex: tabs.currentIndex
            StartPage {
                kaiwa: kaiwa
                onTalk: function (scenario, resume) { root.talk(scenario, resume) }
                onShowTab: function (i) { kaiwa.openReview(""); tabs.currentIndex = i }
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
                root.checkText(home)
                home.grabToImage(function (r) { r.saveToFile(root.shot); Qt.quit() })
            }
        }
    }
}
