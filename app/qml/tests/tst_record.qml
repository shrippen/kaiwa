import QtQuick
import QtTest
import "../record.js" as Record

// The summary's record (record.js): what the page shows with and without one.
// Run: tools/qml-tests.sh.
TestCase {
    name: "Record"

    // Recording the talk failed: `recorded` is empty, the page has no record.
    function test_no_record_shows_no_hint_and_no_numbers() {
        compare(Record.hint(null), "none")
        compare(Record.hasXp(null), false)
    }

    function test_a_record_says_what_a_mark_does() {
        compare(Record.hint({ minutes: 3, xp: 12 }), "correct")
        compare(Record.hint({ minutes: 3, xp: 12, markKept: true }), "kept")
    }

    // `undefined` is not a value: the KPI row must not show it.
    function test_xp_is_a_number_or_absent() {
        compare(Record.hasXp({ minutes: 3 }), false)
        compare(Record.hasXp({ xp: null }), false)
        compare(Record.hasXp({ xp: 0 }), true)
        compare(Record.hasXp({ xp: 12 }), true)
    }
}
