import QtQuick
import QtQuick.Layouts
import Kante

// The sync tile (/LF700/, ADR-010): where this device syncs (a folder or a WebDAV folder),
// the passphrase that seals the files, and the sync itself. On the first sync the learner
// says whether this device begins or joins with an empty profile. The secrets stay in the
// keyring (/LF713/). No talk is open while this page shows, so a sync never meets one.
KanteSettingGroup {
    id: group
    // The settings page: its profile, texts and `kaiwa`.
    required property var host
    readonly property var s: host.p.sync || ({ kind: "none" })
    // The kind of place the rows show; a pick shows its rows before anything is saved.
    property string kind: s.kind || "none"
    // Where a secret is kept: the keyring, or, for a profile without one, until Kaiwa quits.
    readonly property string keptText: s.keyring ? host.t("Liegt im Schlüsselbund.", "In the keyring.")
                                                 : host.t("Gilt bis zum Beenden.", "Holds until Kaiwa quits.")
    readonly property string keepText: s.keyring ? host.t("Im Schlüsselbund speichern", "Keep in the keyring")
                                                 : host.t("Übernehmen", "Use")
    Layout.fillWidth: true
    countText: host.counted(count)

    Text {
        Layout.fillWidth: true
        Layout.topMargin: KanteStyle.unit(10)
        Layout.bottomMargin: KanteStyle.unit(2)
        text: host.t("Hält dein Profil auf mehreren Geräten gleich, ohne Kaiwa-Server: Jedes Gerät legt eine verschlüsselte Datei in einen Ordner, den du wählst.",
                     "Keeps your profile the same on several devices, without a Kaiwa server: each device puts a sealed file into a folder you choose.")
        wrapMode: Text.Wrap
        color: KanteStyle.mutedTextColor
        font: KanteStyle.smallFont
    }
    SettingRow {
        title: host.t("Ort", "Place")
        hint: host.t("Ein Ordner auf diesem Rechner, auch einer, den Syncthing oder Nextcloud abgleicht, oder ein WebDAV-Ordner.",
                     "A folder on this computer, also one Syncthing or Nextcloud keeps in step, or a WebDAV folder.")
        Choice {
            readonly property var kinds: ["none", "folder", "webdav"]
            model: [host.t("Keiner", "None"), host.t("Ordner", "Folder"), "WebDAV"]
            shown: Math.max(0, kinds.indexOf(group.kind))
            onActivated: function (i) {
                group.kind = kinds[i]
                if (kinds[i] === "none") {
                    host.kaiwa.setSetting(group.s.kind === "webdav" ? "syncUrl" : "syncFolder", "")
                }
            }
        }
    }
    SettingRow {
        visible: group.kind === "folder"
        title: host.t("Ordner", "Folder")
        KanteTextField {
            width: KanteStyle.unit(300)
            text: group.s.folder || ""
            placeholderText: host.t("/home/du/Nextcloud/Kaiwa", "/home/you/Nextcloud/Kaiwa")
            onEditingFinished: if (text !== (group.s.folder || "")) host.kaiwa.setSetting("syncFolder", text)
            Accessible.name: host.t("Ordner zum Abgleichen", "Folder to sync with")
        }
    }
    SettingRow {
        visible: group.kind === "webdav"
        title: host.t("Adresse", "Address")
        hint: host.t("Die Adresse des Ordners, mit https.", "The folder's address, with https.")
        KanteTextField {
            width: KanteStyle.unit(300)
            text: group.s.url || ""
            placeholderText: "https://cloud.example.org/remote.php/dav/files/du/Kaiwa/"
            onEditingFinished: if (text !== (group.s.url || "")) host.kaiwa.setSetting("syncUrl", text)
            Accessible.name: host.t("WebDAV-Adresse", "WebDAV address")
        }
    }
    SettingRow {
        visible: group.kind === "webdav"
        title: host.t("Benutzer", "User")
        KanteTextField {
            width: KanteStyle.unit(200)
            text: group.s.user || ""
            onEditingFinished: if (text !== (group.s.user || "")) host.kaiwa.setSetting("syncUser", text)
            Accessible.name: host.t("WebDAV-Benutzer", "WebDAV user")
        }
    }
    SettingRow {
        visible: group.kind === "webdav"
        title: host.t("Passwort", "Password")
        hint: group.s.password ? group.keptText : ""
        RowLayout {
            spacing: KanteStyle.unit(8)
            KanteTextField {
                id: password
                Layout.preferredWidth: KanteStyle.unit(200)
                echoMode: TextInput.Password
                Accessible.name: host.t("WebDAV-Passwort", "WebDAV password")
            }
            KanteButton {
                text: group.keepText
                enabled: password.text.trim().length > 0
                onClicked: { host.kaiwa.setSyncSecret("password", password.text); password.text = "" }
            }
        }
    }
    SettingRow {
        visible: group.kind !== "none"
        title: host.t("Passphrase", "Passphrase")
        hint: host.t("Verschlüsselt die Dateien (age). Auf jedem Gerät dieselbe; ohne sie lassen sich die Dateien nicht öffnen, auch nicht von Kaiwa.",
                     "Seals the files (age). The same on every device; without it nobody can open the files, Kaiwa neither.")
              + (group.s.passphrase ? " " + group.keptText : "")
        RowLayout {
            spacing: KanteStyle.unit(8)
            KanteTextField {
                id: passphrase
                Layout.preferredWidth: KanteStyle.unit(200)
                echoMode: TextInput.Password
                Accessible.name: host.t("Passphrase", "Passphrase")
            }
            KanteButton {
                text: group.keepText
                enabled: passphrase.text.trim().length > 0
                onClicked: { host.kaiwa.setSyncSecret("passphrase", passphrase.text); passphrase.text = "" }
            }
        }
    }
    // What the next sync does; before the first one, begin here or join.
    SettingRow {
        visible: group.kind !== "none"
        title: host.t("Abgleichen", "Sync")
        hint: group.s.text || ""
        RowLayout {
            spacing: KanteStyle.unit(8)
            enabled: !host.kaiwa.syncing
            KanteButton {
                visible: !group.s.syncs
                text: host.t("Hier beginnen", "Begin here")
                onClicked: host.kaiwa.syncNow("here")
            }
            KanteButton {
                visible: !group.s.syncs
                text: host.t("Beitreten", "Join")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: host.kaiwa.syncNow("join")
            }
            KanteButton {
                visible: !!group.s.syncs
                text: host.t("Jetzt abgleichen", "Sync now")
                onClicked: host.kaiwa.syncNow("join")
            }
        }
    }
}
