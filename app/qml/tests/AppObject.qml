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
    property var calls: []

    function record(name, args) { calls = calls.concat([[name].concat(args)]) }
    function count(name) { return calls.filter(c => c[0] === name).length }
    function last(name) {
        const all = calls.filter(c => c[0] === name)
        return all.length > 0 ? all[all.length - 1].slice(1) : null
    }
    function downloadModel(dir) { record("downloadModel", [dir]) }
    function stopDownload() { record("stopDownload", []) }
    function setSetting(key, value) { record("setSetting", [key, value]) }
}
