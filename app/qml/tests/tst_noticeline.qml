import QtQuick
import QtTest
import ".."

// The notice under the main tabs (NoticeLine.qml): a page that failed to load, or a skip that
// was not saved, says so there; no notice, no line; none where the page has its own line.
// Run: tools/qml-tests.sh.
Item {
    width: 400
    height: 200

    AppObject { id: app }

    NoticeLine {
        id: line
        width: parent.width
        kaiwa: app
    }

    TestCase {
        name: "NoticeLine"
        when: windowShown

        function test_the_line_shows_the_notice_and_hides_without() {
            app.notice = ""
            verify(!line.visible, "nothing to say")
            app.notice = "Das Profil ließ sich nicht vollständig laden: disk full"
            verify(line.visible)
            compare(line.text, "Das Profil ließ sich nicht vollständig laden: disk full")
            app.notice = ""
            verify(!line.visible)
        }

        function test_a_page_with_its_own_line_gets_none() {
            app.notice = "x"
            line.wanted = false
            verify(!line.visible)
            line.wanted = true
        }
    }
}
