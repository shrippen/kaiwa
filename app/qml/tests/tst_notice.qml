import QtQuick
import QtTest

// A save or load that failed shows on the conversation page as "Nicht gespeichert", beside
// the tutor's errors, and only then (the backend reports it, the app object sets `notice`).
// Run: tools/qml-tests.sh.
ConversationTest {
    id: t

    TestCase {
        name: "NoticeOfAFailedSave"
        when: windowShown

        function callout() {
            return t.find(i => i.title === "Nicht gespeichert")
        }

        function test_a_notice_is_shown_with_its_text() {
            t.conv.notice = ""
            verify(!callout().visible, "nothing to show yet")
            t.conv.notice = "Das Profil ließ sich nicht lesen oder schreiben: disk full"
            verify(callout().visible)
            compare(callout().text, "Das Profil ließ sich nicht lesen oder schreiben: disk full")
            t.conv.notice = ""
            verify(!callout().visible, "cleared with the next talk")
        }
    }
}
