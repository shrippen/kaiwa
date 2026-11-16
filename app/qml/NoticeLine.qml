import QtQuick
import QtQuick.Controls as QQC2
import Kante

// The app's notice (`kaiwa.notice`) on the pages that have no line of their own for it: what
// failed to load or save, as text under the tabs. Empty notice, no line.
QQC2.Label {
    id: line

    // The app object (`kaiwa`).
    property var kaiwa
    // False where the page shows the notice itself.
    property bool wanted: true

    leftPadding: KanteStyle.unit(18)
    rightPadding: KanteStyle.unit(18)
    topPadding: KanteStyle.unit(6)
    bottomPadding: KanteStyle.unit(6)
    visible: wanted && kaiwa.notice.length > 0
    text: kaiwa.notice
    wrapMode: Text.Wrap
    font: KanteStyle.defaultFont
    color: KanteStyle.textColor
}
