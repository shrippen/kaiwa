import QtQuick
import QtTest
import Kante

// The space bar and the talk button on the conversation page (/LF100/, /LL202/): the
// real page, not a copy of its code. The key starts and ends a turn from wherever the focus
// is, the answer field keeps its space, and while the tutor thinks the key does nothing.
// The page's own buttons stand in for the ones a learner clicked before: "Später weiter" in the
// content, "Hilfe" in the footer, the chip "Lauter".
// Run: tools/qml-tests.sh (needs Qt 6 with QtTest and Kirigami).
ConversationTest {
    id: t
    property int again: 0

    TestCase {
        name: "SpaceToTalk"
        when: windowShown

        function hold() {
            keyPress(Qt.Key_Space)
            keyRelease(Qt.Key_Space)
        }

        function test_1_new_page_nothing_clicked() {
            t.page.done.connect(() => t.again++)
            keyPress(Qt.Key_Space)
            compare(t.conv.count("startTalk"), 1)
            compare(t.conv.count("stopTalk"), 0)
            keyRelease(Qt.Key_Space)
            compare(t.conv.count("stopTalk"), 1)
        }
        function test_2_after_a_click_on_a_button_it_talks_and_does_not_press_it_again() {
            mouseClick(t.button("Später weiter"))
            compare(t.again, 1)
            hold()
            compare(t.conv.count("startTalk"), 2)
            compare(t.conv.count("stopTalk"), 2)
            compare(t.again, 1)
        }
        function test_3_the_field_keeps_its_space() {
            mouseClick(t.answerField())
            keyClick(Qt.Key_A)
            keyClick(Qt.Key_Space)
            compare(t.answerField().text, "a ")
            compare(t.conv.count("startTalk"), 2)
        }
        function test_4_a_space_while_the_tutor_answers_leaves_the_field_the_focus() {
            t.answerField().text = ""
            t.conv.busy = true
            hold()
            verify(!t.talkButton().activeFocus)
            t.conv.busy = false
            verify(t.answerField().activeFocus)
            keyClick(Qt.Key_C)
            keyClick(Qt.Key_Space)
            keyClick(Qt.Key_D)
            compare(t.answerField().text, "c d")
            compare(t.conv.count("startTalk"), 2)
        }
        function test_5_field_used_then_a_button_of_the_page_clicked() {
            // The footer is a focus scope: the field stays its focus, the key must talk.
            mouseClick(t.button("Später weiter"))
            compare(t.again, 2)
            verify(t.answerField().focus)
            hold()
            compare(t.conv.count("startTalk"), 3)
            compare(t.conv.count("stopTalk"), 3)
            compare(t.again, 2)
        }
        function test_6_field_used_then_the_focus_on_a_chip() {
            mouseClick(t.answerField())
            t.button("Lauter").forceActiveFocus()
            hold()
            compare(t.conv.count("startTalk"), 4)
            compare(t.conv.count("setSound"), 0)
        }
        function test_7_field_used_then_a_button_of_the_footer_clicked() {
            mouseClick(t.answerField())
            mouseClick(t.button("Hilfe"))
            compare(t.conv.count("help"), 1)
            hold()
            compare(t.conv.count("startTalk"), 5)
            compare(t.conv.count("stopTalk"), 5)
        }
        function test_8_thinking_takes_no_key_and_no_focus() {
            mouseClick(t.button("Später weiter"))
            t.conv.busy = true
            hold()
            compare(t.conv.count("startTalk"), 5)
            verify(!t.talkButton().activeFocus)
            t.conv.busy = false
            hold()
            compare(t.conv.count("startTalk"), 6)
        }
        function test_9_hands_free_one_press_starts_one_ends() {
            t.conv.handsFree = true
            mouseClick(t.button("Später weiter"))
            hold()
            compare(t.conv.count("startTalk"), 7)
            compare(t.conv.count("stopTalk"), 6)
            hold()
            compare(t.conv.count("stopTalk"), 7)
            t.conv.handsFree = false
        }
        function test_a_a_dialog_takes_the_key_away() {
            // A talk with a learner's word asks before Back leaves. In the app the dialog
            // holds the focus; here a button does, so the page's shortcut is the only way
            // the key could reach the talk button.
            t.conv.turns = JSON.stringify([{ who: "learner", text: "はい", typed: true, corrections: [] }])
            mouseClick(t.button("Später weiter"))
            t.page.backRequested({ accepted: false })
            hold()
            compare(t.conv.count("startTalk"), 7)
            mouseClick(t.button("Weiterreden"))
            hold()
            compare(t.conv.count("startTalk"), 8)
        }
        // A click on the talk button, and the key on the button itself, work as well.
        function test_b_the_button_talks_by_click_and_by_its_own_key() {
            const talk = t.talkButton()
            mousePress(talk)
            compare(t.conv.count("startTalk"), 9)
            mouseRelease(talk)
            compare(t.conv.count("stopTalk"), 9)
            talk.forceActiveFocus()
            keyPress(Qt.Key_Space)
            compare(t.conv.count("startTalk"), 10)
            keyRelease(Qt.Key_Space)
            compare(t.conv.count("stopTalk"), 10)
        }
        // Every state has a word, never only a colour; the button is at
        // least 48 units high.
        function test_c_every_state_has_a_name_and_the_button_is_tall_enough() {
            const talk = t.talkButton()
            // The page's own words, in both languages (Kante's defaults differ).
            const words = {
                de: ["Sprechen", "Hört zu", "Denkt nach", "Spricht", "Kein Sprechen"],
                en: ["Speak", "Listening", "Thinking", "Speaking", "No speech"]
            }
            for (const lang of ["de", "en"]) {
                t.page.lang = lang
                for (let state = 0; state < 5; state++) {
                    t.conv.talkState = state
                    compare(talk.title, words[lang][state], lang + " state " + state)
                }
            }
            t.page.lang = "de"
            t.conv.talkState = 0
            verify(talk.height >= 48, "height " + talk.height)
        }
    }
}
