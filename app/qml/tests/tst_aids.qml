import QtQuick
import QtTest

// /LF111/: each line of the character can be heard again, slower and translated; the page
// asks the app object for aid 0, 1 and 2 of that line (mara_listen_tools).
// Run: tools/qml-tests.sh.
ConversationTest {
    id: t

    TestCase {
        name: "ListeningAids"
        when: windowShown

        function init() {
            t.conv.calls = []
            t.conv.turns = JSON.stringify([
                { who: "character", markup: "{私|わたし}はマラです。" },
                { who: "character", markup: "{何|なに}を{飲|の}みますか。" }
            ])
        }

        function test_each_line_has_its_three_aids() {
            const aids = [["Nochmal", 0], ["Langsamer", 1], ["Übersetzen", 2]]
            for (const [text, aid] of aids) {
                const buttons = []
                t.find(i => { if (i.text === text && i.clicked !== undefined) { buttons.push(i) } return false })
                compare(buttons.length, 2, text)
                mouseClick(buttons[1])
                compare(JSON.stringify(t.conv.last("listen")), JSON.stringify([1, aid]), text)
            }
        }

        function test_no_aid_while_the_tutor_thinks_or_without_a_voice() {
            t.conv.voice = ""
            for (const text of ["Nochmal", "Langsamer"]) {
                verify(!t.button(text).enabled, text)
            }
            t.conv.voice = "Kokoro"
            t.conv.busy = true
            for (const text of ["Nochmal", "Langsamer", "Übersetzen"]) {
                verify(!t.button(text).enabled, text)
            }
            t.conv.busy = false
        }
    }
}
