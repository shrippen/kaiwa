import QtQuick

// Stands in for the Rust object `Conversation` in the QML tests: the page's real code runs
// against it, and every call the page makes lands in `calls` as [name, args...]. A test sets
// the properties the way the object would have them.
QtObject {
    id: stub

    property var calls: []
    function record(name, args) { calls = calls.concat([[name].concat(args)]) }
    function count(name) { return calls.filter(function (c) { return c[0] === name }).length }
    function last(name) {
        var all = calls.filter(function (c) { return c[0] === name })
        return all.length > 0 ? all[all.length - 1].slice(1) : null
    }

    property string title: ""
    property string turns: ""
    property string goals: ""
    property string summary: ""
    property string recorded: ""
    property string voice: ""
    property string voiceNote: ""
    function help() { record("help", []) }
    property string helpText: ""
    property string error: ""
    property string notice: ""
    property string grammar: ""
    property string scenario: ""
    property string stage: ""
    property string character: ""
    property bool placing: false
    property bool placementAsk: false
    property bool finished: false
    property bool busy: false
    property bool idle: true
    property bool demo: false
    property bool voiceOn: true
    property bool louder: false
    property bool clear: false
    property bool handsFree: false
    property bool speechError: false
    property bool speechReady: true
    property int talkState: 0
    property int helpLevel: 0
    property real level: 0
    property real longerPause: 0

    function start(id) { record("start", [id]) }
    function resume() { record("resume", []) }
    function startPlacement() { record("startPlacement", []) }
    function startTalk() { record("startTalk", []); talkState = 1 }
    function stopTalk() { record("stopTalk", []); talkState = 0 }
    function interrupt() { record("interrupt", []) }
    function bargeIn() { record("bargeIn", []) }
    function retryTalk() { record("retryTalk", []) }
    function pollLevel() { }
    function say(text, typed) { record("say", [text, typed]) }
    function setSound(louder, clear) { record("setSound", [louder, clear]) }
    function listen(index, aid) { record("listen", [index, aid]) }
    function sayText(text) { record("sayText", [text]) }
    function sayWord(line, word) { record("sayWord", [line, word]) }
    function sayWav(path) { record("sayWav", [path]) }
    function lookup(line, word) { record("lookup", [line, word]); return lookupAnswer }
    function remember(line, word) { record("remember", [line, word]); return true }
    function dispute(index) { record("dispute", [index]) }
    function markMisrecognized(index) { record("markMisrecognized", [index]) }
    function finishPlacement() { record("finishPlacement", []) }
    function takePlacementOffer() { record("takePlacementOffer", []) }
    function takeLongerPause() { record("takeLongerPause", []) }
    function declineLongerPause() { record("declineLongerPause", []) }
    property string lookupAnswer: "{\"found\":false}"
}
