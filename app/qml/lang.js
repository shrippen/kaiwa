// The one place in the pages that knows the interface languages and what an unknown or empty
// one means: German, as in the backend (`Lang` in kaiwa-core). A page reads the code from
// its view (`kaiwa.lang`) and asks here.
.pragma library

var codes = ["de", "en"]

// The text for `lang`: `pick(kaiwa.lang, "Hallo", "Hello")`.
function pick(lang, de, en) {
    return lang === "en" ? en : de
}

// Where a language stands in a choice that lists `codes` (`kaiwa.setSetting("uiLang", ...)`).
function indexOf(lang) {
    return pick(lang, 0, 1)
}

function code(index) {
    return codes[index] || codes[0]
}
