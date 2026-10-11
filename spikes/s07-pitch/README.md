# Vorstudie Pitch-Akzent an echten Stimmen (/LF321/, F-09, N-03)

Lässt sich aus dem F0-Verlauf echter Stimmen ablesen, ob ein Wort oder eine Akzentphrase mit dem Akzent gesprochen wurde, den OpenJTalk vorhersagt, oder mit einem falschen Akzentkern? Nur Messung, kein App-Code (N-03). Ziel aus ADR-005: Erkennung ≥ 80 % bei höchstens 5 % Fehlalarmen auf richtig gesprochenen Wörtern.

**Kurz: Nein, nicht in dieser Form.** Bei 5 % Fehlalarmen findet die beste Prüfung 25–35 % der falschen Akzente in realistischer Größe. Selbst die OpenJTalk-Stimme, deren falsche Akzente echt gesprochen sind, erreicht nur 45 %. Damit gilt F-09: 1.0 erscheint ohne Pitch-Bewertung.

| Datei | Was |
|---|---|
| `pitch.py` | Hauptmessung: Akzentphrasen (OpenJTalk), Moren (CTC und MFA), F0 je More, Muster-Anpassung, falsche Akzente per WORLD-Resynthese, Prüfungen und Schwellen |
| `consensus.py` | Wer irrt, OpenJTalk oder die Sprecher? Sätze aus Common Voice `other`, die 6–10 Personen gelesen haben; liefert auch die Lerndaten für `templ` und `gelernt` |
| `ref.py` | Vergleich mit der Tutorstimme (OpenJTalk spricht jede Variante) und die Obergrenze „TTS als Sprecher“ |
| `form.py` | Falsche Akzente mit natürlicher Form: die Änderung, die die OpenJTalk-Stimme zwischen zwei Akzenten macht, auf die echte Stimme übertragen |
| `results/` | `pitch-cv.json` (alle Varianten), `consensus-cv.json` (jede Phrase), `pitch-ref.json` |

```sh
V=../../target/spike-venv/bin/python  # mit ../requirements.txt (neu: pyworld, praat-parselmouth, setuptools<81)
$V -I pitch.py --clips ../../target/s06-real/clips.json --model ../../target/kana-ctc-v2/model --work ../../target/pitch --out results
$V -I consensus.py --cv ../../target/cv19-ja --model ../../target/kana-ctc-v2/model --work ../../target/pitch-consensus --out results --sentences 120
$V -I ref.py --model ../../target/kana-ctc-v2/model --work ../../target/pitch --out results --template ../../target/pitch-consensus/clips.json
$V -I form.py --model ../../target/kana-ctc-v2/model --work ../../target/pitch
$V -I pitch.py --clips ../../target/s06-real/clips.json --model - --work ../../target/pitch --out results \
   --analyse-only --template ../../target/pitch-consensus/clips.json --extra ../../target/pitch/measured-form.json
```

Laufzeit insgesamt etwa 30 Minuten (RTX 3090, 20 Prozesse). Audio und Messwerte liegen unter `target/pitch*`, nicht im Repository.

## Aufbau

**Material:** die Common-Voice-19-Clips aus Spike 0.6 (`real.py`): Test-Split, eine Aufnahme je Person, CC0. 416 von 500 bleiben (74 hört das Kana-Modell nicht wie geschrieben, 9 ohne brauchbare MFA-Grenzen, 1 mit abweichender Morenzahl). Das ergibt 1 988 Akzentphrasen mit mindestens zwei Moren (Median 4 Moren).

**Messung je Clip:**

1. **Soll:** OpenJTalk (Full-Context-Labels) gibt die Akzentphrasen, ihre Morenzahl und den Akzentkern.
2. **Moren:** einmal aus dem eigenen Kana-CTC-Modell v2 (eine More reicht vom ersten Frame ihres Kanas bis zum nächsten), einmal aus den MFA-Grenzen von Spike 0.6 (vom Ende der vorigen More bis zum Ende ihres Vokals). Die CTC-Grenze liegt im Median 40 ms hinter dem Konsonantbeginn, also etwa am Vokalbeginn.
3. **F0:** Praat (Autokorrelation, Bereich je Stimme in zwei Durchgängen) und pYIN wie in ADR-005. Je More der Median in Halbtönen relativ zum Median des Clips.
4. **Anpassung:** Jede Phrase wird gegen jedes Tokio-Muster mit Kern 0 (heiban) bis n−1 gehalten. Kern n (odaka) sieht innerhalb der Phrase aus wie heiban und zählt als heiban. Verfahren:
   - **fest:** F0 = c + Stufe · Muster (H = 1, L = 0), Stufe = Akzenthub des Clips (mittlere H- minus L-More unter OpenJTalks Muster, auf 1,5–6 Halbtöne begrenzt)
   - **frei:** Stufe frei angepasst (≥ 0)
   - **Vorlage:** statt H/L die mittlere Höhe jeder Moren-Rolle (Anfangs-L, H vor dem Kern, Kern, erste More nach dem Kern, weitere …), gelernt an den 884 Clips von `consensus.py` (andere Sprecher als im Test)
   - **gelernt:** bedingtes Logit über die Kandidaten (die Anpassungen und die lokalen Abfälle als Merkmale), an denselben Clips mit OpenJTalks Kern als Ziel

**Falsche Akzente:** Freie Lerner-Aufnahmen gibt es nicht. Deshalb wird wie in Spike 0.6 an echten Stimmen manipuliert: In ein oder zwei Phrasen je Clip verschiebt WORLD die F0 der Moren, deren H/L sich ändern würde. Die Übergänge sind 25 ms lang, die Zeiten kommen von MFA.

| Art | Was |
|---|---|
| `flach` | Kern weg (akzentuiert → heiban) |
| `kern-1`, `kern+1` | Kern eine More früher oder später |
| `kopf` | heiban → atamadaka |

Gemessen in fünf Größen:

- **binär 2 und 4 Halbtöne:** ganze Moren um die Stufe verschoben
- **binär eigen:** um den Akzenthub des Clips (auf 1,5–6 begrenzt, meist 1,5; Ergebnis wie bei 2 Halbtönen, nicht in der Tabelle)
- **Form:** `form.py` überträgt die Änderung, die die OpenJTalk-Stimme selbst macht, auf die echte Stimme: mit spätem Gipfel und einem Abfall über zwei Moren, auf den Hub des Sprechers skaliert
- **Form voll:** dieselbe Änderung im Hub der OpenJTalk-Stimme, etwa doppelt so groß

**Kontrollen:** `sham` (WORLD-Resynthese, F0 unverändert) und `sham-lage` (ganze Phrase um die Stufe angehoben; das Muster bleibt, die Übergänge sind da). Jede Version wird neu ausgerichtet und gemessen.

**Prüfungen:** Alarm, wenn ein anderes Muster um mehr als eine Schwelle besser passt als das gemeinte.

- **nah:** gegen die naheliegenden Muster (Kern ±1, heiban, atamadaka)
- **Paar:** nur gegen den einen Partner eines Minimalpaars, mit einer Schwelle je Art. So will /LF321/ Hinweise geben.

Die Schwellen werden auf den richtigen Phrasen (Originale und Kontrollen) der einen Sprecherhälfte bei 5 % Fehlalarmen kalibriert und auf der anderen gezählt, dann umgekehrt.

## Ergebnis

### 1. Passt OpenJTalks Muster zu richtiger Aussprache?

Anteil der Phrasen, bei denen OpenJTalks Muster von allen n Mustern am besten passt (Zufall: 25 %). Praat, CTC-Moren:

| Verfahren | alle | n = 2 | n = 3–4 | n = 5–7 | n ≥ 8 | heiban | atamadaka | nakadaka | erste Phrase | letzte Phrase |
|---|---|---|---|---|---|---|---|---|---|---|
| fest (H/L) | 45 % | 69 % | 54 % | 35 % | 26 % | 44 % | 66 % | 30 % | 54 % | 29 % |
| Vorlage | 54 % | | | | | | | | | |
| **gelernt** | **56 %** | 69 % | 65 % | 47 % | 32 % | 60 % | 75 % | 36 % | 67 % | 37 % |

- Mit MFA-Moren sind es 39–51 %, mit pYIN 43–55 %. Die letzten 40 % jeder More statt des Medians heben die Übereinstimmung kaum (fest 47 %) und senken die Erkennung.
- Wo das H/L-Muster danebenliegt, liegt der gemessene Kern meist **eine More später** (332 Phrasen) oder **heiban wird als Kern gemessen** (351).

### 2. Wer irrt: OpenJTalk oder die Sprecher? (`consensus.py`)

Grundlage: 120 Sätze aus `other`, die je 6–10 Personen gelesen haben (884 Clips, 794 Phrasen), Verfahren `fest`.

| Maß | Wert |
|---|---|
| eine Person: OpenJTalks Muster passt am besten | 49 % |
| Mittel aller Personen eines Satzes: OpenJTalks Muster passt am besten | 52 % |
| **zwei Personen, gleicher Satz: gleiches bestes Muster** | **53 %** |
| Mehrheit der Personen = OpenJTalk | 56 % |

Klare Mehrheit (≥ 60 % der Personen einig) gibt es bei 487 Phrasen. Davon:

| Mehrheit gegenüber OpenJTalk | Phrasen |
|---|---|
| gleich | 320 (66 %) |
| Kern eine More später | 75 |
| heiban als atamadaka gemessen | 38 |
| anders | 54 |

Das mittlere Profil aller Personen zeigt die Muster deutlich (z. B. atamadaka mit 4 Moren: +1,6 +1,4 −0,8 −2,2 Halbtöne). Der Gipfel liegt aber oft noch in der More nach dem Kern, und heiban-Phrasen fallen zum Ende ab.

**Vermutung:** Die beiden häufigen Abweichungen sind Messfehler des H/L-Modells, keine Fehler von OpenJTalk. Dafür sprechen der späte Gipfel (peak delay) und die Deklination: Ein heiban, das nach der zweiten More absinkt, passt besser zu „Kopf“.

Unter „anders“ stehen echte Fälle beider Seiten:

- **OpenJTalk (Vermutung):** Nach odaka-Nomen setzt es vor の einen Kern, obwohl der Akzent dort im Tokioter Japanisch wegfällt (おとこの, むらの, しげみの). Bei 19 solchen Phrasen im Test-Split passt heiban 11-mal am besten, OpenJTalks Kern 6-mal.
- **Sprecher:** Varianten, die Wörterbücher beide kennen (たくさん, はっきり, たっぷり), Abflachung in Komposita (とりくむ), Lesefluss.

### 3. Erkennung falscher Akzente bei 5 % Fehlalarmen

Prüfung „Paar“, Fehlalarme auf den nicht gesehenen Sprechern 5–6 %. Die Kontrollen liegen genauso hoch (`sham` 5–6 %, `sham-lage` 5–7 %): Die Resynthese allein löst keine Alarme aus.

| Verfahren | binär 2 Ht. | binär 4 Ht. | Form | Form voll |
|---|---|---|---|---|
| fest, Praat, CTC | 23 % | 51 % | 24 % | 34 % |
| fest, Praat, MFA | **34 %** | **68 %** | 25 % | 35 % |
| fest, pYIN, CTC | 28 % | 54 % | 24 % | 32 % |
| Vorlage, Praat, CTC | 14 % | 29 % | 25 % | 35 % |
| gelernt, Praat, CTC | 14 % | 35 % | 27 % | 37 % |
| gelernt, Praat, MFA | 19 % | 52 % | **29 %** | **40 %** |
| Tutorstimme als Referenz (`ref.py`), frei, MFA | 13 % | 28 % | – | – |

- Prüfung „nah“ (alle naheliegenden Muster statt eines Partners) liegt 2–18 Punkte darunter.
- Je Art, gelernt, Praat, CTC, Form: `kopf` 50 %, `flach` 28 %, `kern+1` 26 %, `kern-1` 13 %.
- Nach Phrasenlänge (n = 2 / 3–4 / ≥ 5) ändert sich wenig, z. B. 25 / 25 / 24 % (fest, Praat, CTC, binär eigen).

Bei lockereren Grenzen (fest, Praat, MFA, Paar):

| Fehlalarme | binär 2 Ht. | binär 4 Ht. | Form | Form voll |
|---|---|---|---|---|
| 5 % | 34 % | 68 % | 25 % | 35 % |
| 10 % | 52 % | 79 % | 36 % | 45 % |
| 20 % | 69 % | 86 % | 48 % | 55 % |

**Obergrenze „TTS als Sprecher“ (`ref.py`):** OpenJTalk spricht jeden Satz richtig und mit jedem falschen Kern; die falschen Akzente sind also echt gesprochen, nicht resynthetisiert. Bei 5 % Fehlalarmen erkennt „Paar“ 38 % (fest), 46 % (Vorlage) und 45 % (gelernt). OpenJTalks Muster passt hier nur bei 37–48 % der Phrasen am besten, obwohl die Stimme den Akzent per Konstruktion trifft.

## Befund

- **Das Ziel aus ADR-005 ist weit verfehlt.** Bei Fehlern in der Größe, in der die Sprecher ihre Akzente selbst machen (1,5–2 Halbtöne, oder die natürliche Form), erkennt die beste Prüfung 25–34 % bei 5 % Fehlalarmen. 80 % erreicht keine Variante, auch nicht bei 20 % Fehlalarmen. Ausnahme ist ein grober, sauber binärer Fehler von 4 Halbtönen (79–86 % bei 10–20 % Fehlalarmen), mehr als doppelt so groß wie der Hub der Sprecher.
- **Das Problem ist die Messgröße, nicht nur die Stimmen.**
  - Selbst die OpenJTalk-Stimme kommt nur auf 45 %.
  - Zwei Muttersprachler desselben Satzes landen nur bei 53 % der Phrasen beim selben besten Muster.
  - **Vermutung:** F0 je More ist zu grob und zu verrauscht für die Lage des Kerns. Mikroprosodie der Konsonanten, später Gipfel, Deklination, Herabstufung nach einem Kern, Phrasenende und Leseprosodie verschieben die Höhen um ähnlich viel wie der Akzent selbst. Der Hub zwischen H- und L-Moren liegt im Median bei 1,7 Halbtönen (OpenJTalk-Stimme 3,1).
- **Kurze Phrasen gehen besser**, aber nicht gut genug. Bei 2 Moren passt OpenJTalks Muster in 69 % (gelernt), die Erkennung steigt nicht.
- **OpenJTalk als Soll ist meist plausibel.** Wo klare Mehrheiten von ihm abweichen, ist es meist das Messmodell. Echte Lücken gibt es aber, z. B. vermutlich der Kern vor の nach odaka-Nomen. In der App würde jede solche Lücke als Fehlalarm beim Lernenden ankommen.
- **Die Tutorstimme als Referenz hilft nicht.**
  - Die OpenJTalk-Stimme (HTS „mei“) spricht mit fast doppeltem Hub und deutlich spätem Gipfel. Ihr Verlauf passt schlechter zu echten Sprechern als das einfache H/L-Muster (Erkennung 7–13 % statt 23–34 % bei 2 Halbtönen).
  - **Vermutung:** Das gilt für jede einzelne Referenzstimme, die anders spricht als die Person.
- **Praat oder pYIN** ist gleichgültig (±3 Punkte). **MFA-Moren** helfen beim binären Fehler, die CTC-Moren sind für das gelernte Verfahren gleichwertig.

## Empfehlung

1. **F-09 anwenden:** 1.0 ohne Pitch-Bewertung. Keine Akzent-Hinweise, auch nicht nur für Minimalpaare: Bei 5 % Fehlalarmen würde höchstens jeder dritte falsche Akzent gemeldet, und die Hinweise wären oft an der falschen Stelle (Ort richtig in 53–69 % der Treffer).
2. **Wenn /LF321/ in 1.0 etwas zeigen soll:** den eigenen F0-Verlauf neben dem der Tutorstimme anzeigen, ohne Urteil. Das braucht nur F0 und die Moren, die es schon gibt, und kann keinen Fehlalarm erzeugen. Das wäre eine Änderung von /LF321/ (Entscheidung des Eigentümers).
3. **Für später, falls Pitch-Bewertung wieder auf den Plan kommt:**
   - ein eigenes Akzentmodell auf dem Signal statt Regeln auf F0 je More, z. B. ein Kopf auf dem wav2vec2-Encoder des Kana-Modells, trainiert auf Daten mit Akzent-Labels
   - Abnahme an den eigenen Aufnahmen (F-04) mit echten Akzentfehlern
   - **Vermutung:** Das ist ein Projekt von Wochen, kein Spike-Nachtrag.
4. **OpenJTalk-Akzent nur als Anzeige des Solls nutzen**, mit Vorsicht bei langen Phrasen. Die Lücke vor の nach odaka-Nomen und ähnliche Sandhi-Fälle gegen ein Wörterbuch prüfen (UniDic `aType`, ADR-007), bevor der Akzent dem Lernenden angezeigt wird.

## Grenzen

- **Keine echten Lernerfehler.** Die falschen Akzente sind resynthetisiert (binär oder in der Form der OpenJTalk-Stimme) und **nicht angehört** worden. Die Kontrollen zeigen nur, dass die Resynthese die Messung nicht stört, nicht dass sie natürlich klingt. Echte Lernerfehler sind vermutlich weniger sauber als die binären und eher kleiner, die Zahlen also eher zu gut.
- **Common Voice ist vorgelesene Sprache von Freiwilligen,** von monoton bis lebhaft, mit Dialekten und vielleicht auch Nicht-Muttersprachlern. Gesprochene Sätze im Dialog könnten deutlichere Akzente haben, kurze Lernersätze andere.
- **Die Wahrheit ist OpenJTalk.** Seine Fehler zählen als Fehlalarme (so wäre es in der App). Ein Wörterbuch-Soll (UniDic) wurde nicht gemessen.
- **odaka und heiban** sind innerhalb der Phrase nicht zu unterscheiden (erst an der nächsten Phrase) und zählen als gleich.
- **Moren-Grenzen:** Die MFA-Grenzen von Spike 0.6 sind teils falsch (in einem Clip liegen alle Moren in der letzten Sekunde). Die CTC-Grenzen sind Spitzen, keine Lautgrenzen.
- **Lerndaten von Vorlage und gelernt:** OpenJTalks Kerne an 884 Clips anderer Sprecher. Sie übernehmen OpenJTalks Fehler und die Verzerrungen des Materials.
- **Einige hundert Clips, eine Stimme je Person;** die Prozentwerte schwanken um einige Punkte. Die Schwellen sind je Sprecherhälfte kalibriert und dort auch gezählt, eine dritte Datenmenge gibt es nicht.
- **Wahl des Verfahrens:** Die Tabellen zeigen viele Varianten. Die beste Zeile ist an denselben Daten ausgewählt, also eher zu gut.
