# Inhaltsstruktur Kaiwa: Entwurf N5 bis N1

Stand 2026-10-13 · Entwurf zur Entscheidung · Bezug: /LF220/, /LF221/, /LD040/, /LD050/, ADR-007 (L-06, L-09), ROADMAP M2, M7, „Nach 1.0“

Dieses Papier schlägt vor, wie die Inhalte von Kaiwa über alle Stufen aufgebaut werden. Es baut auf
`data/curriculum/n5.json` (12 Einheiten) und `data/scenarios/*.json` auf und erweitert deren Format,
statt es zu ersetzen. Für 1.0 zählen N5 und N4, N3 bis N1 sind ein Umriss für danach.

---

## 1. Ergebnisse der Recherche

### 1.1 Was die JLPT-Stufen offiziell bedeuten

Die JEES beschreibt jede Stufe nur über Lesen und Hören, nicht über Listen
([Summary of Linguistic Competence](https://www.jlpt.jp/e/about/levelsummary.html)). Kurzfassung:

| Stufe | Lesen (offiziell, gekürzt) | Hören (offiziell, gekürzt) | CEFR-Bezug (JEES, ab Dez. 2025) |
|---|---|---|---|
| N5 | typische Ausdrücke und Sätze in Hiragana, Katakana und einfachen Kanji | Gespräche über Alltags- und Unterrichtsthemen; nötige Information aus kurzen, langsam gesprochenen Gesprächen | A1 (ab 80 Punkten) |
| N4 | Texte zu vertrauten Alltagsthemen mit Grundwortschatz und -kanji | Alltagsgespräche, im Großen und Ganzen, wenn langsam gesprochen | A2 (ab 90) |
| N3 | Texte mit konkretem Inhalt zu Alltagsthemen, Schlagzeilen; etwas schwierigere Texte mit Umschreibungshilfe | zusammenhängende Alltagsgespräche nahe natürlichem Tempo, Beziehungen der Beteiligten | A2 (95–103), B1 (ab 104) |
| N2 | klar geschriebene Artikel, Kommentare, einfache Kritiken; Absicht des Schreibenden | Gespräche und Nachrichten in fast natürlichem Tempo, in Alltag und anderen Situationen | B1 (90–111), B2 (ab 112) |
| N1 | logisch komplexe und abstrakte Texte, Leitartikel, Kritiken | Gespräche, Nachrichten, Vorträge in natürlichem Tempo, Details, logische Struktur | B2 (100–141), C1 (ab 142) |

Quelle CEFR-Bezug: [JLPT CEFR reference](https://www.jlpt.jp/e/about/cefr_reference.html). Für Kaiwa heißt das:
N5 ≈ A1, N4 ≈ A2, N3 ≈ A2/B1, N2 ≈ B1/B2, N1 ≈ B2/C1. Das passt zu /LF220/ („JF-Standard A1–A2“ für 1.0).

**Keine offiziellen Listen seit 2010.** Die JLPT-FAQ sagt, die „Test Content Specifications“ würden
nicht mehr veröffentlicht, weil das Ziel Kommunikation sei, nicht das Auswendiglernen von Wörtern,
Kanji und Grammatik ([JLPT FAQ](https://www.jlpt.jp/e/faq/)). Auch KANJIDIC2 vermerkt: „No official
kanji lists are available for the new levels“ (DTD-Kommentar zu `<jlpt>`).

**Größenordnungen.** Belegt sind nur die alten Zahlen (2004er Spezifikation, Tabelle nach
[Wikipedia: JLPT](https://en.wikipedia.org/wiki/Japanese-Language_Proficiency_Test)):
alte Stufe 4 ≈ 100 Kanji / 800 Wörter, Stufe 3 ≈ 300 / 1 500, Stufe 2 ≈ 1 000 / 6 000,
Stufe 1 ≈ 2 000 / 10 000. Die neuen Stufen gelten als gleich, nur die alte Stufe 2 ist auf N3 und N2
verteilt. Für die neuen Stufen üblich zitierte Schätzungen (Community, nicht offiziell):

| Stufe | Wörter (kumuliert) | Kanji (kumuliert) | Grammatikpunkte (je Stufe, grob) | Lernstunden ohne Kanji-Vorwissen (selbst berichtet 2010–15) |
|---|---|---|---|---|
| N5 | 700–800 | 80–100 | 70–90 | 325–600 |
| N4 | 1 500 | 250–300 | 110–130 | 575–1 000 |
| N3 | 3 700 | 600–650 | 120–180 | 950–1 700 |
| N2 | 6 000 | 1 000 | 150–200 | 1 600–2 800 |
| N1 | 10 000 | 2 000 | 150–220 | 3 000–4 800 |

Die Grammatikzahlen schwanken stark je nach Zählweise (ob 〜てください und 〜てもいい ein oder zwei
Punkte sind). Kaiwa sollte eigene Zahlen aus dem eigenen Grammatikkatalog nennen, keine fremden.

### 1.2 Quellen für Wortschatz, Kanji, Grammatik und Kann-Beschreibungen

| Quelle | Inhalt | Lizenz | Für `kaiwa-data` nutzbar? |
|---|---|---|---|
| [tanos.co.uk](http://www.tanos.co.uk/jlpt/) (Jonathan Waller) | Wort-, Kanji- und Grammatiklisten N5–N1 (Schätzungen auf Basis der alten Listen) | **CC BY**, im Webarchiv bestätigt: Seite „Sharing and copyrights“: *„Everything on this site (that I'm not selling), is licenced under Creative Commons "BY" … use anything here however you like (commercial or non-commercial), but credit my site.“* Snapshots [2012-12-16](https://web.archive.org/web/20121216045735/http://www.tanos.co.uk/jlpt/sharing/), [2017-01-18](https://web.archive.org/web/20170118202220/http://www.tanos.co.uk/jlpt/sharing/), [2021-01-22](https://web.archive.org/web/20210122121139/https://www.tanos.co.uk/jlpt/sharing/). Live-Seite antwortet nicht mehr. | **Ja** (L-09 erfüllt), mit Namensnennung und Link. Version (2.0/3.0) nennt die Seite nicht; Namensnennung wie bei CC BY 4.0 genügt allen. Ausgenommen: Verkaufsprodukte (Grammar-Plus-Audio). |
| [open-anki-jlpt-decks](https://github.com/jamsinclair/open-anki-jlpt-decks) | korrigierte tanos-Wortlisten als CSV | Repo MIT, Daten aber von tanos abgeleitet (CC BY) | Ja, als bequemere und gepflegte Abzugsquelle, Namensnennung tanos bleibt Pflicht |
| KANJIDIC2 (EDRDG), Feld `<jlpt>` | alte JLPT-Stufe 1–4 je Kanji, `<freq>` (Zeitungsrang 1–2 500), `<grade>` (Schuljahr, Jōyō) | CC BY-SA 4.0 | Ja (bereits in ADR-007). Achtung: alte 4 Stufen, alte 2 = N3 + N2 |
| JMdict (EDRDG), `ke_pri`/`re_pri` | Häufigkeitsmarker: `news1/2` (Mainichi-Wortliste, je 12 000), `ichi1/2` (Ichimango goi bunruishuu), `spec1/2`, `gai1/2` (Lehnwörter), `nfxx` (500er-Bänder) | CC BY-SA 4.0 | Ja (bereits in ADR-007). Grobes Signal, Zeitungssprache, nicht Gesprächssprache |
| [wordfreq](https://github.com/rspeer/wordfreq) (rspeer) | Zipf-Häufigkeit aus Wikipedia, Untertiteln (OpenSubtitles), Web, Twitter, Reddit | Daten CC BY-SA 4.0, Code Apache-2.0; Stand ~2021, eingefroren | Ja, passt zum CC-BY-SA-Paket. Untertitel-Anteil gut für Gesprächssprache |
| [Wiktionary-Häufigkeitslisten](https://en.wiktionary.org/wiki/Wiktionary:Frequency_lists/Japanese) | Wikipedia-Häufigkeit (2013/2015/2022), Verweise auf Leipzig (CC BY 4.0), FrequencyWords/OpenSubtitles (CC BY-SA 4.0), Leeds (CC BY 2.5) | CC BY-SA 4.0 bzw. wie angegeben | Ja; wordfreq deckt das meiste schon ab |
| BCCWJ-Häufigkeitsliste (NINJAL) | beste ausgewogene Korpusliste des Japanischen | [„free for research or educational purposes“](https://clrd.ninjal.ac.jp/bccwj/en/freq-list.html), Weitergabe nicht geregelt | **Nein** im Paket; höchstens intern zum Abgleich, nach Rückfrage bei NINJAL |
| Tatoeba | Beispielsätze | CC BY 2.0 FR | Ja (ADR-007), nur geprüfte Sätze |
| [JF-Standard, Can-do-Datenbank](https://www.jfstandard.jpf.go.jp/cando/copyright/ja/render.do) | ~Tausende Can-dos (CEFR, JF, JF生活日本語), 15 Themen, Stufen A1–C2 | Eigenes Recht der Japan Foundation: kostenlos ohne Erlaubnis nutzbar; bei Änderung trägt der Nutzer die Verantwortung; Weitergabe und kommerzielle Nutzung nicht ausdrücklich geregelt | **Als Orientierung ja, als Datensatz nein.** Kaiwa formuliert eigene Kann-Sätze (so schon in n5.json) und verweist höchstens auf Themen und Stufen |
| [JF-Standard, 15 Themen](https://marugoto.jpf.go.jp/en/assets/docs/teacher/feature/15_topics.pdf) | Themenliste und welche Marugoto-Stufe welches Thema behandelt | © Japan Foundation | Nur als Struktur (Themennamen sind keine schützbare Schöpfung), nicht kopieren |
| Tae Kims Guide | Grammatik-Erklärungen | Creative Commons, nicht-kommerziell (BY-NC-SA laut Sekundärquellen) | **Nein** (NC passt nicht zu MIT/CC BY-SA) |
| Bunpro, JLPT Sensei, Imabi, Genki, Minna no Nihongo, Marugoto, Tobira, Quartet | Grammatiklisten, Erklärungen, Dialoge | urheberrechtlich geschützt | **Nein**, nur die grobe Reihenfolge als Orientierung (Lastenheft Kap. 1: keine Inhalte Dritter) |

**Wichtigster Befund:** Die Lizenz der tanos-Listen ist im Webarchiv dreifach bestätigt (2012, 2017,
2021, jeweils CC BY). Die Bedingung von L-09 ist damit erfüllt. Zwei Einschränkungen bleiben:
die Listen sind Schätzungen eines Einzelnen (bekannte Fehler, alte Grenze N3/N2 unscharf), und sie
beruhen teils auf den alten offiziellen Listen. Für Kaiwa sind sie ein Signal unter mehreren, nicht
die Wahrheit (Abschnitt 4.2).

### 1.3 JF-Standard: Themen je Stufe

Die 15 Themen des JF-Standards und wo Marugoto sie behandelt
([15 topics](https://marugoto.jpf.go.jp/en/assets/docs/teacher/feature/15_topics.pdf)):

| Nr. | Thema | A1 | A2 | A2/B1 | B1 |
|---|---|---|---|---|---|
| 1 | Ich und meine Familie | ● | ● | ● | ● |
| 2 | Wohnung und Umgebung | ● | | ● | |
| 3 | Freizeit und Unterhaltung | ● | ● | ● | ● |
| 4 | Alltag und Lebensabschnitte | ● | ● | ● | ● |
| 5 | Arbeit und Beruf | | ● | ● | |
| 6 | Reisen und Verkehr | ● | ● | ● | ● |
| 7 | Gesundheit | | ● | | ● |
| 8 | Einkaufen | ● | ● | | ● |
| 9 | Essen | ● | ● | ● | ● |
| 10 | Natur und Umwelt | | ● | ● | |
| 11 | Beziehungen (Umgang, Probleme, Manieren) | (●) | ● | ● | ● |
| 12 | Schule und Bildung | (●) | (●) | ● | |
| 13 | Sprache und Kultur | ● | ● | (●) | ● |
| 14 | Gesellschaft (Politik, Wirtschaft) | | | | ● |
| 15 | Technik (Wissenschaft, Medien) | | | | ● |

Folge: A1/N5 bleibt bei Ich, Wohnen, Freizeit, Alltag, Reisen, Einkaufen, Essen, Sprache. A2/N4 bringt
Arbeit, Gesundheit, Natur/Wetter und Beziehungen dazu. Gesellschaft und Technik erst ab B1.

### 1.4 Wie etablierte Kurse aufbauen (nur Struktur)

Gemeinsamer Kern aller Anfängerwerke, belegt über Inhaltsverzeichnisse und Verlagsangaben
([GENKI 3rd ed.](https://genki3.japantimes.co.jp/en/intro/): „from polite forms through the passive and causative“;
[QUARTET](https://quartet.japantimes.co.jp/en/about/): Band I ≈ N3, Band II ≈ N2, Ziel B1 bis Beginn B2):

| Phase | Typische Reihenfolge | Entspricht |
|---|---|---|
| 1 | です/ます, Partikeln, Zahlen, こそあど, あります/います, ます-Vergangenheit, Adjektive | N5 erste Hälfte (Genki I L1–5, MnN 1–13, Marugoto A1) |
| 2 | て-Form (てください, ている, てもいい, てはいけない), einfache Form, と思う, たい, たことがある, たり, Vergleich, ほうがいい, んです | N5 zweite Hälfte bis N4 Anfang (Genki I L6–12, MnN 14–25) |
| 3 | Potential, Volitional, Geben/Nehmen (+て), たら/ば/と/なら, そう/よう/みたい, てしまう/ておく/てある, Transitiv/Intransitiv | N4 (Genki II L13–18, MnN 26–40) |
| 4 | Keigo (尊敬語, 謙譲語), Passiv, Kausativ, Kausativ-Passiv | N4 Ende (Genki II L19–23, MnN 41–50) |
| 5 | Ausdrucksvarianten für Meinung, Grund, Absicht, Wandel; Erzählen; Keigo im Alltag | N3 (Quartet I, Tobira Anfang, Marugoto A2/B1) |
| 6 | Schriftsprachliche Funktionswörter, Argumentation, Nuance | N2/N1 (Quartet II, Tobira, Mittel-/Oberstufenwerke) |

Gesprächs-Apps (Duolingo, Jumpspeak) ordnen nach Situationen statt nach Grammatik: Grüßen,
Vorstellen, Bestellen, Wegfragen, Einkaufen, Verabreden, Reisen, später Arbeit und Meinung. Kaiwa macht
beides: die Einheit ist eine Situation (Szenario), ihre Grammatik folgt der obigen Reihenfolge.

### 1.5 Gesprächsfunktionen je Stufe

| Stufe | Sprechen und Hören: Funktionen | Register |
|---|---|---|
| N5 / A1 | grüßen, danken, entschuldigen; sich vorstellen; bestellen, kaufen, Preis fragen; Weg und Ort fragen; Zeit und Pläne ausmachen; einfach erzählen, was war; Verständnis sichern („Noch einmal bitte“, „langsamer bitte“) | です/ます; Freundesform nur einfache Muster; Laden-Keigo nur verstehen |
| N4 / A2 | Erfahrungen erzählen; Gründe nennen (から/ので/し); um etwas bitten und Erlaubnis fragen; Rat geben und annehmen; Vorhaben und Pläne begründen; Probleme melden (krank, verloren, kaputt); Vermutungen und Gehörtes weitergeben; Gefälligkeiten; einfache Telefon- und Arbeitsfloskeln | です/ます sicher, Freundesform aktiv, Keigo: feste Formeln aktiv, 尊敬語 verstehen |
| N3 / A2–B1 | zusammenhängend erzählen; Meinung äußern und begründen; vergleichen und abwägen; sich beschweren und verhandeln; am Telefon Termine ändern; Bewerbungsgespräch einfach; Register wechseln | Keigo aktiv in Standardsituationen |
| N2 / B1–B2 | diskutieren mit Gegenargument; erklären und anleiten; Nachrichten zusammenfassen; Kundengespräch und Besprechung; höflich ablehnen; indirekte Sprache verstehen | Keigo flüssig, Höflichkeitsstufen nach Beziehung |
| N1 / B2–C1 | debattieren, überzeugen; Nuancen, Ironie, Andeutungen; formelle Rede, Interview, Vortrag halten; Dialekte verstehen (Kansai) | gesamtes Spektrum, bewusst gewählt |

---

## 2. Inhaltsarchitektur

### 2.1 Grundsätze

1. **Eine Stufe = eine Datei** `data/curriculum/<stufe>.json` im heutigen Format (`id`, `units[]`
   mit `id`, `title`, `can_do`, `grammar`, `words`, `scenarios`, `requires`). Neue Felder nur optional,
   damit `Unit` in `kaiwa-core` ohne Bruch weiterliest (siehe 2.6).
2. **Ein Graph über alle Stufen.** `n4-01` darf `n5-12` voraussetzen. Der Lader fügt die Dateien
   zusammen und prüft den Gesamtgraphen (`Curriculum::check` heute nur je Datei, siehe Frage F-4).
3. **Einheit = Situation mit Kann-Ziel.** Jede Einheit hat 1–2 Szenarien (eines höflich, bei Bedarf
   eines in Freundesform), 3–4 Ziele je Szenario wie heute, 2–5 Grammatikpunkte, 15–25 Kernwörter.
4. **Grammatikpunkte sind Ids aus einem Katalog**, nicht Freitext. Heute steht z. B.
   `"〜ました (Dank)"`. Vorschlag: Katalog `data/grammar/<stufe>.json`; im Curriculum bleibt der
   Anzeigestring, der Katalog bildet ihn auf eine Id ab (rückwärtsverträglich).
5. **Freie Gespräche** (`frei-*`) hängen an keiner Einheit und laufen auf dem Niveau der Lernenden.
   Ab N3 werden sie wichtiger; ab N2 sind Aufgaben-Gespräche (Diskussion, Erklärung) der Normalfall.
6. **Kann-Sätze in eigenen Worten**, orientiert an JF-Themen und -Stufen, nie kopiert.

### 2.2 Überblick

| Stufe | CEFR | Einheiten | Kernwörter aktiv (kumuliert) | Wortschatz passiv (Ziel) | Kanji (nur Lesehilfe) | Szenarien | Für |
|---|---|---|---|---|---|---|---|
| N5 | A1 | 12 vorhanden, Vorschlag 16 | ~350 | ~800 | ~100 | 16–20 | 1.0 |
| N4 | A2 | 12 (+2 optional) | ~700 | ~1 500 | ~300 | 16–20 | 1.0 |
| N3 | A2/B1 | ~16 | ~1 400 | ~3 700 | ~650 | ~24 + frei | nach 1.0 |
| N2 | B1/B2 | ~14 Aufgaben-Einheiten | ~2 500 | ~6 000 | ~1 000 | Aufgaben + frei | nach 1.0 |
| N1 | B2/C1 | ~10 Aufgaben-Einheiten | – | ~10 000 | ~2 000 | Aufgaben + frei | nach 1.0 |

Aktiv heißt: in Szenarien gesprochen und im SRS (/LF230/). Passiv heißt: darf in KI-Äußerungen
vorkommen, ohne den 5-%-Rahmen (/LF211/) zu sprengen. Kanji sind bei Kaiwa nur Lesestütze
(/LF110/, Furigana); Kanji-Ziele steuern die Furigana-Dichte, nicht das Lernen.

```
N5 (A1) ──► N4 (A2) ──► N3 (A2/B1) ──► N2 (B1/B2) ──► N1 (B2/C1)
 Situationen  Situationen   Situationen     Aufgaben         Aufgaben
 です/ます     + Freundesform + Keigo aktiv    + Diskussion     + Nuance
 └────────── 1.0 ──────────┘ └──────────────── nach 1.0 ───────────────┘
```

### 2.3 N5: vorhandene 12 Einheiten und Lücken

| Einheit | Thema (JF) | Grammatik heute | Szenario, Register |
|---|---|---|---|
| n5-01 Grüßen und danken | 11 Beziehungen | 〜ね, 〜ました (Dank) | Nachbar Yamada, höflich |
| n5-02 Sich vorstellen | 1 Ich | 〜です, 〜から来ました, 〜が好きです | Gastgeberin Tanaka, höflich |
| n5-03 Konbini | 8 Einkaufen | 〜はいりません, いくらですか, Zahlen | Kassiererin, höflich (Laden-Keigo hören) |
| n5-04 Café | 9 Essen | 〜をください, 〜をおねがいします, 〜つ | Kellner, höflich |
| n5-05 Bahnhof | 6 Reisen | 〜まで, 何時, 〜枚 | Schalter, höflich |
| n5-06 Weg fragen | 6 Reisen | 〜はどこですか, 〜て、〜 (Reihenfolge), 遠い・近い | Passantin, höflich |
| n5-07 Mit Freunden | 3 Freizeit | einfache Form, 〜よ・〜ね, 〜ない？ | Aoi, **Freundesform** |
| n5-08 Verabredungen | 3 Freizeit | 〜ませんか, 〜たいです, 〜に会います | Kollege Kimura, höflich |
| n5-09 Restaurant | 9 Essen | 〜人, 〜を二つください | Kellnerin, höflich |
| n5-10 Einkaufen | 8 Einkaufen | い-Adjektive, 〜のはありますか, 〜にします | Verkäufer, höflich |
| n5-11 Familie | 1 Ich | 〜人家族, 〜がいます, Familienwörter | Gastmutter Suzuki, höflich |
| n5-12 Gestern | 4 Alltag | 〜ました, 〜かったです, 〜で, 〜と | Lehrer Tanaka, höflich |

**Lücken gegenüber dem üblichen N5-Umfang:**

- Grammatik ohne Einheit: な-Adjektive; Verneinung und Vergangenheit der Adjektive (〜くないです,
  〜じゃないです, 〜くなかったです); 〜に〜があります/います (Ort); 〜ている (Verlauf, Zustand);
  〜てください (Bitte); 〜てもいいですか, 〜てはいけません; 〜ないでください; 〜から (Grund),
  〜が (aber); 〜ましょう/ましょうか; 〜前に, 〜てから; Vergleich (〜より, 〜のほうが, 〜でいちばん);
  〜がほしい; 〜に行く (Zweck); 〜くなる/〜になる; Häufigkeit (よく, ときどき, あまり〜ない);
  Uhrzeit, Wochentage, Datum systematisch; こそあど als System.
- Themen ohne Einheit: Wohnung und Umgebung (JF 2), Tagesablauf (nur `frei-mainichi`),
  Sprache und Unterricht (JF 13; die JLPT-Beschreibung nennt „classroom situations“ ausdrücklich),
  Wetter und Jahreszeiten (nur Smalltalk in n5-01).
- Gesprächsstrategien fehlen als eigenes Ziel: „もう一度おねがいします“, „ゆっくりおねがいします“,
  „〜は日本語で何ですか“. Für ein Sprechprogramm sind sie wichtiger als manche Grammatik und
  passen zu /LF122/ (Code-Switching).

**Vorschlag: N5 um vier Einheiten erweitern** (Frage F-2):

| Einheit | Kann | Grammatik | Szenario, Figur, Register | requires |
|---|---|---|---|---|
| n5-13 Im Unterricht | Ich kann nachfragen, um Wiederholung bitten und fragen, wie etwas auf Japanisch heißt. | 〜は日本語で何ですか, もう一度〜, 〜てください (Bitte), 〜でもいいですか | Sprachkurs, Lehrerin, höflich | n5-02 |
| n5-14 Mein Tag | Ich kann meinen Tagesablauf mit Uhrzeiten und Häufigkeit beschreiben. | Uhrzeit/Wochentage, 〜前に, 〜てから, よく/ときどき/あまり〜ない, 〜ている (Gewohnheit) | Tandempartnerin, höflich (ersetzt nicht `frei-mainichi`) | n5-08 |
| n5-15 Meine Wohnung | Ich kann sagen, wo ich wohne, was es in der Nähe gibt und wie die Wohnung ist. | 〜に〜があります/います, な-Adjektive, Adjektiv-Verneinung, 〜が (aber), 〜から (Grund) | Nachbarin oder Kollegin, höflich | n5-11 |
| n5-16 Darf ich? | Ich kann um Erlaubnis bitten und Verbote verstehen (Tempel, Museum, Onsen). | 〜てもいいですか, 〜てはいけません, 〜ないでください, 〜ましょうか | Aufsicht im Museum oder Onsen-Personal, höflich | n5-06, n5-13 |

Vergleich (〜より, 〜のほうが, いちばん) und 〜がほしい kommen in n5-10 Einkaufen dazu,
〜くなる und Wetter in n5-01. Abschluss von N5 ist dann n5-12 *und* n5-16.

### 2.4 N4: Einheitenliste (fertig zur Umsetzung als `n4.json`)

Leitidee N4: von „Ich kann eine Situation bestehen“ zu „Ich kann begründen, erzählen, bitten und
Probleme lösen“. Register werden bewusst gewechselt: jede zweite Einheit hat auch ein Szenario in
Freundesform. Keigo kommt in zwei Einheiten, aktiv nur mit festen Formeln.

Alle Figuren sind japanische Erwachsene in Japan oder Deutschland, ohne reale Vorbilder. Namen frei
gewählt; Lernende aus der Demowelt (Studio Weber) treten nur als Personas auf, nicht als Figuren.

---

**n4-01 Erfahrungen** · JF 3 Freizeit, 13 Kultur · requires `n5-12`

- Kann: Ich kann erzählen, was ich schon einmal gemacht habe, und nachfragen, wie es war.
- Grammatik: 〜たことがあります, 〜たり〜たりします, 〜し、〜し (Gründe aufzählen), 〜とき
- Szenario `n4-01-keiken`: Tandemabend, Figur Morita Haruka (30), erzählt vom Bergsteigen,
  fragt nach Reisen und Essen. Höflich. Ziele: eine Erfahrung nennen; zwei Tätigkeiten mit たり;
  nachfragen (〜たことがありますか); sagen, wie es war, mit zwei Gründen (し).
- Kernwörter: 一度, 富士山, 登る, 着物, 経験, 〜ことがある, 景色, 珍しい …

**n4-02 Beim Arzt** · JF 7 Gesundheit · requires `n5-14`, `n5-16`

- Kann: Ich kann sagen, was mir fehlt und seit wann, und Ratschläge verstehen.
- Grammatik: 〜んです (Erklärung), 〜ので, 〜すぎる, 〜たほうがいい / 〜ないほうがいい,
  〜なければなりません, 〜なくてもいいです
- Szenario `n4-02-byouin`: Hausarztpraxis in Tokio, Ärztin Dr. Ishikawa, ruhig, fragt nach
  Symptomen, gibt Rat. Höflich. Ziele: Beschwerde nennen (頭が痛いんです); seit wann; Ursache
  (〜すぎました); nachfragen, ob man etwas muss oder nicht muss.
- Zweites Szenario `n4-02-kaze` (optional): Freundin ruft an, man sagt eine Verabredung ab, weil man
  krank ist. Freundesform (〜んだ, 〜から).
- Kernwörter: 熱, 頭, のど, せき, 薬, 〜日前から, 飲む (Medizin), 休む, 無理する …

**n4-03 Wohnung suchen** · JF 2 Wohnung · requires `n5-15`

- Kann: Ich kann Wünsche an eine Wohnung nennen, vergleichen und Hausregeln verstehen.
- Grammatik: 〜より〜のほうが (Wiederholung), 〜やすい / 〜にくい, 〜てあります, 〜ておきます,
  〜ために (Zweck)
- Szenario `n4-03-fudousan`: Maklerbüro, Herr Okada, geschäftlich höflich, Laden-Keigo zum
  Verstehen (ご案内します). Ziele: Wunsch nennen (駅に近い, 静か); zwei Wohnungen vergleichen;
  nach Regeln fragen (ペットは); sagen, was man vorher erledigt (〜ておきます).
- Kernwörter: 部屋, 家賃, 駅から〜分, 広い, 静か, 便利, ごみ, 規則, 契約 …

**n4-04 Geschenke und Gefälligkeiten** · JF 11 Beziehungen · requires `n5-11`

- Kann: Ich kann sagen, wer wem etwas gegeben oder für wen getan hat, und mich angemessen bedanken.
- Grammatik: あげる / くれる / もらう, 〜てあげる / 〜てくれる / 〜てもらう, 〜てくれてありがとう,
  〜てよかった
- Szenario `n4-04-omiyage`: Rückkehr zur Gastfamilie, Gastvater Suzuki (Ehemann aus n5-11),
  höflich-herzlich. Ziele: Mitbringsel übergeben (つまらないものですが …); erzählen, wer geholfen hat;
  danken für eine Gefälligkeit.
- Zweites Szenario `n4-04-tanjoubi`: Geburtstag einer Freundin, Freundesform (〜てくれた, 〜もらった).
- Kernwörter: お土産, プレゼント, 誕生日, 手伝う, 貸す, 借りる, 送る, 喜ぶ …

**n4-05 Vorhaben** · JF 4 Alltag, 3 Freizeit · requires `n4-01`

- Kann: Ich kann sagen, was ich vorhabe und beschlossen habe, und warum.
- Grammatik: Volitionalform (〜よう), 〜ようと思っています, 〜つもりです, 〜予定です,
  〜ことにしました, 〜までに
- Szenario `n4-05-natsuyasumi`: Mittagspause mit Kollegin Endō, Freundesform unter Gleichaltrigen
  (行こうと思ってる). Ziele: Plan für Ferien nennen; Entschluss begründen; Frist nennen (〜までに);
  zurückfragen.
- Zweites Szenario `n4-05-mokuhyou`: Gespräch mit der Lehrerin über Lernziele, höflich.
- Kernwörter: 夏休み, 計画, 決める, 予約, 貯金, 留学, 〜年後 …

**n4-06 Reise planen** · JF 6 Reisen · requires `n5-05`, `n4-05`

- Kann: Ich kann Bedingungen nennen und Vorschläge machen, wenn ich eine Reise plane.
- Grammatik: 〜たら, 〜ば, 〜と (Automatismus: 右に曲がると …), 〜なら (Empfehlung), 〜という (名前)
- Szenario `n4-06-ryokan`: Reservierung am Telefon mit einem Ryokan, Wirtin Ogawa, höflich mit
  festem Keigo zum Verstehen. Ziele: Datum und Personen; fragen, was man tun soll, wenn man spät
  ankommt (〜たら); Empfehlung erfragen (〜なら、どこがいいですか); Name eines Ortes erfragen
  (〜という所).
- Kernwörter: 予約, 泊まる, 一泊, 温泉, 到着, 遅れる, 案内, 名所 …
- Passt zu Persona Selin („im Ryokan einchecken“).

**n4-07 Was ich kann** · JF 12 Bildung, 5 Arbeit · requires `n4-01`

- Kann: Ich kann sagen, was ich kann und was ich inzwischen gelernt habe.
- Grammatik: Potentialform (〜られる, 話せる), 〜ことができる (Wiederholung), 〜ようになりました,
  〜ように (damit), 〜ながら
- Szenario `n4-07-baito`: Vorstellung für einen Aushilfsjob in einem Café, Leiterin Fujii, höflich.
  Ziele: Fähigkeiten nennen; Fortschritt (〜ようになりました); zwei Dinge gleichzeitig (〜ながら);
  Arbeitszeiten absprechen.
- Kernwörter: 運転, 料理, 英語, 〜が話せる, 経験, アルバイト, シフト, 週に〜回 …

**n4-08 Wetter und Gerüchte** · JF 10 Natur, 11 Beziehungen · requires `n4-02`

- Kann: Ich kann Vermutungen äußern und weitergeben, was ich gehört oder gesehen habe.
- Grammatik: 〜そうです (Aussehen: 雨が降りそう), 〜そうです (Hörensagen), 〜ようです / 〜みたい,
  〜かもしれません, 〜でしょう, 〜はずです, 〜って (umgangssprachlich)
- Szenario `n4-08-tenki`: Kollege Nishida am Morgen im Büro, Wetterbericht und Bürogerücht.
  Höflich mit eingestreuter Freundesform (〜らしいよ). Ziele: Wetter vermuten; Gehörtes
  weitergeben (〜そうです); Unsicherheit ausdrücken (〜かもしれません).
- Kernwörter: 天気予報, 台風, 曇り, 雪, 〜によると, 噂, 引っ越す, 結婚する …

**n4-09 Pech gehabt** · JF 6 Reisen, 8 Einkaufen · requires `n4-03`, `n4-06`

- Kann: Ich kann melden, dass etwas verloren oder kaputt ist, und beschreiben, wie es passiert ist.
- Grammatik: 〜てしまいました / 〜ちゃった, transitive und intransitive Verbpaare (開ける/開く,
  壊す/壊れる), 〜ている (Zustand nach Veränderung), 〜のに, 〜ばよかった
- Szenario `n4-09-wasuremono`: Fundbüro am Bahnhof, Beamter Hayashi, sachlich höflich.
  Ziele: Verlust melden; Gegenstand beschreiben (Farbe, Größe, was darin ist); sagen, wo und wann.
- Zweites Szenario `n4-09-hoteru`: Hotelzimmer, Klimaanlage geht nicht (つかない, 壊れている), höflich.
- Kernwörter: 財布, 忘れ物, 落とす, 壊れる, 止まる, 開かない, 届く, 確認 …

**n4-10 Höflich bitten** · JF 11 Beziehungen, 2 Wohnung · requires `n4-04`

- Kann: Ich kann jemanden höflich um etwas bitten, das ihm Mühe macht, und nachfragen, ob etwas geht.
- Grammatik: 〜ていただけませんか, 〜てくださいませんか, 〜てほしい, 〜かどうか, eingebettete
  Fragen (何時に来るか分かりません), 〜てもかまいませんか
- Szenario `n4-10-tonari`: Nachbarin Matsumoto, man bittet, ein Paket anzunehmen und zu sagen,
  ob sie am Samstag zu Hause ist. Höflich. Ziele: Bitte stellen; Grund nennen; nachfragen, ob es
  geht (〜かどうか); danken.
- Kernwörter: 荷物, 預かる, 留守, 届ける, 迷惑, お願い, 都合 …

**n4-11 Keigo am Telefon** · JF 5 Arbeit · requires `n4-07`, `n4-10`

- Kann: Ich kann mich am Telefon in einer Firma vorstellen, nach jemandem fragen und eine Nachricht hinterlassen.
- Grammatik: 謙譲語-Formeln (申します, おります, 伺います, お〜します), 尊敬語 verstehen
  (いらっしゃいます, お〜になる, おっしゃる), 〜とお伝えください, 〜させていただきます (Formel)
- Szenario `n4-11-denwa`: Empfang einer Firma in Osaka, Frau Takeda, **Keigo**. Ziele: sich mit
  Firma und Namen melden; nach Herrn X fragen; Rückruf erbitten; Nachricht hinterlassen.
- Zweites Szenario `n4-11-uketsuke`: Besuch am Empfang (Termin um 14 Uhr), Keigo.
- Kernwörter: 会社, 担当, 席を外す, 折り返し, 伝言, 少々, 失礼します …
- Passt zu Persona Emi (Ziel Keigo) und Mara (Visitenkarten).

**n4-12 Was mir passiert ist** · JF 4 Alltag, 11 Beziehungen · requires `n4-08`, `n4-09`

- Kann: Ich kann erzählen, was mir angetan wurde, was ich tun musste und was ich tun durfte.
- Grammatik: Passiv (auch Leidenspassiv: 雨に降られた), Kausativ (〜させる, 〜させてください),
  Kausativ-Passiv (〜させられた), 〜ても
- Szenario `n4-12-guchi`: Abends mit Freund Kenta im Izakaya, Freundesform, man beklagt sich über
  den Tag. Ziele: etwas Unangenehmes im Passiv erzählen; sagen, was man tun musste (させられた);
  um Erlaubnis bitten (〜させてください); trösten oder relativieren (〜ても).
- Kernwörter: 怒る, 叱る, 踏む, 盗む, 残業, 部長, 褒める, 手伝わせる …

**Optional, falls der Umfang reicht (Frage F-6):**

- n4-13 **Feste und Jahreszeiten** (JF 13): Ausdrücke für Wandel (〜くなってきた), 〜ば〜ほど (N4/N3
  Grenze), Figur Gastmutter, höflich.
- n4-14 **Einkaufen mit Reklamation** (JF 8): Umtausch, 〜てもらえますか, 〜のですが, Verkäuferin, höflich.

**Graph N4:**

```
n5-12 ─► n4-01 Erfahrungen ─┬─► n4-05 Vorhaben ─► n4-06 Reise ─┐
                            └─► n4-07 Können ─────┐            │
n5-14,16 ─► n4-02 Arzt ─► n4-08 Wetter ───────────┼────────────┼─► n4-12 Passiert
n5-15 ─► n4-03 Wohnung ───────────────────────────┼─► n4-09 Pech ┘
n5-11 ─► n4-04 Geschenke ─► n4-10 Bitten ─────────┴─► n4-11 Keigo
```

**N4-Grammatik vollständig nach Einheit** (zum Abhaken gegen den Katalog):

| Einheit | Punkte |
|---|---|
| n4-01 | たことがある, たり〜たり, し, とき, Relativsatz (Verb vor Nomen) |
| n4-02 | んです, ので, すぎる, ほうがいい, なければならない, なくてもいい |
| n4-03 | やすい/にくい, てある, ておく, ために |
| n4-04 | あげる/くれる/もらう, てあげる/てくれる/てもらう, てよかった |
| n4-05 | Volitional, ようと思う, つもり, 予定, ことにする/ことになる, までに |
| n4-06 | たら, ば, と, なら, という |
| n4-07 | Potential, ようになる, ように, ながら |
| n4-08 | そう (Aussehen), そうだ (Hörensagen), ようだ/みたい, かもしれない, でしょう, はず, って |
| n4-09 | てしまう/ちゃう, Transitiv/Intransitiv, ている (Ergebnis), のに, ばよかった |
| n4-10 | ていただけませんか, てくださいませんか, てほしい, かどうか, eingebettete Fragen, てもかまわない |
| n4-11 | 謙譲語-Formeln, 尊敬語 verstehen, お〜ください, させていただく (Formel) |
| n4-12 | Passiv, Kausativ, Kausativ-Passiv, ても |

Das sind gut 60 Punkte; mit den Wiederholungen aus N5 und kleinen Mustern (〜方, 〜始める,
〜終わる, 〜出す, 〜なさい, 〜こと/〜の als Nominalisierung) kommt der Katalog auf ~100, im
üblichen N4-Rahmen. Die kleinen Muster verteilt der Katalog als „Nebenpunkte“ auf passende Einheiten.

### 2.5 N3 bis N1: Umriss (nach 1.0)

**N3 (A2/B1), ~16 Einheiten.** Themen JF 1–13, neu: Arbeit vertieft, Bildung, Natur und Umwelt,
Kultur. Funktionen: zusammenhängend erzählen, Meinung begründen, sich beschweren, Termine ändern,
einfache Bewerbung, Register wechseln.

| Block | Einheiten (Arbeitstitel) | Grammatik-Schwerpunkte |
|---|---|---|
| Meinung | Meine Meinung · Vor- und Nachteile · Ratschlag unter Freunden | と考える, について, にとって, わけだ/わけではない, べき, ばかり |
| Erzählen | Eine Geschichte erzählen · Missverständnis · Erinnerungen | ところだ, ているところ, たばかり, うちに, ように (Wunsch), っぽい |
| Alltag lösen | Beschwerde beim Vermieter · Termin verschieben · Behördengang | ことになっている, ようにする, はずがない, ないことはない, Keigo aktiv |
| Arbeit | Bewerbungsgespräch · Erste Besprechung · Kollegen einarbeiten | 尊敬語/謙譲語 aktiv, させていただく, ことにしている, によって |
| Kultur und Natur | Feste · Umwelt im Alltag · Medien und Serien | として, に対して, をはじめ, ば〜ほど, ことから |
| Freie Aufgaben | Sprachcafé-Diskussion (frei) | Wiederholung |

**N2 (B1/B2), ~14 Aufgaben-Einheiten.** Themen neu: Gesellschaft (14), Technik (15).
Statt einer Situation mit Zielen eine **Aufgabe** mit Rolle und Standpunkt: Kundengespräch mit
Reklamation, Besprechung leiten, Nachricht zusammenfassen und kommentieren, Präsentation mit
Rückfragen, höflich ablehnen, verhandeln (Miete, Preis), Diskussion „Homeoffice ja/nein“.
Grammatik: ものの, にもかかわらず, をめぐって, わけにはいかない, ざるを得ない, に限らず, 一方で,
上で, ことになる (Folgerung), Keigo flüssig.

**N1 (B2/C1), ~10 Aufgaben-Einheiten.** Debatte mit Gegenrede, Interview geben, formelle Rede
(Hochzeit, Abschied), Beratung, Verhandlung unter Zeitdruck, Ironie und Andeutung verstehen, Kansai-
und Tōhoku-Hörproben (nur Hören). Grammatik: をもって, ならでは, にひきかえ, とあって, ずにはいられない,
かねない, いかんによらず, ものを. Ab hier zählt vor allem Wortwahl und Nuance (/LF304/ „viel“).

**Folge für das Format (später):** Ab N2 braucht ein Szenario eine Rolle für die Lernenden
(`learner_role`) und einen Standpunkt; Ziele sind dann Gesprächsfunktionen („Gegenargument nennen“),
keine Sätze. Für N5/N4 nicht nötig.

### 2.6 Vorgeschlagene Formaterweiterung (optional, rückwärtsverträglich)

Alle neuen Felder mit `#[serde(default)]`, damit `n5.json` unverändert gültig bleibt:

```json
{
  "id": "n4",
  "level": { "jlpt": "N4", "cefr": "A2" },
  "units": [
    {
      "id": "n4-01",
      "title": { "de": "Erfahrungen", "en": "Experiences" },
      "can_do": { "de": "Ich kann erzählen, was ich schon einmal gemacht habe, und nachfragen, wie es war.",
                  "en": "I can say what I have done before and ask how it was." },
      "grammar": ["〜たことがあります", "〜たり〜たりします", "〜し、〜し", "〜とき"],
      "words": ["一度", "登る", "着物", "経験", "景色", "珍しい"],
      "scenarios": ["n4-01-keiken"],
      "requires": ["n5-12"],
      "topics": [3, 13],
      "review": { "by": null, "date": null, "hash": null }
    }
  ]
}
```

- `level`: für Anzeige („orientiert an JLPT N4, CEFR A2“) und Statistik (/LF610/).
- `topics`: JF-Themennummern 1–15 für Abdeckungsprüfung (jedes A2-Thema mindestens einmal).
- `review`: Nachweis des Muttersprachler-Reviews (Abschnitt 3.3); alternativ als eigene Datei.
- Grammatik-Katalog `data/grammar/n4.json` (neu): `{ "id": "tari", "label": "〜たり〜たりします",
  "level": "n4", "explain": {de,en}, "pattern": "<Morphologie-Muster>", "examples": [...] }`.

Hinweis nebenbei: Die Szenarien tragen `"unit": "N5-03"` (groß), die Einheiten `"id": "n5-03"`
(klein). Für N4 einheitlich schreiben (Vorschlag: klein, wie die Ids).

---

## 3. Wie Inhalte geprüft werden (/LF221/)

Drei Stufen, jede als Test im Repo bzw. Gate im Release. Was automatisch geht, läuft in der CI
(Gitea), das Review ist ein Nachweis im Datenformat.

### 3.1 Niveau-Check gegen das Wörterbuch (automatisch, CI)

Für jeden japanischen Text in Curriculum und Szenarien (`opening`, `help.keyword/start/example`,
Beispielsätze, später LLM-Vorlagen):

1. Furigana-Markup `{漢字|かんじ}` entfernen, mit vibrato + UniDic zerlegen, Lemma bestimmen.
2. Lemma in JMdict nachschlagen und seine **Kaiwa-Stufe** holen (Abschnitt 4).
3. Regeln:
   - Jedes Lemma ist auf der Stufe der Einheit oder darunter, **oder** steht in `words` dieser Einheit
     oder einer Einheit, die sie (transitiv) voraussetzt.
   - Höchstens 5 % der Tokens eines Szenarios liegen darüber (/LF211/); feste Ladenfloskeln
     (いらっしゃいませ, かしこまりました) stehen in einer Ausnahmeliste je Szenario.
   - Kanji über der Kanji-Stufe der Einheit tragen Furigana (`{…|…}`), sonst Fehler.
   - Jedes Wort in `words` kommt in mindestens einem Szenario der Einheit vor.
4. Ergebnis als Bericht je Einheit (Anteil je Stufe), Fehler bricht die CI.

### 3.2 Grammatik-Lint (automatisch, CI)

- **Abdeckung:** Jeder Grammatikpunkt einer Einheit kommt in mindestens einem `help.example`
  ihrer Szenarien vor (Erkennung über das Morphologie-Muster im Katalog).
- **Vorgriff:** Kein Beispielsatz nutzt einen Grammatikpunkt einer späteren Einheit oder Stufe,
  außer er steht als Ausnahme in der Einheit (z. B. Laden-Keigo zum Verstehen).
- **Register:** `polite` → Hauptsätze enden auf です/ます (Formeln ausgenommen); `casual` → keine
  です/ます am Satzende; `keigo` → mindestens ein 尊敬語- oder 謙譲語-Merkmal je Figurensatz.
  Nutzt dieselbe Logik wie die Register-Prüfung im Gespräch (`kaiwa-core/session.rs`).
- **Form:** de und en vorhanden, Ziele 3–4, Hilfe in drei Stufen (/LF114/), `requires` ohne Zyklus
  über alle Stufen (heutiger `check()` auf den zusammengefügten Graphen).
- **QA-Personas:** Für jede neue Einheit läuft mindestens eine Persona-Reise (`kaiwa-sim`), z. B.
  Theo durch N4, Emi springt per /LF215/ bis n4-11 und scheitert dort ohne Keigo nicht still.

### 3.3 Muttersprachler-Review (manuell, Release-Gate)

- Je Einheit eine Checkliste: natürlich gesagt? Register passend zur Figur? Kulturell stimmig
  (Ryokan, Arzt, Firma)? Lesungen und Furigana richtig? Akzent der Kernwörter (Kanjium/UniDic)
  plausibel? Kann-Satz realistisch für die Stufe?
- Nachweis: `review.by`, `review.date`, `review.hash` (Hash über Einheit plus ihre Szenarien).
  Ändert sich der Inhalt, passt der Hash nicht mehr und das Release-Gate (M7) schlägt an.
- Wer reviewt, ist offen (F-05). Umfang: 16 + 12 Einheiten mit je 1–2 Szenarien, geschätzt
  1–1,5 h je Einheit, zusammen etwa 30–40 h.

---

## 4. JLPT-Zuordnung (/LD050/, L-09)

### 4.1 Befund

L-09 verlangt eine im Webarchiv bestätigte freie Lizenz. Die Seite `tanos.co.uk/jlpt/sharing/` zeigt in
drei Snapshots (2012, 2017, 2021) „Creative Commons BY“ für alles außer Verkaufsprodukten. Die
Bedingung ist erfüllt. Vorschlag: Snapshot-URLs in ADR-007 eintragen, `NOTICE` ergänzen
(„JLPT word and kanji lists © Jonathan Waller, tanos.co.uk, CC BY“), Rohlisten aus dem Archiv
oder aus `open-anki-jlpt-decks` (korrigiert, Herkunft tanos) übernehmen.

### 4.2 Trotzdem eine eigene Kaiwa-Stufe

tanos ist eine Schätzung von einer Person und für Lesen gemacht, Kaiwa braucht Sprechwortschatz.
Darum führt `kaiwa-data` je Wort eine **Kaiwa-Stufe** (`n5` … `n1`, `ausser`) mit Herkunft:

1. **Curriculum zuerst:** Wort steht in `words` einer Einheit → Stufe dieser Einheit (Quelle `curriculum`).
2. **tanos:** Stufe laut tanos (Quelle `tanos`).
3. **Häufigkeit:** sonst aus wordfreq-Zipf (Untertitel-Anteil) und JMdict-Markern (`ichi1`, `news1`,
   `nf01`–`nf48`). Schwellen werden an den Wörtern kalibriert, für die tanos eine Stufe hat
   (z. B. Median-Zipf je tanos-Stufe als Grenze).
4. **Kanji-Korrektur:** Enthält ein Wort ein Kanji über seiner Stufe, gilt die Schreibung als
   höher, die Kana-Lesung nicht (Furigana statt Abwertung). Kanji-Stufen aus tanos, sonst aus
   KANJIDIC2 `<jlpt>` (4 → N5, 3 → N4, 2 → N3 bei `<freq>` ≤ 1 000 sonst N2, 1 → N1) und `<grade>`.
5. **Abweichungen** zwischen Curriculum, tanos und Häufigkeit über eine Stufe kommen in einen
   Bericht; ein Mensch entscheidet, die Entscheidung steht als Override-Datei im Repo.

Falls tanos doch wegfällt, bleiben Schritte 1, 3, 4 und 5; die Kalibrierung erfolgt dann an den
alten KANJIDIC2-Stufen und am Curriculum. In der App heißt es immer „orientiert an JLPT N4“, nie
„JLPT-Wortschatz“, weil es keine offiziellen Listen gibt.

### 4.3 Grammatik-Stufen

Kein freier, vollständiger Grammatikkatalog ist verfügbar (tanos-Grammatiklisten sind CC BY, aber
dünn; Tae Kim ist NC; Bunpro, JLPT Sensei und Lehrwerke sind geschützt). Kaiwa schreibt den
Katalog selbst: Id, Anzeige, Stufe, eigene Erklärung de/en, Morphologie-Muster, eigene oder
Tatoeba-Beispiele. Die Stufe folgt der Einheit, in der der Punkt eingeführt wird; tanos-Grammatik
dient als Gegenprobe.

---

## 5. Abdeckung durch die Personas

| Persona | Start | Was sie im neuen Inhalt prüft |
|---|---|---|
| Hinnerk, Mara | A0 | N5 inklusive n5-13 (Nachfragen) und n5-16; keine Überforderung durch neue Einheiten |
| Jonas | N5 niedrig | Tippen statt Sprechen, Freundesform-Szenarien in N4 |
| Lena | N5 | Übergang n5-12/n5-16 → n4-01, て-Form unsicher → n5-16 als Brücke |
| Selin | Hören N4, Sprechen N5, nur Freundesform | Register-Korrektur in N4-Einheiten mit zwei Registern, n4-06 Ryokan |
| Theo | N4 | Einstufung direkt in N4, Pitch der N4-Kernwörter |
| Emi | Hören/Sprechen fließend, kein Keigo | Überspringen bis n4-10, dann n4-11 Keigo als echte Hürde |

---

## 6. Offene Fragen (Fragenpaket 8)

| Id | Frage | Vorschlag |
|---|---|---|
| F-1 | tanos-Listen übernehmen, da die CC-BY-Lizenz im Webarchiv bestätigt ist (L-09)? | **Ja**, als eine Quelle neben Häufigkeit und Curriculum (Abschnitt 4.2); Snapshot-URLs in ADR-007, Namensnennung in `NOTICE` und „Über“. |
| F-2 | N5-Lücken schließen: N5 um vier Einheiten (n5-13 bis n5-16) erweitern oder die fehlende Grammatik an den Anfang von N4 legen? | **N5 auf 16 Einheiten erweitern.** Die て-Form-Bitten und Gesprächsstrategien gehören an den Anfang, und M2 bleibt bei „N5 als Graph“. |
| F-3 | Wie heißt die Stufe in der App: „N4“, „Stufe 4“ oder „A2“? | „N4 · A2“ mit dem Hinweis „orientiert an JLPT und JF-Standard“, nie „JLPT-Kurs“; keine Bestehensversprechen. |
| F-4 | Eine Datei je Stufe mit stufenübergreifendem `requires`, oder eine große Datei? | **Eine Datei je Stufe**; der Lader fügt zusammen und prüft den Gesamtgraphen. |
| F-5 | Wie tief geht Keigo in N4? | Feste Formeln aktiv (申します, おります, 伺います, お〜ください), 尊敬語 nur verstehen; aktives Keigo in N3. Emi bekommt ihren Weg über das Überspringen. |
| F-6 | Bleibt N4 bei 12 Einheiten, oder kommen n4-13 (Feste) und n4-14 (Reklamation) für 1.0 dazu? | 12 für 1.0, die zwei als erste Inhalte von 1.1; Aufwand für Review spricht dagegen. |
| F-7 | Neue optionale Felder (`level`, `topics`, `review`) und ein Grammatik-Katalog `data/grammar/`? | **Ja**, alle optional, ein Lauf von `check()` prüft sie; ADR „Curriculum-Format v2“. |
| F-8 | Wer macht das Muttersprachler-Review (F-05), und wann? | Bezahltes Lektorat je Einheit, beauftragt zu Beginn von M6, N5 zuerst; Budget für 30–40 h. |
| F-9 | Zwei Register je Einheit (höflich und Freundesform) als Regel für N4? | Für jede zweite N4-Einheit, wo es im Leben vorkommt (Arzt nicht, Freunde ja), siehe 2.4. |
| F-10 | Sollen Kann-Sätze auf JF-Can-do-Ids verweisen? | Nein im Datensatz (Nutzungsbedingungen unklar für Weitergabe), nur die Themennummer `topics` und eigene Formulierung. |
| F-11 | Ab welcher Stufe dürfen KI-Figuren Dialekt sprechen? | Erst N2 (nur Hören), N1 aktiv verstehen; bis dahin Standardsprache (標準語). |
| F-12 | Schreibweise `unit` in Szenarien (`N5-03`) an die Einheiten-Id (`n5-03`) angleichen? | Ja, mit N4 einheitlich klein; N5 in einem kleinen PR nachziehen. |

---

## 7. Quellen

- JEES/JF: [N1–N5 Summary of Linguistic Competence](https://www.jlpt.jp/e/about/levelsummary.html),
  [CEFR reference](https://www.jlpt.jp/e/about/cefr_reference.html), [FAQ (keine Listen seit 2010)](https://www.jlpt.jp/e/faq/)
- [Wikipedia: Japanese-Language Proficiency Test](https://en.wikipedia.org/wiki/Japanese-Language_Proficiency_Test) (alte Listen, Lernstunden)
- tanos.co.uk „Sharing and copyrights“ im Webarchiv:
  [2012](https://web.archive.org/web/20121216045735/http://www.tanos.co.uk/jlpt/sharing/),
  [2017](https://web.archive.org/web/20170118202220/http://www.tanos.co.uk/jlpt/sharing/),
  [2021](https://web.archive.org/web/20210122121139/https://www.tanos.co.uk/jlpt/sharing/)
- [open-anki-jlpt-decks](https://github.com/jamsinclair/open-anki-jlpt-decks) (MIT, Daten von tanos)
- EDRDG: KANJIDIC2 und JMdict, DTD-Kommentare in den Dateien (`kanjidic2.xml.gz`, `JMdict_e.gz`), CC BY-SA 4.0
- [wordfreq](https://github.com/rspeer/wordfreq) (Daten CC BY-SA 4.0)
- [Wiktionary: Frequency lists/Japanese](https://en.wiktionary.org/wiki/Wiktionary:Frequency_lists/Japanese)
- [NINJAL BCCWJ frequency list](https://clrd.ninjal.ac.jp/bccwj/en/freq-list.html)
- Japan Foundation: [Can-do-Seite, Copyright](https://www.jfstandard.jpf.go.jp/cando/copyright/ja/render.do),
  [Nutzungsbedingungen](https://www.jfstandard.jpf.go.jp/cando/terms/ja/render.do),
  [15 topics of the JF Standard](https://marugoto.jpf.go.jp/en/assets/docs/teacher/feature/15_topics.pdf)
- Verlage (nur Struktur): [GENKI 3rd ed.](https://genki3.japantimes.co.jp/en/intro/), [QUARTET](https://quartet.japantimes.co.jp/en/about/)
