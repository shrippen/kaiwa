# Stand und Übergabe (2026-10-12)

Wo Kaiwa steht, was offen ist und was die Arbeit bisher gelehrt hat. Für die nächste Session zuerst lesen, dann `ROADMAP.md`. Welche Anforderung wo umgesetzt und geprüft ist, zeigt [`RUECKVERFOLGUNG.md`](RUECKVERFOLGUNG.md) (erzeugt mit `cargo run -p kaiwa-sim -- trace --write`). Starten und prüfen: `./start.sh` (`demo`, `sim`, `check`).

## Was da ist

| Bereich | Stand | PRs |
|---|---|---|
| Planung | Lastenheft 0.2 (mit den Entscheidungen der Fragenpakete), Roadmap, 7 QA-Personas mit 82 Prüfregeln, ADR-001 bis -008 | #1, #3 |
| Kern (`kaiwa-core`) | Gesprächslogik, Hilfe in drei Stufen, Korrekturen mit Schweregrad, Lernermodell, Schwierigkeitsregler (8 Stellgrößen, Ziel 80 %), FSRS-5, Tagesziel, Serie mit Freeze und Pause, XP gegen Ausnutzen, Curriculum N5 (12 Einheiten), Einstufung per Gespräch, Rōmaji-Eingabe | #6, #14, #16 |
| Profil (`kaiwa-store`) | SQLite, Fortsetzen, Export nach `spec/export.schema.json`, Import, vollständiges Löschen | #15 |
| Backend (`kaiwa-backend`) | Ansichten als JSON, Tutor-Wahl, Kostenbremse, mehrere Profile, Aufnahmen mit Opt-in, Statistik, Erinnerungsregel | #17 |
| KI (`kaiwa-ai`) | Anthropic und OpenAI-kompatibel, aufgezeichnete Antworten (Demo), Token-Zähler, Schlüsselbund, „Was verlässt das Gerät?“ | #6, #17 |
| Sprache (`kaiwa-speech`) | Push-to-Talk und Freisprechen (Ende per Pause), ReazonSpeech (sherpa-onnx), Übergangsstimme Kokoro ohne Pitch-Akzent, „Lauter“ und „Klar“, Moren-Prüfung, Modell-Download mit Prüfsumme; Erkennung und Stimme hinter Traits | #11, #12, #26, #27 |
| App | Heute, Lernpfad, Freies Gespräch (4 Themen), Wiederholen, Profil mit Statistik und Wochenrückblick, Einstellungen (Modelle, Textgröße bis 200 %, Modelle entladen), Gespräch mit Bilanz, Widersprechen, „Falsch erkannt“, Hörhilfen (nochmal, langsamer, übersetzen), Moren-Hinweise (Vorschau), Einstufung; Deutsch und Englisch; Stile System, Kante, Kante Light | #7, #16, #17, #26, #27 |
| Pakete | Symbol, Desktop-Datei, AppStream, PKGBUILD `kaiwa-git`, Flatpak-Manifest (App) | #17 |
| QA | `kaiwa-sim`: 48 von 83 Prüfregeln automatisch (Textmodus, 30-Tage-Läufe, Profil, Engine, QML-Prüfungen) | laufend |

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

1. **Moren-Prüfung** (ADR-005, Fragenpakete 5 und 6 entschieden): eigenes Kana-CTC-Modell trainiert (`spikes/s06-moren/finetune.py`, v1 base, v2 large), Test mit echten Stimmen (`real.py`, Common Voice mit hineingeschnittenen Fehlern): nur Kürzungen geprüft erkennt v2 61 % bei 4,8 % Fehlalarmen (v1 62 % bei 6,0 %; v3 mit ReazonSpeech dazu nicht besser, 59 % bei 5,1 %), alle Fehlerarten zusammen unter 25 %. Daten und Modelle liegen unter `target/` (Common Voice 8 GB, `kana-ctc-v1`, `kana-ctc-v2`), nicht im Repository. Rust: `kaiwa-speech::mora` (Varianten, CTC, `Checker` über ONNX Runtime, `G2p::pronounced`). v2 ist auf Hugging Face veröffentlicht ([`shrippen/kaiwa-kana-ctc`](https://huggingface.co/shrippen/kaiwa-kana-ctc), Apache-2.0, in `NOTICE`; Download-Adresse `kaiwa_speech::models::KANA_CTC_URL`, feste Revision). Die Hinweise sind in der App (nach einem gesprochenen Satz, höchstens einer, ohne Schweregrad), als **Vorschau standardmäßig aus**: Auf dem App-Weg erkennen sie 55,8 % bei 4,8 % Fehlalarmen, K-02 verlangt 60 % (ADR-005, „Umsetzung in der App“). Offen: Entscheidung des Eigentümers dazu. Die Modelle (ReazonSpeech, Kokoro, kaiwa-kana-ctc) lassen sich in den Einstellungen unter „Modelle“ laden, nur nach Klick, mit Prüfsumme (N-04).
2. **Pitch-Akzent** (/LF321/): Die Vorstudie an echten Stimmen (`spikes/s07-pitch/`) findet bei 5 % Fehlalarmen nur 25–35 % der falschen Akzente (Ziel 80 %). Damit gilt F-09: 1.0 ohne Pitch-Bewertung. Offen beim Eigentümer: nur Kurven ohne Urteil zeigen? Pitch nach 1.0 mit eigenem Modell?
3. **Lokaler Test** (`spec/LOKALER-TEST.md`): KI-Anbindung (Spike 0.5), VOICEVOX (H-01), Messwerte auf dem Framework 13, eigene Aufnahmen. Ergebnisse als PR nach `qa/local/<datum>/` (H-05).
4. **35 Prüfregeln** sind noch nicht automatisch, meist Audio, GUI (AT-SPI, Tastatur) oder Verhalten des echten Tutors. Sie brauchen den gui- und audio-Modus von `kaiwa-sim` oder den lokalen Test.
5. **Stimme:** H-01 (VOICEVOX) entscheidet die endgültige Stimme; bis dahin Kokoro als Übergang.
6. **Secret `ANTHROPIC_API_KEY`** fehlt noch in der Cloud-Umgebung (G-04).

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
- **Synthetische Stimmen sind keine Wahrheit für Aussprache.** Kokoro spricht げんいん als げにん und ありがと als ありがとう; ein Teil der „Fehlalarme“ war die Stimme. Für Moren-Fehler OpenJTalk nehmen (spricht wie geschrieben), für die Abnahme eigene Aufnahmen.
- **Rohwerte ansehen, bevor ein Verfahren verworfen wird.** Die MFA-Dauern trennten einzelne Paare deutlich (`aː` 69 gegen 139 ms), die erste Auswertung zeigte trotzdem fast nichts. Erst der Blick auf die Einzelwerte zeigte, warum: Die Streuung über Sätze ist größer als der Fehler.
- **Synthetische Erfolge halten auf echten Stimmen nicht.** Dasselbe Verfahren erkannte synthetisch 58–74 %, auf Common Voice mit allen Fehlerarten unter 25 %. Früh an echten Aufnahmen messen; wo es keine Lerner-Aufnahmen gibt, Fehler in echte Muttersprachler hineinschneiden und mit einem Schein-Schnitt kontrollieren.
- **Wartende Shell-Schleifen mit `pgrep -f`** finden ihre eigene Befehlszeile, wenn das Muster darin steht, und warten ewig; `pkill -f` beendet so die eigene Shell.
- **Fragen gebündelt** im Artifact mit Vorschlag und Datenübergabe funktionierten gut: Antworten kamen schnell und ließen sich direkt einarbeiten.
- **Spike-Zahlen auf dem App-Weg nachmessen.** Die Moren-Prüfung hatte im Spike 4,8 % Fehlalarme mit OpenJTalks Lesung; mit der Lesung der App waren es 6,9 %. Erst `examples/mora_hints.rs` (derselbe Test, aber App-G2P, ein Hinweis je Zug, Resampling) zeigte das.
- **Was ein Regler steuern soll, auch dort ankommen lassen.** Das Sprechtempo war ein Knopf der Schwierigkeit, die Stimme sprach trotzdem fest mit 0,9; die Einstellung „Ende eines Zugs“ wurde gespeichert, aber nirgends benutzt. Bei jeder Einstellung prüfen, wer sie liest.
- **`io::copy` wiederholt `ErrorKind::Interrupted` endlos.** Ein Abbruch-Signal als `Interrupted` ließ den Download-Test mit 100 % CPU hängen; `io::Error::other` nehmen.
- **Beim Umschreiben von Dateien per Skript nie im selben Ausdruck lesen und zum Schreiben öffnen** (`open(p, "w").write(open(p).read())` leert die Datei). Ungesicherte Änderungen wären verloren gewesen; vorher committen hilft.
