import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami
import "Kante"

// Kaiwa in the panel (Lastenheft /LF500/–/LF507/): compact a daily-goal meter with the streak,
// expanded a short talk and five reviews. Everything goes through kaiwad (ADR-008); the
// plasmoid works without the app open (/LF506/). Style: System by default, Kante opt-in.
PlasmoidItem {
    id: root

    property var today: null
    readonly property string lang: today && today.lang === "en" ? "en" : "de"
    function t(de, en) { return lang === "en" ? en : de }

    Kaiwad { id: kaiwad }
    function refresh() { kaiwad.call("Today", "", [], function (v) { root.today = v }) }

    Timer {
        interval: 60000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: root.refresh()
    }
    onExpandedChanged: if (expanded) refresh()

    Binding { target: KanteStyle; property: "kind"; value: Plasmoid.configuration.style }

    toolTipMainText: "Kaiwa"
    toolTipSubText: !kaiwad.reachable ? t("Der Kaiwa-Dienst antwortet nicht.", "The Kaiwa service does not answer.")
        : today ? today.goalText + (today.gamification ? " · " + t(today.streakDays + " Tage in Folge", today.streakDays + " days in a row") : "") : ""

    compactRepresentation: MouseArea {
        Layout.minimumWidth: meter.implicitWidth + Kirigami.Units.smallSpacing * 2
        hoverEnabled: true
        onClicked: root.expanded = !root.expanded
        KanteGoalMeter {
            id: meter
            anchors.centerIn: parent
            value: root.today ? root.today.goalFraction : 0
            label: root.today && root.today.gamification ? root.today.streakDays + root.t(" T", " d") : ""
            stackHeight: Math.max(Kirigami.Units.iconSizes.small, parent.height - Kirigami.Units.smallSpacing * 2)
            accessibleName: root.t("Tagesziel", "Daily goal")
        }
    }

    fullRepresentation: Full {
        kaiwad: kaiwad
        app: root
    }
}
