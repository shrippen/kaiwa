import QtQuick
import QtTest
import "tree.js" as Tree

// The settings' choices for daily goal, unload time and text size come from the profile view
// (goalSteps, unloadSteps, textScaleSteps), not from the page. Run: tools/qml-tests.sh.
Item {
    id: root
    width: 640
    height: 480

    PageWindow { id: appWindow }
    Component { id: appObject; AppObject { } }

    TestCase {
        name: "SettingsSteps"
        when: windowShown

        function choice(page, title) {
            const row = Tree.find(page, i => i.title === title)
            verify(row !== null, "no row " + title)
            const c = Tree.find(row, i => i.select !== undefined && i.model !== undefined)
            verify(c !== null, "no choice in " + title)
            return c
        }

        function test_the_lists_of_the_view_drive_the_choices() {
            const kaiwa = createTemporaryObject(appObject, root)
            kaiwa.profile = JSON.stringify({
                goal: { kind: "minutes", value: 7 }, goalSteps: [2, 7, 9],
                unloadAfter: null, unloadSteps: [1, 4, null],
                textScale: 1.1, textScaleSteps: [1.0, 1.1, 1.75],
                feedback: "much", feedbackSteps: ["much", "little"],
                keepRecordings: 3, keepSteps: [null, 3, 9]
            })
            const page = createTemporaryObject(appWindow.settings, root, { kaiwa: kaiwa, width: root.width, height: root.height })

            const goal = choice(page, "Tagesziel")
            compare(JSON.stringify(goal.model), JSON.stringify(["2 min", "7 min", "9 min"]))
            compare(goal.currentIndex, 1)
            goal.select(2)
            compare(JSON.stringify(kaiwa.last("setSetting")), JSON.stringify(["goal", "9"]))

            const unload = choice(page, "Modelle entladen")
            compare(JSON.stringify(unload.model), JSON.stringify(["1 min", "4 min", "Nie"]))
            compare(unload.currentIndex, 2)
            unload.select(0)
            compare(JSON.stringify(kaiwa.last("setSetting")), JSON.stringify(["unloadAfter", "1"]))

            const size = choice(page, "Textgröße")
            compare(JSON.stringify(size.model), JSON.stringify(["100 %", "110 %", "175 %"]))
            compare(size.currentIndex, 1)
            size.select(2)
            compare(JSON.stringify(kaiwa.last("setSetting")), JSON.stringify(["textScale", "1.75"]))

            const amount = choice(page, "Verbesserungen")
            compare(JSON.stringify(amount.model), JSON.stringify(["Viel", "Wenig"]))
            compare(amount.currentIndex, 0)
            amount.select(1)
            compare(JSON.stringify(kaiwa.last("setSetting")), JSON.stringify(["feedback", "little"]))

            const keep = choice(page, "Aufnahmen behalten")
            compare(JSON.stringify(keep.model), JSON.stringify(["Nein", "3 Tage", "9 Tage"]))
            compare(keep.currentIndex, 1)
            keep.select(2)
            compare(JSON.stringify(kaiwa.last("setSetting")), JSON.stringify(["keepRecordings", "9"]))
        }
    }
}
