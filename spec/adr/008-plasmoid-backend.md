# ADR-008: Backend für App und Plasmoid

**Status:** Angenommen · **Bezug:** /LF500/–/LF510/, /LL011/, /LL012/, /LL501/

## Kontext

Ein Plasmoid läuft im Prozess von `plasmashell`. Modelle mit mehreren hundert MB bis GB, Audioaufnahme und lange Rechnungen gehören dort nicht hin: Ein Absturz oder Hänger träfe den ganzen Desktop. App und Plasmoid sollen sich außerdem Modelle, Sitzung und Mikrofon teilen (/LF510/).

## Optionen

| Option | Für | Gegen |
|---|---|---|
| A: QML-Plugin mit vollem Kern im Plasmoid | einfach | Modelle und Audio in `plasmashell`, doppelt geladen mit der App |
| **B: Dienst `kaiwad` (D-Bus, Session-Aktivierung), dünner Client** | Isolation, ein Modell-Ladevorgang, gemeinsame Sitzung, Neustart ohne Desktop-Neustart | IPC-Schnittstelle pflegen, auf Windows/macOS/Android nicht nötig |
| C: Plasmoid startet die App im Hintergrund | wenig Code | träge, kein echtes Widget |

## Empfehlung

**B.**

- **`kaiwad`** (Rust, zbus) besitzt Kern, Modelle, Audio und Datenbank. Schnittstelle `org.shrippen.Kaiwa1` mit Sitzungen, Zügen als Signale und Lernerdaten. Er startet per D-Bus-Aktivierung und entlädt Modelle nach Leerlauf (/LL012/).
- **Client:** ein schlankes cxx-qt-QML-Modul `org.shrippen.kaiwa.client` (ohne Modelle), genutzt von Plasmoid und App unter Plasma.
- **Andere Plattformen:** Unter Windows, macOS und Android nutzt die App dieselbe Schnittstelle als in-process-Implementierung (Trait `Backend`). Es gibt dort keinen Dienst.
- **Paketierung:** Das Plasmoid kommt mit dem Distro-Paket und dem AUR-Paket, nicht über den KDE Store (der kann keine Binärmodule).

## Entschieden (Fragenpaket 1)

- **F-11:** Das Plasmoid erscheint nur über Pakete (Distro, AUR) und nicht im KDE Store.
- **G-08:** Die App-ID ist `org.shrippen.kaiwa`. Sie gilt für die Desktop-Datei, das Flatpak und das Plasmoid (`org.shrippen.kaiwa.plasmoid`). Der D-Bus-Dienst heißt `org.shrippen.Kaiwa1`.
