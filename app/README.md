# Kaiwa (App)

Kirigami-App mit der GUI aus Kante 1.25 über `kaiwa-core` und `kaiwa-store`. Stand M2:

- **Heute:** Fortsetzen einer unterbrochenen Sitzung, Tagesziel mit Serie, das nächste Szenario auf dem Lernpfad und die fälligen Wiederholungen.
- **Lernpfad:** zwölf N5-Einheiten mit Zuständen in Zeichen und Wort.
- **Gespräch:** gesprochen (Push-to-Talk, ReazonSpeech) oder getippt, Hilfe in drei Stufen, Korrekturen als Hint-Cards mit Diff. Die Figur spricht mit der Übergangsstimme (Kokoro, H-02). Die Bilanz am Ende sagt, was sich für den Lernstand ändert.
- **Wiederholen:** fällige Wörter und eigene Korrekturen zum Nachsprechen oder Tippen (FSRS).
- **Profil:** der Lernstand in Worten, mit „Kann ich schon“ und „Zu schwer“, die aktuelle Schwierigkeit und zwölf Wochen Statistik.
- **Einstellungen:** Tagesziel, Feedbackmenge, Schrift, Sprechpause, Punkte und Serie, Pause, Erinnerung, Stil, Sprache, „Was verlässt das Gerät?“, Export und Löschen.

Das Profil ist eine Datei: `~/.local/share/kaiwa/profiles/default.kaiwa` (oder `$KAIWA_DATA`). Unterbrochene Gespräche gehen nach dem Neustart weiter.

| Heute (Kante) | Heute (Leinen) | Heute (System) |
|---|---|---|
| ![](shots/start-kante.png) | ![](shots/start-light.png) | ![](shots/start-system.png) |

| Lernpfad | Gespräch mit Bilanz | Wiederholen |
|---|---|---|
| ![](shots/path-kante.png) | ![](shots/talk-kante.png) | ![](shots/review-kante.png) |

| Profil | Einstellungen |
|---|---|
| ![](shots/profile-kante.png) | ![](shots/settings-kante.png) |

Einstufung per Gespräch (/LF200/): erster Start, eine Stufe, Ergebnis (deutsch und englisch).

| Erster Start | Stufe 1 von 4 | Ergebnis | Result |
|---|---|---|---|
| ![](shots/start-new-kante.png) | ![](shots/place-kante.png) | ![](shots/place-result-kante.png) | ![](shots/place-result-en.png) |

## Bauen und starten

```sh
# Qt ≥ 6.6, Kirigami 6, Rust; Kante wird nicht gebraucht (einkompiliert)
cargo run -p kaiwa-app                    # Demo: aufgezeichnete Antworten (qa/fixtures)
```

## Modelle

Spracherkennung, Stimme und Moren-Prüfung laufen lokal mit Modellen, die nicht mit der App kommen. Die Einstellungen laden sie unter „Modelle“, jedes erst nach einem Klick, mit Zweck, Lizenz, Quelle und Größe vorher (F-08, /LL401/). Jede Datei wird über ihre SHA-256 geprüft, bevor die App sie benutzt; ein abgebrochener Download hinterlässt nur eine `.part`-Datei.

| Modell | Wofür | Lizenz | Download | auf der Platte |
|---|---|---|---|---|
| ReazonSpeech (sherpa-onnx) | Spracherkennung | Apache-2.0 | 713 MB (Archiv, nur die int8-Dateien bleiben) | 160 MB |
| Kokoro-82M (kokoro-onnx, int8) | Übergangsstimme der Figur | Apache-2.0 | 121 MB | 121 MB |
| kaiwa-kana-ctc (huggingface.co/shrippen) | Moren-Hinweise | Apache-2.0 | 356 MB | 356 MB |
| kaiwa-data 0.1.0 (github.com/shrippen) | Wörterbuch: Wort nachschlagen | CC BY-SA 4.0 | 26 MB | 84 MB |

Ohne Oberfläche: `cargo run -p kaiwa-speech --example fetch_models` (alle fehlenden, oder Namen als Argumente). Die Modelle liegen in `~/.local/share/kaiwa/models`, ein anderer Ort geht über `KAIWA_MODELS=/pfad`.

## Sprechen

Die Spracherkennung (ReazonSpeech über sherpa-onnx, ADR-002) läuft lokal. Fehlt das Modell, zeigt der Sprechknopf den Fehlerzustand und auf Klick, was zu tun ist; Tippen geht weiter. Sprechen: Leertaste oder Knopf halten, loslassen schickt den Satz.

| Gesprochen (Demo) | Ohne Modell |
|---|---|
| ![](shots/speech-kante.png) | ![](shots/nospeech-kante.png) |

Bauen braucht ALSA (`alsa-lib`, Debian/Ubuntu `libasound2-dev`). Der Build lädt die statischen sherpa-onnx-Bibliotheken; ohne Netz `SHERPA_ONNX_ARCHIVE_DIR` auf das vorher geladene Archiv setzen.

Freisprechen (Einstellungen, „Sprechen“): Kaiwa hört nach jeder Antwort der Figur von selbst zu, und die Pause aus „Ende eines Zugs“ (1,2–5 s) beendet den Satz; Füllwörter wie えーと zählen als Sprechen (/LF100/–/LF102/). Klingen zwei Sätze abgeschnitten, bietet das Gespräch eine längere Pause an. Ohne Sprechen lädt Kaiwa die Modelle nach der eingestellten Zeit aus dem Speicher (Voreinstellung 10 min, /LL012/).

## Stimme der Figur (Übergangsstimme)

Die Figur spricht ihre Antworten mit Kokoro-82M (Apache-2.0), Satz für Satz: Der erste Satz läuft, während der nächste entsteht. Kokoro kennt den Tonhöhenakzent nicht (Spike 0.4). Deshalb sagt die Seite „Übergangsstimme“, und Korrekturen, Beispiele zum Nachsprechen und Shadowing bleiben stumm, bis VOICEVOX geprüft ist (H-01).

Die Stimme spricht im Tempo, das die Schwierigkeit gerade vorgibt (0,7–1,1×, /LF211/). Unter jeder Zeile der Figur stehen „Nochmal“, „Langsamer“ (0,7 davon) und „Übersetzen“ (/LF111/); jede Nutzung zählt als Hilfe. Die Übersetzung liefert der Tutor, die der Eröffnung steht im Szenario. „Lauter“ und „Klar“ oben machen die Stimme lauter und heben die Konsonanten an (/LL204/); beides merkt sich das Profil.

`kokoro-v1.0.onnx` (310 MB, volle Genauigkeit) geht auch und hat Vorrang. Die Stimme wählt das Szenario (`voice`, z. B. `jf_tebukuro`). „Laut/Stumm“ oben schaltet sie ab; Sprechen oder ein Tipp auf den Sprechknopf unterbricht die Figur. Ohne Modell steht oben „Ohne Stimme“, der Tooltip sagt, was fehlt.

| Kante (dunkel) | Kante (Leinen) |
|---|---|
| ![](shots/voice-kante.png) | ![](shots/voice-light.png) |

Die Lautumschrift ist ein Nachbau von misakis japanischer G2P auf lindera mit IPADIC (in `kaiwa-speech`). Abgleich mit misaki über die 100 Korpussätze: `cargo run -p kaiwa-speech --example g2p_compare -- golden.json` (67 von 100 gleich, der Rest sind Wortgrenzen und Lesarten des Wörterbuchs). Einen Satz hören: `cargo run -p kaiwa-speech --example say -- "コーヒーが好きです。" out.wav`.

## Echter Tutor (für den lokalen Test, ADR-004)

```sh
KAIWA_PROVIDER=anthropic ANTHROPIC_API_KEY=… cargo run -p kaiwa-app
KAIWA_PROVIDER=openai KAIWA_OPENAI_BASE=http://localhost:11434/v1 KAIWA_MODEL=qwen3:14b cargo run -p kaiwa-app
```

Ohne diese Variablen läuft die App im Demo-Modus, was ein Banner oben anzeigt. Anthropic nutzt die Voreinstellung aus ADR-004: ein schnelles Modell für die Antwort, ein stärkeres für die Analyse.

## Kante aktualisieren

```sh
tools/sync-kante.sh ../Kante   # Kopie unverändert (GUI-Regel), danach committen
```

## Screenshots

`KAIWA_DEMO=1` startet mit einem frischen Profil im Temp-Verzeichnis, gefüllt mit zwei Wochen Verlauf. `KAIWA_NOW=<unix>:<offset in Minuten>` hält die Uhr fest.

```sh
export KAIWA_DEMO=1 KAIWA_NOW=1792478400:120
cargo run -p kaiwa-app -- --shot out.png --view path --kind 1           # start, path, review, profile, settings
cargo run -p kaiwa-app -- --shot out.png --view talk --scenario n5-03-konbini --kind 1 \
  --say 'はい、おねがいします。|袋はいりません。'                        # "?" Hilfe, "!" Sprechhilfe, "wav:datei", "~" wartet auf die Stimme
cargo run -p kaiwa-app -- --shot out.png --view talk --resume           # die unterbrochene Sitzung
KAIWA_DICT=data/dict/kaiwa-dict-excerpt.sqlite \
  cargo run -p kaiwa-app -- --shot out.png --view talk --say 'lookup:0:3' # Wort 3 der Zeile 0 nachschlagen: das Bild zeigt nur das Popup
cargo run -p kaiwa-app -- --shot out.png --view place --say '…|read:21'  # Einstufung; "read:KK" beantwortet die Lesefragen
```

## Wort nachschlagen

Ein Klick auf ein Wort in einem Satz der Figur (oder Tab, Pfeiltasten, Eingabe) öffnet ein Popup mit Lesung, Wortart, Bedeutungen und dem Soll-Akzent als Kurve, ohne Urteil über die eigene Aussprache (/LF112/, M-04). „Anhören“ spricht das Wort, „In die Wiederholung“ übernimmt es (/LF230/). Die Bedeutungen kommen in der UI-Sprache, wo JMdict sie hat, sonst englisch mit Hinweis (M-01). Das Wörterbuch ist das Datenpaket `kaiwa-data` (CC BY-SA, ADR-007); fehlt es, bietet das Popup den Download an, nur nach Klick (M-03). Die Demo liest den Auszug `data/dict/` über `KAIWA_DICT`. Ein Satz von der Kommandozeile: `cargo run -p kaiwa-lang --example lookup -- <kaiwa-dict.sqlite> de "駅まで歩いて五分です。"`.

## Moren-Hinweise

Nach einem gesprochenen Satz prüft die App im Hintergrund die Moren-Längen mit dem eigenen Modell `kaiwa-kana-ctc` (ADR-005). Gemeldet wird höchstens eine Kürzung je Satz, ein zu kurzer langer Vokal oder ein fehlendes っ, als Hinweis ohne Schweregrad („どーして → どして?“), nie als Fehler. Auf echten Stimmen findet das sechs von zehn Kürzungen und irrt sich bei etwa jedem zwanzigsten richtigen Satz (59,7 % bei 4,8 %, ADR-005). Die Hinweise sind standardmäßig an; abschalten lassen sie sich in den Einstellungen („Aussprache-Hinweise“).

## Freies Gespräch und Textgröße

Unter „Freies Gespräch“ auf der Startseite liegen vier Themen (Hobbys, Essen, Reisen, Alltag) außerhalb des Lernpfads, auf dem eigenen Niveau (/LF116/). Die Textgröße in den Einstellungen (100–200 %) vergrößert die ganze Oberfläche samt Lesehilfen, ab dem nächsten Start (/LL202/).
