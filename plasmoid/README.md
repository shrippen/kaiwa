# Kaiwa-Plasmoid

Kaiwa im Panel (Lastenheft /LF500/–/LF507/, ADR-008): kompakt ein Tagesziel-Messer mit der Serie, ausgeklappt ein Kurzgespräch mit zwei Zielen, fünf Wiederholungen und „In der App ansehen“.

- **Kein Kern im Plasmoid.** Alles läuft über den Dienst `kaiwad` (`org.shrippen.Kaiwa1` auf dem Session-Bus, `crates/kaiwad`). Er startet per D-Bus-Aktivierung und beendet sich nach zehn Minuten ohne Aufruf. Modelle, Audio und Datenbank bleiben außerhalb von `plasmashell`.
- **Ein Profil.** Ein Kurzgespräch ist eine gewöhnliche Sitzung im Profil: Die App setzt es fort und umgekehrt (/LF507/).
- **Kante-Regel (/LF504/).** Standard ist das Plasma-Theme; Kante und Kante Light sind Opt-in (Einstellung „Stil“). `contents/ui/Kante` und `contents/ui/KantePlasma` sind unveränderte Kopien aus Kante (`tools/sync-kante.sh`, die CI prüft das).
- **Sprechen:** Die Sprechtaste im Kurzgespräch ruft `StartTalk`/`StopTalk` auf; `kaiwad` hört zu und erkennt (ReazonSpeech), das Plasmoid hält kein Audio (/LF505/). Dafür wird `kaiwad` mit `--features speech` gebaut (das PKGBUILD tut das). Tippen geht immer, auch in Rōmaji.
- **Noch nicht:** ein globales Tastenkürzel für Push-to-Talk (/LF521/) und die Stimme der Figur im Plasmoid.

## Installieren (Entwicklung)

```sh
cargo build --release -p kaiwad --features speech
install -Dm755 target/release/kaiwad ~/.local/bin/kaiwad
sed "s|/usr/bin/kaiwad|$HOME/.local/bin/kaiwad|" crates/kaiwad/dbus/org.shrippen.Kaiwa1.service \
  > ~/.local/share/dbus-1/services/org.shrippen.Kaiwa1.service
kpackagetool6 -t Plasma/Applet -i plasmoid/package     # später: -u zum Aktualisieren
plasmoidviewer -a plasmoid/package                      # zum Ausprobieren ohne Panel
```

Das Plasmoid erscheint nur über Pakete (Distro, AUR), nicht im KDE Store (F-11).

## Prüfen

`dbus-run-session -- tools/kaiwad-smoke.sh` ruft jede Methode des Dienstes mit dem Demo-Profil auf. Mit `PLASMOID=1` lädt es zusätzlich das Plasmoid in `plasmoidviewer` und verlangt ein QML-Log ohne Fehler (CI-Job `qt`).
