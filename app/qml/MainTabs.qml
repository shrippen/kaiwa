import QtQuick
import Kante

// The main tab bar. Says every change of tab, so the views reload with the time of now, and
// says apart a change that leaves the review tab: a review opened for one unit (from the path)
// forgets that unit only then. Enter or Space on the tab already shown (Kante #166) is no change.
KanteTabBar {
    id: bar

    enum Switch {
        ToTab,
        FromReview
    }

    // The index of the review tab.
    property int review: -1

    // A change of tab; `kind` is a MainTabs.Switch.
    signal switched(int kind)

    // The tab shown before a change. Set once, not bound: it must still hold the old tab when
    // the change arrives. The start tab (--view) is set before completion and says nothing.
    property int shown: -1
    Component.onCompleted: shown = currentIndex

    onCurrentIndexChanged: {
        if (shown === -1) {
            return
        }
        const left = shown === review && currentIndex !== review
        shown = currentIndex
        switched(left ? MainTabs.FromReview : MainTabs.ToTab)
    }
}
