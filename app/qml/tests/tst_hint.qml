import QtQuick
import QtTest

// /LF301/: a correction under the learner's turn shows both forms as a diff and the page
// can play the better form (mara_diff_audio).
// Run: tools/qml-tests.sh.
ConversationTest {
    id: t

    TestCase {
        name: "HintCard"
        when: windowShown

        function init() {
            t.conv.calls = []
            t.conv.turns = JSON.stringify([{
                who: "learner", typed: true, text: "コーヒーを好きです。", corrections: [{
                    tier: "yellow", tierText: "Hinweis", explanation: "Bei 好き steht が.", meta: "N5",
                    better: "コーヒーが好きです。", disputed: false,
                    parts: [{ text: "コーヒー" }, { text: "を", kind: "removed" }, { text: "が", kind: "added" }, { text: "好きです。" }]
                }]
            }])
        }

        function test_the_card_plays_the_better_form() {
            mouseClick(t.button("Anhören"))
            compare(JSON.stringify(t.conv.last("sayText")), JSON.stringify(["コーヒーが好きです。"]))
        }

        function test_no_playing_without_a_voice() {
            t.conv.voice = ""
            verify(!t.button("Anhören").enabled)
            t.conv.voice = "Kokoro"
        }
    }
}
