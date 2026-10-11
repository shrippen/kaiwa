# ADR-007: Sprachdaten und Lizenzen

**Status:** Angenommen (Lizenz F-02: MIT) · **Bezug:** /LD050/, /LF220/, /LF221/

| Daten | Quelle | Lizenz | Folge |
|---|---|---|---|
| Wörterbuch | JMdict (EDRDG) | CC BY-SA 4.0 | Namensnennung in „Über“, abgeleitete Wörterbuch-DB als eigenes Datenpaket unter CC BY-SA |
| Kanji | KANJIDIC2 (EDRDG) | CC BY-SA 4.0 | wie JMdict |
| Morphologie und Akzentkern | UniDic (NINJAL) | BSD-3 / GPL / LGPL (wählbar) | BSD-3 wählen; `aType` liefert Akzentkerne |
| Morphologie-Engine | vibrato oder lindera (Rust) | MIT/Apache-2.0 | unproblematisch |
| Morphologie für die Übergangsstimme | IPADIC (NAIST, über lindera einkompiliert) | IPADIC-Lizenz (frei, mit Hinweispflicht) | in `NOTICE`; nur Lesarten, kein Akzent |
| Lautumschrift-Tabelle | misaki (Apache-2.0), aus cutlet (MIT) | Apache-2.0 / MIT | Herkunft im Quelltext (`g2p/table.rs`), in `NOTICE` |
| Stimme | Kokoro-82M v1.0 (hexgrad) | Apache-2.0 | Modell wird geladen, nicht mitgeliefert |
| JLPT-Zuordnung | keine offiziellen Listen seit 2010; Community-Listen, am verbreitetsten die von Jonathan Waller (tanos.co.uk) | laut mehreren abgeleiteten Projekten CC BY; die Seite selbst ist nicht mehr erreichbar (2026-10-11), die Lizenz also nicht an der Quelle bestätigt | nur mit bestätigter freier Lizenz (Archivkopie der Seite prüfen); sonst eigene Zuordnung über Häufigkeit und Lehrwerk-unabhängige Kann-Beschreibungen |
| Akzent zusätzlich | OJAD (nicht weiterverteilbar), Kanjium-Akzentdaten (`mifunetoshiro/kanjium`, `accents.txt`, 124 137 Wörter) | Kanjium: CC BY-SA 4.0 laut README des Repos (2026-10-11 geprüft), Teile aus EDRDG-Daten unter deren Lizenz; OJAD nicht weiterverteilbar | Kanjium passt ins Datenpaket (CC BY-SA, Namensnennung Uros Ozvatic); OJAD nicht |
| Beispielsätze | Tatoeba | CC BY 2.0 FR | Namensnennung; nur geprüfte Sätze |

**Folge für das Paket:** `NOTICE` mit allen Quellen. Die Daten liegen als getrenntes, versioniertes Paket vor (`kaiwa-data`), damit Code (MIT) und Daten (CC BY-SA) sauber getrennt sind. Die Share-alike-Pflicht gilt nur für das Datenpaket.

## Entschieden

- **F-02:** Code unter **MIT** (wie Kante). Abhängigkeiten müssen dazu passen: keine (A)GPL-Bibliotheken im ausgelieferten Programm, LGPL (Qt, Kirigami) nur dynamisch gelinkt. `cargo deny` prüft das ab M1.
