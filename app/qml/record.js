// What the summary shows of the record of the talk. Without one (recording failed) the page
// says nothing about marks correcting a saved talk and shows no numbers.
.pragma library

// "none" without a record, "kept" when a mark could not correct it, else "correct".
function hint(recorded) {
    if (!recorded) {
        return "none"
    }
    return recorded.markKept === true ? "kept" : "correct"
}

// A real value: absent (undefined) and off (null, no gamification) are not.
function hasXp(recorded) {
    return !!recorded && typeof recorded.xp === "number"
}
