import QtQuick
import QtTest
import "tree.js" as Tree

// The settings are tiles (design A, as in Kontra): nine groups in order, a tab in the sticky
// header for each, a count that matches the rows shown, and the Sprechen choice writes the
// same setting as the segment row did. Run: tools/qml-tests.sh.
Item {
    id: root
    width: 640
    height: 480

    PageWindow { id: appWindow }
    Component { id: appObject; AppObject { } }

    TestCase {
        name: "SettingsGroups"
        when: windowShown

        readonly property var titles: ["Profil", "Üben", "Sprechen", "Aussehen", "Tutor",
                                       "Was verlässt das Gerät?", "Modelle", "Deine Daten", "Über Kaiwa"]

        function page(profile) {
            const kaiwa = createTemporaryObject(appObject, root)
            kaiwa.profile = JSON.stringify(profile || { goal: {} })
            kaiwa.privacy = JSON.stringify([{ part: "Tutor", provider: "Ollama", sends: "Text", where: "lokal", cloud: false }])
            kaiwa.models = JSON.stringify([{
                name: "Whisper small", purpose: "Spracherkennung", licence: "MIT-Lizenz", origin: "example.org",
                downloadMB: 466, diskMB: 610, dir: "asr", present: true
            }])
            return { kaiwa: kaiwa, page: createTemporaryObject(appWindow.settings, root, { kaiwa: kaiwa, width: root.width, height: root.height }) }
        }

        // The groups in the order of the page.
        function groups(item, found) {
            const all = found || []
            if (item.settingGroup === true) {
                all.push(item)
                return all
            }
            for (let c = 0; c < item.children.length; c++) {
                groups(item.children[c], all)
            }
            return all
        }

        // The rows a learner sees: visible settings, not looking into other rows.
        function shownRows(item) {
            let n = 0
            for (let c = 0; c < item.children.length; c++) {
                const kid = item.children[c]
                if (!kid.visible) {
                    continue
                }
                n += kid.isSetting === true ? 1 : shownRows(kid)
            }
            return n
        }

        function test_nine_groups_in_order() {
            const p = page().page
            const found = groups(Tree.find(p, i => i.flickableDirection === Flickable.VerticalFlick).contentItem)
            compare(JSON.stringify(found.map(g => g.title)), JSON.stringify(titles))
        }

        function test_the_header_has_a_tab_per_group() {
            const p = page().page
            const tabs = Tree.find(p, i => i.objectName === "headerTabs")
            verify(tabs !== null, "no tab bar in the header")
            compare(JSON.stringify(tabs.model), JSON.stringify(titles))
        }

        function test_a_count_matches_the_rows_shown() {
            const p = page({ goal: {}, key: { label: "Anthropic", text: "", canStore: true, stored: false },
                             tutor: { provider: "x", presets: [{ id: "x", label: "X", needsModel: true, needsServer: false, exampleModel: "m", server: "" }] } }).page
            const found = groups(Tree.find(p, i => i.flickableDirection === Flickable.VerticalFlick).contentItem)
            let counted = 0
            for (const g of found) {
                if (g.countText.indexOf("Einstellung") < 0) {
                    continue
                }
                counted++
                tryCompare(g, "count", shownRows(g), 2000, g.title)
                compare(g.countText, g.count === 1 ? "1 Einstellung" : g.count + " Einstellungen", g.title)
            }
            verify(counted >= 5, "too few counted groups: " + counted)
        }

        function test_speaking_choice_sets_the_same_setting() {
            const k = page({ goal: {}, handsFree: false })
            const free = Tree.find(k.page, i => i.text === "Freisprechen" && i.checkable === true)
            const hold = Tree.find(k.page, i => i.text === "Taste halten" && i.checkable === true)
            verify(free !== null && hold !== null, "no tiles")
            verify(hold.checked && !free.checked)
            free.click()
            compare(JSON.stringify(k.kaiwa.last("setSetting")), JSON.stringify(["handsFree", "true"]))
            hold.click()
            compare(JSON.stringify(k.kaiwa.last("setSetting")), JSON.stringify(["handsFree", "false"]))
        }
    }
}
