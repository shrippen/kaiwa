import QtQuick
import org.kde.plasma.workspace.dbus as DBus

// The plasmoid's only way to Kaiwa (ADR-008): calls to kaiwad on the session bus. Every
// answer is JSON; `reachable` turns false when the service does not answer. A refusal of the
// service itself (no microphone, no model) goes to `fail` and leaves `reachable` alone.
QtObject {
    id: kaiwad

    property bool reachable: true
    property string lastError: ""

    function call(member, signature, args, done, fail) {
        var message = {
            "service": "org.shrippen.Kaiwa1",
            "path": "/org/shrippen/Kaiwa1",
            "iface": "org.shrippen.Kaiwa1",
            "member": member,
            "arguments": args || []
        }
        if (signature) message.signature = signature
        DBus.SessionBus.asyncCall(message, function (reply) {
            kaiwad.reachable = true
            kaiwad.lastError = ""
            if (done) done(typeof reply.value === "string" && reply.value.length > 0 ? JSON.parse(reply.value) : null)
        }, function (reply) {
            var e = reply && reply.error ? reply.error : null
            var refused = e && e.name === "org.freedesktop.DBus.Error.Failed"
            if (!refused) kaiwad.reachable = false
            kaiwad.lastError = e ? e.message : ""
            if (fail) fail(kaiwad.lastError)
        })
    }
    function str(s) { return new DBus.string(s) }
    function bool(b) { return new DBus.bool(b) }
    function uint(n) { return new DBus.uint32(n) }
}
