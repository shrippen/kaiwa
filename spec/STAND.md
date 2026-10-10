# Stand und Übergabe (2026-10-10)

Wo Kaiwa steht, was offen ist und was die Arbeit bisher gelehrt hat. Für die nächste Session zuerst lesen, dann `ROADMAP.md`. Starten und prüfen: `./start.sh` (`demo`, `sim`, `check`).

## Was da ist

| Bereich | Stand | PRs |
|---|---|---|
| Planung | Lastenheft, Roadmap, 7 QA-Personas mit 82 Prüfregeln, ADR-001 bis -008 | #1, #3 |
| Kern (`kaiwa-core`) | Gesprächslogik, Hilfe in drei Stufen, Korrekturen mit Schweregrad, Lernermodell, Schwierigkeitsregler (8 Stellgrößen, Ziel 80 %), FSRS-5, Tagesziel, Serie mit Freeze und Pause, XP gegen Ausnutzen, Curriculum N5 (12 Einheiten), Einstufung per Gespräch, Rōmaji-Eingabe | #6, #14, #16 |
| Profil (`kaiwa-store`) | SQLite, Fortsetzen, Export nach `spec/export.schema.json`, Import, vollständiges Löschen | #15 |
| Backend (`kaiwa-backend`) | Ansichten als JSON, Tutor-Wahl, Kostenbremse, mehrere Profile, Aufnahmen mit Opt-in, Statistik, Erinnerungsregel | #17 |
| KI (`kaiwa-ai`) | Anthropic und OpenAI-kompatibel, aufgezeichnete Antworten (Demo), Token-Zähler, Schlüsselbund, „Was verlässt das Gerät?“ | #6, #17 |
| Sprache (`kaiwa-speech`) | Push-to-Talk, ReazonSpeech (sherpa-onnx), Übergangsstimme Kokoro ohne Pitch-Akzent | #11, #12 |
| App | Heute, Lernpfad, Wiederholen, Profil mit Statistik, Einstellungen, Gespräch mit Bilanz, Widersprechen, „Falsch erkannt“, Einstufung; Deutsch und Englisch; Stile System, Kante, Kante Light | #7, #16, #17 |
| Pakete | Symbol, Desktop-Datei, AppStream, PKGBUILD `kaiwa-git`, Flatpak-Manifest (App) | #17 |
| QA | `kaiwa-sim`: 42 von 82 Prüfregeln automatisch (Textmodus, 30-Tage-Läufe, Profil, Engine, QML-Prüfungen) | laufend |

**Geparkt:** Plasmoid und Dienst `kaiwad` (M4) auf dem Zweig `later/plasmoid`, bis die App nahe 1.0 ist. Dort sind Kurzgespräch, fünf Wiederholungen, Sprechtaste über den Dienst und Erinnerungen gebaut. **Offen:** In der CI lädt `plasmoidviewer` das Plasmoid ohne QML-Fehler, aber sein `Today`-Aufruf kommt bei `kaiwad` nicht an (Lauf 1530, „plasmoid: no call reached kaiwad“). Ob das am Offscreen-Betrieb, an der Wartezeit (20 s) oder am D-Bus-Modul von Plasma liegt, ist beim Wiederaufnehmen als Erstes zu klären. Die Erinnerungs-Einstellung der App ist bis dahin ausgeblendet.

## Entscheidungen aus Fragenpaket 4

| ID | Entscheidung | Wo |
|---|---|---|
| I-01 | Moren-Längen mit eigenem Kana-CTC-Aligner (ONNX); Julius und MFA als automatische Gegenprobe | ADR-005 |
| I-02 – I-04 | Ruhen bis M4 (Plasmoid nahe 1.0) | Roadmap M4 |
| I-05 | Claude prüft die Preise zu jedem Release | `crates/kaiwa-ai/data/prices.json` |
| I-06 | Kein Kostenlimit voreingestellt | wie umgesetzt |
| I-07 | Erinnerung mit freier Uhrzeit | mit M4 |
| I-08 | Kein Schutz für Profile | wie umgesetzt |
| I-09 | `kaiwa-git` ins AUR mit 1.0 | `packaging/README.md` |
| I-10 | Nächster Schwerpunkt: Spike 0.6 nach I-01 | Roadmap M3 |

## Offen

1. **Spike 0.6** (Moren-Längen, ADR-005) nach I-01: CTC-Modell wählen (Lizenz), Aligner in Rust, Gegenprobe gegen Julius und MFA an den synthetischen Fehlern aus `qa/corpus`.
2. **Lokaler Test** (`spec/LOKALER-TEST.md`): KI-Anbindung (Spike 0.5), VOICEVOX (H-01), Messwerte auf dem Framework 13, eigene Aufnahmen. Ergebnisse als PR nach `qa/local/<datum>/` (H-05).
3. **40 Prüfregeln** sind noch nicht automatisch, meist Audio, GUI (AT-SPI, Tastatur) oder Verhalten des echten Tutors. Sie brauchen den gui- und audio-Modus von `kaiwa-sim` oder den lokalen Test.
4. **Stimme:** H-01 (VOICEVOX) entscheidet die endgültige Stimme; bis dahin Kokoro als Übergang.
5. **Secret `ANTHROPIC_API_KEY`** fehlt noch in der Cloud-Umgebung (G-04).

## Lessons learned

- **Vor jedem Commit `cargo fmt --all --check`.** Ein unformatierter Push kostet einen ganzen CI-Lauf. Lokal hilft ein pre-commit-Hook.
- **Ein Runner, seriell.** Jeder Push stellt einen Lauf von etwa 12 Minuten in die Schlange; Läufe lassen sich über die API nicht abbrechen. Änderungen bündeln und selten pushen. Steht der Runner (wie am 2026-10-10 von 07:30 bis 07:55 UTC), wartet alles.
- **Kante bewegt sich.** Der Abgleich `tools/sync-kante.sh` prüft gegen Kantes `main`; zieht Kante nach dem letzten Abgleich nach, wird `main` hier rot. Vor dem Mergen neu abgleichen.
- **Positive Signale prüfen.** „Keine Fehler im Log“ bewies nicht, dass das Plasmoid lief. Die strengere Prüfung (der Aufruf muss beim Dienst ankommen) schlug fehl und zeigte, dass es in der CI nie bis zum Dienst kam.
- **Prüfregeln finden echte Fehler.** Die neuen Prüfungen fanden deutsche Reste in der englischen Oberfläche, XP trotz ausgeschalteter Gamification und Felder ohne Namen. Prüfregeln früh automatisieren, auch statisch.
- **Spezifiziertes Verhalten nicht „verbessern“.** Die Freeze-Regel verbraucht Freezes Tag für Tag; das verlangt `jonas_freeze`. Ein Umbau wurde zurückgenommen, als die Prüfung fiel.
- **Demo-Daten wirken auf alles.** Zusätzliche Verlaufsdaten änderten Serie und „Willkommen zurück“. Nach jeder Änderung an `seed` die Startseite ansehen. Der Seed nutzt nie einen eingestellten Tutor.
- **Platz in der Session.** `target/` wächst auf über 20 GB; bei „No space left“ `target/debug` löschen und neu bauen.
- **Branch-Wechsel mit neuen Dateien:** `git stash -u`, sonst bleiben unversionierte Dateien liegen und stören den Build.
- **`tea pr edit` kann keinen Titel ändern;** dafür die API (`PATCH /repos/…/pulls/{n}`).
- **Fragen gebündelt** im Artifact mit Vorschlag und Datenübergabe funktionierten gut: Antworten kamen schnell und ließen sich direkt einarbeiten.
