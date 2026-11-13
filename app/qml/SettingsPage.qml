import QtQuick
import QtQuick.Layouts
import QtQuick.Dialogs
import QtQuick.Controls as QQC2
import Kante
import org.shrippen.kaiwa

// Settings (Lastenheft Kap. 7): daily goal (/LF601/), feedback amount (/LF304/), script
// (/LF110/), end of a spoken turn (/LF101/), the microphone and its test (/LF104/), mora hints (/LF320/), gamification (/LF605/),
// pause mode (/LF602/),
// reminder (/LF620/: at most one a day, off by default), style and language; the tutor and
// its key (Q-15, /LF713/); what leaves the device (/LF711/); export and deletion of all data
// (/LF710/).
QQC2.ScrollView {
    id: page
    required property var kaiwa
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var p: kaiwa.profile.length > 0 ? JSON.parse(kaiwa.profile) : ({ goal: {} })
    readonly property var privacy: kaiwa.privacy.length > 0 ? JSON.parse(kaiwa.privacy) : []
    readonly property var models: kaiwa.models.length > 0 ? JSON.parse(kaiwa.models) : []
    readonly property var download: kaiwa.download.length > 0 ? JSON.parse(kaiwa.download) : ({})
    readonly property real titleWidth: KanteStyle.unit(210)
    contentWidth: availableWidth

    ColumnLayout {
        width: page.availableWidth
        spacing: 0

        Item { implicitHeight: KanteStyle.unit(4) }
        KanteHeading {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.bottomMargin: KanteStyle.unit(8)
            text: page.t("Einstellungen", "Settings")
            pageTitle: true
        }

        component Row: KanteSettingRow {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            titleWidth: page.titleWidth
            narrow: applicationWindow().compact
        }

        // Several profiles on one device (/LF720/), each fully separate.
        Row {
            section: page.t("Profil", "Profile")
            title: page.t("Wer übt?", "Who practises?")
            hint: page.t("Jedes Profil hat eigenen Lernstand, Einstellungen und Kosten", "Each profile has its own progress, settings and costs")
            RowLayout {
                spacing: KanteStyle.unit(8)
                KanteComboBox {
                    id: who
                    readonly property var list: page.p.profiles || []
                    model: list.map(function (x) { return x.name })
                    currentIndex: Math.max(0, list.findIndex(function (x) { return x.current }))
                    onActivated: function (i) { if (!list[i].current) page.kaiwa.switchProfile(list[i].file) }
                    Accessible.name: page.t("Profil", "Profile")
                }
                KanteTextField {
                    id: newName
                    Layout.preferredWidth: KanteStyle.unit(200)
                    placeholderText: page.t("Neues Profil: Name", "New profile: name")
                    Accessible.name: page.t("Name des neuen Profils", "Name of the new profile")
                    onAccepted: page.addProfile()
                }
                KanteButton {
                    text: page.t("Anlegen", "Add")
                    enabled: newName.text.trim().length > 0
                    onClicked: page.addProfile()
                }
            }
        }
        Row {
            section: page.t("Üben", "Practice")
            title: page.t("Tagesziel", "Daily goal")
            hint: page.t("Minuten gesprochen pro Tag; Getipptes zählt nicht", "Minutes spoken a day; typing does not count")
            KanteSegmented {
                readonly property var values: [3, 5, 10, 15, 20]
                model: values.map(function (m) { return m + " min" })
                currentIndex: Math.max(0, values.indexOf(page.p.goal.value))
                onActivated: function (i) { page.kaiwa.setSetting("goal", String(values[i])) }
            }
        }
        Row {
            title: page.t("Verbesserungen", "Corrections")
            hint: page.t("Wenig: nur Fehler. Viel: auch Stil", "Little: mistakes only. Much: style too")
            KanteSegmented {
                readonly property var values: ["little", "normal", "much"]
                model: [page.t("Wenig", "Little"), page.t("Normal", "Normal"), page.t("Viel", "Much")]
                currentIndex: Math.max(0, values.indexOf(page.p.feedback))
                onActivated: function (i) { page.kaiwa.setSetting("feedback", values[i]) }
            }
        }
        Row {
            title: page.t("Schrift der Figur", "The character's text")
            hint: page.t("Ohne Text übst du reines Hören", "Without text you practise pure listening")
            KanteSegmented {
                model: [page.t("Mit Furigana", "With furigana"), page.t("Nur Kana", "Kana only"), page.t("Nur hören", "Listen only")]
                currentIndex: page.p.script || 0
                onActivated: function (i) { page.kaiwa.setSetting("script", String(i)) }
            }
        }
        // /LF100/: hold the key, or hands-free with the pause below as the end of a turn.
        Row {
            title: page.t("Sprechen", "Speaking")
            hint: page.t("Freisprechen: Kaiwa hört nach jeder Antwort der Figur von selbst zu, eine Pause beendet deinen Satz",
                         "Hands-free: Kaiwa listens by itself after each reply of the character, a pause ends your sentence")
            KanteSegmented {
                model: [page.t("Taste halten", "Hold the key"), page.t("Freisprechen", "Hands-free")]
                currentIndex: page.p.handsFree === true ? 1 : 0
                onActivated: function (i) { page.kaiwa.setSetting("handsFree", i === 1 ? "true" : "false") }
            }
        }
        Row {
            title: page.t("Ende eines Zugs", "End of a turn")
            hint: page.t("Beim Freisprechen: so lange Stille, bis Kaiwa annimmt, dass du fertig bist", "Hands-free: silence until Kaiwa takes it you are done")
            KanteSegmented {
                readonly property var values: [1.2, 2, 3, 5]
                model: [page.t("1,2 s", "1.2 s"), "2 s", "3 s", "5 s"]
                currentIndex: Math.max(0, values.indexOf(page.p.endPause))
                onActivated: function (i) { page.kaiwa.setSetting("endPause", String(values[i])) }
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
        Row {
            id: micRow
            readonly property var devices: mic.devices.length > 0 ? JSON.parse(mic.devices) : []
            readonly property var chosen: devices.find(function (d) { return d.id === page.p.microphone }) || null
            title: page.t("Mikrofon", "Microphone")
            hint: page.p.microphone && !chosen
                ? page.t("Das gewählte Mikrofon ist gerade nicht da; Kaiwa nimmt den Systemstandard.", "The chosen microphone is not there right now; Kaiwa uses the system default.")
                : page.t("Systemstandard folgt dem, was im System eingestellt ist", "System default follows what is set in the system")
            RowLayout {
                spacing: KanteStyle.unit(8)
                KanteComboBox {
                    Layout.preferredWidth: KanteStyle.unit(300)
                    model: [page.t("Systemstandard", "System default")].concat(micRow.devices.map(function (d) { return d.name }))
                    currentIndex: micRow.chosen ? micRow.devices.indexOf(micRow.chosen) + 1 : 0
                    enabled: mic.phase !== 1
                    onActivated: function (i) { page.kaiwa.setSetting("microphone", i === 0 ? "" : micRow.devices[i - 1].id) }
                    Accessible.name: page.t("Mikrofon", "Microphone")
                }
                KanteButton {
                    text: page.t("Neu suchen", "Search again")
                    emphasis: KanteButton.Emphasis.Quiet
                    enabled: mic.phase !== 1
                    onClicked: mic.refreshDevices()
                }
            }
        }
        Row {
            title: page.t("Mikrofon testen", "Test the microphone")
            hint: page.t("Sag einen Satz, etwa „はじめまして。“ Kaiwa prüft bis zu 6 s Lautstärke und Hintergrund und spielt sie dir vor. Nichts wird gespeichert oder gesendet.",
                         "Say a sentence, e.g. “はじめまして。” Kaiwa checks up to 6 s for volume and background and plays them back. Nothing is kept or sent.")
            RowLayout {
                spacing: KanteStyle.unit(12)
                KanteButton {
                    text: mic.phase === 1 ? page.t("Fertig", "Done") : page.t("Aufnehmen", "Record")
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
                    Accessible.name: page.t("Eingangspegel", "Input level")
                }
                Text {
                    visible: mic.phase === 1
                    text: page.t("noch " + Math.ceil(mic.remaining) + " s", Math.ceil(mic.remaining) + " s left")
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
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            spacing: KanteStyle.unit(8)
            KanteCallout {
                Layout.fillWidth: true
                kind: mic.phase === 4 ? KanteCallout.Kind.Danger
                    : !micResult.r ? KanteCallout.Kind.Info
                    : micResult.r.verdict === "good" ? KanteCallout.Kind.Ok
                    : micResult.r.verdict === "silent" ? KanteCallout.Kind.Danger
                    : KanteCallout.Kind.Warn
                title: mic.phase === 4 ? page.t("Mikrofon", "Microphone") : page.t("Ergebnis", "Result")
                text: mic.phase === 4 ? mic.error
                    : !micResult.r ? ""
                    : micResult.r.text + " "
                      + page.t("Stimme " + micResult.r.speechDb + " dB, Hintergrund " + micResult.r.noiseDb + " dB (dBFS).",
                               "Voice " + micResult.r.speechDb + " dB, background " + micResult.r.noiseDb + " dB (dBFS).")
                KanteButton {
                    visible: mic.phase !== 4 && micResult.r !== null
                    text: mic.playing ? page.t("Spielt …", "Playing …") : page.t("Anhören", "Listen")
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
                    : micResult.r.recognition === "checking" ? page.t("Die Spracherkennung hört zu …", "The recognition is listening …")
                    : micResult.r.recognition === "failed" ? micResult.r.why
                    : micResult.r.recognition === "missing" ? page.t("Was die Spracherkennung versteht, zeigt sich, sobald ReazonSpeech unter „Modelle“ geladen ist.", "What the recognition understands shows once ReazonSpeech is downloaded under “Models”.")
                    : micResult.r.heard.length > 0 ? page.t("Verstanden: ", "Understood: ") + micResult.r.heard
                    : page.t("Die Spracherkennung hat nichts verstanden.", "The recognition understood nothing.")
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
        Row {
            readonly property var mora: page.p.moraModel || ({})
            title: page.t("Aussprache-Hinweise", "Pronunciation hints")
            hint: mora.present === false
                ? page.t("Dafür fehlt das Modell kaiwa-kana-ctc (" + mora.licence + ", " + mora.size + "); es lässt sich unten unter „Modelle“ laden.",
                         "This needs the model kaiwa-kana-ctc (" + mora.licence + ", " + mora.size + "); download it below under “Models”.")
                : page.t("Nach einem gesprochenen Satz ein Hinweis, wenn ein langer Vokal oder ein っ zu kurz klang. Findet etwa sechs von zehn solchen Fehlern und irrt sich bei etwa jedem zwanzigsten richtigen Satz; ein Hinweis zählt nie als Fehler.",
                         "After a spoken sentence, a hint when a long vowel or a っ sounded too short. Finds about six in ten such mistakes and is wrong for about one correct sentence in twenty; a hint never counts as a mistake.")
            QQC2.Switch {
                checked: page.p.moraHints === true
                text: checked ? page.t("An", "On") : page.t("Aus", "Off")
                onToggled: page.kaiwa.setSetting("moraHints", checked ? "true" : "false")
                KanteCheckSkin { control: parent; shape: KanteCheckSkin.Shape.Switch }
            }
        }
        // /LL012/: the speech models leave the memory after this idle time.
        Row {
            title: page.t("Modelle entladen", "Unload models")
            hint: page.t("Nach so langer Pause gibt Kaiwa den Speicher der Sprachmodelle frei; beim nächsten Sprechen lädt es sie wieder (ein paar Sekunden)",
                         "After a pause this long Kaiwa frees the memory of the speech models; the next time you speak it loads them again (a few seconds)")
            KanteSegmented {
                readonly property var values: [5, 10, 30, null]
                model: ["5 min", "10 min", "30 min", page.t("Nie", "Never")]
                currentIndex: Math.max(0, values.indexOf(page.p.unloadAfter === undefined ? 10 : page.p.unloadAfter))
                onActivated: function (i) { page.kaiwa.setSetting("unloadAfter", values[i] === null ? "" : String(values[i])) }
            }
        }
        Row {
            title: page.t("Punkte und Serie", "Points and streak")
            hint: page.t("Aus: nur der Lernpfad, keine XP, keine Serie", "Off: only the path, no XP, no streak")
            QQC2.Switch {
                checked: page.p.gamification === true
                text: checked ? page.t("An", "On") : page.t("Aus", "Off")
                onToggled: page.kaiwa.setSetting("gamification", checked ? "true" : "false")
                KanteCheckSkin { control: parent; shape: KanteCheckSkin.Shape.Switch }
            }
        }
        Row {
            title: page.t("Pause", "Pause")
            // A running pause shows here with its last day and can be ended (T-08).
            hint: page.p.pausedUntil
                ? page.t("Pause bis einschließlich " + page.p.pausedUntil.day + "." + page.p.pausedUntil.month + ".",
                         "Paused up to and including " + page.p.pausedUntil.iso)
                // The streak only where gamification is on (/LF605/).
                : page.p.gamification === true
                ? page.t("Urlaub oder krank: die Serie wartet", "Holiday or ill: the streak waits")
                : page.t("Urlaub oder krank: Kaiwa wartet", "Holiday or ill: Kaiwa waits")
            RowLayout {
                spacing: KanteStyle.unit(8)
                KanteButton { text: page.t("3 Tage", "3 days"); onClicked: page.kaiwa.pause(3) }
                KanteButton { text: page.t("7 Tage", "7 days"); onClicked: page.kaiwa.pause(7) }
                KanteButton {
                    visible: !!page.p.pausedUntil
                    text: page.t("Pause beenden", "End the pause")
                    onClicked: page.kaiwa.stopPause()
                }
            }
        }
        // Reminders come with the plasmoid (M4, before 1.0), which runs all day.
        Row {
            visible: false
            title: page.t("Erinnerung", "Reminder")
            hint: page.t("Höchstens eine am Tag, über das Plasmoid; nur solange das Tagesziel offen ist", "At most one a day, through the plasmoid; only while the daily goal is open")
            KanteSegmented {
                readonly property var values: [null, 8 * 60, 12 * 60 + 30, 19 * 60]
                model: [page.t("Aus", "Off"), "08:00", "12:30", "19:00"]
                currentIndex: Math.max(0, values.indexOf(page.p.reminder === undefined ? null : page.p.reminder))
                onActivated: function (i) { page.kaiwa.setSetting("reminder", values[i] === null ? "" : String(values[i])) }
            }
        }

        Row {
            section: page.t("Aussehen", "Look")
            title: page.t("Textgröße", "Text size")
            hint: page.t("Vergrößert die ganze Oberfläche mit den Lesehilfen, zusätzlich zur Systemeinstellung; gilt ab dem nächsten Start",
                         "Enlarges the whole interface with the readings, on top of the system setting; from the next start")
            KanteSegmented {
                // Up to 160 % (owner, 10.11.2026; kaiwa_store::TEXT_SCALE_MAX).
                readonly property var values: [1.0, 1.25, 1.5, 1.6]
                model: ["100 %", "125 %", "150 %", "160 %"]
                currentIndex: Math.max(0, values.indexOf(page.p.textScale === undefined ? 1.0 : page.p.textScale))
                onActivated: function (i) { page.kaiwa.setSetting("textScale", String(values[i])) }
            }
        }
        Row {
            title: page.t("Stil", "Style")
            KanteSegmented {
                model: ["System", "Kante", "Kante Light"]
                currentIndex: page.p.style || 0
                onActivated: function (i) { page.kaiwa.setSetting("style", String(i)) }
            }
        }
        Row {
            title: page.t("Sprache", "Language")
            KanteSegmented {
                model: ["Deutsch", "English"]
                currentIndex: page.p.uiLang === "en" ? 1 : 0
                onActivated: function (i) { page.kaiwa.setSetting("uiLang", i === 1 ? "en" : "de") }
            }
        }

        // The tutor (Q-15): which language model plays the characters and checks the turns.
        // Without one there are no conversations; the key stays in the system keyring.
        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(18)
            text: page.t("Tutor", "Tutor")
            rule: true
        }
        Text {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            text: page.t("Der Tutor ist das Sprachmodell, das die Figuren spielt und deine Sätze prüft. Ollama und LM Studio laufen auf deinem Rechner, dann verlässt nichts das Gerät. Anthropic, OpenRouter, OpenAI, Google und Mistral laufen in der Cloud und brauchen einen API-Schlüssel; über OpenRouter erreichst du mit einem Schlüssel die Modelle vieler Anbieter. Ein API-Schlüssel ist eine Art Passwort, mit dem Kaiwa den Dienst des Anbieters nutzen darf; du bekommst ihn in deinem Konto beim Anbieter.",
                         "The tutor is the language model that plays the characters and checks your sentences. Ollama and LM Studio run on your computer, then nothing leaves the device. Anthropic, OpenRouter, OpenAI, Google and Mistral run in the cloud and need an API key; OpenRouter reaches the models of many providers with one key. An API key is a kind of password that lets Kaiwa use the provider's service; you get it in your account with the provider.")
            wrapMode: Text.Wrap
            color: KanteStyle.mutedTextColor
            font: KanteStyle.smallFont
        }
        Row {
            id: tutorRow
            readonly property var tutor: page.p.tutor || ({})
            readonly property var presets: tutor.presets || []
            readonly property var preset: presets.find(function (x) { return x.id === (tutor.provider || "") }) || null
            title: page.t("Anbieter", "Provider")
            hint: (tutor.status || "") + (tutor.fromEnvironment ? page.t(" Die Umgebung (KAIWA_PROVIDER) legt ihn gerade fest.", " The environment (KAIWA_PROVIDER) sets it right now.") : "")
            KanteComboBox {
                Layout.preferredWidth: KanteStyle.unit(260)
                model: [page.t("Keiner", "None")].concat(tutorRow.presets.map(function (x) {
                    return x.label + (x.local ? page.t(" · auf diesem Rechner", " · on this computer") : "")
                }))
                currentIndex: tutorRow.preset ? tutorRow.presets.indexOf(tutorRow.preset) + 1 : 0
                onActivated: function (i) { page.kaiwa.setSetting("tutorProvider", i === 0 ? "" : tutorRow.presets[i - 1].id) }
                Accessible.name: page.t("Tutor-Anbieter", "Tutor provider")
            }
        }
        Row {
            visible: !!tutorRow.preset && tutorRow.preset.needsModel
            title: page.t("Modell", "Model")
            hint: tutorRow.preset
                  ? page.t("Der Name, unter dem der Anbieter das Modell kennt, z. B. " + tutorRow.preset.exampleModel + ". Japanisch und Erklärungen auf Deutsch gelingen größeren Modellen besser.",
                           "The name the provider knows the model by, e.g. " + tutorRow.preset.exampleModel + ". Larger models do better at Japanese and at explaining in German.")
                  : ""
            KanteTextField {
                Layout.preferredWidth: KanteStyle.unit(260)
                text: tutorRow.tutor.model || ""
                placeholderText: tutorRow.preset ? tutorRow.preset.exampleModel : ""
                onEditingFinished: if (text !== (tutorRow.tutor.model || "")) page.kaiwa.setSetting("tutorModel", text)
                Accessible.name: page.t("Modell", "Model")
            }
        }
        Row {
            visible: !!tutorRow.preset && tutorRow.preset.needsModel
            title: page.t("Server", "Server")
            hint: tutorRow.preset && tutorRow.preset.needsServer
                  ? page.t("Die Adresse eines OpenAI-kompatiblen Servers. Liegt er außerhalb dieses Rechners, ist er eine Cloud: dann gehen die Gespräche dorthin.",
                           "The address of an OpenAI-compatible server. Outside this computer it is a cloud: the conversations then go there.")
                  : page.t("Leer heißt die Adresse des Anbieters. Nur ändern, wenn er woanders läuft; ein Server außerhalb dieses Rechners ist eine Cloud.",
                           "Empty means the provider's address. Change it only when it runs elsewhere; a server outside this computer is a cloud.")
            KanteTextField {
                Layout.preferredWidth: KanteStyle.unit(300)
                text: tutorRow.tutor.baseUrl || ""
                placeholderText: tutorRow.preset && tutorRow.preset.server ? tutorRow.preset.server : "http://localhost:8080/v1"
                invalid: !!tutorRow.preset && tutorRow.preset.needsServer && text.trim().length === 0
                onEditingFinished: if (text !== (tutorRow.tutor.baseUrl || "")) page.kaiwa.setSetting("tutorBase", text)
                Accessible.name: page.t("Server-Adresse", "Server address")
            }
        }
        // The API key, only in the system keyring (/LF713/).
        Row {
            visible: !!page.p.key
            title: page.t("API-Schlüssel", "API key") + (page.p.key ? " · " + page.p.key.label : "")
            hint: page.p.key ? page.p.key.text : ""
            RowLayout {
                spacing: KanteStyle.unit(8)
                enabled: !!page.p.key && page.p.key.canStore
                KanteTextField {
                    id: apiKey
                    Layout.preferredWidth: KanteStyle.unit(240)
                    echoMode: TextInput.Password
                    placeholderText: page.p.key && page.p.key.stored ? page.t("Neuer Schlüssel …", "New key …") : page.t("Schlüssel einfügen …", "Paste the key …")
                    Accessible.name: page.t("API-Schlüssel", "API key")
                }
                KanteButton {
                    text: page.t("Im Schlüsselbund speichern", "Keep in the keyring")
                    enabled: apiKey.text.length > 0
                    onClicked: { page.kaiwa.storeKey(apiKey.text); apiKey.text = "" }
                }
                KanteButton {
                    visible: !!page.p.key && page.p.key.stored && page.p.key.canStore
                    text: page.t("Entfernen", "Remove")
                    emphasis: KanteButton.Emphasis.Quiet
                    onClicked: page.kaiwa.storeKey("")
                }
            }
        }
        // What leaves the device (/LF711/).
        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(18)
            text: page.t("Was verlässt das Gerät?", "What leaves the device?")
            rule: true
        }
        Repeater {
            model: page.privacy
            delegate: KanteListRow {
                required property var modelData
                Layout.fillWidth: true
                Layout.leftMargin: KanteStyle.unit(18)
                Layout.rightMargin: KanteStyle.unit(18)
                text: modelData.part + " · " + modelData.provider
                subtitle: page.t("Übertragen: ", "Sent: ") + modelData.sends
                meta: modelData.where
            }
        }
        // The speech models (F-08, /LL401/): source, size and licence first; Kaiwa connects
        // only when the learner asks for the download.
        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(18)
            text: page.t("Modelle", "Models")
            rule: true
        }
        Text {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            text: page.t("Modelle sind die Dateien, mit denen Kaiwa auf deinem Rechner Sprache erkennt, spricht, die Aussprache prüft, Wörter nachschlägt und Furigana liest. Kaiwa lädt eines nur, wenn du hier auf „Herunterladen“ tippst. Dabei geht nur die Anfrage an die genannte Quelle hinaus, keine Daten von dir. Jede Datei wird vor der Benutzung geprüft.",
                         "Models are the files Kaiwa uses on your computer to recognise speech, to speak, to check pronunciation, to look words up and to read furigana. Kaiwa downloads one only when you tap “Download” here. Only the request goes to the named source, none of your data. Every file is checked before it is used.")
            wrapMode: Text.Wrap
            color: KanteStyle.mutedTextColor
            font: KanteStyle.smallFont
        }
        Repeater {
            model: page.models
            delegate: Row {
                id: modelRow
                required property var modelData
                readonly property bool running: page.download.dir === modelData.dir
                title: modelData.name
                hint: modelData.purpose + " · " + modelData.licence + " · " + modelData.origin + " · "
                    + page.t("Download " + modelData.downloadMB + " MB, danach " + modelData.diskMB + " MB auf der Platte",
                             "download " + modelData.downloadMB + " MB, then " + modelData.diskMB + " MB on disk")
                RowLayout {
                    spacing: KanteStyle.unit(8)
                    Text {
                        visible: modelRow.modelData.present
                        text: page.t("Vorhanden", "Present")
                        color: KanteStyle.textColor
                        font: KanteStyle.defaultFont
                    }
                    KanteButton {
                        visible: !modelRow.modelData.present && !modelRow.running
                        enabled: page.download.dir === undefined
                        text: page.t("Herunterladen", "Download")
                        Accessible.description: page.t("Lädt " + modelRow.modelData.name + " (" + modelRow.modelData.downloadMB + " MB) von " + modelRow.modelData.origin + ".",
                                                        "Downloads " + modelRow.modelData.name + " (" + modelRow.modelData.downloadMB + " MB) from " + modelRow.modelData.origin + ".")
                        onClicked: page.kaiwa.downloadModel(modelRow.modelData.dir)
                    }
                    KanteProgressBar {
                        visible: modelRow.running
                        Layout.preferredWidth: KanteStyle.unit(160)
                        value: modelRow.running && page.download.total > 0 ? page.download.done / page.download.total : 0
                        Accessible.name: page.t("Download von ", "Download of ") + modelRow.modelData.name
                    }
                    Text {
                        visible: modelRow.running
                        text: modelRow.running ? Math.round(page.download.done / 1e6) + " / " + Math.round(page.download.total / 1e6) + " MB" : ""
                        color: KanteStyle.mutedTextColor
                        font: KanteStyle.smallFont
                    }
                    KanteButton {
                        visible: modelRow.running
                        emphasis: KanteButton.Emphasis.Quiet
                        text: page.t("Abbrechen", "Stop")
                        onClicked: page.kaiwa.stopDownload()
                    }
                }
            }
        }
        // Response time (/LL010/): measured on this device; when slow, a suggestion, no switch.
        Row {
            id: latencyRow
            readonly property var lat: page.p.latency || null
            visible: lat !== null
            title: page.t("Antwortzeit", "Response time")
            hint: lat ? (lat.text + (lat.slow ? " " + lat.suggestion : "")) : ""
            Text {
                text: latencyRow.lat ? (latencyRow.lat.p50 / 1000).toFixed(1).replace(".", page.t(",", ".")) + " s" : ""
                color: latencyRow.lat && latencyRow.lat.slow ? KanteStyle.warningColor : KanteStyle.textColor
                font: KanteStyle.defaultFont
            }
        }
        // The cost brake (/LF714/): only with a cloud tutor.
        Row {
            visible: page.privacy.some(function (f) { return f.cloud })
            title: page.t("Kostenbremse", "Cost limit")
            hint: (page.p.cost ? [page.p.cost.text, page.p.cost.unknown].filter(function (x) { return x.length > 0 }).join(" ") : "")
                + page.t(" Geschätzt aus dem Preis, den der Anbieter meldet, sonst aus den Listenpreisen. Am Limit antwortet der Cloud-Tutor bis Monatsende nicht mehr.",
                         " Estimated from the price the provider reports, else from list prices. At the limit the cloud tutor stops answering until the month ends.")
            KanteSegmented {
                readonly property var values: [null, 200, 500, 1000, 2000]
                model: [page.t("Ohne", "None"), "2 €", "5 €", "10 €", "20 €"]
                currentIndex: Math.max(0, values.indexOf(page.p.costLimit === undefined ? null : page.p.costLimit))
                onActivated: function (i) { page.kaiwa.setSetting("costLimit", values[i] === null ? "" : String(values[i])) }
            }
        }

        // About (/LD050/): version, licence and the sources of the dictionary, each with its
        // licence and attribution (tanos: Q-01).
        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(18)
            text: page.t("Über Kaiwa", "About Kaiwa")
            rule: true
        }
        Text {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            readonly property var about: page.p.about || ({ sources: [] })
            text: page.t("Kaiwa " + (about.version || "") + " · MIT-Lizenz. ", "Kaiwa " + (about.version || "") + " · MIT licence. ")
                + (about.sources.length > 0
                   ? page.t("Wörterbuch: ", "Dictionary: ") + about.sources.map(function (s) { return s.name + " (" + s.by + ", " + s.licence + ")" }).join("; ") + "."
                   : page.t("Das Wörterbuch ist noch nicht geladen; seine Quellen stehen nach dem Download hier.", "The dictionary is not downloaded yet; its sources appear here afterwards."))
                + page.t(" Die JLPT-Stufen sind Schätzungen, keine offiziellen Listen.", " The JLPT levels are estimates, not official lists.")
            wrapMode: Text.Wrap
            color: KanteStyle.mutedTextColor
            font: KanteStyle.smallFont
        }

        // Data (/LF710/, /LF712/).
        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(18)
            text: page.t("Deine Daten", "Your data")
            rule: true
        }
        // Recordings only with the opt-in, for a chosen time (/LF712/).
        Row {
            title: page.t("Aufnahmen behalten", "Keep recordings")
            hint: page.t("Zum Vergleichen mit früher. Abschalten löscht sie sofort; die Abschriften bleiben", "To compare with earlier. Turning it off deletes them at once; the transcripts stay")
            KanteSegmented {
                readonly property var values: [null, 7, 30]
                model: [page.t("Nein", "No"), page.t("7 Tage", "7 days"), page.t("30 Tage", "30 days")]
                currentIndex: Math.max(0, values.indexOf(page.p.keepRecordings === undefined ? null : page.p.keepRecordings))
                onActivated: function (i) { page.kaiwa.setSetting("keepRecordings", values[i] === null ? "" : String(values[i])) }
            }
        }
        QQC2.Label {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(6)
            text: (page.p.keepRecordings ? page.t("Aufnahmen bleiben " + page.p.keepRecordings + " Tage. Profil: ", "Recordings stay " + page.p.keepRecordings + " days. Profile: ")
                                         : page.t("Aufnahmen werden nicht gespeichert. Profil: ", "Recordings are not stored. Profile: ")) + (page.p.path || "")
            wrapMode: Text.Wrap
            font: KanteStyle.smallFont
            color: KanteStyle.mutedTextColor
        }
        RowLayout {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(10)
            spacing: KanteStyle.unit(8)
            KanteButton {
                text: page.t("Alles exportieren (JSON)", "Export everything (JSON)")
                onClicked: page.kaiwa.exportProfile(page.exportPath())
            }
            // Into an empty profile only (T-08); the app says so when this one has data.
            KanteButton {
                text: page.t("Export importieren …", "Import an export …")
                onClicked: importDialog.open()
            }
            KanteButton {
                text: page.t("Profil löschen …", "Delete profile …")
                emphasis: KanteButton.Emphasis.Destructive
                onClicked: confirm.open()
            }
        }
        QQC2.Label {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(6)
            visible: page.kaiwa.notice.length > 0
            text: page.kaiwa.notice
            wrapMode: Text.Wrap
            font: KanteStyle.defaultFont
            color: KanteStyle.textColor
        }
        Item { implicitHeight: KanteStyle.unit(18) }
    }

    function addProfile() {
        page.kaiwa.newProfile(newName.text)
        newName.text = ""
    }

    function exportPath() {
        // Named after the profile and the second: neither another profile's export nor an
        // earlier one of the same day is written over.
        var path = page.p.path || ""
        var dir = path.replace(/[^\/]*$/, "")
        var profile = path.replace(/^.*\//, "").replace(/\.kaiwa$/, "")
        // Local time, as the learner's clock shows it.
        var now = new Date()
        var two = function (n) { return (n < 10 ? "0" : "") + n }
        var stamp = now.getFullYear() + two(now.getMonth() + 1) + two(now.getDate())
            + "-" + two(now.getHours()) + two(now.getMinutes()) + two(now.getSeconds())
        return dir + "kaiwa-export-" + (profile.length > 0 ? profile + "-" : "") + stamp + ".json"
    }

    // The system's file dialog: choosing a file is the platform's, not a Kante element.
    FileDialog {
        id: importDialog
        title: page.t("Export von Kaiwa wählen", "Choose an export of Kaiwa")
        nameFilters: [page.t("Kaiwa-Export (*.json)", "Kaiwa export (*.json)")]
        fileMode: FileDialog.OpenFile
        // A local path, not a URL: names with spaces or umlauts come percent-encoded.
        onAccepted: page.kaiwa.importProfile(decodeURIComponent(selectedFile.toString().replace(/^file:\/\//, "")))
    }

    KanteDialog {
        id: confirm
        anchors.centerIn: parent
        modal: true
        title: page.t("Profil löschen?", "Delete the profile?")
        kanteBarColor: KanteStyle.negativeTextColor
        standardButtons: QQC2.Dialog.Ok | QQC2.Dialog.Cancel
        QQC2.Label {
            width: KanteStyle.unit(360)
            text: page.t("Gesprächsverlauf, Lernstand, Wiederholung und Einstellungen werden endgültig gelöscht. Ein Export vorher behält eine Kopie.",
                         "Conversations, progress, review and settings are deleted for good. Export first to keep a copy.")
            wrapMode: Text.Wrap
            font: KanteStyle.defaultFont
            color: KanteStyle.textColor
        }
        onAccepted: page.kaiwa.deleteProfile()
    }
}
