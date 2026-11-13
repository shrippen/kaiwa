// The main tab bar. A review opened for one unit (from the path) stays that review while it is
// shown, also when Enter or Space activates its tab again (Kante #166); leaving the review tab
// forgets the unit, so coming back shows everything due.
.pragma library

// Whether going from tab `shown` to tab `now` leaves the review tab `review`.
function leaves(shown, now, review) {
    return shown === review && now !== review
}
