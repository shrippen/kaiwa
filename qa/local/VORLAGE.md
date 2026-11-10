# Lokaler Test <datum>

Vorlage zu `spec/LOKALER-TEST.md`. Kopie als `qa/local/<datum>/notizen.md`. Je Zeile ankreuzen: **ok**, **nicht ok** (mit Notiz) oder leer lassen (nicht geprüft). Stichworte reichen.

Rechner: … · Tutor: … (Anbieter, Modell) · Kaiwa-Commit: `git rev-parse --short HEAD`

## 1. `kaiwa-sim run --live`

`live.json` und `live.txt` liegen im Ordner.

| Was `kaiwa-sim` nicht selbst prüft | ok | nicht ok | Notiz |
|---|---|---|---|
| `theo_fp`: Vorschläge zu korrekten Sätzen nur als „Auch möglich“ | | | |
| `mara_explain_de`: keine Fachbegriffe ohne Erklärung | | | |
| `mara_own_furigana`: die Lesungen über Maras Sätzen stimmen (`live.txt`) | | | |

Wenn sonst etwas auffiel:

- …

## 2. Der Tutor in der App

| Prüfregel | ok | nicht ok | Notiz |
|---|---|---|---|
| Sich vorstellen, richtig und mit Fehlern (allgemeiner Eindruck) | | | |
| `emi_explain_examples`: Fachbegriffe höchstens in Klammern, Beispielpaar | | | |
| `jonas_follow_up`: Figur fragt bei はい nach, Ziel bleibt offen | | | |
| `jonas_inappropriate`: lenkt freundlich zurück, keine Moralpredigt | | | |
| `hinnerk_german_word_help`: bietet とうだい an, bittet ums Nachsprechen | | | |
| Wiederholung als Mini-Gespräch: Frage passt, Wertung stimmt | | | |
| Latenz Senden → Antwort, fünf Züge (Sekunden) | | | |

## 3. Sprechen und Stimme

| Prüfregel | ok | nicht ok | Notiz |
|---|---|---|---|
| Mikrofon-Test in den Einstellungen: Liste, Urteil, Anhören, „Verstanden“ | | | |
| Erkennung und Übergangsstimme (allgemeiner Eindruck) | | | |
| `hinnerk_no_cutoff`: kein Zug abgeschnitten, Angebot längerer Pause | | | |
| Freisprechen über Lautsprecher (3.2a): kein fremder Satz, hört von selbst wieder zu | | | |
| Mikrofon weg und wieder da (3.2b): die Taste geht wieder | | | |
| Unterbrechen und fortsetzen (3.2c): beide Teile zählen | | | |
| Nach der Leerlaufzeit (3.2d): „Nochmal“ zählt erst, wenn es spielt | | | |
| `mara_mora_hint`: höchstens ein Moren-Hinweis, keiner bei richtigem Satz | | | |
| `hinnerk_german_word_help` (Audio): とうだい langsam gesprochen | | | |
| `mara_codeswitch` (Audio): プロデューサー hörbar gesprochen | | | |
| `theo_latency`: Median … s, 95 % … s, „denkt nach“ sichtbar | | | |

## 4. Rechner und Oberfläche

| Prüfregel | ok | nicht ok | Notiz |
|---|---|---|---|
| `lena_idle_memory`: … MB nach 10 min, Modelle entladen | | | |
| `lena_responsive`: Bildzeiten < 50 ms (`frames.log`), Abbrechen geht | | | |
| `lena_zero_egress`: nur lokale Verbindungen (`egress.log`) | | | |
| `hinnerk_font`: 160 %, nichts abgeschnitten oder überlappend | | | |

## 5.–7. VOICEVOX, Messwerte, Aufnahmen

Dateien im Ordner (`voicevox.json`, `s03/`, `s04/`), Aufnahmen per LFS. Notizen:

- …
