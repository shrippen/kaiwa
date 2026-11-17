import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante
import org.shrippen.kaiwa

// The "Speaking" tile of the settings: how a turn is taken, the end of a turn, the microphone
// and its test, mora hints, unloading the speech models (/LF100/, /LF101/, /LF104/, /LF320/,
// /LL012/).
KanteSettingGroup {
    id: group
    // The settings page: its profile, texts and `kaiwa`.
    required property var host
    Layout.fillWidth: true
    countText: host.counted(count)

    // /LF100/: hold the key, or hands-free with the pause below as the end of a turn.
    SettingRow {
        title: host.t("Sprechen", "Speaking")
        KanteInfoTip {
            term: host.t("Sprechen", "Speaking")
            info: host.t("Freisprechen: Kaiwa hört nach jeder Antwort der Figur von selbst zu, eine Pause beendet deinen Satz",
                         "Hands-free: Kaiwa listens by itself after each reply of the character, a pause ends your sentence")
            accessibleName: host.t("Hilfe zu Sprechen", "Help on speaking")
        }
    }
    GridLayout {
        Layout.fillWidth: true
        Layout.bottomMargin: KanteStyle.unit(8)
        columns: applicationWindow().compact ? 1 : 2
        columnSpacing: KanteStyle.unit(12)
        rowSpacing: KanteStyle.unit(12)
        KanteOptionTile {
            Layout.fillWidth: true
            text: host.t("Taste halten", "Hold the key")
            hint: host.t("Du hältst die Taste, solange du sprichst", "You hold the key while you speak")
            checked: host.p.handsFree !== true
            onClicked: host.kaiwa.setSetting("handsFree", "false")
        }
        KanteOptionTile {
            Layout.fillWidth: true
            text: host.t("Freisprechen", "Hands-free")
            hint: host.t("Kaiwa hört nach jeder Antwort von selbst zu; eine Pause beendet deinen Satz",
                         "Kaiwa listens by itself after each reply; a pause ends your sentence")
            checked: host.p.handsFree === true
            onClicked: host.kaiwa.setSetting("handsFree", "true")
        }
    }
    SettingRow {
        title: host.t("Ende eines Zugs", "End of a turn")
        hint: host.t("Beim Freisprechen: so lange Stille, bis Kaiwa annimmt, dass du fertig bist", "Hands-free: silence until Kaiwa takes it you are done")
        Choice {
            readonly property var values: host.p.pauseSteps || []
            model: values.map(function (s) { return String(Math.round(s * 100) / 100).replace(".", host.t(",", ".")) + " s" })
            shown: Math.max(0, values.indexOf(host.p.endPause))
            onActivated: function (i) { host.kaiwa.setSetting("endPause", String(values[i])) }
        }
    }
    // The microphone: which one, and a test recording with level, verdict, playback and
    // what the recognition understood. Nothing is kept or sent.
    MicTest {
        id: mic
        Component.onCompleted: refreshDevices()
    }
    Timer {
        interval: 50
        repeat: true
        running: mic.phase === 1 || mic.playing
        onTriggered: mic.poll()
    }
    SettingRow {
        id: micRow
        readonly property var devices: mic.devices.length > 0 ? JSON.parse(mic.devices) : []
        readonly property var chosen: devices.find(function (d) { return d.id === host.p.microphone }) || null
        title: host.t("Mikrofon", "Microphone")
        hint: host.p.microphone && !chosen
            ? host.t("Das gewählte Mikrofon ist gerade nicht da; Kaiwa nimmt den Systemstandard.", "The chosen microphone is not there right now; Kaiwa uses the system default.")
            : host.t("Systemstandard folgt dem, was im System eingestellt ist", "System default follows what is set in the system")
        RowLayout {
            spacing: KanteStyle.unit(8)
            KanteComboBox {
                Layout.preferredWidth: KanteStyle.unit(300)
                model: [host.t("Systemstandard", "System default")].concat(micRow.devices.map(function (d) { return d.name }))
                currentIndex: micRow.chosen ? micRow.devices.indexOf(micRow.chosen) + 1 : 0
                enabled: mic.phase !== 1
                onActivated: function (i) { host.kaiwa.setSetting("microphone", i === 0 ? "" : micRow.devices[i - 1].id) }
                Accessible.name: host.t("Mikrofon", "Microphone")
            }
            KanteButton {
                text: host.t("Neu suchen", "Search again")
                emphasis: KanteButton.Emphasis.Quiet
                enabled: mic.phase !== 1
                onClicked: mic.refreshDevices()
            }
        }
    }
    SettingRow {
        title: host.t("Mikrofon testen", "Test the microphone")
        hint: host.t("Sag einen Satz, etwa „はじめまして。“ Kaiwa prüft bis zu 6 s Lautstärke und Hintergrund und spielt sie dir vor. Nichts wird gespeichert oder gesendet.",
                     "Say a sentence, e.g. “はじめまして。” Kaiwa checks up to 6 s for volume and background and plays them back. Nothing is kept or sent.")
        RowLayout {
            spacing: KanteStyle.unit(12)
            KanteButton {
                text: mic.phase === 1 ? host.t("Fertig", "Done") : host.t("Aufnehmen", "Record")
                emphasis: mic.phase === 1 ? KanteButton.Emphasis.Primary : KanteButton.Emphasis.Normal
                enabled: mic.phase !== 2
                onClicked: mic.phase === 1 ? mic.stop() : mic.start()
            }
            KanteAudioMeter {
                Layout.preferredWidth: KanteStyle.unit(220)
                level: mic.phase === 1 ? mic.level * 60 - 60 : undefined
                floor: -60
                warnAt: -12
                dangerAt: -3
                showScale: true
                scaleUnit: "dB"
                Accessible.name: host.t("Eingangspegel", "Input level")
            }
            Text {
                visible: mic.phase === 1
                text: host.t("noch " + Math.ceil(mic.remaining) + " s", Math.ceil(mic.remaining) + " s left")
                color: KanteStyle.mutedTextColor
                font: KanteStyle.smallFont
            }
        }
    }
    ColumnLayout {
        id: micResult
        readonly property var r: mic.result.length > 0 ? JSON.parse(mic.result) : null
        visible: mic.phase === 4 || (r !== null && mic.phase !== 1)
        Layout.fillWidth: true
        Layout.topMargin: KanteStyle.unit(4)
        Layout.bottomMargin: KanteStyle.unit(8)
        spacing: KanteStyle.unit(8)
        KanteCallout {
            Layout.fillWidth: true
            kind: mic.phase === 4 ? KanteCallout.Kind.Danger
                : !micResult.r ? KanteCallout.Kind.Info
                : micResult.r.verdict === "good" ? KanteCallout.Kind.Ok
                : micResult.r.verdict === "silent" ? KanteCallout.Kind.Danger
                : KanteCallout.Kind.Warn
            title: mic.phase === 4 ? host.t("Mikrofon", "Microphone") : host.t("Ergebnis", "Result")
            text: mic.phase === 4 ? mic.error
                : !micResult.r ? ""
                : micResult.r.text + " "
                  + host.t("Stimme " + micResult.r.speechDb + " dB, Hintergrund " + micResult.r.noiseDb + " dB (dBFS).",
                           "Voice " + micResult.r.speechDb + " dB, background " + micResult.r.noiseDb + " dB (dBFS).")
            KanteButton {
                visible: mic.phase !== 4 && micResult.r !== null
                text: mic.playing ? host.t("Spielt …", "Playing …") : host.t("Anhören", "Listen")
                enabled: !mic.playing
                onClicked: mic.play()
            }
        }
        Text {
            visible: micResult.r !== null && mic.phase !== 4 && micResult.r.recognition !== "skipped"
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: KanteStyle.textColor
            font: KanteStyle.defaultFont
            text: !micResult.r ? ""
                : micResult.r.recognition === "checking" ? host.t("Die Spracherkennung hört zu …", "The recognition is listening …")
                : micResult.r.recognition === "failed" ? micResult.r.why
                : micResult.r.recognition === "missing" ? host.t("Was die Spracherkennung versteht, zeigt sich, sobald ReazonSpeech unter „Modelle“ geladen ist.", "What the recognition understands shows once ReazonSpeech is downloaded under “Models”.")
                : micResult.r.heard.length > 0 ? host.t("Verstanden: ", "Understood: ") + micResult.r.heard
                : host.t("Die Spracherkennung hat nichts verstanden.", "The recognition understood nothing.")
        }
        // A speaker problem while listening back; the result above stays.
        Text {
            visible: mic.phase !== 4 && mic.error.length > 0
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: KanteStyle.negativeTextColor
            font: KanteStyle.defaultFont
            text: mic.error
        }
    }
    // /LF320/: on by default (L-01); without the model it says where to get it (F-08).
    SettingRow {
        readonly property var mora: host.p.moraModel || ({})
        title: host.t("Aussprache-Hinweise", "Pronunciation hints")
        hint: mora.present === false
            ? host.t("Dafür fehlt das Modell kaiwa-kana-ctc (" + mora.licence + ", " + mora.size + "); es lässt sich unten unter „Modelle“ laden.",
                     "This needs the model kaiwa-kana-ctc (" + mora.licence + ", " + mora.size + "); download it below under “Models”.")
            : host.t("Nach einem gesprochenen Satz ein Hinweis, wenn ein langer Vokal oder ein っ zu kurz klang. Findet etwa sechs von zehn solchen Fehlern und irrt sich bei etwa jedem zwanzigsten richtigen Satz; ein Hinweis zählt nie als Fehler.",
                     "After a spoken sentence, a hint when a long vowel or a っ sounded too short. Finds about six in ten such mistakes and is wrong for about one correct sentence in twenty; a hint never counts as a mistake.")
        QQC2.Switch {
            checked: host.p.moraHints === true
            text: checked ? host.t("An", "On") : host.t("Aus", "Off")
            onToggled: host.kaiwa.setSetting("moraHints", checked ? "true" : "false")
            KanteCheckSkin { control: parent; shape: KanteCheckSkin.Shape.Switch }
        }
    }
    // /LL012/: the speech models leave the memory after this idle time.
    SettingRow {
        title: host.t("Modelle entladen", "Unload models")
        hint: host.t("Nach so langer Pause gibt Kaiwa den Speicher der Sprachmodelle frei; beim nächsten Sprechen lädt es sie wieder (ein paar Sekunden)",
                     "After a pause this long Kaiwa frees the memory of the speech models; the next time you speak it loads them again (a few seconds)")
        Choice {
            readonly property var values: host.p.unloadSteps || []
            model: values.map(function (m) { return m === null ? host.t("Nie", "Never") : m + " min" })
            shown: Math.max(0, values.indexOf(host.p.unloadAfter === undefined ? null : host.p.unloadAfter))
            onActivated: function (i) { host.kaiwa.setSetting("unloadAfter", values[i] === null ? "" : String(values[i])) }
        }
    }
}
