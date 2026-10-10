# Pakete (/LL500/)

| Datei | Wofür |
|---|---|
| `org.shrippen.kaiwa.desktop` | Startereintrag (App-ID `org.shrippen.kaiwa`, G-08) |
| `org.shrippen.kaiwa.metainfo.xml` | AppStream: Beschreibung, Bildschirmfotos, Versionen (Discover, Flathub) |
| `arch/PKGBUILD` | `kaiwa-git` für das AUR: App und Symbol; `kaiwad` und Plasmoid kommen mit M4 vor 1.0 dazu |
| `flatpak/org.shrippen.kaiwa.yml` | Flatpak der App; Plasmoid und `kaiwad` (M4) später aus den Distributionspaketen (ADR-008, F-11) |
| `../icons/kaiwa.svg`, `../icons/kaiwa-mark.svg` | Symbol nach den Kante-Regeln: themed (`currentColor`) für Plasma, Mark in Creme für Web und Info-Seiten |

Die CI (Job `qt`) prüft den Startereintrag (`desktop-file-validate`), die Metainfo (`appstreamcli validate`), und die Syntax des PKGBUILD. Ein echter `makepkg`- und `flatpak-builder`-Lauf gehört zum Release (M5); für das Flatpak muss vorher `cargo-sources.json` mit `flatpak-cargo-generator` erzeugt werden.

Pakete für openSUSE, Fedora und Debian folgen nach demselben Muster wie das PKGBUILD. Ins AUR lädt der Besitzer `kaiwa-git` mit 1.0 hoch (I-09), zusammen mit der Übersichtsseite (G-09).
