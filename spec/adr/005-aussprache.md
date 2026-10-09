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
