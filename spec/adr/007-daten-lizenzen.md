# ADR-007: Sprachdaten und Lizenzen

**Status:** Vorgeschlagen · **Bezug:** /LD050/, /LF220/, /LF221/

| Daten | Quelle | Lizenz | Folge |
|---|---|---|---|
| Wörterbuch | JMdict (EDRDG) | CC BY-SA 4.0 | Namensnennung in „Über“, abgeleitete Wörterbuch-DB als eigenes Datenpaket unter CC BY-SA |
| Kanji | KANJIDIC2 (EDRDG) | CC BY-SA 4.0 | wie JMdict |
| Morphologie und Akzentkern | UniDic (NINJAL) | BSD-3 / GPL / LGPL (wählbar) | BSD-3 wählen; `aType` liefert Akzentkerne |
| Morphologie-Engine | vibrato oder lindera (Rust) | MIT/Apache-2.0 | unproblematisch |
| JLPT-Zuordnung | keine offiziellen Listen seit 2010; Community-Listen | **zu prüfen** je Liste | nur Listen mit freier Lizenz; sonst eigene Zuordnung über Häufigkeit und Lehrwerk-unabhängige Kann-Beschreibungen |
| Akzent zusätzlich | OJAD (nicht weiterverteilbar), Kanjium-Akzentdaten | **zu prüfen** | nur mit freier Lizenz; sonst UniDic allein |
| Beispielsätze | Tatoeba | CC BY 2.0 FR | Namensnennung; nur geprüfte Sätze |

**Folge für das Paket:** `NOTICE` mit allen Quellen. Die Daten liegen als getrenntes, versioniertes Paket vor (`kaiwa-data`), damit Code (GPL) und Daten (CC BY-SA) sauber getrennt sind.

## Offen

- **F-02:** Lizenz des Codes: GPL-3.0-or-later wie die Plasma-Projekte, oder MIT wie Kante?
