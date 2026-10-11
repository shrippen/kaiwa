# Spike 0.6: Moren-Längen (ADR-005, H-04, I-01)

Findet ein Kana-CTC-Modell Fehler bei Langvokal, っ und ん? Die Spracherkennung glättet diese Fehler (Spike 0.3), also muss eine eigene Prüfung sie melden. Ziel aus ADR-005: Erkennung ≥ 80 % bei höchstens 5 % Fehlalarmen auf richtig gesprochenen Äußerungen.

| Datei | Was |
|---|---|
| `synth.py` | Synthetischer Korpus: Minimalpaare m01–m13 (allein und im Rahmen これは＿です), alle 50 Sätze richtig, die 9 Sätze mit Moren-Fehler auch falsch. Stimmen: 5 × Kokoro, OpenJTalk „Mei“ in drei Tempi (0,8×, 1×, 1,25×). 888 Dateien. |
| `ctc.py` | CTC-Teile in numpy, als Vorlage für Rust: Wahrscheinlichkeit einer Kana-Folge (Vorwärts), Ausrichtung (Viterbi), Fehlervarianten einer Lesung. |
| `measure.py` | Prüfungen mit einem Kana-CTC-Modell, Schwellen per Kreuzvalidierung über die Stimmen. |
| `mfa_check.py` | Gegenprobe mit dem Montreal Forced Aligner (HMM, `japanese_mfa`). |
| `real.py` | Echte Stimmen (J-04): Common Voice Japanisch, Testteil (CC0), 500 Sprecher; Moren-Fehler hineingeschnitten. |
| `finetune.py` | Eigenes Kana-CTC-Modell (J-02) auf Common Voice mit Aussprache-Kana. |
| `results/` | Messwerte je Modell, jeder Versuch einzeln. |

```sh
python -I synth.py --models <kokoro-dir> --corpus ../../qa/corpus --out ../../target/s06
python -I measure.py --corpus-audio ../../target/s06 --out results --model hiragana [--noise 10]
python -I mfa_check.py --corpus-audio ../../target/s06 --mfa <env>/bin/mfa --work ../../target/s06/mfa --out results
python -I finetune.py --cv <common-voice-19-ja> --work ../../target/kana-ctc-v1 --steps 12000
python -I real.py --cv <common-voice-19-ja> --mfa <env>/bin/mfa --work ../../target/s06-real --out results --model hiragana|<model-dir>
```

Common Voice 19.0 Japanisch kommt aus dem CC0-Spiegel `fsicoli/common_voice_19_0` auf Hugging Face (`transcript/ja/*.tsv`, `audio/ja/<split>/*.tar`). Daten und Modelle liegen unter `target/`, nicht im Repository.

MFA kommt aus conda-forge (`montreal-forced-aligner` 3.4.3, ohne root über micromamba), dazu `mfa model download acoustic|dictionary japanese_mfa`.

## Aufbau

Ein **Versuch** ist eine Aufnahme und die Form, die die Person sagen wollte. Jede Aufnahme eines Minimalpaars ergibt zwei Versuche: gemeint wie gesprochen (richtig) und gemeint wie das andere Wort (Fehler). Die Sätze ergeben 400 richtige und 72 falsche Versuche. Insgesamt sind es 816 richtige und 488 falsche Versuche.

Gemessene Prüfungen:

- **Dauer** (ADR-005, Punkt 3): Die gemeinte Lesung wird ausgerichtet. Jede More bekommt ihre Länge relativ zur mittleren More der Person; ein zu kurzes langes Element (Langvokal, っ, ん vor Vokal) oder eine zu lange kurze More gilt als Fehler. Einmal aus der CTC-Ausrichtung, einmal aus MFA, dort auch je Lautklasse (`Vː`, `Cː`, `ɰ̃`, kurzer Vokal, stimmloser Konsonant).
- **Varianten-Vergleich:** Aus der gemeinten Lesung entstehen alle Formen, die ein Moren-Fehler ergäbe (おばあさん → おばさん, きって → きて, げんいん → げにん, びょういん → びよういん, auch umgekehrt). Liegt die CTC-Wahrscheinlichkeit einer Variante um mehr als eine Schwelle über der gemeinten Form, ist das ein Fehler, und die Variante nennt die Stelle. Gemessen
  - mit einer Schwelle für alles,
  - mit einer Schwelle **je Fehlerart** (`long-`, `long+`, `q-`, `q+`, `n-`, `n+`, `y`), gemeinsam auf 5 % Fehlalarme kalibriert,
  - mit **Referenz-Normierung:** Von jeder Variante wird abgezogen, wie stark das Modell sie schon bei richtiger Aussprache desselben Textes durch andere Stimmen hört. In der App wäre die Referenz die Tutorstimme, die den gemeinten Satz vorspricht.
- **MFA-Aussprachewahl:** Das Wörterbuch enthält die gemeinte Aussprache und alle Varianten mit kleinerem Vorrang; MFA wählt.

Die Schwellen werden für jede Stimme auf den anderen Stimmen kalibriert (OpenJTalks drei Tempi zählen als eine Stimme). Die Fehlalarmrate der Tabelle ist also an Daten gemessen, die die Schwelle nicht gesehen haben, und kann über 5 % liegen.

## Ergebnis

Erkennung bei Fehlalarmen, ohne Störgeräusch, alle Stimmen. „Rahmen“ = Minimalpaar im Satz これは＿です; „Sätze“ = Korpussätze. Einzelwörter ohne Satz sind für alle Verfahren schwer (≤ 42 %) und in der App selten.

| Verfahren | Rahmen | Sätze | alle Versuche |
|---|---|---|---|
| Dauer, CTC | 5 % bei 0 % | 3 % bei 6,8 % | 4 % bei 3,6 % |
| Dauer, MFA je Lautklasse | 10 % bei 2,4 % | 12 % bei 9,5 % | 10 % bei 5,6 % |
| MFA-Aussprachewahl | 23 % bei 4,3 % | 11 % bei 3,0 % | 19 % bei 4,4 % |
| Varianten, eine Schwelle | 62 % bei 2,4 % | 50 % bei 3,8 % | 47 % bei 5,3 % |
| **Varianten je Fehlerart** | **76 % bei 4,3 %** | 51 % bei 5,2 % | 58 % bei 6,6 % |
| Varianten mit Referenz | 86 % bei 7,7 % | 68 % bei 7,0 % | 60 % bei 7,5 % |

Nur OpenJTalk (die Stimme, die die Fehler sicher so spricht wie geschrieben): Varianten mit Referenz erkennen **alle** Satzfehler (27 von 27) bei 6,7 % Fehlalarmen, im Rahmen 91 % bei 12,8 %.

Modelle (Varianten je Fehlerart, alle Versuche):

| Modell | Lizenz | Ausgabe | Erkennung |
|---|---|---|---|
| `vumichien/wav2vec2-large-xlsr-japanese-hiragana` | Apache-2.0, trainiert auf Common Voice und **JSUT** | Hiragana je Zeichen | **58 %** bei 6,6 % |
| `reazon-research/japanese-wav2vec2-large-rs35kh` | Apache-2.0 | Teilwörter mit Kanji | 11 % bei 5,8 % |
| `reazon-research/japanese-wav2vec2-base-rs35kh` | Apache-2.0 | Teilwörter mit Kanji | 12 % bei 5,1 % |
| `facebook/wav2vec2-xlsr-53-espeak-cv-ft` | Apache-2.0 | Phoneme (IPA) | nicht gemessen: kennt Länge und っ nicht verlässlich (とう → `t o`, きって → `k i t e`) |

Mit rosa Rauschen bei 10 dB fällt das Hiragana-Modell auf 52 % im Rahmen und 60 % in Sätzen (je Fehlerart, Fehlalarme 3,8 und 5,8 %).

Trifft der Varianten-Vergleich, nennt er in 88 % der Fälle die richtige Stelle (die Variante liegt näher an dem, was gesprochen wurde).

## Befund

- **Das Ziel ist nicht erreicht.** Das beste Verfahren erkennt im Satzrahmen 76 % bei 4,3 % Fehlalarmen, über alle Versuche 58 %.
- **Die Dauer-Prüfung aus ADR-005 (Punkt 3) trägt nicht.**
  - Die CTC-Ausrichtung gibt Spitzen, keine Lautgrenzen; die Dauern sagen nichts.
  - MFA misst die Dauern gut (OpenJTalk おばさん/おばあさん: `aː` 69 gegen 139 ms, きて/きって: `tː` 49 gegen 110 ms). Aber die natürliche Streuung ist größer als der Fehler: Phrasenende, Lehnwörter (らーめん) und gelegentliche Ausrichtungsfehler machen richtige Langvokale so kurz wie falsche.
- **Der Varianten-Vergleich trägt**, und er nennt die Stelle gleich mit. Er braucht keine Dauerschwellen und keine Sprechgeschwindigkeit. Das Tempo ändert wenig: OpenJTalk im Rahmen und in Sätzen erkennt bei 0,8× 63 %, bei 1× 71 %, bei 1,25× 69 %.
- **Das Hiragana-Modell hat ein Vorwissen über Schreibungen.**
  - Richtig gesprochenes げんいん, きんえん oder たんい hört es als げにん, きねん oder たに (Δ 30–40 gegen sonst 15), bei jeder Stimme.
  - Eigene Schwellen je Fehlerart und die Referenz-Normierung fangen das zum Teil ab.
  - Ein Modell, das auf Aussprache (ー, っ, ん) statt auf Schreibung trainiert ist, sollte es nicht haben.
- **Die ReazonSpeech-Modelle helfen nicht,** obwohl sie auf 35 000 Stunden trainiert sind. Sie schreiben Kanji, eine reine Kana-Folge ist für sie keine sinnvolle Hypothese.
- **Kokoro taugt nicht als Wahrheit für Moren.**
  - Es spricht げんいん als げにん (jm_kumo) und ゆうびん als ゆびん, aber ありがと als ありがとう.
  - Ein Teil der gemessenen „Fehlalarme“ und „Verpasser“ bei Kokoro sind Fehler der Stimme.
  - OpenJTalk spricht die Längen wie geschrieben; die eigenen Aufnahmen (F-04) bleiben die eigentliche Probe.
- **Lizenz:** Das Hiragana-Modell steht unter Apache-2.0, wurde aber auch auf JSUT trainiert. JSUTs Audio ist nur für Forschung und persönliche Zwecke frei, kommerzielle Nutzung nach Anfrage. Ob das für ausgelieferte Gewichte gilt, ist offen (Fragenpaket 5).

## Empfehlung

1. ADR-005 Punkt 3: **Varianten-Vergleich statt Dauerschwellen**, Schwellen je Fehlerart.
2. **Ein eigenes Kana-CTC-Modell feinjustieren:** `reazon-research/japanese-wav2vec2-base` (vortrainiert, Apache-2.0) auf Common Voice Japanisch (CC0) mit Aussprache-Kana aus OpenJTalk als Ziel (ー, っ, ん wie gesprochen). Das löst die Lizenzfrage und das Schreibungs-Vorwissen zugleich. Lokal auf der GPU ein paar Stunden.
3. **Eigene Aufnahmen (F-04)** vor der Abnahme: erst sie zeigen, ob die Zahlen mit einer echten Stimme halten.
4. Bis dahin gilt F-09: Moren-Hinweise nur dort, wo die Fehlalarm-Grenze hält, also im Satz und mit den Schwellen je Fehlerart.
5. Julius als zweite Gegenprobe ist nach dem MFA-Befund verzichtbar: Beide HMM-Aligner messen Dauern, und die Dauern sind das Problem, nicht ihre Messung.

## Echte Stimmen und eigenes Modell (Fragenpaket 5: J-02, J-04)

Nach J-04 gibt es keine eigenen Aufnahmen. Frei lizenzierte Aufnahmen von *Lernenden* mit Moren-Fehlern gibt es nicht; NHK-Nachrichten sind geschützt. `real.py` nimmt deshalb echte Muttersprachler aus Common Voice (Testteil, CC0) und schneidet die Fehler hinein:

- 500 Sprecher, je ein Satz. Nur Sätze, deren Lesung das Hiragana-Modell hört (Zeichenfehler ≤ 20 %); die Auswahl steht in `clips.json` und gilt für alle Modelle.
- MFA richtet die Laute aus. Dann wird je Satz eine Stelle geändert:
  - **long-:** Langvokal auf den mittleren kurzen Vokal des Satzes gekürzt.
  - **q-:** っ auf den mittleren kurzen Konsonanten gekürzt.
  - **long+:** kurzer Vokal auf das Doppelte gedehnt (ganze Grundperioden wiederholt).
  - **q+:** Verschlusspause vor k, t, p um eine More verlängert.
- **Kontrolle:** jede Aufnahme unverändert und mit einem Schein-Schnitt (eine Grundperiode aus einem kurzen Vokal). Der Schein-Schnitt löst nicht öfter Alarm aus als die unveränderte Aufnahme; die Prüfung reagiert also nicht auf Schnittspuren.
- ん vor Vokal ändert einen Laut, keine Länge, und bleibt hier draußen.
- Schwellen auf der einen Hälfte der Sprecher kalibriert, auf der anderen gezählt (und umgekehrt).

Eigene Modelle (`finetune.py`): `reazon-research/japanese-wav2vec2-base` bzw. `-large` (Apache-2.0), feinjustiert auf Common Voice Japanisch (CC0, ohne die Test-Sprecher) mit Aussprache-Kana aus OpenJTalk.

| Modell | Training | Zeichenfehler (40 zurückgehaltene Sprecher) |
|---|---|---|
| v0 | base, `train` + `dev`, 16 h, 3000 Schritte | 32 % |
| v1 | base, dazu `other` mit Zustimmung, 49 h, 12 000 Schritte | 24,7 % |

**Echte Stimmen**, Erkennung bei Fehlalarmen (Varianten-Vergleich):

| Modell | alle Fehlerarten | nur Kürzungen (long-, q-) | davon long- / q- |
|---|---|---|---|
| Hiragana (vumichien) | 18,5 % bei 5,1 % | 44,7 % bei 4,8 % | 42 / 48 % |
| v0 | 15,4 % bei 4,5 % | **63,8 % bei 5,5 %** | 68 / 59 % |
| v1 | 16,2 % bei 5,1 % | **62,3 % bei 6,0 %** | 64 / 60 % |

- **Dehnungen und eingefügte っ erkennt der Varianten-Vergleich kaum** (long+, q+ unter 18 % bei jedem Modell). Sie teilen sich aber das Fehlalarm-Budget mit den Kürzungen. Prüft man nur die Kürzungen, steigt deren Erkennung auf etwa das Doppelte.
- **Referenz-Normierung** (OpenJTalk spricht den gemeinten Satz) hilft dem Hiragana-Modell (alle Arten 31 % statt 19 %), den eigenen Modellen kaum: Sie haben das Schreibungs-Vorwissen nicht, das die Referenz abzieht.
- **„Gestützt“** (nur Varianten, denen auch die freie Erkennung näher ist) senkt die Fehlalarme auf 1,6–3,7 %, kostet aber mehr Erkennung, als es spart.
- **Synthetisch** ist v1 viel besser als das Hiragana-Modell: im Satzrahmen 83 % bei 3,8 % (Hiragana 62 %), alle Versuche 65 % bei 5,1 % (Hiragana 47 %). Auf echten Stimmen ist v1 nicht besser als v0; Common Voice ist mit Laptop- und Handymikrofonen aufgenommen und schwerer als jede Synthese.

**App:** `kaiwa-speech::mora` (Rust) rechnet dasselbe über ONNX Runtime. `export_onnx.py` exportiert das Modell (fp32 378 MB, exakt wie PyTorch; int8 nur für MatMul 122 MB, Varianten-Werte im Median 0,04 daneben, auf der CPU dieses Rechners mit 24 Kernen 25-mal schneller als Echtzeit). Welche Fehlerarten geprüft werden, steht in `thresholds.json` neben dem Modell.
