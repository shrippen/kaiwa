import QtQuick
import ".."
import org.shrippen.kaiwa
import "tree.js" as Tree

// The base of the conversation page's tests: the real ConversationPage.qml on the stub
// `Conversation` of tests/stub (no microphone, no models, no tutor). `page` is the page,
// `conv` the stub, whose `calls` record everything the page asks of the app object.
Item {
    id: root
    width: 640
    height: 480

    property var page: null
    property var conv: null

    PageWindow { id: appWindow }

    function find(test) {
        return Tree.find(page, test)
    }
    // A shown button with this text.
    function button(text) {
        return find(i => i.text === text && i.clicked !== undefined && i.visible)
    }
    function talkButton() {
        return find(i => i.talkState !== undefined && i.talkStarted !== undefined)
    }
    function answerField() {
        return find(i => i.placeholderText !== undefined && i.placeholderText !== "")
    }

    Component.onCompleted: {
        page = appWindow.conversation.createObject(root, { width: root.width, height: root.height })
        conv = find(i => i.calls !== undefined)
        // A voice, so the listening aids and the sound chips are there.
        conv.voice = "Kokoro"
    }
}
