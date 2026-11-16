import QtQuick
import QtTest

// /LF112/: a word of a line is looked up by the keyboard (Tab to the line, Enter), the card
// says it and takes it into the review (mara_word_lookup).
// Run: tools/qml-tests.sh.
ConversationTest {
    id: t

    TestCase {
        name: "WordLookup"
        when: windowShown

        function init() {
            t.conv.calls = []
            t.conv.turns = JSON.stringify([{ who: "character", segments: [
                { text: "私", reading: "わたし", word: 0 }, { text: "は", word: 1 }, { text: "マラです。", word: 2 }] }])
            t.conv.lookupAnswer = JSON.stringify({
                found: true, word: "私", reading: "わたし", pos: ["noun"], senses: [["I"]],
                accent: null, inReview: false, name: false, fallback: false
            })
        }

        function test_enter_on_a_word_looks_it_up_and_the_card_acts() {
            const line = t.find(i => i.interactive === true && i.wordActivated !== undefined)
            verify(line !== null, "no interactive line")
            line.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(t.conv.count("lookup"), 1)
            compare(JSON.stringify(t.conv.last("lookup")), JSON.stringify([0, 0]))

            mouseClick(t.button("Anhören"))
            compare(JSON.stringify(t.conv.last("sayWord")), JSON.stringify([0, 0]))

            mouseClick(t.button("In die Wiederholung"))
            compare(JSON.stringify(t.conv.last("remember")), JSON.stringify([0, 0]))
            verify(t.button("In der Wiederholung") !== null)
        }
    }
}
