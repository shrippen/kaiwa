import QtQuick

// Stands in for the app object (`kaiwa`) that the settings page gets: the strings it shows
// as JSON, and every call the page makes in `calls` as [name, args...].
QtObject {
    property string lang: "de"
    property string profile: JSON.stringify({ goal: {} })
    property string privacy: ""
    property string models: ""
    property string download: ""
    property string notice: ""
    property string path: ""
    property string learner: ""
    property string stats: ""
    property var calls: []

    function record(name, args) { calls = calls.concat([[name].concat(args)]) }
    function count(name) { return calls.filter(c => c[0] === name).length }
    function last(name) {
        const all = calls.filter(c => c[0] === name)
        return all.length > 0 ? all[all.length - 1].slice(1) : null
    }
    function downloadModel(dir) { record("downloadModel", [dir]) }
    function stopDownload() { record("stopDownload", []) }
    function openReview(unit) { record("openReview", [unit]) }
    function skipUnit(unit) { record("skipUnit", [unit]) }
    function nudge(skill, up) { record("nudge", [skill, up]) }
    function setSetting(key, value) { record("setSetting", [key, value]) }
}
