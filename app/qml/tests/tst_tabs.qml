import QtQuick
import QtTest
import "../tabs.js" as Tabs

// The main tab bar (tabs.js): only a change of tab opens a page afresh.
// Run: tools/qml-tests.sh.
TestCase {
    name: "Tabs"

    // Enter or Space on the tab already shown (Kante #166) must keep its page as it is, e.g.
    // the review of one unit opened from the path.
    function test_the_shown_tab_is_no_change() {
        compare(Tabs.changes(2, 2), false)
    }

    function test_another_tab_is_a_change() {
        compare(Tabs.changes(2, 0), true)
        compare(Tabs.changes(0, 2), true)
    }
}
