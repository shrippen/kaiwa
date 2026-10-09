import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Settings (Lastenheft Kap. 7): daily goal (/LF601/), feedback amount (/LF304/), script
// (/LF110/), end of a spoken turn (/LF101/), gamification (/LF605/), pause mode (/LF602/),
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
        Row {
            title: page.t("Erinnerung", "Reminder")
            hint: page.t("Höchstens eine am Tag", "At most one a day")
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

        // Data (/LF710/, /LF712/).
        KanteSectionLabel {
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(18)
            text: page.t("Deine Daten", "Your data")
            rule: true
        }
        QQC2.Label {
            Layout.fillWidth: true
            Layout.leftMargin: KanteStyle.unit(18)
            Layout.rightMargin: KanteStyle.unit(18)
            Layout.topMargin: KanteStyle.unit(6)
            text: page.t("Aufnahmen werden nicht gespeichert. Profil: ", "Recordings are not stored. Profile: ") + (page.p.path || "")
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
