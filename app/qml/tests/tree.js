.pragma library

// The first object at or below `from` for which `test` holds: through children, footer,
// header, content and the dialogs and popups of a page (those hang in `data`). The pages'
// items have no names for the outside; a test finds one by what it shows.
function find(from, test, seen) {
    const visited = seen || new Set()
    if (!from || visited.has(from)) {
        return null
    }
    visited.add(from)
    if (test(from)) {
        return from
    }
    const kids = []
    for (let d = 0; from.data && d < from.data.length; d++) {
        kids.push(from.data[d])
    }
    for (let c = 0; from.children && c < from.children.length; c++) {
        kids.push(from.children[c])
    }
    kids.push(from.footer, from.header, from.contentItem)
    for (const kid of kids) {
        const hit = find(kid, test, visited)
        if (hit) {
            return hit
        }
    }
    return null
}
