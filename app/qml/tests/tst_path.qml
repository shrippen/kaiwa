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

    function unit(id, title, state, extra) {
        return Object.assign({
            id: id.toLowerCase(), number: id, title: title, canDo: "Ich kann " + title + ".", state: state,
            scenarios: [{ id: id.toLowerCase() + "-a", title: title + " A", state: state === "done" ? "done" : state }],
            due: 0, dueText: "", note: "", folded: false
        }, extra || {})
    }

    readonly property var levels: [{
        level: "N5 · A1",
        units: [
            unit("N5-01", "Grüßen", "done", { due: 2, dueText: "◆ 2 fällig", note: "◆ 2 fällig" }),
            unit("N5-02", "Vorstellen", "current", {
                note: "Als nächstes",
                scenarios: [{ id: "n5-02-a", title: "Sich vorstellen", state: "current" },
                            { id: "n5-02-b", title: "Nach dem Namen fragen", state: "open" }]
            }),
            unit("N5-03", "Konbini", "locked", { note: "nach Vorstellen (N5-02)" }),
            unit("N5-04", "Essen", "locked", { note: "nach Konbini (N5-03)" }),
            unit("N5-05", "Reisen", "locked", { note: "nach Essen (N5-04)" }),
            unit("N5-06", "Arzt", "locked", { note: "nach Reisen (N5-05)", folded: true }),
            unit("N5-07", "Bank", "locked", { note: "nach Arzt (N5-06)", folded: true })
        ],
        foldedCount: 2,
        foldedNames: "Arzt, Bank"
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
            compare(found.length, 5)
            compare(found[0].model[0].title, "N5-01 · Grüßen")
            compare(found[0].model[0].meta, "◆ 2 fällig")
            compare(found[1].model[0].meta, "Als nächstes")
            compare(found[2].model[0].meta, "nach Vorstellen (N5-02)")
            verify(fold(r.page))
            compare(fold(r.page).subtitle, "Arzt, Bank")
        }

        function test_the_fold_button_shows_the_rest() {
            const r = page()
            fold(r.page).clicked()
            compare(paths(r.page).length, 7)
            verify(!fold(r.page))
        }

        function test_the_due_part_of_a_done_unit_opens_its_review() {
            const r = page()
            paths(r.page)[0].activated(0)
            compare(r.kaiwa.last("openReview")[0], "n5-01")
            compare(r.talks.length, 0)
        }

        function test_the_next_unit_lists_its_scenarios_with_a_start_each() {
            const r = page()
            button(r.page, "Nach dem Namen fragen").clicked()
            compare(r.talks[0], "n5-02-b")
            paths(r.page)[1].activated(0)
            compare(r.talks[1], "n5-02-a")
        }

        function test_a_locked_unit_starts_nothing() {
            const r = page()
            verify(!button(r.page, "Konbini A"))
        }
    }
}
