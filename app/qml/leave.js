// When Back asks before it leaves the conversation page (owner, 9.11.2026): during a talk the
// learner spoke in, and during a placement up to its result. At the reading questions the
// stages are played but nothing is stored yet; leaving there loses the placement too.
.pragma library

function asks(started, finished, placementOver, placementAsk) {
    if (placementAsk) {
        return true
    }
    return started && !finished && !placementOver
}
