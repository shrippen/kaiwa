import QtQuick
import QtQuick.Layouts
import Kante

// The tutor tile of the settings (Q-15): which language model plays the characters and checks
// the turns. Without one there are no conversations; the key stays in the system keyring.
// Then what the tutor costs and how fast it answers (/LL010/, /LF714/).
KanteSettingGroup {
    id: group
    // The settings page: its profile, texts and `kaiwa`.
    required property var host
    Layout.fillWidth: true
    countText: host.counted(count)

    // The first sentence here, the rest behind the "?".
    RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: KanteStyle.unit(10)
        Layout.bottomMargin: KanteStyle.unit(2)
        spacing: KanteStyle.unit(8)
        Text {
            Layout.fillWidth: true
            text: host.t("Der Tutor ist das Sprachmodell, das die Figuren spielt und deine Sätze prüft.",
                         "The tutor is the language model that plays the characters and checks your sentences.")
            wrapMode: Text.Wrap
            color: KanteStyle.mutedTextColor
            font: KanteStyle.smallFont
        }
        KanteInfoTip {
            term: host.t("Tutor", "Tutor")
            info: host.t("Der Tutor ist das Sprachmodell, das die Figuren spielt und deine Sätze prüft. Ollama und LM Studio laufen auf deinem Rechner, dann verlässt nichts das Gerät. Anthropic, OpenRouter, OpenAI, Google und Mistral laufen in der Cloud und brauchen einen API-Schlüssel; über OpenRouter erreichst du mit einem Schlüssel die Modelle vieler Anbieter. Ein API-Schlüssel ist eine Art Passwort, mit dem Kaiwa den Dienst des Anbieters nutzen darf; du bekommst ihn in deinem Konto beim Anbieter.",
                         "The tutor is the language model that plays the characters and checks your sentences. Ollama and LM Studio run on your computer, then nothing leaves the device. Anthropic, OpenRouter, OpenAI, Google and Mistral run in the cloud and need an API key; OpenRouter reaches the models of many providers with one key. An API key is a kind of password that lets Kaiwa use the provider's service; you get it in your account with the provider.")
            accessibleName: host.t("Hilfe zum Tutor", "Help on the tutor")
        }
    }
    SettingRow {
        id: tutorRow
        readonly property var tutor: host.p.tutor || ({})
        readonly property var presets: tutor.presets || []
        readonly property var preset: presets.find(function (x) { return x.id === (tutor.provider || "") }) || null
        title: host.t("Anbieter", "Provider")
        hint: (tutor.status || "") + (tutor.fromEnvironment ? host.t(" Die Umgebung (KAIWA_PROVIDER) legt ihn gerade fest.", " The environment (KAIWA_PROVIDER) sets it right now.") : "")
        KanteComboBox {
            width: KanteStyle.unit(260)
            model: [host.t("Keiner", "None")].concat(tutorRow.presets.map(function (x) {
                return x.label + (x.local ? host.t(" · auf diesem Rechner", " · on this computer") : "")
            }))
            currentIndex: tutorRow.preset ? tutorRow.presets.indexOf(tutorRow.preset) + 1 : 0
            onActivated: function (i) { host.kaiwa.setSetting("tutorProvider", i === 0 ? "" : tutorRow.presets[i - 1].id) }
            Accessible.name: host.t("Tutor-Anbieter", "Tutor provider")
        }
    }
    SettingRow {
        visible: !!tutorRow.preset && tutorRow.preset.needsModel
        title: host.t("Modell", "Model")
        hint: tutorRow.preset
              ? host.t("Der Name, unter dem der Anbieter das Modell kennt, z. B. " + tutorRow.preset.exampleModel + ". Japanisch und Erklärungen auf Deutsch gelingen größeren Modellen besser.",
                       "The name the provider knows the model by, e.g. " + tutorRow.preset.exampleModel + ". Larger models do better at Japanese and at explaining in German.")
              : ""
        KanteTextField {
            width: KanteStyle.unit(260)
            text: tutorRow.tutor.model || ""
            placeholderText: tutorRow.preset ? tutorRow.preset.exampleModel : ""
            onEditingFinished: if (text !== (tutorRow.tutor.model || "")) host.kaiwa.setSetting("tutorModel", text)
            Accessible.name: host.t("Modell", "Model")
        }
    }
    SettingRow {
        visible: !!tutorRow.preset && tutorRow.preset.needsModel
        title: host.t("Server", "Server")
        hint: tutorRow.preset && tutorRow.preset.needsServer
              ? host.t("Die Adresse eines OpenAI-kompatiblen Servers. Liegt er außerhalb dieses Rechners, ist er eine Cloud: dann gehen die Gespräche dorthin.",
                       "The address of an OpenAI-compatible server. Outside this computer it is a cloud: the conversations then go there.")
              : host.t("Leer heißt die Adresse des Anbieters. Nur ändern, wenn er woanders läuft; ein Server außerhalb dieses Rechners ist eine Cloud.",
                       "Empty means the provider's address. Change it only when it runs elsewhere; a server outside this computer is a cloud.")
        KanteTextField {
            width: KanteStyle.unit(300)
            text: tutorRow.tutor.baseUrl || ""
            placeholderText: tutorRow.preset && tutorRow.preset.server ? tutorRow.preset.server : "http://localhost:8080/v1"
            invalid: !!tutorRow.preset && tutorRow.preset.needsServer && text.trim().length === 0
            onEditingFinished: if (text !== (tutorRow.tutor.baseUrl || "")) host.kaiwa.setSetting("tutorBase", text)
            Accessible.name: host.t("Server-Adresse", "Server address")
        }
    }
    // The API key, only in the system keyring (/LF713/).
    SettingRow {
        visible: !!host.p.key
        title: host.t("API-Schlüssel", "API key") + (host.p.key ? " · " + host.p.key.label : "")
        hint: host.p.key ? host.p.key.text : ""
        RowLayout {
            spacing: KanteStyle.unit(8)
            enabled: !!host.p.key && host.p.key.canStore
            KanteTextField {
                id: apiKey
                Layout.preferredWidth: KanteStyle.unit(240)
                echoMode: TextInput.Password
                placeholderText: host.p.key && host.p.key.stored ? host.t("Neuer Schlüssel …", "New key …") : host.t("Schlüssel einfügen …", "Paste the key …")
                Accessible.name: host.t("API-Schlüssel", "API key")
            }
            KanteButton {
                text: host.t("Im Schlüsselbund speichern", "Keep in the keyring")
                enabled: apiKey.text.trim().length > 0
                onClicked: { host.kaiwa.storeKey(apiKey.text); apiKey.text = "" }
            }
            KanteButton {
                visible: !!host.p.key && host.p.key.stored && host.p.key.canStore
                text: host.t("Entfernen", "Remove")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: host.kaiwa.removeKey()
            }
        }
    }
    // Response time (/LL010/): measured on this device; when slow, a suggestion, no switch.
    SettingRow {
        id: latencyRow
        readonly property var lat: host.p.latency || null
        visible: lat !== null
        title: host.t("Antwortzeit", "Response time")
        hint: lat ? (lat.text + (lat.slow ? " " + lat.suggestion : "")) : ""
        Text {
            text: latencyRow.lat ? (latencyRow.lat.p50 / 1000).toFixed(1).replace(".", host.t(",", ".")) + " s" : ""
            color: latencyRow.lat && latencyRow.lat.slow ? KanteStyle.warningColor : KanteStyle.textColor
            font: KanteStyle.defaultFont
        }
    }
    // The cost brake (/LF714/): only with a cloud tutor.
    SettingRow {
        visible: host.privacy.some(function (f) { return f.cloud })
        title: host.t("Kostenbremse", "Cost limit")
        hint: (host.p.cost ? [host.p.cost.text, host.p.cost.unknown].filter(function (x) { return x.length > 0 }).join(" ") : "")
            + host.t(" Geschätzt aus dem Preis, den der Anbieter meldet, sonst aus den Listenpreisen. Am Limit antwortet der Cloud-Tutor bis Monatsende nicht mehr.",
                     " Estimated from the price the provider reports, else from list prices. At the limit the cloud tutor stops answering until the month ends.")
        Choice {
            readonly property var values: [null, 200, 500, 1000, 2000]
            model: [host.t("Ohne", "None"), "2 €", "5 €", "10 €", "20 €"]
            shown: Math.max(0, values.indexOf(host.p.costLimit === undefined ? null : host.p.costLimit))
            onActivated: function (i) { host.kaiwa.setSetting("costLimit", values[i] === null ? "" : String(values[i])) }
        }
    }
}
