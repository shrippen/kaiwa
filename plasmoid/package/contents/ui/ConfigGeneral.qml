import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM

// Style (/LF504/): the Plasma theme by default; Kante and Kante Light are opt-in.
KCM.SimpleKCM {
    property alias cfg_style: style.currentIndex

    Kirigami.FormLayout {
        QQC2.ComboBox {
            id: style
            Kirigami.FormData.label: i18n("Style:")
            model: [i18n("System"), "Kante", "Kante Light"]
        }
    }
}
