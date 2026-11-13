// The main tab bar. Kante's bar also activates the tab already shown (Enter, Space); only a
// change of tab opens its page afresh, so a review opened for one unit stays that review.
.pragma library

// Whether picking tab `picked` changes the tab `shown`.
function changes(shown, picked) {
    return picked !== shown
}
