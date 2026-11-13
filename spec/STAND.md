# Stand und Übergabe (2026-11-13)

Wo Kaiwa steht, was offen ist und was die Arbeit bisher gelehrt hat. Für die nächste Session zuerst lesen, dann `ROADMAP.md`. Welche Anforderung wo umgesetzt und geprüft ist, zeigt [`RUECKVERFOLGUNG.md`](RUECKVERFOLGUNG.md) (erzeugt mit `cargo run -p kaiwa-sim -- trace --write`). Starten und prüfen: `./start.sh` (`demo`, `sim`, `check`).

## Was da ist

| Bereich | Stand | PRs |
|---|---|---|
| Planung | Lastenheft 0.2 (mit den Entscheidungen der Fragenpakete), Roadmap, 7 QA-Personas mit 82 Prüfregeln, ADR-001 bis -008 | #1, #3 |
| Kern (`kaiwa-core`) | Gesprächslogik, Hilfe in drei Stufen, Korrekturen mit Schweregrad, Lernermodell, Schwierigkeitsregler (8 Stellgrößen, Ziel 80 %), FSRS-5, Tagesziel, Serie mit Freeze und Pause, XP gegen Ausnutzen, Curriculum N5 (16 Einheiten) und N4 (14 Einheiten, 22 Szenarien), Format v2 nach ADR-009 mit Grammatik-Katalog (119 Punkte), Einstufung per Gespräch, Rōmaji-Eingabe | #6, #14, #16 |
| Profil (`kaiwa-store`) | SQLite, Fortsetzen, Export nach `spec/export.schema.json`, Import, vollständiges Löschen | #15 |
| Backend (`kaiwa-backend`) | Ansichten als JSON, Tutor-Wahl, Kostenbremse, mehrere Profile, Aufnahmen mit Opt-in, Statistik, Erinnerungsregel | #17 |
| KI (`kaiwa-ai`) | Anthropic und OpenAI-kompatibel, aufgezeichnete Antworten (Demo), Token-Zähler, Schlüsselbund, „Was verlässt das Gerät?“ | #6, #17 |
| Sprachdaten (`kaiwa-lang`) | Wörter einer Zeile (lindera/IPADIC: Verb mit Endungen, Zahl mit Zählwort, お mit Nomen), Nachschlagen im Datenpaket [`kaiwa-data`](https://github.com/shrippen/kaiwa-data) (JMdict en/de, KANJIDIC2, Kanjium-Akzente; Release 0.1.0, Repo auf Gitea, Spiegel auf GitHub), Soll-Akzent; Furigana mit UniDic (Download unter „Modelle“, vorsichtig: Zahl mit Zählwort, Namen und 何 ohne Lesung), ohne UniDic vorsichtig mit IPADIC; über den eigenen Sätzen ersetzen die Lesungen des Tutors (Feld `reading` der Analyse) die des Wörterbuchs; Auszug `data/dict/` für Demo und Tests; Inhalts-Check „jedes Wort steht im Wörterbuch“ | #31 |
| Sprache (`kaiwa-speech`) | Push-to-Talk und Freisprechen (Ende per Pause), ReazonSpeech (sherpa-onnx), Übergangsstimme Kokoro ohne Pitch-Akzent, „Lauter“ und „Klar“, Moren-Prüfung, Modell-Download mit Prüfsumme; Erkennung und Stimme hinter Traits | #11, #12, #26, #27 |
| App | Heute, Lernpfad, Freies Gespräch (4 Themen), Wiederholen, Profil mit Statistik und Wochenrückblick, Einstellungen (Modelle, Textgröße bis 160 %, Modelle entladen), Gespräch mit Bilanz, Widersprechen, „Falsch erkannt“, Hörhilfen (nochmal, langsamer, übersetzen), Moren-Hinweise (standardmäßig an), Einstufung, Wort nachschlagen (Klick auf ein Wort der Figur: Lesung, Wortart, Bedeutung, Soll-Akzent, Anhören, in die Wiederholung; Wörterbuch-Download nur nach Klick), bessere Form einer Korrektur anhören; Deutsch und Englisch; Stile System, Kante, Kante Light | #7, #16, #17, #26, #27, #31 |
| Pakete | Symbol, Desktop-Datei, AppStream, PKGBUILD `kaiwa-git`, Flatpak-Manifest (App) | #17 |
| QA | `kaiwa-sim`: 60 von 89 Prüfregeln automatisch (Textmodus, 30-Tage-Läufe, Profil, Engine, QML-Prüfungen); Inhaltsprüfungen /LF221/ als Tests: Grammatik-Lint, Niveau der Hilfesätze, Register, Kernwörter, Review-Hash; `kaiwa-sim review-sheet` für das Muttersprachler-Review | laufend |

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

## Nacht zum 2026-11-10

[Nachtbericht mit Fragenpaket 11](https://claude.ai/artifact/77zr8JceriXZ9TA3Aowmtk) (T-01 bis T-11, Antworten unten). Gemergt: #64 (was die Stimme schon gesagt hat, spielt „Nochmal“ sofort, /LF111/), #65 (Zug-Ende im Kern, `kaiwa_core::turn_end`, mit Hinnerks Pausen getestet; ein Angebot verkürzt die Pause nie), #66 (jede Figur spricht mit ihrer Stimme, /LF117/; `mara_word_lookup` mit und ohne UniDic), #67 (Wochenrückblick mit Fortschritt pro Fähigkeit, /LF604/; Fehlerverlauf je Gespräch statt je Kalenderwoche, /LF610/; `mara_progress`), #68 (Tests und `kaiwa-sim` räumen ihre Temp-Ordner auf; `lena_stats_fast` nimmt die schnellste von drei Messungen), #69 (freies Gespräch auf dem eigenen Niveau, /LF116/; das Kostenlimit bremst nur den Cloud-Tutor, /LF714/; `theo_free_talk`, `jonas_cost_limit`, `hinnerk_own_profile`), #70 (Zustandsmaschine des Gesprächs: das Fenster wartet nicht mehr auf den Tutor, `Session::ask`/`take`; eine vom Tutor nie beantwortete Zeile ist kein Zug, `kaiwa_backend::talk`; ein fortgesetztes Gespräch zählt seine ganze Zeit, `spent_secs`; Freisprechen schließt das Mikrofon, während die Figur spricht, und hört von selbst wieder zu; ein Mikrofonfehler ist kein Endzustand mehr). Die Änderungen an Mikrofon und Stimme aus #70 sind ohne Ton gebaut: `LOKALER-TEST.md` 3.2a bis 3.2d. Danach #71 (ein zweiter Leser für die übrigen Seiten: die Serie beurteilt jeden Tag nach seinem eigenen Tagesziel, `DayRecord::goal`; der Zähler „Wiederholen“ zeigt alles Fällige; der Export trägt Profil und Ortszeit im Namen; die „Zwölf Wochen“ summieren nur diese zwölf Wochen). Zuletzt #72 (ein dritter Leser für Tutor-Anbindung und Lernkern: die Anthropic-Anfragen passen zu den aktuellen Modellen, strukturierte Ausgabe statt erzwungenem Tool-Aufruf, die Unterhaltung beginnt mit dem Lernenden, Platz zum Nachdenken; was ein Modell um die Analyse herum schreibt, stört nicht mehr, `Analysis::from_text`, das Objekt selbst muss weiter zum Schema passen (ADR-004, Frage T-11); die Preise der Kostenbremse sind auf dem Stand 11/2026; „auch möglich“ wird kein Wiederholungselement; ein fehlerfreies leichtes Gespräch senkt kein Niveau; Rōmaji wird Wort für Wort umgewandelt, ein einzelnes fremdes Wort macht den Satz nicht zur Frage in der eigenen Sprache). Aus dessen Review #73 (ein Entwurf im Nachdenken eines Modells gilt nie als Analyse, auch bei `</think>` ohne Anfang), aus dessen Review wiederum #74 (das Nachdenken eines lokalen Modells wird nicht als Zeile der Figur gezeigt und gesprochen, `parse_reply`; nicht mit einem lokalen Modell ausgeführt: `LOKALER-TEST.md` 2.8).

## Nacht zum 2026-11-13

[Nachtbericht mit Fragenpaket 14](https://claude.ai/artifact/RcXkBTsUqC3LX1gUjoVcMn) (V-01 bis V-05). Sechs zweite Leser (T-05) für Code ohne Tests: Einstufung, Wiederholen, Einstellungen, Export/Import/Löschen, Gesprächsseite, Backend-Module. Gemergt, je mit Test, der vorher scheiterte:

- Einstufung: #101 (eine Stufe, die durch ihre Ziele früh endet, beendet die Einstufung nicht; `placement::stage_session` mit `Coaching::Off`: Hilfe nur auf Wunsch, nicht leichter; neu einstufen behält Fehlerverlauf und eigene Korrekturen; Zurück an der Lesefrage fragt, `app/qml/leave.js`), #108 (Überspringen nach der Einstufung, `views::placement_offer`; `emi_skip` läuft jetzt über das Backend).
- Wiederholen: #106 (der Knoten am Lernpfad öffnet die Elemente seiner Einheit), #107 (FSRS-5 wie fsrs-rs v2.0: ganze Tage, Grenze nach „Again“), #112 (eine Wiederholung in einer Transaktion; ein Fehler heißt „Nicht gespeichert“).
- Einstellungen: #100 (lokaler Server nur bei genauem Host), #104 (Segmentschalter folgen dem Profil, `app/qml/Choice.qml`), #105 (Pausenhinweis ohne Serie, wenn Gamification aus ist), #109 (ein Schlüssel aus Leerzeichen löscht nichts; „Entfernen“ ist ein eigener Schritt), #112 (ein unlesbares Profil wird nicht überschrieben, `Store::update_profile`).
- Export, Import, Löschen: #102 (der Import behält die letzte Wiederholung; unvollständige Dateien werden abgelehnt), #110 (scheitert das Löschen, bleibt die App auf der Datei, `Backend::wipe`).
- Gespräch: #113 (nach dem Aufzeichnen keine Markierungen mehr, `Session::close`; eine Hilfe zählt für die angeklickte Zeile), #115 (kein Zug geht verloren: `busy` gilt auch während der Erkennung, ein getippter Zug schließt das Mikrofon; eine Antwort vor der Stimme wird gesprochen, sobald sie geladen ist, `talk::Debt`; ein geändertes Kostenlimit gilt sofort).
- Nachschlagen: #114 (das Wort im Satz statt eines Zwillings: した → 舌, ここ → 九, きょう → 京, なら → 奈良; gemessen über alle Szenariotexte mit `examples/lookup_corpus.rs` gegen `data/dict/lookup-expected.tsv`).
- Kante: #103, #111, #116, #117 (Abgleiche); Kante #161 behebt Titel der Breite 0 in `KanteSettingRow` aus Kante #160.

Lokale App-Prüfungen: Den Screenshot-Modus gibt es nur im Demo-Build (`cargo build --release -p kaiwa-app --features demo`); ohne ihn startet die App normal und `kaiwa-sim run --app` wartet bis zum Zeitlimit.

## Nacht zum 2026-11-11

[Nachtbericht mit Fragenpaket 13](https://claude.ai/artifact/Y3ak5E9vXKRKJSfY5i6bfo) (U-01 bis U-04, beantwortet am 2026-11-11, siehe unten). Gemergt: #90 (Mindestbreite 533 px, S-01 bis S-07; Kante 1.78 aus Kante #139; die Wochenkarte endet am Tag des Backends, `heatEnd`), #91 (eine gescheiterte Ansicht lässt der Seite ihre Vorgaben, `views::text`), #92 (Zeitlimit für Anfrage und Antwortkopf des Modell-Downloads), #93 (Pausenmodus über das Backend, `jonas_sick_pause`). Gemessen: Kaiwa im Leerlauf 178 MB RSS, 120 MB PSS (Release, offscreen, frisches Profil); /LL012/ nennt 150 MB, ob RSS oder PSS, entscheidet U-01.

## Entscheidungen aus Fragenpaket 13 (2026-11-11)

[Nachtbericht mit Fragenpaket 13](https://claude.ai/artifact/Y3ak5E9vXKRKJSfY5i6bfo), Antworten des Eigentümers am 2026-11-11.

| Nr. | Entscheidung | Stand |
|---|---|---|
| U-01 | /LL012/: 150 MB gelten als PSS | umgesetzt: Lastenheft, `lena_idle_memory` misst PSS neben einem zweiten Kirigami-Fenster (wie auf dem Desktop), 122 MB |
| U-02 | Download: eigene Verbindung mit Zeitlimit je Lesevorgang | umgesetzt (#96): `download::Idle` begrenzt jede Leseoperation auf 30 s |
| U-03 | App-Prüfungen in der CI | umgesetzt: `kaiwa-sim run --app` im Qt-Job; `hinnerk_narrow`, `hinnerk_font`, `lena_idle_memory`, `lena_app_egress` (strace, aufgezeichneter Tutor). Lokal bleiben `lena_zero_egress` (lokaler Tutor, Modelle, kaiwad), `lena_models_unload`, der Blick auf die Bilder, Überlappungen und Furigana |
| U-04 | /LF215/: Überspringen ohne Prüfgespräch, Lastenheft anpassen | umgesetzt (#97) |

## Entscheidungen aus Fragenpaket 12 (2026-11-10)

[Fragenpaket 12: Schmales Fenster](https://claude.ai/artifact/D2QspX7v2YijZtcDQs4tdX), Antworten des Eigentümers am 2026-11-10. Die zuerst gemergte Mindestbreite von 800 px (#89) war auf einem mit 150 % skalierten Bildschirm gemessen: gemeint waren 800 Bildschirmpixel, also 533 px in der App.

| Nr. | Entscheidung | Stand |
|---|---|---|
| S-01 | Mindestbreite 533 px (800 bei 150 %), alle Ansichten anpassen | umgesetzt: `narrowest` in `Main.qml`, /LL303/; unter 700 px (`compactBelow`) stapeln Einstellungen und Gesprächsleiste |
| S-02 | Profil, „Neu einstufen“: Knöpfe unter den Text, wenn die Zeile nicht reicht | umgesetzt: `KanteListRow.minTextWidth` und `stacked` (Kante 1.78) |
| S-03 | Profil: Zahlen umbrechen (2×2), Stellschrauben einspaltig | umgesetzt |
| S-04 | Einstellungen: unter 700 px Bezeichnung oben, Bedienelement darunter | umgesetzt: `KanteSettingRow.narrow` |
| S-05 | Gespräch: unter 700 px zwei Zeilen, Sprechknopf oben über die ganze Breite, darunter Eingabe, Hilfe, Senden | umgesetzt |
| S-06 | Lernpfad: Kann-Satz in normaler Schrift, umbrechend; Kennung bleibt Mono | umgesetzt: `text` je Knoten (`views::path`, `KantePath`, Kante 1.78) |
| S-07 | Profil: Heatmap mit Wochentagen, Wochen und Legende, im selben PR | umgesetzt: `KanteCalendarHeatmap` mit `KanteLegend`; `heatDays`, `heatTips`, `heatScale` (`views::stats`) |

## Entscheidungen aus Fragenpaket 11 (2026-11-10)

[Fragenpaket 11](https://claude.ai/artifact/77zr8JceriXZ9TA3Aowmtk), Antworten des Eigentümers am 2026-11-10. „offen“ in der Spalte Stand heißt: entschieden, noch nicht umgesetzt.

| ID | Entscheidung | Stand |
|---|---|---|
| T-01 | Freisprechen: Das Angebot „längere Pause“ kommt schon nach dem ersten erkennbar abgeschnittenen Zug und geht zwei Schritte weiter (1,2 → 3 → 5 s). Die Regel von `hinnerk_no_cutoff` wird entsprechend umgeschrieben: Nachdenken und Länge beenden keinen Zug; nach dem ersten erkennbar abgeschnittenen Zug kommt das Angebot; mit passender Pause wird keiner mehr abgeschnitten | umgesetzt: `CutOffs` (`kaiwa_core::turn_end`), Regel in `qa/personas/hinnerk.json`, Prüfung `crates/kaiwa-sim/src/hands_free.rs` |
| T-02 | Das Angebot bekommt einen zweiten Knopf „So lassen“; danach zählt die App neu | umgesetzt: nach „So lassen“ kommt das Angebot im selben Gespräch erst nach zwei weiteren erkennbar abgeschnittenen Zügen wieder (`CutOffs::decline`); ein neues Gespräch zählt von vorn |
| T-03 | Schriftgröße 160 % (`hinnerk_font`): nur als lokales Skript, die CI bleibt, wie sie ist | umgesetzt: `tools/font-160.sh` (Bilder aller Ansichten bei 160 %, Liste der Texte, die nicht passen); `--check-text` nur im Demo-Build |
| T-04 | Freies Gespräch nennt der Figur das Sprechniveau aus dem Lernermodell (#69): so lassen, im lokalen Test mit ansehen | #69; `LOKALER-TEST.md` |
| T-05 | Nächster Schwerpunkt: A, weiter Prüfregeln für Anforderungen ohne Prüfung und zweite Leser für Code ohne Tests (als Nächstes Wiederholen, Einstellungen, Einstufung, Modell-Downloads) | laufend |
| T-06 | Wiederholungen sperren den Lernpfad gar nicht mehr; sie werden nur empfohlen | umgesetzt: `Curriculum::path` sperrt nur noch über fehlende Einheiten; eine Einheit mit schwachen Wiederholungen (`UnitNode::shaky`) bekommt am Wiederholungsknoten „empfohlen, bevor es weitergeht“; es zählen nur die letzten fünf Antworten der Einheit (Eigentümer, 10.11.2026) |
| T-07 | Auffrischung nach einer Pause: zehn sind zehn, der Rest wird beim Wiedereinstieg einmal auf die nächsten Tage verteilt; Wiederholen zählt als Übung | umgesetzt: `srs::spread_after_break` (zehn bleiben fällig, der Rest je zehn auf die nächsten Tage), `Store::reenter` einmal je Pause, `Store::last_practice` zählt Wiederholungen |
| T-08 | Alle drei bauen: Import eines Exports in ein leeres Profil, ein Knopf „Überspringen“ am Hinweis des Lernpfads, „Pause beenden“ in den Einstellungen | umgesetzt: „Export importieren …“ in den Einstellungen (`importProfile`, nur in ein leeres Profil), „… überspringen“ am Hinweis des Lernpfads (`skipUnit` mit der aktuellen Einheit; ein Angebot überspringt genau eine Einheit, das nächste braucht drei neue fast fehlerfreie Gespräche, keine Einheit mit 敬語 ohne das Register; `views::take_skip`), „Pause beenden“ und der letzte Tag der Pause in den Einstellungen (`Store::end_pause`) |
| T-09 | Modell-Download mit Zeitlimit je Lesevorgang (30 s ohne Daten gelten als abgerissen), „Abbrechen“ greift dann immer | umgesetzt (#79): `kaiwa-speech` liest den Download in einem eigenen Thread (`Patient`), 30 s ohne Daten enden mit `DownloadError::Stalled`, „Abbrechen“ wird alle 200 ms geprüft |
| T-10 | Anthropic: geringe Denktiefe für die Antwort der Figur, die Analyse wie voreingestellt; Ersatz bei abgelehnten Antworten einschalten. Beides nach dem ersten Live-Lauf des Eigentümers, mit Zeiten davor und danach | wartet auf den Live-Lauf (`LOKALER-TEST.md` 1) |
| T-11 | Analyse des Tutors: reparieren, aber sichtbar. Der Zug zeigt, dass die Analyse unvollständig war, und zählt nicht für das Niveau (ändert ADR-004) | ADR-004 („Unvollständige Analyse“), `Analysis::from_text`, `TurnRecord::incomplete`, `jonas_analysis_gaps` |

## Entscheidungen aus Fragenpaket 10 (2026-11-09)

[Fragenpaket 10](https://claude.ai/artifact/J9bfCbFdaHanLQEckNEtsC), Antworten des Eigentümers am 2026-11-09.

| ID | Entscheidung | Stand |
|---|---|---|
| S-01 | `mara_success_band`: gleitend über 8 Sitzungen, 12 Seeds, mindestens 10 im Band; der Regler bleibt bei einem Schritt (ersetzt R-01) | `kaiwa-sim` (`days::check_seeds`, `BAND_*`), `qa/personas/mara.json`; gemessen 12 von 12 |
| S-02 | チーズ, テニス, うち als N5-Kernwörter (n5-05, n5-09, n5-11), raus aus `data/dict/level-exceptions.txt` | `data/curriculum/n5.json`; Band danach weiter 12 von 12 |
| S-03 | Nächster Schwerpunkt: B, den lokalen Test vorbereiten (echter Tutor, Spracherkennung, Stimme; die nicht automatischen Prüfregeln), und C, Wiederholung als Mini-Gespräch (/LF230/, `emi_srs_keigo`). Das erste Paket (A) wartet | C umgesetzt: eine Korrektur merkt sich die Zeile der Figur (`Item::cue`), die Wiederholung spielt sie als Mini-Gespräch (`ReviewPage`), eine Antwort zählt, wenn sie die Form enthält und den alten Fehler nicht an seiner Stelle wiederholt (`Item::answered`); `emi_srs_keigo` automatisch. B vorbereitet: `spec/LOKALER-TEST.md` als Ablauf mit Vorlage `qa/local/VORLAGE.md`; fünf Prüfregeln, die den Tutor beurteilen, prüft `kaiwa-sim run --live` selbst (`run::LIVE`). Der Test selbst liegt beim Eigentümer |

## Entscheidungen aus Fragenpaket 9 (2026-10-14)

Allgemein (gilt für alle Projekte, `../CLAUDE.md`): Vor dem ersten Release sind Breaking Changes in Ordnung, migrieren statt um alte Ids herum bauen.

| ID | Entscheidung | Stand |
|---|---|---|
| R-01 | `mara_success_band` über 12 Seeds prüfen (≥ 10 im Band), Regler zwei Schritte über 0,95 | **Ersetzt durch S-01.** gemessen hält das Band mit Fenster 5 nur 7 von 12 Seeds; zwei Schritte über 0,95 verschlechtern auf 4 von 12 (überschießt), ein Mittel über zwei Sitzungen bringt 8 von 12. Mit Fenster 8 halten alle Varianten das Band: Bei 3–4 Zielen je Sitzung schwankt der Erfolg in Schritten von 0,25, fünf Sitzungen sind dafür zu wenig. |
| R-02 | Simulation wiederholt, wenn alles gesperrt ist | `kaiwa-sim` (days): bis zu zehn Elemente statt einer gesperrten Einheit; seit T-06 sperren Wiederholungen nichts mehr, der Fall tritt nur noch ein, wenn der Pfad durchgespielt ist |
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
| G-11 | Seit 18.10.2026 (Eigentümer, alle shrippen-Repos), ersetzt L-08: vor jedem Merge prüft ein neuer Claude-Agent; gemergt wird erst, wenn das letzte Review den neuesten Commit abdeckt und kein **Wichtig** offen ist, und die CI auf dem neuesten Commit grün ist | `RULES.md`, Skill `pr-review`, `REVIEW.md` |
| G-12 | Seit 22.10.2026 (Eigentümer, alle shrippen-Repos), ergänzt G-11: zuerst prüft PR-Agent (die Session postet `/review` je Runde mit neuem Code); ein neuer Claude-Agent prüft die **Wichtig** früherer Runden und reviewt, wenn PR-Agent zweimal scheitert | `RULES.md`, Skill `pr-review`, `REVIEW.md` |
| L-09 | JLPT-Listen von tanos.co.uk nur mit im Webarchiv bestätigter Lizenz | ADR-007 |

## Offen

1. **Moren-Prüfung** (ADR-005, Fragenpakete 5 und 6 entschieden): eigenes Kana-CTC-Modell trainiert (`spikes/s06-moren/finetune.py`, v1 base, v2 large), Test mit echten Stimmen (`real.py`, Common Voice mit hineingeschnittenen Fehlern): nur Kürzungen geprüft erkennt v2 61 % bei 4,8 % Fehlalarmen (v1 62 % bei 6,0 %; v3 mit ReazonSpeech dazu nicht besser, 59 % bei 5,1 %), alle Fehlerarten zusammen unter 25 %. Daten und Modelle liegen unter `target/` (Common Voice 8 GB, `kana-ctc-v1`, `kana-ctc-v2`), nicht im Repository. Rust: `kaiwa-speech::mora` (Varianten, CTC, `Checker` über ONNX Runtime, `G2p::pronounced`). v2 ist auf Hugging Face veröffentlicht ([`shrippen/kaiwa-kana-ctc`](https://huggingface.co/shrippen/kaiwa-kana-ctc), Apache-2.0, in `NOTICE`; Download-Adresse `kaiwa_speech::models::KANA_CTC_URL`, feste Revision). Die Hinweise sind in der App (nach einem gesprochenen Satz, höchstens einer, ohne Schweregrad), **standardmäßig an** (L-01): Auf dem App-Weg erkennen sie 59,7 % bei 4,8 % Fehlalarmen, das gilt als K-02 (ADR-005, „Umsetzung in der App“). Die Modelle (ReazonSpeech, Kokoro, kaiwa-kana-ctc) lassen sich in den Einstellungen unter „Modelle“ laden, nur nach Klick, mit Prüfsumme (N-04).
2. **Pitch-Akzent** (/LF321/): Die Vorstudie an echten Stimmen (`spikes/s07-pitch/`) findet bei 5 % Fehlalarmen nur 25–35 % der falschen Akzente (Ziel 80 %). Damit gilt F-09: 1.0 ohne Pitch-Bewertung (L-02). Mit einer akzenttreuen Stimme die eigene Kurve neben der der Figur, ohne Urteil (L-03); Bewertung nach 1.0 mit eigenem Akzentmodell (L-04).
3. **Lokaler Test** (`spec/LOKALER-TEST.md`, bereit seit 2026-11-09): `kaiwa-sim run --live` mit echtem Tutor, der Tutor in der App, Sprechen und Stimme, Rechner und Oberfläche, VOICEVOX (H-01), Messwerte, eigene Aufnahmen. Ergebnisse mit `qa/local/VORLAGE.md` als PR nach `qa/local/<datum>/` (H-05).
4. **29 Prüfregeln** (von 89) laufen ohne Tutor nicht automatisch: 6 prüft `kaiwa-sim run --live` (der Tutor selbst), 10 der lokale Test von Hand (Tutor-Urteil, Audio, Rechner, Oberfläche), 13 gehören zu Plasmoid, Android, Windows oder Pitch und warten auf ihre Meilensteine.
5. **Stimme:** H-01 (VOICEVOX) entscheidet die endgültige Stimme; bis dahin Kokoro als Übergang.
6. **Secret `ANTHROPIC_API_KEY`** fehlt noch in der Cloud-Umgebung (G-04).
7. **Demo im Release** (Q-15, umgesetzt: Tutor in den Einstellungen, Build-Feature `demo`, `packaging/check-release.sh` in PKGBUILD, Flatpak und CI). Vorher: Der Demo-Modus hängt nur an `KAIWA_DEMO`, und ohne `KAIWA_PROVIDER` antwortet die App mit den Aufzeichnungen. Vor dem ersten Paket braucht es eine Tutor-Einrichtung in den Einstellungen, dann ein Build-Feature `demo` (aus) und eine Prüfung im Paketbau.
8. **Kanji in Stücken:** `KanteRubyText` setzt ein `Text` je Zeichen; unter deutscher Locale nahm Qt für ein einzelnes Kanji oder 。 eine chinesische Schrift (andere Zeichenformen, tiefere Grundlinie). Kante 1.34 (Kante #59) setzt eine japanische Schrift, die Kopie hier ist abgeglichen.
9. **Freisprechen schneidet langsame Sprecher ab, bis die Pause passt** (T-01, T-02 umgesetzt): Das Angebot kommt nach dem ersten erkennbar abgeschnittenen Zug und geht zwei Schritte weiter; Hinnerk wird in der Simulation zweimal abgeschnitten (alle 40 Seeds), dann steht die Pause auf 5 s. Schnitte nach einem bloßen Adverb (まいあさ) erkennt `sounds_unfinished` weiter nicht; ein solcher Zug bringt kein Angebot. `hinnerk_no_cutoff` läuft mit der neuen Regel automatisch (ohne Erkenner und Stimme); mit echter Stimme bleibt Schritt 3.2 des lokalen Tests.
10. **Schriftgröße** (`hinnerk_font`, /LL202/, T-03): `tools/font-160.sh` findet bei 160 % auf einem Laptop mit 1366 px (Fenster 853 breit) keinen Text, der nicht passt, deutsch wie englisch (10.11.2026). Bei 200 % auf demselben Laptop (Fenster 683 breit) ragte die Profilseite rechts über das Fenster; der Eigentümer hat darum entschieden (10.11.2026): 160 % ist vorerst die obere Grenze (`kaiwa_store::TEXT_SCALE_MAX`, Lastenheft /LL202/, Einstellungen 100 bis 160 %). Ein Fenster unter etwa 700 px ließ Profilseite und Einstellungen rechts hinausragen; seit /LL303/ ist das Fenster mindestens 533 px breit (800 auf einem mit 150 % skalierten Bildschirm), und dort passt jeder Text (`SCALE=1 SCREEN=533 tools/font-160.sh`, 10.11.2026). Überlappungen und Furigana prüft das Skript nicht, die Bilder wollen einen Blick (lokaler Test 4.4).
11. **Aus dem zweiten Leser, noch offen** (T-06 bis T-09 im Fragenpaket 11):
    - **Lernpfad sperrte nach drei falschen Wiederholungen** (T-06, umgesetzt): Wiederholungen sperren nichts mehr. Sind von den letzten fünf Antworten einer Einheit (ab drei) unter 60 % richtig, steht an ihrem Wiederholungsknoten „empfohlen, bevor es weitergeht“; drei richtige nach einem schlechten Tag nehmen den Hinweis wieder weg (Eigentümer, 10.11.2026). Ein gespeicherter Zählerstand von vorher (Antworten, richtige) liest sich als „noch keine Antworten“; der Knoten erscheint wie bisher nur, wenn etwas fällig ist.
    - **Auffrischung nach einer Pause** (T-07, /LF231/, umgesetzt): Beim Wiedereinstieg bleiben die zehn Elemente fällig, die am ehesten vergessen sind; der Rest des Fälligen wird einmal je Pause auf die nächsten Tage verteilt, zehn je Tag. Der Tag des Wiedereinstiegs bleibt „Willkommen zurück“, was auch immer zuerst geöffnet wird; Wiederholen zählt als Übung. Ein Export trägt den Tag des Wiedereinstiegs nicht mit.
    - **Was die Oberfläche versprach, aber nicht konnte** (T-08, umgesetzt): Import eines Exports in ein leeres Profil (Knopf in den Einstellungen, Dateidialog des Systems), das Überspringen der aktuellen Einheit am Hinweis des Lernpfads, eine laufende Pause sehen (letzter Tag) und beenden. Nicht in der laufenden App geprüft. Nach dem Klick ist das Angebot verbraucht (Prüfungen `emi_skip_once`, `lena_import`). Der Knopf überspringt ohne Prüfgespräch; /LF215/ sagt das seit U-04 so. Pause und „Pause beenden“ gehen über das Backend (`Backend::pause`, `end_pause`); `jonas_sick_pause` prüft beide.
    - **Ein hängender Modell-Download** (T-09, umgesetzt in #79): 30 s ohne Daten gelten als abgerissen, „Abbrechen“ greift auch, während nichts ankommt. Namensauflösung, Verbindungsaufbau und Antwortkopf haben je 30 s (`download::open`), sonst hielt ein Server, der nie antwortet, seinen Thread für immer. Schweigt der Server mitten im Rumpf, endet auch der Thread nach 30 s: jede Leseoperation hat ein eigenes Zeitlimit (U-02, `download::Idle` über ureqs Verbindung).
12. **Die Anthropic-Anbindung ist nicht live geprüft** (T-10): #72 hat die Anfragen nach der API-Referenz umgebaut (`crates/kaiwa-ai/src/anthropic.rs`), in der Sitzung gab es keinen Schlüssel. Der erste Schritt des lokalen Tests (`kaiwa-sim run --live` mit `KAIWA_PROVIDER=anthropic`) zeigt, ob sie stimmen. Offen dabei: Die Modelle denken vor der Antwort nach, in der Voreinstellung recht gründlich; für die Antwortzeit (/LL010/) lohnt `output_config.effort` „low“ für die Antwort der Figur, das ist eine Abwägung zwischen Tempo und Güte und nicht gesetzt. Und: Lehnt ein Modell eine Antwort ab (Sicherheitsfilter), gibt es keinen Ersatz; die API bietet dafür `fallbacks` an.

13. **Kleinigkeiten der zweiten Leser vom 13.11., noch offen:**
    - Gespräch: Die Zeit nach dem letzten Zug zählt nicht, und Schließen des Fensters verliert einen laufenden Zug. Stimme aus, Freisprechen an, Gespräch noch bei der ersten Zeile: Nach dem Entladen und Neuladen der Modelle bleibt das Mikrofon zu, der Lernende muss erneut klicken (`talk::Debt::voice_ready` liefert `Opening` statt `Nothing`, die stumme Eröffnung ruft kein `listen_again`).
    - Kosten: Zwei Prozesse auf einem Profil (zweite App, später Plasmoid) können das Limit zusammen doppelt ausgeben, und `book` liest und schreibt den Monat ohne Transaktion.
    - Aufnahmen: Die Frist wird nur beim Öffnen des Profils und beim Ändern der Einstellung durchgesetzt, nicht im laufenden Betrieb.
    - Erinnerung: „Später“ über Mitternacht verschiebt die Erinnerung auf den nächsten Tag (wirkt erst mit M4).
    - Nachschlagen: IPADIC liefert für かって/よんだ die Grundform かつ/よぶ; ohne Beugungsform im `LineWord` nicht zu lösen. Noch Zwillinge (auch vor #114): こと → 琴, そう/よう, と als „if“ statt „with“. IPADIC trennt こうちゃ in こう + ちゃ; ちゃ zeigt dann ては.
    - Export: Exporte im Format 1 vom 9. bis 14.10.2026 tragen alte Einheiten-Ids (nur Testdaten).

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
- **Die Rückverfolgungstabelle auf einem ruhigen Rechner und mit leerem `KAIWA_MODELS` erzeugen.** Mit lokalem UniDic und unter Last (ein Build nebenher) kamen rote Zeilen hinein, die es in der CI nicht gibt; ein Skript, das vor dem Push alle Prüfungen fährt und beim ersten Fehler abbricht, fängt das.
- **Eine Prüfregel darf nicht grüner sein als ihr Satz.** `hinnerk_no_cutoff` lief grün, weil die Prüfung abgeschnittene Züge nur zählte; die Regel verspricht, dass es keine gibt. Vor dem Automatisieren den Satz der Persona gegen das halten, was der Lauf zeigt; passt es nicht, ist das eine Frage an den Eigentümer, kein Umschreiben.
- **Urteile über Zeiträume an Rändern prüfen.** Der Fehlerverlauf urteilte zweimal falsch: erst über Wochen ohne Übung, dann über angebrochene Kalenderwochen. Je Gespräch und je Zug zu vergleichen hat beide Fälle erledigt.
- **Einen zweiten Leser auf Code ohne Tests ansetzen.** `app/src/conversation.rs` hat keine Tests und lässt sich in der CI nicht mit Ton fahren. Ein neuer Agent, nur lesend, mit dem Auftrag „Abläufe, die hängen bleiben oder falsch zählen, mit Zeile und Ablauf belegen“ fand acht Fehler; jeder ließ sich am Code bestätigen. Was sich als reine Logik herauslösen lässt (`turn_of`, `Session::ask`), bekommt dabei Tests im Kern.
- **Tests mit eigenem `TMPDIR` laufen lassen** (`~/.cache/claude/…`); `/tmp` liegt im RAM. Ein eigenes Verzeichnis zeigt auch, was ein Lauf liegen lässt.
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
