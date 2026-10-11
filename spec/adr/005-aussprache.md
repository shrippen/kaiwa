# ADR-005: Aussprache-Pipeline

**Status:** Vorgeschlagen, Spike 0.6 · **Bezug:** /LF320/, /LF321/, /LF330/

## Kontext

Bewertet werden sollen Moren-Struktur (Langvokale, っ, ん, Entstimmung, Rhythmus) und Pitch-Akzent. Fehlalarme (Theo) sind gefährlicher als übersehene Fehler.

## Entscheidungsvorschlag

1. **Soll-Form:** Die erwartete Äußerung kommt aus dem STT-Transkript, das an die Absicht angeglichen ist (Analyse, ADR-004). Daraus entstehen die Kana-Lesung und die Moren mit Akzentkern (UniDic `aType`, ADR-007).
2. **Alignment:** Ein CTC-Phonemmodell (ONNX) richtet die Moren an der Aufnahme aus. Ergebnis: Dauer pro More.
3. **Moren-Prüfung:** Dauerverhältnisse relativ zur Sprechgeschwindigkeit der Person. Langvokal ≥ 1,6× einer kurzen More, っ als Verschlusspause ≥ 0,8 More. Die Schwellen werden im Spike kalibriert.
4. **Pitch:** F0 per pYIN (in Rust), normiert auf die Stimmlage der Person. Bewertet wird nur die Lage des Abfalls (Akzentkern), keine absolute Höhe. Hinweise gibt es nur bei bedeutungsunterscheidenden Paaren oder bei Feedback „viel“ (/LF321/).
5. **Optional Cloud:** Azure Pronunciation Assessment (ja-JP) als zweite Meinung, wenn Cloud erlaubt ist.

## Spike 0.6 (Abnahme)

- **Material:** 20 Minimalpaare, gesprochen richtig und falsch (synthetisch mit gezielt verändertem Akzent und echt).
- **Ziel:** Erkennung ≥ 80 %, Fehlalarme ≤ 5 % bei richtig gesprochenen Wörtern.

## Entschieden (Fragenpaket 1)

- **F-09:** Aussprache-Feedback bleibt **Soll** für 1.0. Scheitert der Spike, erscheint 1.0 ohne Pitch-Bewertung; Moren-Prüfung und Nachsprechen kommen dann so weit, wie sie die Fehlalarm-Grenze halten.

## Entschieden (Fragenpaket 3)

- **H-04:** Spike 0.6 beginnt jetzt mit den Moren-Längen: Langvokal, っ und ん an den synthetischen Fehlern. Beide Erkenner glätten diese Fehler (Spike 0.3), also muss die eigene Prüfung sie finden. Der Pitch-Teil folgt mit einer Stimme, die den Akzent trifft (H-01), und mit den eigenen Aufnahmen (F-04).

## Entschieden (Fragenpaket 4)

- **I-01:** Die Moren-Längen misst ein eigener Aligner mit einem Kana-CTC-Modell (wav2vec2, Ausgabe Hiragana) über ONNX Runtime: Zeiten pro Kana aus den CTC-Wahrscheinlichkeiten. Julius (Segmentation-Kit) und der Montreal Forced Aligner laufen im Spike als automatische Gegenprobe mit: Wo sie sich um mehr als eine Toleranz unterscheiden, meldet der Spike den Satz. Die Lizenz des CTC-Modells wird vor der Wahl geprüft.

## Befund Spike 0.6, Moren-Längen (2026-10-11)

Messung und Zahlen in [`spikes/s06-moren`](../../spikes/s06-moren/README.md). Kurz:

- **Ziel nicht erreicht.** Bestes Verfahren: Erkennung 76 % bei 4,3 % Fehlalarmen im Satzrahmen, 58 % über alle Versuche (synthetisch, Kokoro und OpenJTalk).
- **Punkt 3 (Dauerverhältnisse) trägt nicht**, weder aus der CTC-Ausrichtung noch aus MFA: Die natürliche Streuung der Längen ist größer als der Fehler.
- **Was trägt, ist der Varianten-Vergleich:** die CTC-Wahrscheinlichkeit der gemeinten Kana gegen die Formen, die ein Moren-Fehler ergäbe (おばあさん/おばさん), mit einer Schwelle je Fehlerart. Er nennt die Stelle gleich mit (88 %).
- **Modell:** Nur `vumichien/wav2vec2-large-xlsr-japanese-hiragana` gibt Kana je Zeichen. Es hört ん vor Vokal grundsätzlich als n-Reihe (Schreibungs-Vorwissen) und wurde auch auf JSUT trainiert (Audio nur nicht-kommerziell frei). Die ReazonSpeech-Modelle schreiben Kanji und helfen nicht.
- Vorschläge gingen als Fragenpaket 5 an den Eigentümer, Antworten unten.

## Entschieden (Fragenpaket 5)

- **J-01:** Die Moren prüft der **Varianten-Vergleich** (Schwelle je Fehlerart). Die **Dauer bleibt als Zusatzsignal**; sie entscheidet nicht allein, weil sie im Spike nichts trennte. Ihre Quelle in der App ist die Ausrichtung des eigenen Modells (J-02), MFA wird nicht ausgeliefert.
- **J-02:** **Eigenes Kana-CTC-Modell:** `reazon-research/japanese-wav2vec2-base` (Apache-2.0), feinjustiert auf Common Voice Japanisch (CC0) mit Aussprache-Kana aus OpenJTalk (ー, っ, ん wie gesprochen). Das gemessene Hiragana-Modell (JSUT im Training) wird nicht ausgeliefert.
- **J-03:** Julius entfällt; MFA war die Gegenprobe.
- **J-04:** Der Eigentümer hat keine richtige Aussprache als Referenz. Statt eigener Aufnahmen: **echte Stimmen aus frei lizenzierten Korpora**, die Fehler hineingeschnitten (Common Voice Test, CC0; `spikes/s06-moren/real.py`). Nachrichtensendungen (NHK) sind geschützt und kommen nicht in Frage; ReazonSpeech (Fernsehen, nur nach Art. 30-4 des japanischen Urheberrechts) erst nach Annahme der Bedingungen durch den Eigentümer.
- **J-05:** Moren-Hinweise sind **an, nur in Sätzen**, mit den Schwellen je Fehlerart (F-09), sobald das eigene Modell (J-02) in der App ist.

## Entschieden (Fragenpaket 6)

- **K-01:** Gemeldet werden nur **Kürzungen**: Langvokal zu kurz, っ weggelassen. Dehnungen und eingefügte っ erkennt kein Verfahren brauchbar (Echt-Stimmen-Test unter 18 %).
- **K-02:** Die Hinweise erscheinen, sobald im Echt-Stimmen-Test (`spikes/s06-moren/real.py`) insgesamt mindestens 60 % und je gemeldeter Art mindestens 55 % erkannt werden, bei höchstens 5 % Fehlalarmen. Die 80 % oben bleiben Ziel für 1.0. Der Hinweis ist vorsichtig formuliert und zählt nicht als Fehler.
- **K-03:** ReazonSpeech kommt als Trainingsdaten dazu; der Eigentümer nimmt die Bedingungen an (Art. 30-4) und hinterlegt ein Hugging-Face-Token. Ergebnis (v3, ab v2 mit ReazonSpeech `small`, 110 h): nicht besser als v2 (echte Stimmen, nur Kürzungen 59,3 % bei 5,1 % gegen 61,2 % bei 4,8 %). Ausgeliefert wird v2, das nur Common Voice (CC0) gesehen hat.
- **K-04 bis K-06** gelten für ein eigenes Modell: Hosting auf Hugging Face unter `shrippen`, Lizenz Apache-2.0 mit Eintrag in `NOTICE`, auf dem Desktop v2 (large), für Android später ein base-Modell. Nach der Antwort wurden zwei fertige, auf Aussprache trainierte Modelle gefunden (`prj-beatrice/japanese-hubert-base-phoneme-ctc-v4`, `kizuna-intelligence/hiragana-parakeet-tdt-ctc-0.6b-ja-beta`); gemessen fanden sie auf echten Stimmen weniger Kürzungen als v2 (prj-beatrice 52 %, Parakeet 39 %, v2 61 %; `spikes/s06-moren/README.md`). Es bleibt beim eigenen Modell v2.

## Umsetzung in der App (2026-10-11)

Nach einem gesprochenen Zug prüft die App die Aufnahme im Hintergrund gegen die Lesung des erkannten Satzes (`kaiwa_speech::mora::Checker::hints`, `G2p::pronounced_words`). Die Antwort der Figur wartet darauf nicht. Gezeigt wird höchstens ein Hinweis je Zug: das Wort, gemeint und gehört („どーして → どして?“), vorsichtig formuliert, ohne Schweregrad, nicht als Fehler gezählt. Nur Kürzungen (K-01), nur ab sechs Moren (J-05), die Aufnahme bleibt im Speicher (/LF712/). Fehlt das Modell, nennen die Einstellungen Quelle, Größe und Lizenz.

**Gemessen auf dem App-Weg** (`crates/kaiwa-speech/examples/mora_hints.rs`, dieselben 500 Common-Voice-Sprecher wie `real.py`; gezählt wird nur ein Hinweis der richtigen Art):

| Lesung | Fehlalarme | Langvokal zu kurz | っ weggelassen | zusammen |
|---|---|---|---|---|
| OpenJTalk (wie `real.py`) | 5,2 % | 58,8 % | 57,1 % | 58,1 % |
| App-G2P vorher | 6,9 % | 58,6 % | 57,0 % | 57,9 % |
| App-G2P mit drei Lese-Regeln | 5,6 % | 58,3 % | 57,0 % | 57,7 % |
| dazu Marge 0,3 (`APP_MARGIN`) | **4,8 %** | **55,6 %** | **56,1 %** | **55,8 %** |
| dasselbe über 48 kHz (Mikrofon-Weg, Resampling) | 5,0 % | 55,3 % | 57,4 % | 56,2 % |

- Die App liest anders als OpenJTalk, und das kostet Fehlalarme. Drei Regeln im G2P bringen die Übereinstimmung mit OpenJTalk von 85,8 auf 90,2 %: 言う bleibt いう (という), das Willens-う dehnt das Verb (帰ろう → かえろー), 一人 und 二人 lesen ひとり und ふたり. Außerdem wird ヴ zu ぶ, wie bei OpenJTalk.
- **K-02 ist auf dem App-Weg nicht ganz erreicht:** je Art über 55 %, zusammen 55,8 % statt 60 % (bei 4,8 % Fehlalarmen). `real.py` zählte 61,2 %, aber mit OpenJTalks Lesung und ohne die Grenze von einem Hinweis je Zug.
- Darum sind die Hinweise vorerst eine **Vorschau, standardmäßig aus** (Einstellung „Aussprache-Hinweise“). Ob sie so an sein sollen, entscheidet der Eigentümer (Morgenbericht 2026-10-12).

## Vorstudie Pitch-Akzent (2026-10-11, N-03)

Nur Messung an echten Stimmen, kein App-Code (`spikes/s07-pitch/README.md`). 416 Common-Voice-Sprecher aus Spike 0.6, 1 988 Akzentphrasen; Soll-Akzent aus OpenJTalk, Moren aus dem Kana-CTC-Modell und MFA, F0 mit Praat und pYIN. Falsche Akzente per WORLD-Resynthese in echte Stimmen geschnitten (Kern weg, eine More früher oder später, Kopfakzent), mit zwei Schein-Kontrollen.

- **Bei 5 % Fehlalarmen findet die beste Prüfung 25–35 % der falschen Akzente** in realistischer Größe; Ziel sind 80 %. Erst bei Stufen von 4 Halbtönen (weit über dem eigenen Hub der Sprecher von etwa 1,7) sind es 51–68 %.
- Selbst wenn die OpenJTalk-Stimme selbst spricht (falsche Akzente also echt gesprochen), sind es 38–46 %. Gegen die Kontur der Tutorstimme verglichen 7–13 %.
- OpenJTalks Muster passt bei richtig sprechenden Muttersprachlern nur in 45 % der Phrasen am besten (gelernt 56 %, Zufall 25 %); zwei Muttersprachler, die denselben Satz lesen, kommen in 53 % auf dasselbe beste Muster. Die Messung streut mehr als der Fehler.
- Vermutlich eine Lücke in OpenJTalk (wenige Fälle, Vermutung): Kern vor の nach endbetonten Nomen (おとこの, むらの).
- **Folge: F-09 greift.** 1.0 erscheint ohne Pitch-Bewertung, auch nicht für Minimalpaare. Ob /LF321/ stattdessen nur die eigene Kurve neben der Kurve der Figur zeigt (ohne Urteil, kann keinen Fehlalarm machen) und ob Pitch nach 1.0 mit eigenem Akzentmodell und eigenen Aufnahmen (F-04) zurückkommt, entscheidet der Eigentümer (Morgenbericht 2026-10-12).
