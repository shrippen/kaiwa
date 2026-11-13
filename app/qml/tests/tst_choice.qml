import QtQuick
import QtTest
import ".."

// The settings' segment rows (Choice.qml) show the profile's value after a click and a later
// change elsewhere: a profile switch, an import, a longer pause taken in a talk.
// Run: tools/qml-tests.sh.
TestCase {
    id: tc
    name: "Choice"
    when: windowShown

    // The profile: the row follows it, a click writes it (like setSetting and refresh).
    property int goal: 5
    readonly property var values: [3, 5, 10]

    Choice {
        id: row
        model: ["3", "5", "10"]
        shown: Math.max(0, tc.values.indexOf(tc.goal))
        onActivated: function (i) { tc.goal = tc.values[i] }
    }

    function test_after_a_click_the_row_follows_the_profile() {
        row.select(2)
        compare(tc.goal, 10)
        compare(row.currentIndex, 2)

        // Another profile with another goal.
        tc.goal = 3
        compare(row.currentIndex, 0)

        // A click on what the old profile had works again.
        row.select(2)
        compare(tc.goal, 10)
    }
}
