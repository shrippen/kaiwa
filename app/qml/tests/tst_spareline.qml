import QtQuick
import QtTest
import ".."

// The line under the main tabs while a stand-in runs for a profile that could not be opened
// (SpareLine.qml): it stays, whatever the notice does. Run: tools/qml-tests.sh.
Item {
    width: 400
    height: 200

    AppObject { id: app }

    SpareLine {
        id: line
        width: parent.width
        kaiwa: app
    }

    TestCase {
        name: "SpareLine"
        when: windowShown

        function test_the_line_stays_while_the_profile_is_a_stand_in() {
            app.profile = JSON.stringify({ goal: {}, spare: false })
            verify(!line.visible, "a profile that opened has no line")

            app.profile = JSON.stringify({ goal: {}, spare: true })
            verify(line.visible)
            verify(line.text.indexOf("Nichts") > 0)

            // Neither a notice nor its clearing touches it.
            app.notice = "x"
            app.notice = ""
            verify(line.visible)

            app.lang = "en"
            verify(line.text.indexOf("Nothing you do now is saved") >= 0)
            app.lang = "de"
        }
    }
}
