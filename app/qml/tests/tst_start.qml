import QtQuick
import QtTest
import "tree.js" as Tree

// The start page (StartPage.qml): the next scenario and the review estimate on the left, the
// goal, streak, XP and the week's chart with its legend on the right; the figures and the chart
// only with gamification. Run: tools/qml-tests.sh.
Item {
    id: root
    width: 900
    height: 900

    PageWindow { id: appWindow }
    Component { id: appObject; AppObject { } }

    // As the view delivers it (kaiwa-backend `today`).
    readonly property var view: ({
        name: "Mara", gamification: true, placed: true, needsTutor: false,
        goalFraction: 0.4, goalSpoken: 2, goalRest: "von 5 Minuten gesprochen", goalValue: 5,
        goalMet: false, streakDays: 13, streakNote: "", xpToday: 40,
        due: 7, reviewMinutes: 4, refresher: false,
        next: { id: "n5-03-a", title: "Im Unterricht", unit: "N5-03", canDo: "Ich kann nachfragen." },
        free: [{ id: "frei-alltag", topic: "Alltag" }],
        week: { labels: ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"], minutes: [3, 5, 1, 6, 0, 2, 0],
                met: [false, true, false, true, false, false, false], today: 5 }
    })

    TestCase {
        name: "StartPage"
        when: windowShown

        function page(view) {
            const kaiwa = createTemporaryObject(appObject, root)
            kaiwa.today = JSON.stringify(view)
            const p = createTemporaryObject(appWindow.start, root, { kaiwa: kaiwa, width: root.width, height: root.height })
            return p
        }

        function texts(item) {
            const all = []
            Tree.find(item, n => { if (n.visible && typeof n.text === "string") { all.push(n.text) } return false })
            return all
        }

        function test_the_review_names_its_minutes() {
            const t = texts(page(view))
            verify(t.indexOf("7 Wörter und Wendungen sind fällig, etwa 4 Minuten.") >= 0, t.join("|"))
            verify(t.indexOf("Jetzt wiederholen") >= 0)
        }

        function test_the_week_colours_today_goal_and_the_rest() {
            const chart = Tree.find(page(view), n => n.objectName === "weekChart")
            verify(chart)
            compare(chart.values[1], [0, 5, 0])
            compare(chart.values[5], [0, 0, 2])
            compare(chart.values[0], [3, 0, 0])
            compare(chart.goal, 5)
            verify(chart.axis)
        }

        function test_the_chart_has_a_legend() {
            const p = page(view)
            const legend = Tree.find(p, n => n.hasOwnProperty("keySize") && n.hasOwnProperty("model"))
            verify(legend)
            compare(legend.model.map(m => m.text), ["heute", "Ziel erreicht"])
        }

        function test_without_gamification_no_chart_streak_or_xp() {
            const p = page(Object.assign({}, view, { gamification: false }))
            const chart = Tree.find(p, n => n.objectName === "weekChart")
            verify(!chart || !chart.visible)
            const t = texts(p)
            verify(t.indexOf("Serie") < 0)
            verify(t.indexOf("XP") < 0)
            verify(t.indexOf("von 5 Minuten gesprochen") >= 0)
        }

        function test_two_columns_in_a_wide_window() {
            const p = page(view)
            const grid = Tree.find(p, n => n.hasOwnProperty("columnSpacing") && n.hasOwnProperty("columns"))
            compare(grid.columns, 2)
        }
    }
}
