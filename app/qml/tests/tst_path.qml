import QtQuick
import QtTest
import "tree.js" as Tree

// The learning path (PathPage.qml): one group per unit with its id once, a done unit in one
// line with its due reviews that open the review, the next unit marked, a locked one naming
// what unlocks it, and the locked units beyond the first three folded into one button that
// shows them. Run: tools/qml-tests.sh.
Item {
    id: root
    width: 640
    height: 900

    PageWindow { id: appWindow }
    Component { id: appObject; AppObject { } }

    // A unit as the view delivers it (kaiwa-backend `path`): the page holds no rule.
    function unit(id, title, state, extra) {
        const scenario = { id: id.toLowerCase() + "-a", label: title + " A", emphasis: state === "done" ? "quiet" : "normal" }
        const shows = state !== "locked"
        return Object.assign({
            id: id.toLowerCase(), number: id, title: title, canDo: "Ich kann " + title + ".", state: state,
            showCanDo: shows && state !== "done", showScenarios: shows, scenarios: [scenario],
            due: 0, review: null, open: shows ? { scenario: scenario.id } : null, note: "", folded: false
        }, extra || {})
    }

    readonly property var levels: [{
        level: "N5 · A1",
        units: [
            unit("N5-01", "Grüßen", "done", {
                due: 2, note: "◆ 2 fällig", open: { review: "n5-01" },
                review: { unit: "n5-01", label: "Wiederholung: Grüßen · ◆ 2 fällig" },
                scenarios: [{ id: "n5-01-a", label: "Grüßen A (erledigt)", emphasis: "quiet" },
                            { id: "n5-01-b", label: "Grüßen B (erledigt)", emphasis: "quiet" }]
            }),
            unit("N5-02", "Vorstellen", "current", {
                note: "Als nächstes", open: { scenario: "n5-02-a" },
                scenarios: [{ id: "n5-02-a", label: "Sich vorstellen", emphasis: "primary" },
                            { id: "n5-02-b", label: "Nach dem Namen fragen", emphasis: "normal" }]
            }),
            unit("N5-03", "Konbini", "open", {
                due: 3, review: { unit: "n5-03", label: "Wiederholung: Konbini · ◆ 3 fällig" }
            }),
            unit("N5-04", "Essen", "locked", { note: "nach Konbini (N5-03)" }),
            unit("N5-05", "Reisen", "locked", { note: "nach Essen (N5-04)" }),
            unit("N5-06", "Arzt", "locked", { note: "nach Reisen (N5-05)" }),
            unit("N5-07", "Bank", "locked", { note: "nach Arzt (N5-06)", folded: true }),
            unit("N5-08", "Post", "locked", { note: "nach Bank (N5-07)", folded: true })
        ],
        foldedCount: 2,
        foldedNames: "Bank, Post"
    }]

    TestCase {
        name: "PathPage"
        when: windowShown

        function page() {
            const kaiwa = createTemporaryObject(appObject, root)
            kaiwa.path = JSON.stringify(root.levels)
            const talks = []
            const p = createTemporaryObject(appWindow.path, root, { kaiwa: kaiwa, width: root.width, height: root.height })
            p.talk.connect(s => talks.push(s))
            return { kaiwa: kaiwa, page: p, talks: talks }
        }

        function paths(item) {
            const all = []
            Tree.find(item, n => { if (n.hasOwnProperty("doneText") && n.hasOwnProperty("activated")) { all.push(n) } return false })
            return all
        }

        function fold(item) {
            return Tree.find(item, n => n.hasOwnProperty("overflow") && n.visible && n.text === "+ 2 weitere gesperrte Einheiten")
        }

        function button(item, text) {
            return Tree.find(item, n => n.hasOwnProperty("emphasis") && n.visible && n.text === text)
        }

        function test_one_group_per_unit_with_the_folded_ones_hidden() {
            const r = page()
            const found = paths(r.page)
            compare(found.length, 6)
            compare(found[0].model[0].title, "N5-01 · Grüßen")
            compare(found[0].model[0].meta, "◆ 2 fällig")
            compare(found[1].model[0].meta, "Als nächstes")
            compare(found[3].model[0].meta, "nach Konbini (N5-03)")
            verify(fold(r.page))
            compare(fold(r.page).subtitle, "Bank, Post")
        }

        function test_the_fold_button_shows_the_rest() {
            const r = page()
            fold(r.page).clicked()
            compare(paths(r.page).length, 8)
            verify(!fold(r.page))
        }

        function test_a_click_does_what_the_view_says() {
            const r = page()
            paths(r.page)[0].activated(0)
            compare(r.kaiwa.last("openReview")[0], "n5-01")
            compare(r.talks.length, 0)
            paths(r.page)[1].activated(0)
            compare(r.talks[0], "n5-02-a")
        }

        function test_a_done_unit_keeps_each_scenario_as_a_button() {
            const r = page()
            button(r.page, "Grüßen B (erledigt)").clicked()
            compare(r.talks[0], "n5-01-b")
        }

        function test_the_review_button_names_its_unit() {
            const r = page()
            const b = button(r.page, "Wiederholung: Konbini · ◆ 3 fällig")
            verify(b)
            compare(b.Accessible.name, "Wiederholung: Konbini · ◆ 3 fällig")
            b.clicked()
            compare(r.kaiwa.last("openReview")[0], "n5-03")
        }

        function test_the_next_unit_lists_its_scenarios_with_a_start_each() {
            const r = page()
            button(r.page, "Nach dem Namen fragen").clicked()
            compare(r.talks[0], "n5-02-b")
        }

        function test_a_locked_unit_starts_nothing() {
            const r = page()
            verify(!button(r.page, "Essen A"))
            paths(r.page)[3].activated(0)
            compare(r.talks.length, 0)
        }
    }
}
