# Spike 0.3: lokale Spracherkennung (ADR-002)

`measure.py` lässt drei Erkenner über den synthetischen Korpus aus Spike 0.4 laufen: 50 Sätze richtig und mit den eingebauten Persona-Fehlern, Stimmen jf_alpha und jm_kumo, je ohne Störgeräusch, mit rosa Rauschen bei 10 dB und bei 5 dB. Das sind 486 Dateien, 13 Minuten Audio.

Die Maße, nach Umwandlung in Katakana (pyopenjtalk), damit die Schreibweise nicht zählt:

- **CER:** Zeichenfehlerrate auf den richtigen Sätzen.
- **Fehlertreue:** Anteil der Fehlerformen, deren Transkript näher an der Fehlerform liegt als an der richtigen Form. Der Lernfehler bleibt also für die Korrektur erhalten. ADR-002 verlangt mindestens 85 %.
- **RTF:** Rechenzeit geteilt durch Audiozeit, 4 Threads. Gemessen auf einem Intel Xeon mit 2,1 GHz und 4 Kernen, nicht auf der Referenzhardware.

```sh
python -I measure.py --corpus-audio ../s04-tts/out/corpus --corpus ../../qa/corpus \
  --whisper-cli <whisper.cpp>/build/bin/whisper-cli --models <dir> --out out \
  [--engines reazonspeech-zipformer,whisper-small] [--subset small]
```

## Ergebnis

| Erkenner | CER ohne Störgeräusch | 10 dB | 5 dB | Fehlertreue ohne Störgeräusch / 10 / 5 dB | RTF |
|---|---|---|---|---|---|
| ReazonSpeech Zipformer (sherpa-onnx, int8) | **1,3 %** | 1,6 % | 2,9 % | 91 / 85 / 81 % | **0,07** |
| whisper.cpp small | 2,0 % | 3,0 % | 4,7 % | 91 / 91 / 85 % | 3,6 |
| whisper.cpp large-v3-turbo q5 | folgt (Teilmenge) | | | | |

**Befund:**

- **ReazonSpeech ist genauer und 50-mal schneller.** Mit RTF 0,07 bleibt viel Luft für das Latenzbudget (700 ms lokal für den Rest nach dem Sprechende, ADR-006).
  - Wichtig: Ohne 0,3 s Stille vor und nach der Äußerung verschluckt der Zipformer die ersten Wörter (CER 19 %, コーヒー, ビール und これ fehlten). Die Stille gehört in die Pipeline, nicht nur in den Spike.
- **Grammatikfehler bleiben erhalten**, bei beiden Erkennern in 81–91 % der Fälle: を statt が, fehlende Zählwörter, falsche Zeitform, salopper Ton.
- **Längen- und っ-Fehler glätten beide:** ありがとございます wird zu ありがとうございます, ちょと待て zu ちょっと待って. Die Spracherkennung kann diese Fehler nicht melden. Die eigene Aussprache-Prüfung muss das leisten (ADR-005, Spike 0.6).
  - Ob der Fehler schon in der synthetischen Stimme verloren ging, zeigen erst die eigenen Aufnahmen (F-04).
- Zum Vergleich ADR-002: CER ≤ 12 % lokal erfüllt, Fehlertreue ≥ 85 % ohne Störgeräusch erfüllt, bei 5 dB knapp darunter.

**Empfehlung:** ReazonSpeech als Standard, whisper als Option (Fragenpaket 3, H-03). Lizenz: Modell und Code unter Apache-2.0, sherpa-onnx ebenfalls Apache-2.0. Beides passt zu MIT (F-02).
