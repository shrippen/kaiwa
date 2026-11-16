import QtQuick
import QtTest
import "tree.js" as Tree

// /LL010/: a slow response time is shown in the settings with its suggestion, and a quick
// one without (lena_suggest_smaller). The page runs on a stand-in for the app object that
// has the profile only.
// Run: tools/qml-tests.sh.
Item {
    id: root
    width: 640
    height: 480

    PageWindow { id: appWindow }

    TestCase {
        name: "LatencySuggestion"
        when: windowShown

        function shownHint(latency) {
            const kaiwa = createTemporaryObject(appObject, root)
            kaiwa.profile = JSON.stringify({ goal: {}, latency: latency })
            const page = createTemporaryObject(appWindow.settings, root, { kaiwa: kaiwa, width: root.width, height: root.height })
            const row = Tree.find(page, i => i.title === "Antwortzeit")
            verify(row !== null, "no row for the response time")
            return row.hint
        }

        function test_a_slow_answer_shows_its_suggestion() {
            const hint = shownHint({ text: "Median 3,6 s", slow: true, suggestion: "Ein kleineres Modell antwortet schneller.", p50: 3600 })
            verify(hint.indexOf("Ein kleineres Modell antwortet schneller.") >= 0, hint)
        }

        function test_a_quick_answer_shows_none() {
            const hint = shownHint({ text: "Median 0,9 s", slow: false, suggestion: "Ein kleineres Modell antwortet schneller.", p50: 900 })
            verify(hint.indexOf("kleineres") < 0, hint)
        }
    }

    Component { id: appObject; AppObject { } }
}
