import QtQuick
import QtQuick.Controls as QQC2
import "."

/**
 * Multi-line text field: KanteTextField's look (KanteFieldSkin) around a QQC2.TextArea.
 *   System  the platform's text area (unchanged).
 *   Kante   square sunken box with a 2 px bottom edge, cyan on focus (red when `invalid`),
 *           the placeholder inside the box.
 * Grows with its text up to `maxLines`, then scrolls. Enter starts a new line; Tab and
 * Shift+Tab leave the field (no tab character). Set Accessible.name.
 * Other TextArea properties: `area`.
 *
 *   KanteTextArea { placeholderText: "Contact"; maxLines: 4; Accessible.name: "Contact" }
 */
QQC2.ScrollView {
    id: control

    /** The text, as in a text field. */
    property alias text: area.text
    property alias placeholderText: area.placeholderText
    property alias readOnly: area.readOnly
    property alias wrapMode: area.wrapMode
    property alias selectedText: area.selectedText
    property alias cursorPosition: area.cursorPosition
    /** The inner QQC2.TextArea, for everything not aliased here. */
    readonly property alias area: area
    /** Lines shown before the field stops growing and scrolls. */
    property int maxLines: 6
    /** Kante: mark the field as invalid (red bottom edge). */
    property bool invalid: false

    // Focus and a Tab in the text area reach the field's own text, not the scroll view.
    activeFocusOnTab: false
    implicitWidth: KanteStyle.unit(160)
    implicitHeight: Math.min(
        Math.max(area.implicitHeight, KanteStyle.heightMedium),
        Math.max(1, maxLines) * metrics.lineSpacing + area.topPadding + area.bottomPadding)

    FontMetrics {
        id: metrics
        font: area.font
    }

    // As background, so the skin stays put while the text scrolls.
    background: KanteFieldSkin {
        control: area
        invalid: control.invalid
    }

    QQC2.TextArea {
        id: area
        wrapMode: TextEdit.Wrap
        selectByMouse: true
        activeFocusOnTab: true
        Accessible.name: control.Accessible.name
        Accessible.description: control.Accessible.description

        // Tab moves on instead of typing a tab character.
        Keys.onTabPressed: function (event) {
            nextItemInFocusChain(true).forceActiveFocus(Qt.TabFocusReason)
            event.accepted = true
        }
        Keys.onBacktabPressed: function (event) {
            nextItemInFocusChain(false).forceActiveFocus(Qt.BacktabFocusReason)
            event.accepted = true
        }
    }

    Binding {
        target: area
        property: "color"
        value: KanteStyle.textColor
        when: KanteStyle.themed
        restoreMode: Binding.RestoreBindingOrValue
    }
    Binding {
        target: area
        property: "selectionColor"
        value: KanteStyle.accentColor
        when: KanteStyle.themed
        restoreMode: Binding.RestoreBindingOrValue
    }
    Binding {
        target: area
        property: "selectedTextColor"
        value: KanteStyle.accentForegroundColor
        when: KanteStyle.themed
        restoreMode: Binding.RestoreBindingOrValue
    }
}
