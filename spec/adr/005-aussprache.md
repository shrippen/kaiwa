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
- **Vorschlag (offen, Fragenpaket 5):** Punkt 3 durch den Varianten-Vergleich ersetzen; ein eigenes Kana-CTC-Modell feinjustieren (ReazonSpeech-wav2vec2-base, Common Voice CC0, Aussprache-Kana aus OpenJTalk); Abnahme an den eigenen Aufnahmen (F-04). Bis dahin gilt F-09.
