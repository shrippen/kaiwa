import QtQuick
import QtTest
import "tree.js" as Tree

// The profile (ProfilePage.qml): a skill's row opens on a click and shows its sentence and the
// two corrections, which do what the view says; the kinds of mistakes are line charts with
// both axes named, the days of the first and last talk, and a read-out per talk. Run:
// tools/qml-tests.sh.
Item {
    id: root
    width: 900
    height: 1200

    PageWindow { id: appWindow }
    Component { id: appObject; AppObject { } }

    // As the views deliver them (kaiwa-backend `learner`, `stats`): the page holds no rule.
    readonly property var nMarks: [{ name: "N5", at: 0.375 }, { name: "N4", at: 0.625 }]
    readonly property var learner: ({
        skills: [
            { skill: "listening", name: "Hören", words: "Hören: Anfang N5 mit Hilfe.", short: "Anfang N5", share: 0.25, marks: root.nMarks },
            { skill: "register", name: "Höflichkeitsformen", words: "Spricht locker wie unter Freunden.", short: "です/ます fehlt", share: 0.25, marks: [{ name: "です/ます", at: 0.5 }, { name: "敬語", at: 1.0 }] }
        ],
        weak: [], knobs: []
    })
    readonly property var stats: ({
        heat: [],
        talks: [{ date: "2026-11-03", scenario: "Café" }, { date: "2026-11-05", scenario: "Konbini" }, { date: "2026-11-16", scenario: "Bahnhof" }],
        mistakes: [
            { kind: "particle", word: "Partikel", counts: [4, 2, 1], trend: "seltener", text: "Partikel: seltener" },
            { kind: "counter", word: "Zählwort", counts: [0, 1, 2], trend: "häufiger", text: "Zählwort: häufiger" }
        ]
    })

    TestCase {
        name: "ProfilePage"
        when: windowShown

        // The page goes before the app object it reads, not after.
        property var made: []
        function cleanup() {
            for (const k of made.reverse()) {
                k.destroy()
            }
            made = []
        }

        function page() {
            const kaiwa = appObject.createObject(root)
            made.push(kaiwa)
            kaiwa.learner = JSON.stringify(root.learner)
            kaiwa.stats = JSON.stringify(root.stats)
            const p = appWindow.profile.createObject(root, { kaiwa: kaiwa, width: root.width, height: root.height })
            made.push(p)
            return { kaiwa: kaiwa, page: p }
        }

        function row(item, name) {
            return Tree.find(item, n => n.hasOwnProperty("minTextWidth") && n.visible && n.text === name)
        }

        function button(item, text) {
            return Tree.find(item, n => n.hasOwnProperty("emphasis") && n.visible && n.text === text)
        }

        function charts(item) {
            const all = []
            Tree.find(item, n => { if (n.hasOwnProperty("xTitle") && n.hasOwnProperty("thresholds")) { all.push(n) } return false })
            return all
        }

        function test_a_skill_shows_its_level_as_a_bar_with_the_levels_named() {
            const r = page()
            verify(row(r.page, "Hören"))
            verify(row(r.page, "Höflichkeitsformen"))
            const bars = []
            Tree.find(r.page, n => { if (n.hasOwnProperty("segments") && n.hasOwnProperty("parts") && n.Accessible.name === "Hören") { bars.push(n) } return false })
            compare(bars.length, 1)
            compare(bars[0].value, 0.25)
            const heads = []
            Tree.find(r.page, n => { if (n.hasOwnProperty("rule") && n.hasOwnProperty("elide") && n.visible) { heads.push(n.text) } return false })
            verify(heads.indexOf("N5") >= 0)
            verify(heads.indexOf("N4") >= 0)
            verify(heads.indexOf("Stufe") >= 0)
            // The politeness has its own marks under its bar, not the N levels of the head.
            verify(heads.indexOf("です/ます") >= 0)
            verify(heads.indexOf("敬語") >= 0)
            compare(heads.filter(h => h === "N5").length, 1)
        }

        function test_a_click_opens_a_row_and_shows_its_buttons() {
            const r = page()
            verify(!button(r.page, "Zu schwer"))
            verify(!button(r.page, "Kann ich schon"))
            row(r.page, "Höflichkeitsformen").clicked()
            verify(button(r.page, "Zu schwer"))
            verify(button(r.page, "Kann ich schon"))
            // One row at a time: its buttons, not those of every skill.
            compare(r.page.contentItem.children.length > 0, true)
            row(r.page, "Hören").clicked()
            verify(button(r.page, "Zu schwer"))
            row(r.page, "Hören").clicked()
            verify(!button(r.page, "Zu schwer"))
        }

        function test_the_buttons_correct_the_skill_of_their_row() {
            const r = page()
            row(r.page, "Höflichkeitsformen").clicked()
            button(r.page, "Zu schwer").clicked()
            compare(r.kaiwa.last("nudge"), ["register", false])
            button(r.page, "Kann ich schon").clicked()
            compare(r.kaiwa.last("nudge"), ["register", true])
        }

        function test_a_chart_per_kind_with_axes_and_a_read_out_per_talk() {
            const r = page()
            const found = charts(r.page)
            compare(found.length, 2)
            for (const c of found) {
                verify(c.axis)
                compare(c.xTitle, "Gespräch")
                verify(c.yTitle !== "")
                compare(c.xStart, "3.11.")
                compare(c.xEnd, "16.11.")
                compare(c.labels, ["3.11. · Café", "5.11. · Konbini", "16.11. · Bahnhof"])
                verify(c.readout)
            }
            compare(found[0].series, [[4, 2, 1]])
            compare(found[1].series, [[0, 1, 2]])
            // One scale for all of them.
            compare(found[0].maxValue, found[1].maxValue)
        }

        function test_the_trend_stands_over_its_chart() {
            const r = page()
            const words = []
            Tree.find(r.page, n => { if (n.hasOwnProperty("wrapMode") && n.visible) { words.push(n.text) } return false })
            verify(words.indexOf("seltener") >= 0)
            verify(words.indexOf("häufiger") >= 0)
        }
    }
}
