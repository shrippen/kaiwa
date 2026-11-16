import QtQuick
import QtTest
import "tree.js" as Tree

// /LL401/, F-08: the settings name source, licence and size of a model before its download
// button, and the download starts only from that button (lena_download_announced).
// Run: tools/qml-tests.sh.
Item {
    id: root
    width: 640
    height: 480

    PageWindow { id: appWindow }
    Component { id: appObject; AppObject { } }

    TestCase {
        name: "ModelDownload"
        when: windowShown

        function test_the_row_names_its_terms_and_downloads_only_on_a_click() {
            const kaiwa = createTemporaryObject(appObject, root)
            kaiwa.models = JSON.stringify([{
                name: "Whisper small", purpose: "Spracherkennung", licence: "MIT-Lizenz",
                origin: "huggingface.co/example", downloadMB: 466, diskMB: 610, dir: "asr", present: false
            }])
            const page = createTemporaryObject(appWindow.settings, root, { kaiwa: kaiwa, width: root.width, height: root.height })

            const row = Tree.find(page, i => i.title === "Whisper small")
            verify(row !== null, "no row for the model")
            for (const term of ["MIT-Lizenz", "huggingface.co/example", "466 MB"]) {
                verify(row.hint.indexOf(term) >= 0, term + " in: " + row.hint)
            }
            compare(kaiwa.count("downloadModel"), 0)

            const button = Tree.find(row, i => i.text === "Herunterladen" && i.clicked !== undefined)
            verify(button !== null, "no download button")
            // Scrolled to the button below the sticky header, as a learner would.
            const flick = Tree.find(page, i => i.flickableDirection === Flickable.VerticalFlick)
            flick.contentY = button.mapToItem(flick.contentItem, 0, 0).y - 250
            mouseClick(button)
            compare(JSON.stringify(kaiwa.last("downloadModel")), JSON.stringify(["asr"]))
        }
    }
}
