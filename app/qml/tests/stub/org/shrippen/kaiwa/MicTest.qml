import QtQuick

// Stands in for the Rust object `MicTest` in the QML tests: no microphone, no devices.
QtObject {
    property string devices: ""
    property string result: ""
    property string error: ""
    property int phase: 0
    property bool playing: false
    property real level: 0
    property real remaining: 0

    function refreshDevices() { }
    function poll() { }
    function start() { }
    function stop() { }
    function play() { }
}
