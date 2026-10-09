import QtQuick
import org.kde.kirigami as Kirigami
import Kante

// Kaiwa's window. Style follows the Kante rule: System by default, Kante and Kante Light on
// request (the page header switches; `--kind 0|1|2` on the command line for screenshots).
Kirigami.ApplicationWindow {
    id: root
    width: Number(arg("--width", "900"))
    height: Number(arg("--height", "760"))
    visible: true
    title: "Kaiwa"

    readonly property var args: Qt.application.arguments
    function arg(name, fallback) {
        var i = args.indexOf(name)
        return i >= 0 && i + 1 < args.length ? args[i + 1] : fallback
    }
    property int styleKind: Number(arg("--kind", "0"))
    Binding { target: KanteStyle; property: "kind"; value: root.styleKind }
    Binding { target: KanteStyle; property: "preferDark"; value: root.arg("--light", "0") !== "1" }

    pageStack.initialPage: ConversationPage {
        styleKind: root.styleKind
        onStyleKindChangeRequested: function (k) { root.styleKind = k }
        shot: root.arg("--shot", "")
        script: root.arg("--say", "")
    }
}
