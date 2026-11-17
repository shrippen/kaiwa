import QtQuick
import QtTest
import "tree.js" as Tree

// Two rows around the speech models. The talk page without the recognition model shows one
// calm line with "Modelle laden" instead of the talk button's steps, and the button leaves for
// the settings. The mora check's row, while its model is missing, shows a badge and a download
// button of the same height, with the size from the profile view; once loaded it is a switch.
// Run: tools/qml-tests.sh.
Item {
    id: root
    width: 640
    height: 480

    PageWindow { id: appWindow }
    Component { id: appObject; AppObject { } }
    Component { id: talkPage; ConversationTest { } }

    TestCase {
        name: "SpeechRows"
        when: windowShown

        function moraRow(profile) {
            const kaiwa = createTemporaryObject(appObject, root)
            kaiwa.profile = JSON.stringify(profile)
            const page = createTemporaryObject(appWindow.settings, root, { kaiwa: kaiwa, width: root.width, height: root.height })
            const row = Tree.find(page, i => i.title === "Aussprache-Hinweise")
            verify(row !== null, "no row for the mora check")
            return { kaiwa: kaiwa, row: row, page: page }
        }

        function test_without_the_model_the_badge_and_the_button_are_equally_tall() {
            const r = moraRow({ goal: {}, moraHints: true, moraModel: { present: false, id: "kaiwa-kana-ctc", downloadMB: 356, size: "356 MB", licence: "Apache-2.0" } })
            const badge = Tree.find(r.row, i => i.text === "Braucht Modell" && i.status !== undefined)
            const button = Tree.find(r.row, i => i.text === "Laden · 356 MB" && i.clicked !== undefined)
            verify(badge !== null && badge.visible, "no badge")
            verify(button !== null && button.visible, "no button")
            verify(button.height > 0)
            compare(badge.height, button.height)
            compare(badge.mapToItem(r.row, 0, 0).y + badge.height / 2, button.mapToItem(r.row, 0, 0).y + button.height / 2)
            verify(Tree.find(r.row, i => i.checkable === true && i.visible && (i.text === "An" || i.text === "Aus")) === null, "no switch before the model is there")

            // Scrolled to the button below the sticky header, as a learner would.
            const flick = Tree.find(r.page, i => i.flickableDirection === Flickable.VerticalFlick)
            flick.contentY = button.mapToItem(flick.contentItem, 0, 0).y - 250
            mouseClick(button)
            compare(JSON.stringify(r.kaiwa.last("downloadModel")), JSON.stringify(["kaiwa-kana-ctc"]))
        }

        function test_with_the_model_it_is_a_plain_switch() {
            const r = moraRow({ goal: {}, moraHints: true, moraModel: { present: true, id: "kaiwa-kana-ctc", downloadMB: 356 } })
            verify(Tree.find(r.row, i => i.text === "Braucht Modell") === null || !Tree.find(r.row, i => i.text === "Braucht Modell").visible)
            verify(Tree.find(r.row, i => i.checkable === true && i.visible && (i.text === "An" || i.text === "Aus")) !== null, "a switch")
            verify(Tree.find(r.row, i => i.text === "Laden · 356 MB" && i.visible) === null)
        }
    }

    TestCase {
        name: "SpeechMissingLine"
        when: windowShown

        function test_the_line_replaces_the_steps_and_leads_to_the_settings() {
            const t = createTemporaryObject(talkPage, root, { width: root.width, height: root.height })
            const talk = t.talkButton()
            verify(talk.visible, "the steps before")
            t.conv.speechMissing = true
            verify(!talk.visible, "no steps without the model")
            const load = t.button("Modelle laden")
            verify(load !== null, "no button to the models")
            verify(t.find(i => typeof i.text === "string" && i.text.indexOf("Bis dahin tippst du.") >= 0 && i.visible) !== null)

            const shown = createTemporaryObject(spy, root, { target: t.page, signalName: "showSettings" })
            wait(50)
            mouseClick(load)
            compare(shown.count, 1)
            compare(t.conv.count("interrupt"), 1)
        }
    }
    Component { id: spy; SignalSpy { } }
}
