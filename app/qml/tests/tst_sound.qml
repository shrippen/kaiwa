import QtQuick
import QtTest

// /LL204/: "louder" and "clear" sit on the conversation page; each keeps the other's
// setting when it is changed (hinnerk_tts_slow).
// Run: tools/qml-tests.sh.
ConversationTest {
    id: t

    TestCase {
        name: "SoundChips"
        when: windowShown

        function test_the_chips_change_one_setting_each() {
            mouseClick(t.button("Lauter"))
            compare(JSON.stringify(t.conv.last("setSound")), JSON.stringify([true, false]))

            // "Clear" alone: "louder" stays as the profile has it.
            t.conv.louder = false
            mouseClick(t.button("Klar"))
            compare(JSON.stringify(t.conv.last("setSound")), JSON.stringify([false, true]))
        }
    }
}
