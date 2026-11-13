import QtQuick
import Kante

// A segment row of the settings that shows a value of the profile. A click sets KanteSegmented's
// currentIndex and so breaks its binding; bound again here (as in KanteTransportBar), the row
// keeps showing the profile after a profile switch, an import or a change made elsewhere.
KanteSegmented {
    id: choice

    /** The index of the profile's value. */
    property int shown: 0

    currentIndex: shown
    Component.onCompleted: activated.connect(function () {
        choice.currentIndex = Qt.binding(function () { return choice.shown })
    })
}
