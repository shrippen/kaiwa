pragma ComponentBehavior: Bound
import QtQuick
import "."

/**
 * Scroll area (Kante 1.83, for Klecks' side panel): a vertical scroll container with a scroll
 * bar that stays visible while there is something to scroll. Children go in as content, e.g.
 * a ColumnLayout with `width: parent.width`; the content is as high as its children (their
 * height, a layout's implicit height) and as wide as the area less the bar.
 *   wheelLines     lines one wheel notch scrolls (default 3, a line is `lineHeight`); at most half
 *                  the view. Touchpad pixel deltas scroll 1:1 and at once
 *   lineHeight     px of a line (default KanteStyle.unit(24))
 *   contentY       the scroll position (writable); scrollBy(px), scrollToTop(), scrollToEnd()
 *   scrollable     content is higher than the view; only then the bar shows and takes its gutter
 *   barWidth       width of the bar's gutter
 * Bar: drag the handle, click the track to page, wheel anywhere. Keys (when the area has focus):
 * Page Up / Down, Home, End. Notches ease over `durationFast` with KanteStyle.animate, else jump.
 * A child that scrolls itself (a ListView, a Flickable) keeps its wheel events while it can
 * move in that direction; at its end the area takes them. Kante, Kante Light and System: all
 * colours and sizes from KanteStyle. Wheel with Ctrl, Shift or another modifier is not taken: it
 * goes on to the parent. No touch drag or flick: wheel, touchpad, bar and keys scroll.
 */
Item {
    id: area

    default property alias content: holder.data
    property int wheelLines: 3
    property real lineHeight: KanteStyle.unit(24)
    property real barWidth: KanteStyle.unit(10)
    property alias contentY: flick.contentY
    readonly property alias contentHeight: flick.contentHeight
    readonly property bool scrollable: shown
    readonly property real maxY: Math.max(0, flick.contentHeight - flick.height)

    // Set by refresh(), not bound: the gutter narrows the content, which changes its height,
    // which is what decides the gutter. Assigned, that settles instead of looping.
    property bool shown: false

    // Where the next scroll ends; follows contentY when nothing eases.
    property real target: 0

    // Later, outside the evaluation of the content's height that triggered it.
    function refresh() {
        Qt.callLater(function () {
            area.shown = flick.contentHeight > flick.height + 0.5
        })
    }

    function clamp(y) {
        return Math.max(0, Math.min(maxY, y))
    }

    function scrollTo(y, immediate) {
        target = clamp(y)
        if (immediate || KanteStyle.durationFast === 0) {
            ease.stop()
            flick.contentY = target
            return
        }
        ease.to = target
        ease.restart()
    }

    function scrollBy(px) {
        scrollTo((ease.running ? target : flick.contentY) + px, false)
    }

    function scrollToTop() {
        scrollTo(0, false)
    }

    function scrollToEnd() {
        scrollTo(maxY, false)
    }

    implicitWidth: KanteStyle.unit(240)
    implicitHeight: KanteStyle.unit(240)
    clip: true
    activeFocusOnTab: false

    Keys.onPressed: function (event) {
        event.accepted = true
        if (event.key === Qt.Key_PageUp) {
            scrollBy(-flick.height * 0.9)
        } else if (event.key === Qt.Key_PageDown) {
            scrollBy(flick.height * 0.9)
        } else if (event.key === Qt.Key_Home) {
            scrollToTop()
        } else if (event.key === Qt.Key_End) {
            scrollToEnd()
        } else {
            event.accepted = false
        }
    }

    Flickable {
        id: flick
        objectName: "kanteScrollFlick"
        anchors.fill: parent
        contentWidth: width
        contentHeight: holder.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        // Off: the Flickable's own wheel handling (Qt's slow step, also with Ctrl or Shift) is
        // gone, so a wheel with a modifier reaches the parent's handlers. Touch drag goes with it.
        interactive: false
        clip: true

        onContentHeightChanged: area.refresh()
        onHeightChanged: area.refresh()
        onContentYChanged: {
            if (!ease.running) {
                area.target = contentY
            }
        }

        Item {
            id: holder
            objectName: "kanteScrollHolder"
            width: flick.width - (area.scrollable ? area.barWidth + KanteStyle.unit(4) : 0)
            implicitHeight: childrenRect.y + childrenRect.height
        }

        // Qt's own wheel step is small on Linux. One notch is `wheelLines` lines; a touchpad's
        // pixel deltas pass through 1:1.
        WheelHandler {
            id: wheel
            target: null
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            acceptedModifiers: Qt.NoModifier
            enabled: area.scrollable
            onWheel: function (event) {
                if (event.pixelDelta.y !== 0) {
                    area.scrollTo(flick.contentY - event.pixelDelta.y, true)
                    return
                }
                const notches = event.angleDelta.y / 120
                const step = Math.min(area.wheelLines * area.lineHeight, flick.height / 2)
                area.scrollBy(-notches * step)
            }
        }
    }

    NumberAnimation {
        id: ease
        target: flick
        property: "contentY"
        duration: KanteStyle.durationFast
        easing.type: Easing.OutCubic
    }

    // The bar: a track the full height and a handle sized by the share of the content in view.
    Item {
        id: bar
        objectName: "kanteScrollBar"
        visible: area.scrollable
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: area.barWidth

        readonly property real minHandle: KanteStyle.unit(28)
        readonly property real handleHeight: Math.min(height, Math.max(minHandle, height * flick.height / Math.max(1, flick.contentHeight)))
        readonly property real room: Math.max(1, height - handleHeight)
        readonly property real handleY: area.maxY > 0 ? area.clamp(flick.contentY) / area.maxY * room : 0

        Rectangle {
            id: track
            anchors.fill: parent
            color: KanteStyle.sunkenColor
            radius: KanteStyle.edgeRadius
            border.width: KanteStyle.borderWidth
            border.color: KanteStyle.ruleColor
        }

        Rectangle {
            id: handle
            objectName: "kanteScrollHandle"
            x: KanteStyle.borderWidth
            y: bar.handleY
            width: parent.width - 2 * KanteStyle.borderWidth
            height: bar.handleHeight
            radius: KanteStyle.edgeRadius
            color: mouse.dragging ? KanteStyle.focusColor : (mouse.containsMouse ? KanteStyle.accentColor : KanteStyle.mutedTextColor)
            Behavior on color {
                ColorAnimation { duration: KanteStyle.durationFast }
            }
        }

        MouseArea {
            id: mouse
            anchors.fill: parent
            hoverEnabled: true
            property bool dragging: false
            property real grabY: 0

            onPressed: function (e) {
                const onHandle = e.y >= bar.handleY && e.y <= bar.handleY + bar.handleHeight
                if (!onHandle) {
                    // Click on the track: one page towards the click.
                    area.scrollBy((e.y < bar.handleY ? -1 : 1) * flick.height * 0.9)
                    return
                }
                dragging = true
                grabY = e.y - bar.handleY
                ease.stop()
            }
            onPositionChanged: function (e) {
                if (!dragging) {
                    return
                }
                const y = (e.y - grabY) / bar.room * area.maxY
                area.scrollTo(y, true)
            }
            onReleased: dragging = false
            onCanceled: dragging = false
        }
    }
}
