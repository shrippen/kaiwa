# Sprachkorpus für die Spikes 0.3 und 0.6

Entscheidung F-04: Der Korpus ist gemischt, aus synthetischen Stimmen (mit den Fehlern der Personas) und eigenen Aufnahmen. Die eigenen Aufnahmen zeigen, ob Spracherkennung und Aussprache-Prüfung mit einer echten Stimme so arbeiten wie mit der Synthese.

| Datei | Inhalt |
|---|---|
| `saetze.tsv` | 50 Sätze (N5/N4), jeweils richtig und, wo es eine gibt, in der Fehlerform. `fehler_id` verweist auf den eingebauten Fehler einer Persona (`qa/personas/*.json`). s33–s43 gibt es nur richtig, sie messen Fehlalarme (/LL110/). |
| `minimalpaare.tsv` | 20 Paare für Länge (Langvokal), っ, ん und Pitch-Akzent (Spike 0.6). Die Akzentangaben (Zahl = Lage des Akzentkerns) werden vor der Messung gegen UniDic `aType` geprüft (ADR-007). |

## Einsprechen

1. Ruhiger Raum. Einmal mit dem Laptop-Mikrofon, einmal mit Headset, wenn möglich.
2. **Sätze:** Jeden Satz zweimal sprechen, einmal `richtig`, einmal `fehlerform`. Bei Akzent- und Aussprachezeilen (gleicher Text) den Fehler aus `hinweis` absichtlich machen. Bei `—` nur die richtige Form.
3. **Minimalpaare:** a und b je dreimal, in normalem Tempo.
4. Format: WAV, mono, 48 kHz, 16 bit. Dateiname `<id>_<richtig|fehler>_<mic>.wav`, z. B. `s04_fehler_laptop.wav`, bei Paaren `m14_a_1_headset.wav`.
5. Ablage: `qa/corpus/audio/` in Git LFS (G-03).

Die eigenen Aufnahmen sind Stimmdaten. Sie bleiben im privaten Repository und werden nie an einen Cloud-Dienst geschickt (Cloud-STT wird ohnehin nicht gemessen, F-03).
