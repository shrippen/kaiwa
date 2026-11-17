import QtQuick
import QtQuick.Layouts
import QtQuick.Dialogs
import QtQuick.Controls as QQC2
import Kante

// Settings (Lastenheft Kap. 7): daily goal (/LF601/), feedback amount (/LF304/), script
// (/LF110/), end of a spoken turn (/LF101/), the microphone and its test (/LF104/), mora hints (/LF320/), gamification (/LF605/),
// pause mode (/LF602/),
// reminder (/LF620/: at most one a day, off by default), style and language; the tutor and
// its key (Q-15, /LF713/); what leaves the device (/LF711/); export and deletion of all data
// (/LF710/).
// Laid out as in Kontra (design A, owner 16.11.2026): a sticky header with a tab per tile
// over the scrolling tiles; each category is a KanteSettingGroup. The big tiles live in
// SettingsSpeech, SettingsTutor and SettingsModels.
Item {
    id: page
    required property var kaiwa
    function t(de, en) { return kaiwa.lang === "en" ? en : de }

    readonly property var p: kaiwa.profile.length > 0 ? JSON.parse(kaiwa.profile) : ({ goal: {} })
    readonly property var privacy: kaiwa.privacy.length > 0 ? JSON.parse(kaiwa.privacy) : []
    readonly property var models: kaiwa.models.length > 0 ? JSON.parse(kaiwa.models) : []
    readonly property var download: kaiwa.download.length > 0 ? JSON.parse(kaiwa.download) : ({})
    readonly property real side: KanteStyle.unit(18)

    // The words for a tile's number of settings ("1 Einstellung", "5 Einstellungen").
    function counted(n) {
        return n === 1 ? t("1 Einstellung", "1 setting") : t(n + " Einstellungen", n + " settings")
    }

    // The tiles in order: the sticky header's tabs.
    readonly property var titles: [
        t("Profil", "Profile"), t("Üben", "Practice"), t("Sprechen", "Speaking"), t("Aussehen", "Look"),
        t("Tutor", "Tutor"), t("Was verlässt das Gerät?", "What leaves the device?"), t("Modelle", "Models"),
        t("Deine Daten", "Your data"), t("Über Kaiwa", "About Kaiwa")
    ]

    Flickable {
        id: flick
        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: col.implicitHeight + KanteStyle.unit(24)
        flickableDirection: Flickable.VerticalFlick
        boundsBehavior: Flickable.StopAtBounds
        // The scrollbar runs below the header.
        QQC2.ScrollBar.vertical: QQC2.ScrollBar { topPadding: header.height }

        ColumnLayout {
            id: col
            x: page.side
            y: KanteStyle.unit(12)
            width: flick.width - 2 * page.side
            spacing: KanteStyle.unit(16)

            // Room for the sticky header above the content.
            Item { Layout.preferredHeight: header.expandedHeight }

            // Several profiles on one device (/LF720/), each fully separate.
            KanteSettingGroup {
                id: profileGroup
                Layout.fillWidth: true
                title: page.titles[0]
                countText: page.counted(count)

                SettingRow {
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
            }

            KanteSettingGroup {
                id: practiceGroup
                Layout.fillWidth: true
                title: page.titles[1]
                countText: page.counted(count)

                SettingRow {
                    title: page.t("Tagesziel", "Daily goal")
                    hint: page.t("Minuten gesprochen pro Tag; Getipptes zählt nicht", "Minutes spoken a day; typing does not count")
                    Choice {
                        readonly property var values: page.p.goalSteps || []
                        model: values.map(function (m) { return m + " min" })
                        shown: Math.max(0, values.indexOf(page.p.goal.value))
                        onActivated: function (i) { page.kaiwa.setSetting("goal", String(values[i])) }
                    }
                }
                SettingRow {
                    title: page.t("Verbesserungen", "Corrections")
                    hint: (page.p.feedbackLevel
                           ? page.t("Nach Einstufung (" + page.p.feedbackLevel + "). ",
                                    "By placement (" + page.p.feedbackLevel + "). ")
                           : "")
                          + page.t("Wenig: nur Fehler. Viel: auch Stil", "Little: mistakes only. Much: style too")
                    Choice {
                        readonly property var values: page.p.feedbackSteps || []
                        readonly property var labels: ({ little: page.t("Wenig", "Little"), normal: page.t("Normal", "Normal"), much: page.t("Viel", "Much") })
                        model: values.map(function (a) { return labels[a] })
                        shown: Math.max(0, values.indexOf(page.p.feedback))
                        onActivated: function (i) { page.kaiwa.setSetting("feedback", values[i]) }
                    }
                }
                SettingRow {
                    title: page.t("Schrift der Figur", "The character's text")
                    hint: page.t("Ohne Text übst du reines Hören", "Without text you practise pure listening")
                    Choice {
                        model: [page.t("Mit Furigana", "With furigana"), page.t("Nur Kana", "Kana only"), page.t("Nur hören", "Listen only")]
                        shown: page.p.script || 0
                        onActivated: function (i) { page.kaiwa.setSetting("script", String(i)) }
                    }
                }
                SettingRow {
                    title: page.t("Punkte und Serie", "Points and streak")
                    hint: page.t("Aus: nur der Lernpfad, keine XP, keine Serie", "Off: only the path, no XP, no streak")
                    QQC2.Switch {
                        checked: page.p.gamification === true
                        text: checked ? page.t("An", "On") : page.t("Aus", "Off")
                        onToggled: page.kaiwa.setSetting("gamification", checked ? "true" : "false")
                        KanteCheckSkin { control: parent; shape: KanteCheckSkin.Shape.Switch }
                    }
                }
                SettingRow {
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
                SettingRow {
                    visible: false
                    title: page.t("Erinnerung", "Reminder")
                    hint: page.t("Höchstens eine am Tag, über das Plasmoid; nur solange das Tagesziel offen ist", "At most one a day, through the plasmoid; only while the daily goal is open")
                    Choice {
                        readonly property var values: page.p.reminderSteps || []
                        model: values.map(function (m) {
                            return m === null ? page.t("Aus", "Off")
                                              : String(Math.floor(m / 60)).padStart(2, "0") + ":" + String(m % 60).padStart(2, "0")
                        })
                        shown: Math.max(0, values.indexOf(page.p.reminder === undefined ? null : page.p.reminder))
                        onActivated: function (i) { page.kaiwa.setSetting("reminder", values[i] === null ? "" : String(values[i])) }
                    }
                }
            }

            SettingsSpeech {
                id: speechGroup
                host: page
                title: page.titles[2]
            }

            KanteSettingGroup {
                id: lookGroup
                Layout.fillWidth: true
                title: page.titles[3]
                countText: page.counted(count)

                SettingRow {
                    title: page.t("Textgröße", "Text size")
                    hint: page.t("Vergrößert die ganze Oberfläche mit den Lesehilfen, zusätzlich zur Systemeinstellung; gilt ab dem nächsten Start",
                                 "Enlarges the whole interface with the readings, on top of the system setting; from the next start")
                    Choice {
                        readonly property var values: page.p.textScaleSteps || []
                        model: values.map(function (s) { return Math.round(s * 100) + " %" })
                        shown: Math.max(0, values.indexOf(page.p.textScale))
                        onActivated: function (i) { page.kaiwa.setSetting("textScale", String(values[i])) }
                    }
                }
                SettingRow {
                    title: page.t("Stil", "Style")
                    Choice {
                        model: ["System", "Kante", "Kante Light"]
                        shown: page.p.style || 0
                        onActivated: function (i) { page.kaiwa.setSetting("style", String(i)) }
                    }
                }
                SettingRow {
                    title: page.t("Sprache", "Language")
                    Choice {
                        model: ["Deutsch", "English"]
                        shown: page.p.uiLang === "en" ? 1 : 0
                        onActivated: function (i) { page.kaiwa.setSetting("uiLang", i === 1 ? "en" : "de") }
                    }
                }
            }

            SettingsTutor {
                id: tutorGroup
                host: page
                title: page.titles[4]
            }

            // What leaves the device (/LF711/).
            KanteSettingGroup {
                id: privacyGroup
                Layout.fillWidth: true
                title: page.titles[5]
                countText: page.privacy.length === 0 ? ""
                    : page.t(page.privacy.length + " Einträge", page.privacy.length + (page.privacy.length === 1 ? " entry" : " entries"))

                Repeater {
                    model: page.privacy
                    delegate: KanteListRow {
                        required property var modelData
                        Layout.fillWidth: true
                        text: modelData.part + " · " + modelData.provider
                        subtitle: page.t("Übertragen: ", "Sent: ") + modelData.sends
                        meta: modelData.where
                    }
                }
            }

            SettingsModels {
                id: modelsGroup
                host: page
                title: page.titles[6]
            }

            // Data (/LF710/, /LF712/).
            KanteSettingGroup {
                id: dataGroup
                Layout.fillWidth: true
                title: page.titles[7]
                countText: page.counted(count)

                SettingRow {
                    title: page.t("Aufnahmen behalten", "Keep recordings")
                    hint: page.t("Zum Vergleichen mit früher. Abschalten löscht sie sofort; die Abschriften bleiben", "To compare with earlier. Turning it off deletes them at once; the transcripts stay")
                    Choice {
                        readonly property var values: page.p.keepSteps || []
                        model: values.map(function (d) { return d === null ? page.t("Nein", "No") : page.t(d + " Tage", d + " days") })
                        shown: Math.max(0, values.indexOf(page.p.keepRecordings === undefined ? null : page.p.keepRecordings))
                        onActivated: function (i) { page.kaiwa.setSetting("keepRecordings", values[i] === null ? "" : String(values[i])) }
                    }
                }
                QQC2.Label {
                    Layout.fillWidth: true
                    Layout.topMargin: KanteStyle.unit(6)
                    text: (page.p.keepRecordings ? page.t("Aufnahmen bleiben " + page.p.keepRecordings + " Tage. Profil: ", "Recordings stay " + page.p.keepRecordings + " days. Profile: ")
                                                 : page.t("Aufnahmen werden nicht gespeichert. Profil: ", "Recordings are not stored. Profile: ")) + (page.p.path || "")
                    wrapMode: Text.Wrap
                    font: KanteStyle.smallFont
                    color: KanteStyle.mutedTextColor
                }
                RowLayout {
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
                }
                QQC2.Label {
                    Layout.fillWidth: true
                    Layout.topMargin: KanteStyle.unit(6)
                    visible: page.kaiwa.notice.length > 0
                    text: page.kaiwa.notice
                    wrapMode: Text.Wrap
                    font: KanteStyle.defaultFont
                    color: KanteStyle.textColor
                }
                // The dangerous one, last and set apart by a rule.
                Rectangle {
                    Layout.fillWidth: true
                    Layout.topMargin: KanteStyle.unit(14)
                    Layout.preferredHeight: 1
                    color: KanteStyle.ruleColor
                }
                KanteButton {
                    Layout.topMargin: KanteStyle.unit(12)
                    Layout.bottomMargin: KanteStyle.unit(10)
                    text: page.t("Profil löschen …", "Delete profile …")
                    emphasis: KanteButton.Emphasis.Destructive
                    onClicked: confirm.open()
                }
            }

            // About (/LD050/): version, licence and the sources of the dictionary, each with its
            // licence and attribution (tanos: Q-01).
            KanteSettingGroup {
                id: aboutGroup
                Layout.fillWidth: true
                title: page.titles[8]

                Text {
                    Layout.fillWidth: true
                    Layout.topMargin: KanteStyle.unit(10)
                    Layout.bottomMargin: KanteStyle.unit(12)
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
            }
        }
    }

    // Tab never leaves the focus below the sticky header.
    KanteFocusFollow {
        flickable: flick
        topInset: header.height
    }

    KanteStickyHeader {
        id: header
        // Main.qml's text check: these tabs scroll sideways past the window's edge.
        objectName: "sectionTabs"
        anchors { left: parent.left; right: parent.right; top: parent.top }
        flickable: flick
        title: page.t("Einstellungen", "Settings")
        tabsName: page.t("Zu einer Gruppe springen", "Jump to a group")
        sideMargin: page.side
        model: page.titles
        targets: [profileGroup, practiceGroup, speechGroup, lookGroup, tutorGroup, privacyGroup,
                  modelsGroup, dataGroup, aboutGroup]
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
            text: page.t("Gesprächsverlauf, Lernstand, Wiederholung, Einstellungen und die API-Schlüssel dieses Profils werden endgültig gelöscht. Ein Export vorher behält eine Kopie (ohne Schlüssel).",
                         "Conversations, progress, review, settings and the API keys of this profile are deleted for good. Export first to keep a copy (without keys).")
            wrapMode: Text.Wrap
            font: KanteStyle.defaultFont
            color: KanteStyle.textColor
        }
        onAccepted: page.kaiwa.deleteProfile()
    }
}
