import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Mistakes per talk (/LF610/): per kind of mistake a line over the last talks with its trend
// in a word; on hover the day, the scenario and the number of mistakes of that talk. The
// view brings the talks (`talks`: day and scenario) and per kind the counts, aligned.
ColumnLayout {
    id: charts

    // The app object (`kaiwa`), the stats view's `talks` and `mistakes`.
    required property var kaiwa
    property var talks: []
    property var mistakes: []
    property bool compact: false

    function t(de, en) { return kaiwa.lang === "en" ? en : de }
    // All lines on one scale so they compare; the axis has four steps, the top a multiple of
    // four so that its labels are whole numbers.
    readonly property real scaleTop: {
        let m = 4
        for (const k of mistakes) {
            m = Math.max(m, ...k.counts)
        }
        return Math.ceil(m / 4) * 4
    }
    readonly property var locale: Qt.locale(kaiwa.lang === "en" ? "en_GB" : "de_DE")
    // "2026-11-03" as "3.11." (en: "3 Nov"), from the date's parts: no time zone shifts it.
    function dayText(iso) {
        const [y, m, d] = iso.split("-").map(Number)
        return new Date(y, m - 1, d).toLocaleDateString(locale, kaiwa.lang === "en" ? "d MMM" : "d.M.")
    }
    function tip(i) { return dayText(talks[i].date) + " · " + talks[i].scenario }

    spacing: KanteStyle.unit(8)

    KanteSectionLabel {
        Layout.fillWidth: true
        text: t("Fehler je Gespräch · letzte ", "Mistakes per talk · last ") + talks.length + t(" Gespräche", " talks")
        rule: true
    }
    GridLayout {
        Layout.fillWidth: true
        columns: charts.compact ? 1 : 2
        columnSpacing: KanteStyle.unit(16)
        rowSpacing: KanteStyle.unit(16)
        Repeater {
            model: charts.mistakes
            delegate: Item {
                id: card
                required property var modelData
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                implicitHeight: inner.implicitHeight + KanteStyle.unit(24)

                KanteCard { anchors.fill: parent }
                ColumnLayout {
                    id: inner
                    anchors.fill: parent
                    anchors.margins: KanteStyle.unit(12)
                    spacing: KanteStyle.unit(6)
                    RowLayout {
                        Layout.fillWidth: true
                        QQC2.Label {
                            text: card.modelData.word
                            font: KanteStyle.defaultFont
                            color: KanteStyle.strongTextColor
                        }
                        Item { Layout.fillWidth: true }
                        QQC2.Label {
                            visible: text !== ""
                            text: card.modelData.trend
                            font: KanteStyle.smallFont
                            color: KanteStyle.mutedTextColor
                        }
                    }
                    KanteLineChart {
                        Layout.fillWidth: true
                        Layout.preferredHeight: KanteStyle.unit(150)
                        series: [card.modelData.counts]
                        axis: true
                        maxValue: charts.scaleTop
                        yTitle: charts.t("Fehler", "Mistakes")
                        xTitle: charts.t("Gespräch", "Talk")
                        xStart: charts.talks.length > 0 ? charts.dayText(charts.talks[0].date) : ""
                        xEnd: charts.talks.length > 1 ? charts.dayText(charts.talks[charts.talks.length - 1].date) : ""
                        labels: charts.talks.map((_, i) => charts.tip(i))
                        unit: ""
                        Accessible.role: Accessible.Graphic
                        Accessible.name: card.modelData.word + ": " + card.modelData.counts.join(", ")
                    }
                }
            }
        }
    }
    QQC2.Label {
        Layout.fillWidth: true
        text: charts.t("Jeder Punkt ist ein Gespräch.", "Each point is a talk.")
        wrapMode: Text.Wrap
        font: KanteStyle.smallFont
        color: KanteStyle.mutedTextColor
    }
}
