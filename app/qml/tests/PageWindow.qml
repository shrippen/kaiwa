import QtQuick
import org.kde.kirigami as Kirigami
import ".."

// The window the app's pages are made in for the QML tests. The pages ask
// `applicationWindow().compact`; Kirigami answers that for what is created in the scope of
// its application window, so the test makes the page here and puts it in its own item. The
// window is never shown.
Kirigami.ApplicationWindow {
    readonly property bool compact: false
    readonly property Component conversation: Component { ConversationPage { } }
    readonly property Component path: Component { PathPage { } }
    readonly property Component settings: Component { SettingsPage { } }
}
