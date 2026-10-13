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

**Folge für das Paket:** `NOTICE` mit allen Quellen. Die Daten liegen als getrenntes, versioniertes Paket vor (`kaiwa-data`), damit Code (MIT) und Daten (CC BY-SA) sauber getrennt sind. Die Share-alike-Pflicht gilt nur für das Datenpaket. Es erscheint als GitHub-Release eines eigenen Repos `kaiwa-data` (L-06); wie alle Release-Artefakte baut es GitHub Actions, die App lädt es von dort mit Prüfsumme.

## Entschieden

- **L-06 (2026-10-12):** Das Datenpaket ist ein GitHub-Release des eigenen Repos `kaiwa-data` (Arbeits-Repo auf Gitea, wie alle Projekte).
- **L-09 (2026-10-12):** Die JLPT-Listen von Jonathan Waller (tanos.co.uk) kommen nur hinein, wenn eine Archivkopie der Seite (Internet Archive) die freie Lizenz zeigt; sonst eine eigene Zuordnung über Häufigkeit und Kann-Beschreibungen.
  - *Geprüft 2026-10-13:* Die Seite „Sharing and copyrights“ sagt in drei Archivkopien gleich: „Everything on this site (that I'm not selling), is licenced under Creative Commons "BY" … use anything here however you like (commercial or non-commercial), but credit my site.“ Ausgenommen sind nur Verkaufsprodukte (Grammar-Plus-Audio). Kopien: [2012](https://web.archive.org/web/20121216045735/http://www.tanos.co.uk/jlpt/sharing/), [2017](https://web.archive.org/web/20170118202220/http://www.tanos.co.uk/jlpt/sharing/), [2021](https://web.archive.org/web/20210122121139/https://www.tanos.co.uk/jlpt/sharing/). Eine CC-Version nennt die Seite nicht. Damit ist die Bedingung erfüllt; ob und wie die Listen ins Paket kommen, entscheidet Fragenpaket 8 (Inhaltsaufbau).
  - *Umgesetzt mit kaiwa-data 0.2 (Q-01, Q-13):* Tabelle `jlpt` aus den Wortlisten N5 bis N1, gelesen aus festen Archivkopien von 2020 (`fetch.sh` in kaiwa-data), Namensnennung in `NOTICE` des Pakets und hier; Tabelle `name` aus JMnedict (EDRDG, CC BY-SA 4.0) für Namen im Wort-Popup. Das Paket wächst von 25,9 auf 47,8 MB (entpackt 84 auf 158 MB).

## Umsetzung (2026-10-13)

- Repo `kaiwa-data` (Gitea, Spiegel `github.com/shrippen/kaiwa-data`, M-02): `build.py` (nur Standardbibliothek) baut aus JMdict (Englisch und Deutsch, M-01), KANJIDIC2 und Kanjium eine SQLite-Datei mit Lizenz-Tabelle, Schema 1 (README dort). Release `v0.1.0`: 25,9 MB Download, 84,3 MB entpackt; gebaut von GitHub Actions, nur Tag und Notiz auf Gitea.
- Kaiwa lädt das Paket nur nach Klick (Wort-Popup oder Einstellungen › Modelle, M-03) mit fester Größe und SHA-256 (`kaiwa_speech::download::MODELS`), nach `<Modelle>/kaiwa-data-0.1.0/`. Der Auszug für Demo und Tests liegt unter `data/dict/` (CC BY-SA, eigenes README), nie im Programm. Ein Test prüft, dass jedes Wort der Szenarien und Aufzeichnungen darin steht (/LF221/, bewusste Lücken wie Namen in `data/dict/known-gaps.txt`).
- Lesen: `kaiwa-lang` (Wörter einer Zeile mit lindera/IPADIC, Nachschlagen, Soll-Akzent aus Kanjium).

- **F-02:** Code unter **MIT** (wie Kante). Abhängigkeiten müssen dazu passen: keine (A)GPL-Bibliotheken im ausgelieferten Programm, LGPL (Qt, Kirigami) nur dynamisch gelinkt. `cargo deny` prüft das ab M1.
