# ADR-006: Latenzbudget

**Status:** Vorgeschlagen, Spike 0.7 · **Bezug:** /LL010/, /LF101/

## Kontext

/LL010/ verlangt mit Cloud p50 ≤ 1,5 s und lokal p50 ≤ 3 s. Offen war, ab wann gemessen wird. Die Sprechpause, nach der ein Zug als beendet gilt (/LF101/, Standard 1,2 s, bei Hinnerk bis 5 s), ist eine bewusste Wartezeit und darf das Budget nicht auffressen.

## Entscheidungsvorschlag

Gemessen wird **ab erkanntem Sprechende** (nach Ablauf der eingestellten Pause) bis zum ersten hörbaren Ton der Figur. Die Pause selbst wird getrennt ausgewiesen. Push-to-Talk: ab Loslassen.

| Stufe | Cloud p50 | Lokal p50 | Technik |
|---|---|---|---|
| STT (Rest nach Sprechende) | 300 ms | 700 ms | Streaming/Chunks während des Sprechens |
| LLM bis erster Satz | 700 ms | 1 600 ms | Streaming, Satzgrenze erkennen |
| TTS erster Satz | 300 ms | 500 ms | satzweise synthetisieren |
| Puffer (Audio, IPC) | 200 ms | 200 ms | kaiwad hält Modelle warm |
| **Summe** | **1,5 s** | **3,0 s** | |

Überbrückung: Die Figur zeigt sofort „hört zu → denkt nach“. Eine kurze Rückmeldung (うん, はい) als Hörersignal gibt es optional, wenn die Antwort länger als 2 s braucht.

## Spike 0.7 (Abnahme)

Durchstich mit Mock-Daten und echten Modellen auf dem Framework 13, gemessen über 100 Züge.

## Entschieden (Fragenpaket 1)

- **F-10:** Die Referenzhardware ist das Framework Laptop 13 (Ryzen 7 7840U, 32 GB, ohne dGPU).
