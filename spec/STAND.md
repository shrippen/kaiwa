# Stand und Übergabe (2026-10-14)

Wo Kaiwa steht, was offen ist und was die Arbeit bisher gelehrt hat. Für die nächste Session zuerst lesen, dann `ROADMAP.md`. Welche Anforderung wo umgesetzt und geprüft ist, zeigt [`RUECKVERFOLGUNG.md`](RUECKVERFOLGUNG.md) (erzeugt mit `cargo run -p kaiwa-sim -- trace --write`). Starten und prüfen: `./start.sh` (`demo`, `sim`, `check`).

## Was da ist

| Bereich | Stand | PRs |
|---|---|---|
| Planung | Lastenheft 0.2 (mit den Entscheidungen der Fragenpakete), Roadmap, 7 QA-Personas mit 82 Prüfregeln, ADR-001 bis -008 | #1, #3 |
| Kern (`kaiwa-core`) | Gesprächslogik, Hilfe in drei Stufen, Korrekturen mit Schweregrad, Lernermodell, Schwierigkeitsregler (8 Stellgrößen, Ziel 80 %), FSRS-5, Tagesziel, Serie mit Freeze und Pause, XP gegen Ausnutzen, Curriculum N5 (16 Einheiten) und N4 (14 Einheiten, 22 Szenarien), Format v2 nach ADR-009 mit Grammatik-Katalog (119 Punkte), Einstufung per Gespräch, Rōmaji-Eingabe | #6, #14, #16 |
| Profil (`kaiwa-store`) | SQLite, Fortsetzen, Export nach `spec/export.schema.json`, Import, vollständiges Löschen | #15 |
| Backend (`kaiwa-backend`) | Ansichten als JSON, Tutor-Wahl, Kostenbremse, mehrere Profile, Aufnahmen mit Opt-in, Statistik, Erinnerungsregel | #17 |
| KI (`kaiwa-ai`) | Anthropic und OpenAI-kompatibel, aufgezeichnete Antworten (Demo), Token-Zähler, Schlüsselbund, „Was verlässt das Gerät?“ | #6, #17 |
| Sprachdaten (`kaiwa-lang`) | Wörter einer Zeile (lindera/IPADIC: Verb mit Endungen, Zahl mit Zählwort, お mit Nomen), Nachschlagen im Datenpaket [`kaiwa-data`](https://github.com/shrippen/kaiwa-data) (JMdict en/de, KANJIDIC2, Kanjium-Akzente; Release 0.1.0, Repo auf Gitea, Spiegel auf GitHub), Soll-Akzent; Auszug `data/dict/` für Demo und Tests; Inhalts-Check „jedes Wort steht im Wörterbuch“ | #31 |
| Sprache (`kaiwa-speech`) | Push-to-Talk und Freisprechen (Ende per Pause), ReazonSpeech (sherpa-onnx), Übergangsstimme Kokoro ohne Pitch-Akzent, „Lauter“ und „Klar“, Moren-Prüfung, Modell-Download mit Prüfsumme; Erkennung und Stimme hinter Traits | #11, #12, #26, #27 |
| App | Heute, Lernpfad, Freies Gespräch (4 Themen), Wiederholen, Profil mit Statistik und Wochenrückblick, Einstellungen (Modelle, Textgröße bis 200 %, Modelle entladen), Gespräch mit Bilanz, Widersprechen, „Falsch erkannt“, Hörhilfen (nochmal, langsamer, übersetzen), Moren-Hinweise (standardmäßig an), Einstufung, Wort nachschlagen (Klick auf ein Wort der Figur: Lesung, Wortart, Bedeutung, Soll-Akzent, Anhören, in die Wiederholung; Wörterbuch-Download nur nach Klick), bessere Form einer Korrektur anhören; Deutsch und Englisch; Stile System, Kante, Kante Light | #7, #16, #17, #26, #27, #31 |
| Pakete | Symbol, Desktop-Datei, AppStream, PKGBUILD `kaiwa-git`, Flatpak-Manifest (App) | #17 |
| QA | `kaiwa-sim`: 53 von 83 Prüfregeln automatisch (Textmodus, 30-Tage-Läufe, Profil, Engine, QML-Prüfungen); Inhaltsprüfungen /LF221/ als Tests: Grammatik-Lint, Niveau der Hilfesätze, Register, Kernwörter, Review-Hash; `kaiwa-sim review-sheet` für das Muttersprachler-Review | laufend |

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

## Entscheidungen aus dem Tagplan (2026-10-13)

| ID | Entscheidung | Wo |
|---|---|---|
| M-01 | Bedeutungen in der UI-Sprache, wo JMdict sie hat, sonst Englisch mit Hinweis | `kaiwa-lang::dict`, Wort-Popup |
| M-02 | GitHub-Repo `kaiwa-data` (öffentlich), Push-Mirror von Gitea, Tag `v0.1.0` | ADR-007 |
| M-03 | Wörterbuch nur nach Klick laden, angeboten im Wort-Popup und unter Einstellungen › Modelle | `kaiwa_speech::download::MODELS` |
| M-04 | Soll-Akzent im Popup ohne Urteil (Kanjium) | `kaiwa-lang::accent`, `KantePitchCurve` |

## Nacht zum 2026-10-14

[Nachtbericht mit Fragenpaket 9](https://claude.ai/artifact/3t85sdQ6hZa4rGRsgvWHra). Umgesetzt: Curriculum-Format v2 (#33), N5 mit 16 Einheiten (#34), Tutor in den Einstellungen und Demo nur im Demo-Build (#35), N4 mit 14 Einheiten und Inhaltsprüfungen (#36), kaiwa-data 0.2.0 (Namen aus JMnedict, JLPT-Stufen von tanos) mit Namen im Wort-Popup und „Über Kaiwa“ (#37).

## Tag 2026-10-14

[Tagesbericht](https://claude.ai/artifact/WygHfHhWkdQg18T8X9mZDi). Gemergt: #38 (R-02, R-03), #39 (Inhaltsprüfungen /LF221/, Grammatik im Gespräch, Review-Vorlagen, `hinnerk_plain_words`, PKGBUILD mit `!lto` und `NOTICE`), #40 (ein Zustand je Grammatikpunkt im Lernermodell, /LF201/; Profil), #41 (der Tutor kennt die Grammatik der Einheit), #42 (LOKALER-TEST), #43 (Muster für die einfache Form), #44 (`kaiwa-sim days --seeds`). Kontra #67 (Cargo-Cache im app-Job, R-04).

## Entscheidungen aus Fragenpaket 10 (2026-11-09)

[Fragenpaket 10](https://claude.ai/artifact/J9bfCbFdaHanLQEckNEtsC), Antworten des Eigentümers am 2026-11-09.

| ID | Entscheidung | Stand |
|---|---|---|
| S-01 | `mara_success_band`: gleitend über 8 Sitzungen, 12 Seeds, mindestens 10 im Band; der Regler bleibt bei einem Schritt (ersetzt R-01) | `kaiwa-sim` (`days::check_seeds`, `BAND_*`), `qa/personas/mara.json`; gemessen 12 von 12 |
| S-02 | チーズ, テニス, うち als N5-Kernwörter (n5-05, n5-09, n5-11), raus aus `data/dict/level-exceptions.txt` | `data/curriculum/n5.json`; Band danach weiter 12 von 12 |
| S-03 | Nächster Schwerpunkt: B, den lokalen Test vorbereiten (echter Tutor, Spracherkennung, Stimme; die nicht automatischen Prüfregeln), und C, Wiederholung als Mini-Gespräch (/LF230/, `emi_srs_keigo`). Das erste Paket (A) wartet | offen |

## Entscheidungen aus Fragenpaket 9 (2026-10-14)

Allgemein (gilt für alle Projekte, `../CLAUDE.md`): Vor dem ersten Release sind Breaking Changes in Ordnung, migrieren statt um alte Ids herum bauen.

| ID | Entscheidung | Stand |
|---|---|---|
| R-01 | `mara_success_band` über 12 Seeds prüfen (≥ 10 im Band), Regler zwei Schritte über 0,95 | **Ersetzt durch S-01.** gemessen hält das Band mit Fenster 5 nur 7 von 12 Seeds; zwei Schritte über 0,95 verschlechtern auf 4 von 12 (überschießt), ein Mittel über zwei Sitzungen bringt 8 von 12. Mit Fenster 8 halten alle Varianten das Band: Bei 3–4 Zielen je Sitzung schwankt der Erfolg in Schritten von 0,25, fünf Sitzungen sind dafür zu wenig. |
| R-02 | Simulation wiederholt, wenn alles gesperrt ist | `kaiwa-sim` (days): bis zu zehn Elemente statt einer gesperrten Einheit |
| R-03 | Ids müssen nicht stabil sein | Einheiten und Szenarien tragen Ids in Pfad-Reihenfolge; Store-Migration 2 schreibt alte Profile um |
| R-04 | Kontras CI anpassen | Kontra #67: Rust-Build im app-Job cachen (der Qt-Cache greift meist schon; die Stapel-Regeln hält Kontra ein) |

## Entscheidungen aus Fragenpaket 8 (2026-10-13)

[Fragenpaket 8](https://claude.ai/artifact/7LU4Pr5HzXW8zzfL43zfFW); Q-01 bis Q-12 auch in `spec/INHALT-ENTWURF.md`, Abschnitt 6. Umgesetzt: Q-03, Q-04, Q-07 (ADR-009, #33), Q-02 (N5 mit 16 Einheiten, #34), Q-05, Q-06, Q-09 und Q-14 (N4); Q-10 (JF-Can-do-Ids bei 21 von 30 Einheiten, aus „JF Can-do for Life in Japan“; für n5-04, n5-09, n5-03, n5-15, n4-03, n4-11, n4-13, n4-14 und n4-12 gibt es dort keinen passenden Eintrag zum Sprechen).

| ID | Entscheidung | Wo |
|---|---|---|
| Q-01 | tanos-Listen als eine Quelle der Kaiwa-Stufe neben Curriculum und Häufigkeit; Namensnennung in `NOTICE` und „Über“ | ADR-007, /LD050/ |
| Q-02 | N5 auf 16 Einheiten erweitern (n5-03 bis n5-08) | `data/curriculum/n5.json` |
| Q-03 | Stufe in der App als „N4 · A2“, Hinweis „orientiert an JLPT und JF-Standard“, kein Bestehensversprechen | UI |
| Q-04 | Eine Curriculum-Datei je Stufe; der Lader prüft den Gesamtgraphen | `Curriculum::check` |
| Q-05 | N4: feste Keigo-Formeln aktiv (申します, おります, 伺います, お〜ください), 尊敬語 nur verstehen; aktives Keigo ab N3 | `n4.json` |
| Q-06 | N4 mit 14 Einheiten schon für 1.0 (n4-09 Feste, n4-12 Reklamation) | `n4.json`, Roadmap |
| Q-07 | Optionale Felder `level`, `topics`, `review` und Grammatik-Katalog `data/grammar/`; ADR „Curriculum-Format v2“ | neues ADR |
| Q-08 | Muttersprachler-Review: später entscheiden | offen |
| Q-09 | Jede zweite N4-Einheit bekommt zusätzlich ein Szenario in Freundesform, wo es im Leben vorkommt | `data/scenarios/` |
| Q-10 | JF-Can-do-Ids im Datensatz (Verweis; Kann-Sätze bleiben eigene Formulierung) | Curriculum-Format v2 |
| Q-11 | Dialekt erst ab N2 und nur zum Hören, ab N1 aktiv verstehen; bis dahin Standardsprache | Szenarien, Tutor-Anweisungen |
| Q-13 | Namen aus JMnedict in kaiwa-data 0.2 (Paket wird deutlich größer); der IPADIC-Entwurf auf `m3/namen` wird nicht gemergt | kaiwa-data, ADR-007 |
| Q-14 | Reihenfolge: Curriculum-Format v2, N5 auf 16 Einheiten, danach `n4.json` nach dem Entwurf | Roadmap M3/M7 |
| Q-15 | Vor dem ersten Paket: Tutor-Einrichtung in den Einstellungen (lokal OpenAI-kompatibel, Anthropic), dann Demo hinter ein Build-Feature (standardmäßig aus), der Paketbau prüft das Binary | Offen Nr. 7 |

Dazu geklärt (ohne Entscheidung nötig): Tae Kims Guide steht unter CC BY-NC-SA 3.0 US; eine zusätzliche Lizenz für Kaiwa hilft nicht (NC und ShareAlike bleiben). Er dient nur als Orientierung für Reihenfolge und Aufbau, Erklärungen schreibt Kaiwa selbst.

## Entscheidungen aus Fragenpaket 7 (2026-10-12)

| ID | Entscheidung | Wo |
|---|---|---|
| L-01 | Moren-Hinweise ohne Marge, standardmäßig an (59,7 % bei 4,8 % gelten als K-02) | ADR-005, `mora::APP_MARGIN`, Profil |
| L-02 – L-04 | 1.0 ohne Pitch-Bewertung; Kurve ohne Urteil mit akzenttreuer Stimme; Bewertung nach 1.0 | ADR-005, /LF321/ |
| L-05 | Nächster Schwerpunkt: Wörterbuch und Wort nachschlagen | /LD050/, /LF112/ |
| L-06 | Datenpaket als GitHub-Release des Repos `kaiwa-data` | ADR-007 |
| L-07 | Kante Leinen: `positive` #3a6e4e (4,92:1), Prüfung `hinnerk_targets` aktiv | Kante #56, `kaiwa-sim` (/LL203/) |
| L-08 | PR-Agent zweimal gescheitert: grüne CI reicht | `agent.md` (#23) |
| G-11 | Seit 18.10.2026 (Eigentümer, alle shrippen-Repos), ersetzt L-08: vor jedem Merge prüft ein neuer Claude-Agent; gemergt wird erst, wenn das letzte Review den neuesten Commit abdeckt und kein **Wichtig** offen ist, und die CI auf dem neuesten Commit grün ist | `agent.md`, `REVIEW.md` |
| G-12 | Seit 22.10.2026 (Eigentümer, alle shrippen-Repos), ergänzt G-11: zuerst prüft PR-Agent (die Session postet `/review` je Runde mit neuem Code); ein neuer Claude-Agent prüft die **Wichtig** früherer Runden und reviewt, wenn PR-Agent zweimal scheitert | `agent.md`, `REVIEW.md` |
| L-09 | JLPT-Listen von tanos.co.uk nur mit im Webarchiv bestätigter Lizenz | ADR-007 |

## Offen

1. **Moren-Prüfung** (ADR-005, Fragenpakete 5 und 6 entschieden): eigenes Kana-CTC-Modell trainiert (`spikes/s06-moren/finetune.py`, v1 base, v2 large), Test mit echten Stimmen (`real.py`, Common Voice mit hineingeschnittenen Fehlern): nur Kürzungen geprüft erkennt v2 61 % bei 4,8 % Fehlalarmen (v1 62 % bei 6,0 %; v3 mit ReazonSpeech dazu nicht besser, 59 % bei 5,1 %), alle Fehlerarten zusammen unter 25 %. Daten und Modelle liegen unter `target/` (Common Voice 8 GB, `kana-ctc-v1`, `kana-ctc-v2`), nicht im Repository. Rust: `kaiwa-speech::mora` (Varianten, CTC, `Checker` über ONNX Runtime, `G2p::pronounced`). v2 ist auf Hugging Face veröffentlicht ([`shrippen/kaiwa-kana-ctc`](https://huggingface.co/shrippen/kaiwa-kana-ctc), Apache-2.0, in `NOTICE`; Download-Adresse `kaiwa_speech::models::KANA_CTC_URL`, feste Revision). Die Hinweise sind in der App (nach einem gesprochenen Satz, höchstens einer, ohne Schweregrad), **standardmäßig an** (L-01): Auf dem App-Weg erkennen sie 59,7 % bei 4,8 % Fehlalarmen, das gilt als K-02 (ADR-005, „Umsetzung in der App“). Die Modelle (ReazonSpeech, Kokoro, kaiwa-kana-ctc) lassen sich in den Einstellungen unter „Modelle“ laden, nur nach Klick, mit Prüfsumme (N-04).
2. **Pitch-Akzent** (/LF321/): Die Vorstudie an echten Stimmen (`spikes/s07-pitch/`) findet bei 5 % Fehlalarmen nur 25–35 % der falschen Akzente (Ziel 80 %). Damit gilt F-09: 1.0 ohne Pitch-Bewertung (L-02). Mit einer akzenttreuen Stimme die eigene Kurve neben der der Figur, ohne Urteil (L-03); Bewertung nach 1.0 mit eigenem Akzentmodell (L-04).
3. **Lokaler Test** (`spec/LOKALER-TEST.md`): KI-Anbindung (Spike 0.5), VOICEVOX (H-01), Messwerte auf dem Framework 13, eigene Aufnahmen. Ergebnisse als PR nach `qa/local/<datum>/` (H-05).
4. **30 Prüfregeln** sind noch nicht automatisch, meist Audio, GUI (AT-SPI, Tastatur) oder Verhalten des echten Tutors. Sie brauchen den gui- und audio-Modus von `kaiwa-sim` oder den lokalen Test.
5. **Stimme:** H-01 (VOICEVOX) entscheidet die endgültige Stimme; bis dahin Kokoro als Übergang.
6. **Secret `ANTHROPIC_API_KEY`** fehlt noch in der Cloud-Umgebung (G-04).
7. **Demo im Release** (Q-15, umgesetzt: Tutor in den Einstellungen, Build-Feature `demo`, `packaging/check-release.sh` in PKGBUILD, Flatpak und CI). Vorher: Der Demo-Modus hängt nur an `KAIWA_DEMO`, und ohne `KAIWA_PROVIDER` antwortet die App mit den Aufzeichnungen. Vor dem ersten Paket braucht es eine Tutor-Einrichtung in den Einstellungen, dann ein Build-Feature `demo` (aus) und eine Prüfung im Paketbau.
8. **Kanji in Stücken:** `KanteRubyText` setzt ein `Text` je Zeichen; unter deutscher Locale nahm Qt für ein einzelnes Kanji oder 。 eine chinesische Schrift (andere Zeichenformen, tiefere Grundlinie). Kante 1.34 (Kante #59) setzt eine japanische Schrift, die Kopie hier ist abgeglichen.

## Lessons learned

- **`read_dir` hat keine Reihenfolge.** Der Grammatik-Katalog war lokal n5-zuerst, im CI-Container n4-zuerst; ein Test hing daran. Alles, was aus Verzeichnissen zusammengesetzt wird, sortiert selbst.
- **Ein echter Paketbau findet, was die CI nicht sieht.** `makepkg` scheiterte an LTO (`options=(!lto)`), und `NOTICE` fehlte im Paket.
- **Eine Prüfung mit festem Seed ist nur eine Stichprobe.** `mara_success_band` war grün, fällt aber bei 4 von 12 Seeds durch. Vor dem Vertrauen in eine Simulations-Prüfung einmal über mehrere Seeds laufen lassen (`kaiwa-sim --root <Kopie>` mit geändertem `seed`).
- **Inhalte von einer zweiten Instanz lesen lassen.** Ein Durchgang durch alle neuen Szenarien fand 17 Punkte, vier echte Fehler (eine falsche Erklärung zu くれる, eine unberechtigte Korrektur). Für neue Inhalte vor dem PR einplanen; das Muttersprachler-Review (Q-08) ersetzt es nicht und umgekehrt.
- **Gestapelte Zweige mit `git rebase --onto`.** Ein einfaches `rebase` auf den neuen Basiszweig spielt die alten Commits noch einmal ab. Der Wörterbuch-Auszug ist binär: bei Konflikten eine Seite nehmen und danach neu bauen.
- **IPADIC liest Zahl und Zählwort oft falsch** (三日 さんにち, 九時 きゅうじ, 七時 ななじ). Das Popup nimmt die Wörterbuch-Lesung; in Hilfesätzen Zeiten wählen, die eindeutig sind.
- **tanos-Zellen trennen Varianten mit 、 ・ / und Leerzeichen.** Erst der Blick auf die Rohzeilen zeigte, warum 何 als N3 galt.

- **Kante zuerst abgleichen, dann die App starten.** Nutzt eine Seite eine neue Eigenschaft einer Kante-Komponente, die die Kopie unter `app/qml/Kante` noch nicht hat, lädt die Seite nicht, und der Shot-Modus wartet stumm bis zum Timeout. `tools/sync-kante.sh` gegen den Kante-Zweig, dann gegen `main` nach dessen Merge.
- **Qt schreibt ohne Terminal ins Journal.** Läuft die App mit umgeleiteter Ausgabe, landen QML-Fehler nicht in der Datei; `QT_FORCE_STDERR_LOGGING=1` holt sie zurück.
- **`grabToImage` sieht keine Popups.** Sie liegen im Overlay des Fensters, das keine QML-Engine hat; der Shot-Modus nimmt bei offenem Wort-Popup dessen Inhalt auf (`--say 'lookup:L:W'`).
- **Wörterbuch-Treffer an echten Sätzen prüfen.** Erst `examples/lookup.rs` über echte Zeilen zeigte, dass Partikeln gleichlautende Nomen trafen (は → 葉) und feste Ausdrücke als Verb nachgeschlagen wurden; die Wortart aus IPADIC als Hinweis und „erst wie geschrieben“ lösten beides.
- **Eine Befehlskette mit `;` läuft weiter.** `cargo fmt --check && … ; git commit` committete trotz Formatfehler; lokal hilft der pre-commit-Hook mit `cargo fmt --all --check`.

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
