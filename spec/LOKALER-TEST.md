# Lokaler Test

Was nur auf deinem Rechner geht: die KI-Anbindung (ADR-004), VOICEVOX mit seinen Stimmlizenzen (ADR-003) und Messwerte auf der Referenzhardware, dem Framework 13 (ADR-006). Jeder Schritt sagt, was du ausführst und was als Ergebnis zurückkommt. Die Ergebnisdateien kommen als PR nach `qa/local/<datum>/` (H-05). Claude wertet sie aus und trägt sie in die ADRs ein.

## 0. Vorbereitung

```sh
git clone https://git.arianw.de/shrippen/Kaiwa.git && cd Kaiwa
# Plasma 6 bringt Qt 6 und Kirigami mit; dazu Rust (rustup) und für die Spikes Python 3.11+
python3 -m venv .venv && . .venv/bin/activate && pip install -r spikes/requirements.txt
```

## 1. KI-Anbindung (ADR-004, Spike 0.5)

Wähle einen Anbieter oder beide. **In der App** geht das seit Q-15 ohne Umgebungsvariablen:
`cargo run -p kaiwa-app`, dann Einstellungen › Tutor: „Lokal (OpenAI-kompatibel)“ mit Modell (z. B.
`qwen3:14b`) und Server (leer ist Ollama auf diesem Rechner) oder „Anthropic Claude“ mit dem
API-Schlüssel, der im Schlüsselbund landet. Ohne Tutor sagt die App, dass keiner eingerichtet ist.

**Für `kaiwa-sim --live`** (kein Profil, keine Einstellungen) nimmt der Tutor die Umgebung:

```sh
export KAIWA_PROVIDER=anthropic ANTHROPIC_API_KEY=…                 # Haiku antwortet, Sonnet analysiert
# oder lokal:
export KAIWA_PROVIDER=openai KAIWA_MODEL=qwen3:14b KAIWA_OPENAI_BASE=http://localhost:11434/v1
```

Die Variablen überschreiben auch in der App die Einstellungen. Der Tutor bekommt die Grammatik der
Einheit mit (#41): Achte darauf, ob die Figur sie benutzt und auf dem Niveau der Einheit bleibt.

1. **Reisen automatisch:** `cargo run -p kaiwa-sim -- run --live --report qa/local/live.json`. Ergebnis: dieselben Prüfregeln wie in der CI, aber mit dem echten Tutor. Interessant sind `mara_recall` und `selin_register_flag` (Trefferquote, /LL111/), außerdem jede Korrektur bei korrekten Sätzen (Fehlalarm, /LL110/).
2. **Von Hand:** `cargo run -p kaiwa-app` (so, wie es ein Paket baut: ohne Demo). Führ das Szenario „Sich vorstellen“ einmal richtig und einmal mit Fehlern (`qa/corpus/saetze.tsv` s01–s03). Notiere, was dir an Antworten, Korrekturen und Erklärungen auffällt. Danach ein N4-Szenario, z. B. „Anruf in der Firma“ (n4-13, Keigo), über „Später weiter“ oder den Lernpfad nach dem Überspringen; der Knopf „Grammatik“ zeigt, was die Einheit übt.
3. **Latenz:** Stopp die Zeit vom Senden bis zur Antwort, fünf Züge, notieren (Ziel /LL010/).
4. **Sprechen und Stimme:** ReazonSpeech und Kokoro in den Einstellungen unter „Modelle“ laden (`app/README.md`, „Modelle“), dann mit Mikrofon und Lautsprecher spielen. Notiere: Wird verstanden, was du sagst? Klingt die Übergangsstimme verständlich, und stören falsche Lesarten oder Pausen? Die Zeit vom Loslassen des Knopfs bis zum ersten Ton der Figur gehört zur Latenz.

## 2. VOICEVOX (ADR-003, Spike 0.4 Teil 2)

Kokoro trifft den Pitch-Akzent nicht (`spikes/s04-tts/README.md`). VOICEVOX holt ihn aus dem Wörterbuch.

1. Lade die Release-Dateien (0.16.x) von github.com/VOICEVOX/voicevox_core: `download-linux-x64` und das Python-Wheel.
2. `./download-linux-x64 -o voicevox_core`. Lies die Nutzungsbedingungen der Stimmen, die angezeigt werden, und nimm nur die an, die du für Kaiwa in Betracht ziehst (F-06: natürlich-erwachsen).
3. `pip install voicevox_core-*.whl && python -I spikes/s04-tts/voicevox.py --core voicevox_core --out qa/local/`. Ergebnis: `voicevox.json` mit der Akzent-Trefferquote pro Stimme. Das Ziel sind 11 oder mehr von 13 Wörtern.

## 3. Messwerte auf dem Framework 13 (ADR-006)

```sh
python -I spikes/s04-tts/measure.py --models <dir> --corpus qa/corpus --out qa/local/s04       # Kokoro: RTF, Latenz
python -I spikes/s03-stt/measure.py --corpus-audio qa/local/s04/corpus --corpus qa/corpus \
    --whisper-cli <whisper.cpp>/build/bin/whisper-cli --models <dir> --out qa/local/s03 \
    --engines reazonspeech-zipformer,whisper-small                                              # STT
```

Die Modelle: `kokoro-v1.0.onnx` und `voices-v1.0.bin` (kokoro-onnx Releases), ggml-Modelle (huggingface.co/ggerganov/whisper.cpp) und `sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01` (k2-fsa/sherpa-onnx Releases, asr-models).

## 4. Eigene Aufnahmen (F-04)

`qa/corpus/README.md`: 50 Sätze und 20 Minimalpaare. Die Aufnahmen kommen per Git LFS nach `qa/corpus/audio/`.
