import QtQuick
import QtTest
import "../leave.js" as Leave

// Back on the conversation page (leave.js): when it asks before leaving.
// Run: tools/qml-tests.sh.
TestCase {
    name: "Leave"

    function test_a_talk_without_a_word_leaves_at_once() {
        compare(Leave.asks(false, false, false, false), false)
    }

    function test_a_running_talk_asks() {
        compare(Leave.asks(true, false, false, false), true)
    }

    function test_a_finished_talk_leaves_at_once() {
        compare(Leave.asks(true, true, false, false), false)
    }

    // The stages are over, the reading questions wait: nothing is stored yet.
    function test_the_reading_questions_ask() {
        compare(Leave.asks(true, true, true, true), true)
    }

    // The result is shown and stored.
    function test_the_result_leaves_at_once() {
        compare(Leave.asks(false, true, true, false), false)
    }
}
