import QtQuick
import QtTest
import "tree.js" as Tree

// /LF700/, ADR-010 step 6: the sync tile. Before the first sync it offers "begin here" and
// "join", afterwards only "sync now"; the passphrase goes to the keyring through its button;
// the buttons wait while a sync runs. Run: tools/qml-tests.sh.
Item {
    id: root
    width: 640
    height: 480

    PageWindow { id: appWindow }
    Component { id: appObject; AppObject { } }

    TestCase {
        name: "SyncTile"
        when: windowShown

        function page(sync) {
            const kaiwa = createTemporaryObject(appObject, root)
            kaiwa.profile = JSON.stringify({ goal: {}, sync: sync })
            const p = createTemporaryObject(appWindow.settings, root, { kaiwa: kaiwa, width: root.width, height: root.height })
            return { kaiwa: kaiwa, page: p }
        }

        function button(item, text) {
            return Tree.find(item, i => i.text === text && i.clicked !== undefined)
        }

        // Scrolls `item` below the sticky header, as a learner would, and clicks it.
        function click(p, item) {
            const flick = Tree.find(p, i => i.flickableDirection === Flickable.VerticalFlick)
            flick.contentY = item.mapToItem(flick.contentItem, 0, 0).y - 250
            mouseClick(item)
        }

        function test_the_first_sync_asks_begin_or_join() {
            const t = page({ kind: "folder", folder: "/home/jonas/Kaiwa", passphrase: true, syncs: false, text: "Noch nicht abgeglichen" })
            const here = button(t.page, "Hier beginnen")
            verify(here !== null && here.visible, "no begin button")
            verify(button(t.page, "Beitreten").visible, "no join button")
            verify(!button(t.page, "Jetzt abgleichen").visible, "sync now before the first sync")
            click(t.page, here)
            compare(JSON.stringify(t.kaiwa.last("syncNow")), JSON.stringify(["here"]))
        }

        function test_a_device_that_syncs_only_syncs_now() {
            const t = page({ kind: "folder", folder: "/home/jonas/Kaiwa", passphrase: true, syncs: true, text: "Dieses Gerät gleicht ab." })
            verify(!button(t.page, "Hier beginnen").visible, "begin once more")
            const now = button(t.page, "Jetzt abgleichen")
            click(t.page, now)
            compare(JSON.stringify(t.kaiwa.last("syncNow")), JSON.stringify(["join"]))

            // While a sync runs the button waits.
            t.kaiwa.syncing = true
            verify(!now.parent.enabled, "the buttons take a second sync")
        }

        function test_the_passphrase_goes_to_the_keyring_through_its_button() {
            const t = page({ kind: "folder", folder: "/home/jonas/Kaiwa", passphrase: false, syncs: false, keyring: true, text: "" })
            const field = Tree.find(t.page, i => i.echoMode !== undefined && i.Accessible.name === "Passphrase")
            verify(field !== null, "no passphrase field")
            field.text = "kirschbluete"
            const keep = Tree.find(field.parent, i => i.text === "Im Schlüsselbund speichern" && i.clicked !== undefined)
            click(t.page, keep)
            compare(JSON.stringify(t.kaiwa.last("setSyncSecret")), JSON.stringify(["passphrase", "kirschbluete"]))
            compare(field.text, "")
        }

        function test_without_a_keyring_the_tile_says_until_quitting() {
            const t = page({ kind: "folder", folder: "/home/jonas/Kaiwa", passphrase: true, syncs: false, keyring: false, text: "" })
            // The tutor tile has a keyring button of its own: only this tile counts.
            const tile = Tree.find(t.page, i => i.title === "Abgleich" && i.countText !== undefined)
            verify(button(tile, "Im Schlüsselbund speichern") === null, "a keyring that is not there")
            verify(button(tile, "Übernehmen") !== null, "no button to use the passphrase")
            const row = Tree.find(t.page, i => i.title === "Passphrase")
            verify(row.hint.indexOf("Gilt bis zum Beenden") >= 0, row.hint)
        }

        function test_without_a_place_only_the_choice_shows() {
            const t = page({ kind: "none", text: "Kein Ort zum Abgleichen gewählt." })
            const here = button(t.page, "Hier beginnen")
            verify(here === null || !here.visible, "a sync without a place")
        }
    }
}
