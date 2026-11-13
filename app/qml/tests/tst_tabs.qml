import QtQuick
import QtTest
import ".."

// The main tab bar (MainTabs.qml), as Main.qml uses it: every change of tab is said, so the
// views reload; leaving the review tab is said apart, so the review forgets a unit opened from
// the path. Enter or Space on the tab already shown changes nothing.
// Run: tools/qml-tests.sh.
TestCase {
    name: "Tabs"
    when: windowShown

    readonly property int review: 2
    // The tab the app starts on (--view), bound as in Main.qml.
    property int view: review

    MainTabs {
        id: bar
        model: ["Heute", "Lernpfad", "Wiederholen", "Profil", "Einstellungen"]
        review: 2
        currentIndex: view
    }

    SignalSpy {
        id: spy
        target: bar
        signalName: "switched"
    }

    function init() {
        bar.currentIndex = review
        spy.clear()
    }

    // Started on the review (--view review): the first change already knows it left the review.
    function test_the_start_tab_counts() {
        compare(bar.currentIndex, review)
        bar.select(0)
        compare(spy.count, 1)
        compare(spy.signalArguments[0][0], MainTabs.FromReview)
    }

    // The path opens the review of one unit; Enter or Space on it keeps it.
    function test_a_key_on_the_shown_tab_changes_nothing() {
        bar.activate(review)
        bar.activate(review)
        compare(spy.count, 0)
    }

    // Any other change reloads the views but keeps what the path set for the review.
    function test_other_changes_reload() {
        bar.select(0)
        spy.clear()
        bar.select(1)
        bar.select(review)
        compare(spy.count, 2)
        compare(spy.signalArguments[0][0], MainTabs.ToTab)
        compare(spy.signalArguments[1][0], MainTabs.ToTab)
    }
}
