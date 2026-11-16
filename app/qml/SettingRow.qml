import QtQuick.Layouts
import Kante

// A row of a settings tile: fills the tile, and goes narrow (control below the title) in a
// compact window.
KanteSettingRow {
    Layout.fillWidth: true
    narrow: applicationWindow().compact
}
