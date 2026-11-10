import QtQuick
import QtQuick.Controls as QQC2
import "."

/**
 * Heat scale: a grid of squares, `levels` a list of 0 to 4 (0 = empty, 4 = full cyan).
 * Steps 22 / 44 / 66 / 100 % of the focus colour over the sunken ground.
 * Axes, legend and tooltips (1.78), all opt-in; without them it draws the bare grid:
 *   rowLabels      one string per row, left of the rows ("" leaves a row unlabelled)
 *   columnLabels   one string per column, above the columns, centred on them (keep them
 *                  within `cellSize`, or leave some out, so they do not run into each other)
 *   tips           one string per cell; when set, hovering a cell shows it as a tooltip, and
 *                  screen readers hear it as the cell's name
 *   legendLess, legendMore  a line under the grid: the first text, the levels 0 to 4 as
 *                  swatches, the second text (e.g. "weniger" / "mehr"); shown when either is set
 * Set `Accessible.name` on the chart itself to name it for screen readers.
 */
Item {
    id: heat

    enum Slot {
        Corner,
        ColumnLabel,
        RowLabel,
        Cell
    }

    property var levels: []
    property int cellSize: KanteStyle.unit(16)
    property int columns: 14
    property real spacing: 3
    property var rowLabels: []
    property var columnLabels: []
    property var tips: []
    property string legendLess: ""
    property string legendMore: ""
    /** Colour of a level; apps read it for their legend. */
    function levelColor(l) {
        return KanteStyle.levelColor(l, 4)
    }

    readonly property int levelCount: 4
    readonly property int tipDelay: 300
    readonly property bool hasRowLabels: rowLabels.length > 0
    readonly property bool hasColumnLabels: columnLabels.length > 0
    readonly property bool hasLegend: legendLess !== "" || legendMore !== ""

    // What the grid holds, in reading order: the header (column labels), then per row its label
    // and its cells. Without labels this is the cells alone.
    //   [corner] [col 0] [col 1] ...
    //   [row 0 ] [cell ] [cell ] ...
    readonly property var slots: {
        const out = []
        const rows = Math.ceil(levels.length / columns)
        if (hasColumnLabels) {
            if (hasRowLabels) {
                out.push({ kind: KanteHeatmap.Slot.Corner })
            }
            for (let c = 0; c < columns; c++) {
                out.push({ kind: KanteHeatmap.Slot.ColumnLabel, index: c })
            }
        }
        for (let r = 0; r < rows; r++) {
            if (hasRowLabels) {
                out.push({ kind: KanteHeatmap.Slot.RowLabel, index: r })
            }
            for (let c = 0; c < columns && r * columns + c < levels.length; c++) {
                out.push({ kind: KanteHeatmap.Slot.Cell, index: r * columns + c })
            }
        }
        return out
    }

    /** One square of the scale: the sunken ground, with the level's colour over it. */
    component Swatch: Item {
        property int level: 0
        width: heat.cellSize
        height: heat.cellSize
        Rectangle { anchors.fill: parent; color: KanteStyle.sunkenColor }
        Rectangle { anchors.fill: parent; color: level > 0 ? heat.levelColor(level) : "transparent" }
    }

    implicitWidth: frame.implicitWidth
    implicitHeight: frame.implicitHeight

    Column {
        id: frame
        spacing: KanteStyle.unit(6)

        Grid {
            spacing: heat.spacing
            columns: heat.columns + (heat.hasRowLabels ? 1 : 0)

            Repeater {
                model: heat.slots
                delegate: Item {
                    id: slot
                    required property var modelData
                    readonly property int kind: modelData.kind
                    readonly property int at: modelData.index === undefined ? -1 : modelData.index
                    readonly property string tip: kind === KanteHeatmap.Slot.Cell ? String(heat.tips[at] || "") : ""
                    readonly property string label: kind === KanteHeatmap.Slot.RowLabel ? String(heat.rowLabels[at] || "")
                                                  : kind === KanteHeatmap.Slot.ColumnLabel ? String(heat.columnLabels[at] || "") : ""
                    readonly property int level: kind === KanteHeatmap.Slot.Cell ? Math.max(0, Math.min(heat.levelCount, heat.levels[at])) : 0

                    width: kind === KanteHeatmap.Slot.Corner ? 1
                         : kind === KanteHeatmap.Slot.RowLabel ? name.implicitWidth + KanteStyle.unit(6) : heat.cellSize
                    height: kind === KanteHeatmap.Slot.Corner ? 1
                          : kind === KanteHeatmap.Slot.ColumnLabel ? name.implicitHeight + KanteStyle.unit(2) : heat.cellSize

                    Text {
                        id: name
                        visible: slot.label !== ""
                        text: slot.label
                        color: KanteStyle.mutedTextColor
                        font: KanteStyle.labelFont()
                        anchors.verticalCenter: slot.kind === KanteHeatmap.Slot.RowLabel ? parent.verticalCenter : undefined
                        anchors.horizontalCenter: slot.kind === KanteHeatmap.Slot.ColumnLabel ? parent.horizontalCenter : undefined
                    }
                    Rectangle { visible: slot.kind === KanteHeatmap.Slot.Cell; anchors.fill: parent; color: KanteStyle.sunkenColor }
                    Rectangle {
                        visible: slot.kind === KanteHeatmap.Slot.Cell
                        anchors.fill: parent
                        color: slot.level > 0 ? heat.levelColor(slot.level) : "transparent"
                    }
                    HoverHandler { id: hover; enabled: slot.tip !== "" }
                    QQC2.ToolTip.text: slot.tip
                    QQC2.ToolTip.visible: hover.hovered && slot.tip !== ""
                    QQC2.ToolTip.delay: heat.tipDelay
                    Accessible.role: Accessible.StaticText
                    Accessible.name: slot.tip
                    Accessible.ignored: slot.tip === ""
                }
            }
        }

        Row {
            visible: heat.hasLegend
            spacing: KanteStyle.unit(4)

            Text {
                visible: heat.legendLess !== ""
                anchors.verticalCenter: parent.verticalCenter
                text: heat.legendLess
                color: KanteStyle.mutedTextColor
                font: KanteStyle.labelFont()
            }
            Repeater {
                model: heat.levelCount + 1
                delegate: Swatch { required property int index; level: index }
            }
            Text {
                visible: heat.legendMore !== ""
                anchors.verticalCenter: parent.verticalCenter
                text: heat.legendMore
                color: KanteStyle.mutedTextColor
                font: KanteStyle.labelFont()
            }
        }
    }
}
