# Spike 0.4: Sprachausgabe mit Kokoro (ADR-003)

`measure.py` misst für jede japanische Stimme von Kokoro-82M (ONNX, Apache-2.0) drei Dinge:

- **Akzent:** Die 13 Wörter der Akzent-Minimalpaare (`qa/corpus/minimalpaare.tsv`) werden mit der Partikel が gesprochen. Gemessen wird F0 pro More (pYIN, Moren gleichmäßig über den stimmhaften Teil). Verglichen wird mit dem Akzentkern aus OpenJTalks Wörterbuch (pyopenjtalk). Ein Treffer zählt nur, wenn das Hoch-Tief-Muster aller Moren stimmt.
- **Tempo:** Synthesezeit pro Satz und Echtzeitfaktor über die 50 Korpussätze.
- **Korpus:** Das Skript schreibt den synthetischen Korpus (richtig und Fehlerform, alle Stimmen) für Spike 0.3.

`arrows.py` ist ein Experiment: Hilft ein Tonpfeil ↘ hinter der Akzentmore? Kokoros Vokabular kennt die Pfeile aus dem Mandarin-Training.

```sh
python -I measure.py --models <dir> --corpus ../../qa/corpus --out out   # kokoro-v1.0.onnx, voices-v1.0.bin
python -I arrows.py --models <dir> --out out
```

## Ergebnis

Gemessen auf 4 Kernen (Intel Xeon 2,1 GHz, keine GPU), nicht auf der Referenzhardware.

| Stimme | Akzent | Akzent mit ↘ | Synthese p50 | p95 | RTF |
|---|---|---|---|---|---|
| jf_alpha | 2/13 | 5/13 | 0,62 s | 1,00 s | 0,39 |
| jf_gongitsune | 1/13 | 1/13 | 0,70 s | 1,14 s | 0,32 |
| jf_nezumi | 1/13 | 1/13 | 0,69 s | 1,78 s | 0,39 |
| jf_tebukuro | 4/13 | 5/13 | 0,64 s | 1,48 s | 0,36 |
| jm_kumo | 4/13 | 5/13 | 0,65 s | 1,15 s | 0,41 |

**Befund:**
- Kokoro trifft den Pitch-Akzent nicht. Jede Stimme legt fast dieselbe Kurve auf jedes Wort.
- Die Ursache liegt vor dem Modell: misakis japanische Lautumschrift (0.7.x) kennt keine Tonhöhe. 箸が, 橋が und 端が werden alle zu `haɕi ɡa`, also können Homophone gar nicht verschieden klingen.
- Tonpfeile helfen kaum: bestenfalls 5/13.
- Die Messung ist grob, weil die Moren gleichmäßig verteilt werden. Der Abstand zu einem brauchbaren Ergebnis ist aber zu groß, um ihn damit zu erklären.

**Tempo:** Es reicht für das Latenzbudget. Etwa 0,65 s pro Satz liegen auf dieser schwächeren CPU nahe dem Ziel von 500 ms für den ersten Satz lokal (ADR-006).

**Folgen:**
- Kokoro ist **kein Aussprache-Vorbild**. Pitch-Akzent, Shadowing und die Hörbeispiele der Korrekturen brauchen eine Stimme, deren Akzent aus dem Wörterbuch gesteuert wird.
- Kandidat ist **VOICEVOX**: OpenJTalk-Akzentphrasen, die Akzentlage ist pro Phrase einstellbar.
- VOICEVOX ließ sich hier nicht messen. Der Downloader braucht die GitHub-API, die der Proxy sperrt. Außerdem müssen vor dem Laden die Nutzungsbedingungen jeder Stimme angenommen werden, und das soll der Eigentümer tun (Fragenpaket 3).
- Bis dahin bleibt Kokoro für den synthetischen Korpus (Spike 0.3) und als Notlösung für normale Antworten der Figur ohne Lernanspruch an den Akzent.
