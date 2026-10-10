# Pakete (/LL500/)

| Datei | Wofür |
|---|---|
| `org.shrippen.kaiwa.desktop` | Startereintrag (App-ID `org.shrippen.kaiwa`, G-08) |
| `org.shrippen.kaiwa.metainfo.xml` | AppStream: Beschreibung, Bildschirmfotos, Versionen (Discover, Flathub) |
| `arch/PKGBUILD` | `kaiwa-git` für das AUR: App, `kaiwad` mit D-Bus-Aktivierung, Plasmoid, Symbol |
| `flatpak/org.shrippen.kaiwa.yml` | Flatpak der App; Plasmoid und `kaiwad` kommen aus den Distributionspaketen (ADR-008, F-11) |
| `../icons/kaiwa.svg`, `../icons/kaiwa-mark.svg` | Symbol nach den Kante-Regeln: themed (`currentColor`) für Plasma, Mark in Creme für Web und Info-Seiten |

Die CI (Job `qt`) prüft den Startereintrag (`desktop-file-validate`), die Metainfo (`appstreamcli validate`), die Syntax des PKGBUILD und die Plasmoid-Metadaten. Ein echter `makepkg`- und `flatpak-builder`-Lauf gehört zum Release (M5); für das Flatpak muss vorher `cargo-sources.json` mit `flatpak-cargo-generator` erzeugt werden.

Pakete für openSUSE, Fedora und Debian folgen mit M5 nach demselben Muster wie das PKGBUILD.
