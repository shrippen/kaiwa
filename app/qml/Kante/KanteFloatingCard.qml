import QtQuick
import "."

/**
 * Floating card (Kante 1.85, for Klappe): a Kante card with a hard shadow that floats over
 * content, such as a search card over a borderless floor plan. No head line; for one that
 * folds, see `KanteFloatingPanel`, which draws its card with this. The caller places it
 * (`x`, `y`, or anchors) and puts the content inside as children.
 *
 *   padding   space between the edge and the content (default `KanteStyle.unit(10)`)
 *
 * The card is as wide and high as its children plus the padding on both sides, so a child
 * must not size itself from the card's width. The shadow is the card's shape again, 3 px
 * down and right, and shows in Kante only (`KanteStyle.themed`); it lies outside the card's
 * size.
 */
Item {
    id: card

    property real padding: KanteStyle.unit(10)
    default property alias content: body.data

    implicitWidth: body.childrenRect.x + body.childrenRect.width + 2 * padding
    implicitHeight: body.childrenRect.y + body.childrenRect.height + 2 * padding

    // The hard shadow: the card's shape again, 3 px down and right (Kante only).
    KantePolygon {
        visible: KanteStyle.themed
        x: KanteStyle.unit(3)
        y: KanteStyle.unit(3)
        z: -1
        width: card.width
        height: card.height
        fillColor: KanteStyle.tint(KanteStyle.strongTextColor, 0.2)
        cutTopRight: KanteStyle.chamferSmall
    }

    KanteCard {
        anchors.fill: parent
        color: KanteStyle.dialogColor
        borderColor: KanteStyle.frameColor
        chamfer: KanteStyle.chamferSmall
    }

    Item {
        id: body
        x: card.padding
        y: card.padding
        width: card.width - 2 * card.padding
        height: card.height - 2 * card.padding
    }
}
