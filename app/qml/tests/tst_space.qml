import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import QtTest
import org.kde.kirigami as Kirigami
import Kante

// The space bar on the conversation page (/LF100/): the page's Shortcut block, word for word
// (mara_ptt checks that), on a page built like it: content with a button and a chip, the talk
// button and the answer field in the footer, the field locked while the tutor answers.
// Run: tools/qml-tests.sh (needs Qt 6 with QtTest and Kirigami; no microphone, no models).
Kirigami.ScrollablePage {
    id: page
    width: 640
    height: 480

    property int started: 0
    property int ended: 0
    property int again: 0
    property int toggled: 0
    property bool busy: false
    property int st: KanteTalkButton.State.Ready
    property bool handsFree: false

    QtObject { id: leaveDialog; property bool visible: false }
    QtObject { id: grammarDialog; property bool visible: false }
    QtObject { id: wordPopup; property bool visible: false }

    ColumnLayout {
        KanteButton { id: againButton; text: "Nochmal"; onClicked: page.again++ }
        KanteChip { id: chip; text: "Lauter"; checkable: true; onClicked: page.toggled++ }
    }

    footer: QQC2.ToolBar {
        id: bar
        contentItem: RowLayout {
            KanteTalkButton {
                id: talk
                talkState: page.busy && page.st === KanteTalkButton.State.Ready ? KanteTalkButton.State.Thinking : page.st
                mode: page.handsFree ? KanteTalkButton.Mode.Toggle : KanteTalkButton.Mode.Hold
                onTalkStarted: { page.started++; page.st = KanteTalkButton.State.Listening }
                onTalkEnded: { page.ended++; page.st = KanteTalkButton.State.Ready }
            }
            KanteTextField { id: input; Layout.fillWidth: true; enabled: !page.busy }
            KanteButton { id: helpButton; text: "Hilfe" }
        }
    }

    Shortcut {
        sequence: "Space"
        autoRepeat: false
        enabled: talk.enabled && !(input.focus && bar.focus) && talk.talkState !== KanteTalkButton.State.Thinking
            && !leaveDialog.visible && !grammarDialog.visible && !wordPopup.visible
        onActivated: { talk.forceActiveFocus(Qt.ShortcutFocusReason); talk.press() }
    }

    TestCase {
        name: "SpaceToTalk"
        when: windowShown

        function hold() {
            keyPress(Qt.Key_Space)
            keyRelease(Qt.Key_Space)
        }
        function test_1_new_page_nothing_clicked() {
            keyPress(Qt.Key_Space)
            compare(page.started, 1)
            compare(page.ended, 0)
            keyRelease(Qt.Key_Space)
            compare(page.ended, 1)
        }
        function test_2_after_a_click_on_a_button_it_talks_and_does_not_press_it_again() {
            mouseClick(againButton)
            compare(page.again, 1)
            hold()
            compare(page.started, 2)
            compare(page.ended, 2)
            compare(page.again, 1)
        }
        function test_3_the_field_keeps_its_space() {
            mouseClick(input)
            keyClick(Qt.Key_A)
            keyClick(Qt.Key_Space)
            compare(input.text, "a ")
            compare(page.started, 2)
        }
        function test_4_a_space_while_the_tutor_answers_leaves_the_field_the_focus() {
            input.text = ""
            page.busy = true
            hold()
            verify(!talk.activeFocus)
            page.busy = false
            verify(input.activeFocus)
            keyClick(Qt.Key_C)
            keyClick(Qt.Key_Space)
            keyClick(Qt.Key_D)
            compare(input.text, "c d")
            compare(page.started, 2)
        }
        function test_5_field_used_then_a_button_of_the_page_clicked() {
            // The footer is a focus scope: the field stays its focus, the key must talk.
            mouseClick(againButton)
            compare(page.again, 2)
            verify(input.focus)
            hold()
            compare(page.started, 3)
            compare(page.ended, 3)
            compare(page.again, 2)
        }
        function test_6_field_used_then_the_focus_on_a_chip() {
            mouseClick(input)
            chip.forceActiveFocus()
            hold()
            compare(page.started, 4)
            compare(page.toggled, 0)
        }
        function test_7_field_used_then_a_button_of_the_footer_clicked() {
            mouseClick(input)
            mouseClick(helpButton)
            hold()
            compare(page.started, 5)
            compare(page.ended, 5)
        }
        function test_8_thinking_takes_no_key_and_no_focus() {
            mouseClick(againButton)
            page.busy = true
            hold()
            compare(page.started, 5)
            verify(!talk.activeFocus)
            page.busy = false
            hold()
            compare(page.started, 6)
        }
        function test_9_hands_free_one_press_starts_one_ends() {
            page.handsFree = true
            mouseClick(againButton)
            hold()
            compare(page.started, 7)
            compare(page.ended, 6)
            hold()
            compare(page.ended, 7)
            page.handsFree = false
        }
        function test_a_a_dialog_takes_the_key_away() {
            // In the app the dialog holds the focus; here the button does, so the shortcut
            // is the only way the key could reach the talk button.
            mouseClick(againButton)
            leaveDialog.visible = true
            hold()
            compare(page.started, 7)
            leaveDialog.visible = false
        }
    }
}
