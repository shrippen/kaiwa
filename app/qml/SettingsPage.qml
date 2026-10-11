import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Settings (Lastenheft Kap. 7): daily goal (/LF601/), feedback amount (/LF304/), script
// (/LF110/), end of a spoken turn (/LF101/), mora hints (/LF320/), gamification (/LF605/),
// pause mode (/LF602/),
// reminder (/LF620/: at most one a day, off by default), style and language; what leaves the
// device (/LF711/); export and deletion of all data (/LF710/).
QQC2.ScrollView {
    id: page
    required property var kaiwa
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var p: kaiwa.profile.length > 0 ? JSON.parse(kaiwa.profile) : ({ goal: {} })
    readonly property var privacy: kaiwa.privacy.length > 0 ? JSON.parse(kaiwa.privacy) : []
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
        Row {
            title: page.t("Ende eines Zugs", "End of a turn")
            hint: page.t("So lange Stille, bis Kaiwa annimmt, dass du fertig bist", "Silence until Kaiwa takes it you are done")
            KanteSegmented {
                readonly property var values: [1.2, 2, 3, 5]
                model: ["1,2 s", "2 s", "3 s", "5 s"]
                currentIndex: Math.max(0, values.indexOf(page.p.endPause))
                onActivated: function (i) { page.kaiwa.setSetting("endPause", String(values[i])) }
            }
        }
        // /LF320/: a preview, off by default until the hints reach K-02 in the app; without the
        // model it says where to get it (F-08).
        Row {
            readonly property var mora: page.p.moraModel || ({})
            title: page.t("Aussprache-Hinweise", "Pronunciation hints")
            hint: mora.present === false
                ? page.t("Dafür fehlt das Modell kaiwa-kana-ctc (" + mora.licence + ", " + mora.size + "): model.int8.onnx, vocab.json und thresholds.json von " + mora.url + " nach " + mora.dir + " laden.",
                         "This needs the model kaiwa-kana-ctc (" + mora.licence + ", " + mora.size + "): download model.int8.onnx, vocab.json and thresholds.json from " + mora.url + " to " + mora.dir + ".")
                : page.t("Vorschau: nach einem gesprochenen Satz ein Hinweis, wenn ein langer Vokal oder ein っ zu kurz klang. Findet etwa jeden zweiten solchen Fehler und irrt sich bei etwa jedem zwanzigsten richtigen Satz.",
                         "Preview: after a spoken sentence, a hint when a long vowel or a っ sounded too short. Finds about every second such mistake and is wrong for about one correct sentence in twenty.")
            QQC2.Switch {
                checked: page.p.moraHints === true
                text: checked ? page.t("An", "On") : page.t("Aus", "Off")
                onToggled: page.kaiwa.setSetting("moraHints", checked ? "true" : "false")
                KanteCheckSkin { control: parent; shape: KanteCheckSkin.Shape.Switch }
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
            hint: page.t("Urlaub oder krank: die Serie wartet", "Holiday or ill: the streak waits")
            RowLayout {
                spacing: KanteStyle.unit(8)
                KanteButton { text: page.t("3 Tage", "3 days"); onClicked: page.kaiwa.pause(3) }
                KanteButton { text: page.t("7 Tage", "7 days"); onClicked: page.kaiwa.pause(7) }
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
        // The API key, only in the system keyring (/LF713/).
        Row {
            visible: !!page.p.key
            title: page.t("API-Schlüssel", "API key") + (page.p.key ? " · " + page.p.key.provider : "")
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
        // The cost brake (/LF714/): only with a cloud tutor.
        Row {
            visible: page.privacy.some(function (f) { return f.cloud })
            title: page.t("Kostenbremse", "Cost limit")
            hint: (page.p.cost ? [page.p.cost.text, page.p.cost.unknown].filter(function (x) { return x.length > 0 }).join(" ") : "")
                + page.t(" Geschätzt aus den Listenpreisen; am Limit antwortet der Cloud-Tutor bis Monatsende nicht mehr.",
                         " Estimated from list prices; at the limit the cloud tutor stops answering until the month ends.")
            KanteSegmented {
                readonly property var values: [null, 200, 500, 1000, 2000]
                model: [page.t("Ohne", "None"), "2 €", "5 €", "10 €", "20 €"]
                currentIndex: Math.max(0, values.indexOf(page.p.costLimit === undefined ? null : page.p.costLimit))
                onActivated: function (i) { page.kaiwa.setSetting("costLimit", values[i] === null ? "" : String(values[i])) }
            }
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
        var dir = (page.p.path || "").replace(/[^\/]*$/, "")
        return dir + "kaiwa-export-" + new Date().toISOString().slice(0, 10) + ".json"
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
