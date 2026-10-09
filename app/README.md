# Kaiwa (App)

Kirigami-App mit der GUI aus Kante 1.25 über `kaiwa-core`. Stand M1: ein Szenario (Sich vorstellen, N5-02). Man antwortet gesprochen (Push-to-Talk, ReazonSpeech, H-06) oder getippt, Hilfe gibt es in drei Stufen. Korrekturen erscheinen als Hint-Cards mit Diff unter dem eigenen Satz, am Ende steht die Bilanz. Die Figur antwortet noch als Text; ihre Stimme folgt (H-02).

| Kante (dunkel) | Kante (Leinen) | System |
|---|---|---|
| ![](shots/kante.png) | ![](shots/light.png) | ![](shots/system.png) |

## Bauen und starten

```sh
# Qt ≥ 6.6, Kirigami 6, Rust; Kante wird nicht gebraucht (einkompiliert)
cargo run -p kaiwa-app                    # Demo: aufgezeichnete Antworten (qa/fixtures)
```

## Sprechen

Die Spracherkennung (ReazonSpeech über sherpa-onnx, ADR-002) läuft lokal. Das Modell kommt nicht mit der App:

```sh
mkdir -p ~/.local/share/kaiwa/models && cd ~/.local/share/kaiwa/models
curl -LO https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01.tar.bz2
tar xf sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01.tar.bz2   # Apache-2.0, etwa 150 MB
```

Ein anderer Ort geht über `KAIWA_MODELS=/pfad`. Fehlt das Modell, zeigt der Sprechknopf den Fehlerzustand und auf Klick, was zu tun ist; Tippen geht weiter. Sprechen: Leertaste oder Knopf halten, loslassen schickt den Satz.

| Gesprochen (Demo) | Ohne Modell |
|---|---|
| ![](shots/speech-kante.png) | ![](shots/nospeech-kante.png) |

Bauen braucht ALSA (`alsa-lib`, Debian/Ubuntu `libasound2-dev`). Der Build lädt die statischen sherpa-onnx-Bibliotheken; ohne Netz `SHERPA_ONNX_ARCHIVE_DIR` auf das vorher geladene Archiv setzen.

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

```sh
cargo run -p kaiwa-app -- --shot out.png --kind 1 --light 0 --height 2300 \
  --say '?|マラです。よろしくおねがいします。|コーヒーを好きです。'   # "?" fragt nach Hilfe
cargo run -p kaiwa-app -- --shot out.png --say 'wav:satz.wav'   # WAV als gesprochener Zug, "!" zeigt die Sprechhilfe
```
