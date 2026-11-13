import QtQuick
import QtTest
import Kante
import "../tabs.js" as Tabs

// The main tab bar (tabs.js): the review forgets its unit when the learner leaves it, not when
// a key activates the tab already shown. A real KanteTabBar, wired as in Main.qml.
// Run: tools/qml-tests.sh.
TestCase {
    name: "Tabs"
    when: windowShown

    readonly property int review: 2

    KanteTabBar {
        id: bar
        model: ["Heute", "Lernpfad", "Wiederholen", "Profil", "Einstellungen"]
        property int shown: -1
        Component.onCompleted: shown = currentIndex
        property int resets: 0
        onCurrentIndexChanged: {
            if (Tabs.leaves(shown, currentIndex, review)) {
                resets += 1
            }
            shown = currentIndex
        }
    }

    function init() {
        bar.currentIndex = 0
        bar.resets = 0
    }

    // The tab before a change is the old one, not already the new one (no binding).
    function test_the_first_change_knows_the_tab_before() {
        compare(bar.shown, 0)
        bar.currentIndex = review
        bar.currentIndex = 1
        compare(bar.resets, 1)
    }

    // The path opens the review of one unit (no activation); Enter or Space on it keeps it.
    function test_a_key_on_the_shown_review_keeps_its_unit() {
        bar.currentIndex = review
        bar.activate(review)
        compare(bar.resets, 0)
    }

    // Leaving the review and coming back shows everything due again.
    function test_leaving_the_review_forgets_its_unit() {
        bar.currentIndex = review
        bar.select(0)
        compare(bar.resets, 1)
        bar.select(review)
        compare(bar.resets, 1, "arriving keeps what the path set")
    }

    function test_other_tabs_reset_nothing() {
        bar.select(1)
        bar.select(3)
        compare(bar.resets, 0)
    }
}
