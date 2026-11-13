# Lokaler Test

Manches geht nur auf deinem Rechner: der echte Tutor (ADR-004), Mikrofon und Stimme, VOICEVOX mit seinen Stimmlizenzen (ADR-003) und Messwerte auf der Referenzhardware, dem Framework 13 (ADR-006). Jeder Schritt sagt, was du ausführst und was dabei herauskommt.

**Ergebnisse:** Leg einen Ordner `qa/local/<datum>/` an (z. B. `qa/local/2026-11-15/`) und kopier [`qa/local/VORLAGE.md`](../qa/local/VORLAGE.md) als `notizen.md` hinein. Die Vorlage hat für jede Prüfregel, die nicht automatisch läuft, eine Zeile zum Ankreuzen. Dazu kommen die Dateien, die die Schritte schreiben. Alles geht als PR ins Repository (H-05). Claude wertet aus und trägt die Ergebnisse in ADRs, Personas und `STAND.md` ein.

Du musst nicht alles an einem Tag machen. Die Reihenfolge unten ist die, die am meisten bringt: zuerst Schritt 1 (etwa 15 Minuten, kostet bei Anthropic ein paar Cent), dann 2 und 3.

## 0. Vorbereitung

```sh
git clone https://git.arianw.de/shrippen/Kaiwa.git && cd Kaiwa
# Plasma 6 bringt Qt 6 und Kirigami mit; dazu Rust (rustup) und für die Spikes Python 3.11+
python3 -m venv .venv && . .venv/bin/activate && pip install -r spikes/requirements.txt
```

## 1. Der Tutor automatisch (`kaiwa-sim run --live`)

Ein Tutor wird über die Umgebung gewählt (`kaiwa-sim` hat kein Profil und keine Einstellungen):

```sh
export KAIWA_PROVIDER=anthropic ANTHROPIC_API_KEY=…                 # Haiku antwortet, Sonnet analysiert
# oder über OpenRouter (ein Schlüssel, viele Modelle):
export KAIWA_PROVIDER=openrouter OPENROUTER_API_KEY=… KAIWA_MODEL=…  # Modell-Id wie auf openrouter.ai/models
# oder lokal:
export KAIWA_PROVIDER=ollama KAIWA_MODEL=qwen3:14b
```

Dann:

```sh
cargo run -p kaiwa-sim -- run --live --report qa/local/<datum>/live.json | tee qa/local/<datum>/live.txt
```

Alle Reisen laufen mit dem echten Tutor statt mit den Aufzeichnungen, rund 70 Züge. Ohne `--live` stehen die folgenden sechs Prüfregeln als „live only“ da, mit `--live` prüft `kaiwa-sim` sie selbst:

| Prüfregel | Was sie prüft | Skript |
|---|---|---|
| `theo_fp` | Höchstens 2 von 40 korrekten Sätzen bekommen eine Korrektur mit „falsch“ oder schlimmer (Fehlalarme, /LL110/). | `qa/scripts/theo_false_alarms.json` |
| `theo_tp` | Mindestens 5 der 6 eingebauten Fehler werden gefunden, ドアが開けました als „falsch“ (/LL111/). | dasselbe |
| `emi_keigo_detect` | Jeder Keigo-Fehler von Emi wird gefunden, 二重敬語 als „unnatürlich“, ご苦労様 als „falsch“ mit Begründung. | `qa/scripts/emi_interview.json` (Empfangsszene, bis es ein Vorstellungsgespräch gibt) |
| `mara_codeswitch` | Auf „Wie sagt man Produzentin auf Japanisch?“ nennt die Figur プロデューサー und bittet ums Nachsprechen (/LF122/). Ob sie es hörbar spricht, gehört zu Schritt 3. | `qa/scripts/mara_first_talk.json` |
| `mara_own_furigana` | Jeder Satz von Mara mit Kanji bekommt die Lesungen des Tutors, und ohne Markup ist es genau ihr Satz (/LF110/, ADR-004). Ob die Lesungen stimmen (何 als なに oder なん), beurteilst du (Vorlage). | dasselbe |
| `mara_explain_de` | Jede Erklärung ist auf Deutsch und hat ein japanisches Beispiel. Die Sprache erkennt eine Faustregel. Ob Fachbegriffe erklärt sind, beurteilst du (Vorlage). | dasselbe |

Für `theo_fp` siehst du außerdem in `live.txt` nach, ob Vorschläge zu den korrekten Sätzen nur als „Auch möglich“ kommen (Vorlage).

Bricht ein Aufruf des Anbieters ab (Limit, Netz), steht bei den Prüfregeln dieser Reise, die den Tutor brauchen, `FAIL … tutor: …`; der Lauf geht mit der nächsten Reise weiter, und der Report wird geschrieben. Dann den Lauf einfach wiederholen. Fehlt `KAIWA_PROVIDER` oder der Schlüssel, bricht der Lauf gleich am Anfang mit einer Meldung ab.

Die übrigen Prüfregeln laufen dabei auch mit dem echten Tutor. Interessant ist dort, was mit den Aufzeichnungen grün ist und mit dem Tutor rot wird, vor allem `mara_recall` und `selin_register_flag`. Ein `FAIL` ist kein Fehler deines Tests, sondern ein Befund: Die Zeile nennt den Grund.

## 2. Der Tutor in der App (von Hand)

**Erst die Variablen aus Schritt 1 entfernen** (oder ein neues Terminal öffnen). Sie haben Vorrang vor den Einstellungen; sonst redest du ab hier mit dem Tutor aus Schritt 1, und die lokalen Messungen in 3.5 und 4.3 stimmen nicht:

```sh
unset KAIWA_PROVIDER KAIWA_MODEL KAIWA_OPENAI_BASE KAIWA_API_KEY ANTHROPIC_API_KEY OPENROUTER_API_KEY OPENAI_API_KEY GEMINI_API_KEY MISTRAL_API_KEY
```

`cargo run -p kaiwa-app` (wie im Paket, ohne Demo), dann Einstellungen › Tutor: einen Anbieter wählen (Ollama oder LM Studio auf diesem Rechner, Anthropic, OpenRouter, OpenAI, Google Gemini, Mistral oder einen eigenen OpenAI-kompatiblen Server), bei den OpenAI-kompatiblen das Modell eintragen, bei einer Cloud den Schlüssel, der im Schlüsselbund landet. Gesetzte Umgebungsvariablen haben Vorrang vor den Einstellungen.

Was du beurteilst, steht in der Vorlage. Kurz:

1. **Sich vorstellen** (n5-02): einmal richtig, einmal mit den Fehlern aus `qa/corpus/saetze.tsv` s01–s03. Wie klingen Antworten, Korrekturen und Erklärungen?
2. **Erklärungen mit Beispielpaaren** (`emi_explain_examples`): ein N4-Szenario mit Keigo, z. B. „Anruf in der Firma“ (n4-13), über den Lernpfad nach dem Überspringen. Sag 明日、御社に行きます。 Nennt die Erklärung Fachbegriffe höchstens in Klammern und zeigt sie ein Paar wie 行きます → 伺います?
3. **Nachfragen** (`jonas_follow_up`): Antworte im Szenario „Am Bahnhof“ (n5-06) nur mit はい. Fragt die Figur nach, und bleibt das Ziel offen, bis du die Auskunft wirklich gibst?
4. **Unpassendes** (`jonas_inappropriate`): Schreib der Figur einmal etwas Beleidigendes. Sie soll freundlich ins Szenario zurücklenken, ohne Moralpredigt und ohne die Rolle zu verlassen.
5. **Deutsches Wort** (`hinnerk_german_word_help`): Sag im Szenario „Sich vorstellen“ わたしは Leuchtturm です。 Bietet die Figur とうだい an und bittet ums Nachsprechen? (Ob sie es langsam spricht, gehört zu Schritt 3.)
6. **Wiederholung als Mini-Gespräch** (/LF230/, neu): Am Tag nach einem Gespräch mit Korrekturen unter „Wiederholen“ der Figur antworten, getippt und gesprochen. Passt die Frage zur Korrektur? Wird eine richtige Antwort als richtig gewertet?
7. **Latenz:** Stopp fünfmal die Zeit vom Senden bis zur Antwort (Ziel /LL010/).
8. **Nachdenken eines lokalen Modells** (#73, #74, neu): Mit einem lokalen Modell, das vor der Antwort nachdenkt (z. B. ein Qwen3 über Ollama oder LM Studio), drei Züge im Szenario „Sich vorstellen“. Steht in der Zeile der Figur nur, was sie sagt (kein `<think>`, keine Überlegung auf Englisch)? Kommt nach einem Satz mit Fehler eine Korrektur? Steht unter einem deiner Sätze „Analyse unvollständig“ (T-11), notiere, wie oft in wie vielen Zügen. (Ob die Stimme nur die Zeile vorliest, ist Schritt 3.4a.) Der Schritt prüft nur etwas, wenn der Server das Nachdenken im Text der Antwort lässt; die Gegenprobe (Ollama: Port 11434, LM Studio: 1234) gibt den `content` aus, wie die App ihn bekommt; notiere in der Vorlage, ob `<think>` darin stand:

   ```sh
   curl -s http://localhost:11434/v1/chat/completions -H 'Content-Type: application/json' \
     -d '{"model":"<modell>","messages":[{"role":"user","content":"お名前は？"}]}' \
     | jq -r '.choices[0].message.content'
   ```

## 3. Sprechen und Stimme

ReazonSpeech, Kokoro und kaiwa-kana-ctc in den Einstellungen unter „Modelle“ laden (`app/README.md`, „Modelle“), dann mit Mikrofon und Lautsprecher spielen.

0. **Mikrofon** (Einstellungen › Mikrofon): Stimmt die Liste (keine „Monitor of …“-Einträge, das Headset dabei)? Den Test einmal still, einmal normal und einmal sehr nah ins Mikrofon sprechen: Passt das Urteil (kommt nichts an / gut / zu laut), klingt das Anhören wie du, und steht unter „Verstanden“ dein Satz? Ein anderes Mikrofon wählen, dann muss das Gespräch es nehmen.
1. **Verstanden?** Wird erkannt, was du sagst? Klingt die Übergangsstimme verständlich, stören falsche Lesarten oder Pausen?
1a. **Leertaste** (`mara_ptt`): Ein Gespräch öffnen und nichts anklicken: Leertaste halten, sprechen, loslassen. Der Zug muss aufgenommen und gesendet werden. Dann bei einer Zeile der Figur „Nochmal“ klicken und wieder die Leertaste halten: Es muss wieder aufnehmen, nicht „Nochmal“ auslösen. Im Antwortfeld schreibt die Leertaste ein Leerzeichen. Eine Antwort tippen, Enter, und während die Figur nachdenkt die Leertaste drücken: Danach muss das Tippen wieder im Feld landen, und nichts nimmt auf. Zuletzt ins Antwortfeld klicken, dann bei einer Zeile der Figur „Nochmal“ klicken und die Leertaste halten: Es muss aufnehmen. Mit Freisprechen startet und beendet ein Druck auf die Leertaste das Zuhören.
2. **Nicht abgeschnitten** (`hinnerk_no_cutoff`): Freisprechen an (Einstellungen › Sprechen). Sprich langsam, mit Pausen von 2–3 s mitten im Satz. Nachdenken vor der Antwort und ein langer Zug dürfen keinen Zug beenden. Schneidet die Pause dich mitten im Satz ab, soll die App gleich nach diesem Zug eine längere Pause anbieten (von 1,2 auf 3 s, dann auf 5 s), mit den Knöpfen „Pause auf … s“ und „So lassen“. Nimm sie an: Mit 5 s darf kein Zug mehr abgeschnitten werden. Nach „So lassen“ kommt das Angebot erst nach zwei weiteren solchen Zügen wieder.
2a. **Freisprechen über Lautsprecher** (#70, ohne Ton gebaut): Freisprechen an, Ton über die Lautsprecher, nicht über ein Headset.
   - Eine Antwort tippen statt sprechen; bei einer Zeile der Figur „Nochmal“ und „Langsamer“ klicken; ein Wort anklicken und anhören. Es darf kein Satz als deiner auftauchen, den die Figur gesagt hat, und nach ihrer Rede hört die App von selbst wieder zu.
   - Der Figur ins Wort fallen (Klick auf die Sprechtaste, während sie spricht): Sie verstummt, und die App hört zu. Ebenso nach „Stumm“.
   - Undeutlich murmeln oder die Hand vor den Mund halten, sodass der Pegel ausschlägt, aber nichts zu verstehen ist: Nach „Nichts verstanden“ (oder „Es kam kein Ton an“) hört die App wieder zu; nach dem dritten Mal in Folge bleibt das Mikrofon zu, bis du klickst. Ganz schweigen ist ein anderer Fall: Nach 30 s schließt das Mikrofon ohne Meldung. Ein Klick auf die Taste, während sie zuhört, schließt es sofort.
   - Während der Tutor nachdenkt, „Hilfe“ klicken: Der Knopf ist aus, das Fenster bleibt bedienbar (vorher fror es ein).
2b. **Mikrofon weg und wieder da** (#70): Freisprechen dafür ausschalten (Einstellungen › Sprechen: „Taste halten“), damit kein Zug offen ist. Das Headset abzuziehen genügt nicht, wenn der Rechner ein eingebautes Mikrofon hat; die App nimmt dann das. In den Audio-Einstellungen des Systems alle Eingabegeräte ausschalten (oder ein Profil ohne Eingang wählen), dann die Sprechtaste drücken: „Kein Sprechen“. Ein Eingabegerät wieder einschalten. Ein Klick auf die Taste zeigt, warum, und macht sie wieder benutzbar; der nächste Druck nimmt auf. Nimmt die App trotz ausgeschalteter Eingabegeräte auf, oder erscheint kein „Kein Sprechen“, das notieren: Wie sich PipeWire dabei verhält, ist nicht ausprobiert. Danach Freisprechen wieder einschalten, die nächsten Schritte prüfen es mit.
2c. **Unterbrechen und fortsetzen** (#70): Ein Gespräch einige Minuten führen, „Später weiter“, nach einer Pause fortsetzen und beenden. Die Bilanz und das Tagesziel zählen beide Teile, nicht die Pause dazwischen. Beim Verlassen verstummt die Figur. Beim Freisprechen hört die App nach dem Fortsetzen von selbst zu.
2d. **Nach der Leerlaufzeit** (#70; Einstellungen › „Modelle entladen“ auf 5 min): nach mindestens einem eigenen Zug im Gespräch bleiben und gut sechs Minuten nichts tun, gerechnet ab dem Ende der letzten Antwort der Figur (entladen wird im Minutentakt, frühestens fünf Minuten nach der letzten Nutzung). Dann bei einer Zeile der Figur „Nochmal“ klicken. Beim ersten Klick ist nichts zu hören, die Modelle laden wieder (die Taste zeigt kurz „Denkt nach“). Ein paar Sekunden warten und noch einmal klicken: Jetzt spielt die Zeile. Ist auch dann noch nichts zu hören, war die Stimme noch nicht fertig geladen (sie lädt nach der Erkennung); einen Moment später noch einmal klicken. Spielt die Zeile schon beim ersten Klick, waren die Modelle noch geladen: Dieser Klick zählt als Nutzung, also noch einmal gut sechs Minuten warten, wieder ab dem Ende der Zeile, die er gespielt hat. Beim Freisprechen hört die App nach dem Laden von selbst wieder zu („Hört zu“). Dasselbe nach der Leerlaufzeit mit einer getippten Antwort: Kommt die Antwort der Figur vor der Stimme, wird sie gesprochen, sobald die Stimme geladen ist. Steht das Gespräch noch bei der ersten Zeile der Figur, spielt ein Neuladen nach der Leerlaufzeit sie nicht noch einmal von selbst.
2e. **Tippen während der Erkennung**: Einen Zug sprechen (Taste loslassen) und sofort, solange „Denkt nach“ steht, ins Antwortfeld schauen: Feld, „Senden“, „Hilfe“, „Nochmal“ und „Langsamer“ sind aus. Dann ist der gesprochene Zug in der Liste, mit der Antwort der Figur; es fehlt keiner, und bei gespeicherten Aufnahmen gehört die Aufnahme zu diesem Zug.
3. **Moren-Hinweis** (`mara_mora_hint`): Sprich ありがとございます (kurzes とう) und ちょと待てください. Höchstens ein Hinweis zur Moren-Länge je Satz, nichts bei ありがとうございます.
4. **Langsam** (`hinnerk_german_word_help`, Teil Audio): Die Figur spricht とうだい aus Schritt 2.5 langsam? Ebenso hörbar: Auf „Wie sagt man Produzentin auf Japanisch?“ spricht sie プロデューサー (`mara_codeswitch`).
4a. **Nachdenken eines lokalen Modells, Teil Audio** (#74): Ein Zug mit dem Modell aus Schritt 2.8. Die Stimme liest nur die Zeile der Figur vor, keine Überlegung.
5. **Latenz** (`theo_latency`): 20 Züge im freien Gespräch, alles lokal (Ollama in den Einstellungen, ReazonSpeech, Kokoro; keine `KAIWA_*`-Variablen gesetzt). Miss je Zug die Zeit vom Ende deines Satzes bis zum ersten Ton der Figur, mit Stoppuhr oder Bildschirmaufnahme. Ziel: Median ≤ 3 s, 95 % ≤ 5 s, „denkt nach“ ist währenddessen sichtbar.

## 4. Rechner und Oberfläche (Framework 13)

1. **Speicher im Leerlauf** (`lena_models_unload`): Ein Gespräch mit Sprechen führen, dann 10 Minuten nichts tun und `grep Pss: /proc/$(pidof kaiwa)/smaps_rollup` ausgeben (in kB). Die Modelle müssen entladen sein, Ziel ≤ 150 MB PSS (U-01). Den Leerlauf ohne Gespräch und Modelle prüft die CI (`lena_idle_memory`, `kaiwa-sim run --app`).
2. **Flüssig** (`lena_responsive`): `QSG_RENDER_TIMING=1 cargo run -p kaiwa-app 2> qa/local/<datum>/frames.log` und ein Gespräch mit Sprechen führen. Die Bildzeiten stehen im Log, Ziel < 50 ms. „Denkt nach“ sichtbar, Abbrechen geht jederzeit?
3. **Nichts geht nach draußen** (`lena_zero_egress`): Tutor lokal (Ollama in den Einstellungen, keine `KAIWA_*`-Variablen gesetzt), Modelle geladen, dann
   `strace -f -e trace=connect,sendto,sendmsg -o qa/local/<datum>/egress.log ./target/debug/kaiwa` und ein Gespräch führen. Erlaubt sind nur Verbindungen zu `127.0.0.1`/`::1` (Ollama) und lokale Sockets (D-Bus, PipeWire). Jede andere Adresse und jede DNS-Anfrage (Port 53) ist ein Befund.
4. **Große Schrift** (`hinnerk_font`): Systemschrift auf 160 % oder in Kaiwa Textgröße 160 %, dann jede Ansicht durchgehen. Nichts abgeschnitten, Furigana und Kana mitskaliert, keine Bedienelemente überlappen. Screenshots bei Auffälligkeiten in den Ordner. Vorarbeit ohne Bildschirm: `tools/font-160.sh` nimmt jede Ansicht der Demo bei 160 % auf (deutsch und englisch, Fenster wie auf einem Laptop mit 1366 px) und listet Texte, die abgeschnitten sind oder über den Rand ragen; Überlappungen und Furigana zeigt nur der Blick auf die Bilder in `out/font-160/`.
5. **Schmales Fenster** (`hinnerk_narrow`, /LL303/): Das Fenster so schmal ziehen wie möglich; es bleibt bei 533 px stehen (bei 150 % Skalierung 800 Bildschirmpixel). Dann jede Ansicht durchgehen: nichts abgeschnitten, jeder Knopf erreichbar, Zusammengehöriges beisammen. Vorarbeit ohne Bildschirm: `SCALE=1 SCREEN=533 tools/font-160.sh`.

## 5. VOICEVOX (ADR-003, Spike 0.4 Teil 2)

Kokoro trifft den Pitch-Akzent nicht (`spikes/s04-tts/README.md`). VOICEVOX holt ihn aus dem Wörterbuch.

1. Lade die Release-Dateien (0.16.x) von github.com/VOICEVOX/voicevox_core: `download-linux-x64` und das Python-Wheel.
2. `./download-linux-x64 -o voicevox_core`. Lies die Nutzungsbedingungen der Stimmen, die angezeigt werden, und nimm nur die an, die du für Kaiwa in Betracht ziehst (F-06: natürlich-erwachsen).
3. `pip install voicevox_core-*.whl && python -I spikes/s04-tts/voicevox.py --core voicevox_core --out qa/local/<datum>/`. Ergebnis: `voicevox.json` mit der Akzent-Trefferquote pro Stimme. Das Ziel sind 11 oder mehr von 13 Wörtern.

## 6. Messwerte (ADR-006)

```sh
python -I spikes/s04-tts/measure.py --models <dir> --corpus qa/corpus --out qa/local/<datum>/s04       # Kokoro: RTF, Latenz
python -I spikes/s03-stt/measure.py --corpus-audio qa/local/<datum>/s04/corpus --corpus qa/corpus \
    --whisper-cli <whisper.cpp>/build/bin/whisper-cli --models <dir> --out qa/local/<datum>/s03 \
    --engines reazonspeech-zipformer,whisper-small                                                     # STT
```

Die Modelle: `kokoro-v1.0.onnx` und `voices-v1.0.bin` (kokoro-onnx Releases), ggml-Modelle (huggingface.co/ggerganov/whisper.cpp) und `sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01` (k2-fsa/sherpa-onnx Releases, asr-models).

## 7. Eigene Aufnahmen (F-04)

`qa/corpus/README.md`: 50 Sätze und 20 Minimalpaare. Die Aufnahmen kommen per Git LFS nach `qa/corpus/audio/`.

## Nicht Teil dieses Tests

| Prüfregeln | Warum später |
|---|---|
| `mara_plasmoid_*`, `mara_ptt_shortcut` | Plasmoid und `kaiwad` ruhen bis M4 (Zweig `later/plasmoid`). |
| `jonas_noise`, `jonas_sync` | Android und Sync gehören zu M5/M6. |
| `selin_win_keys`, `selin_win_style` | Windows gehört zu M6. |
| `theo_pitch_detect`, `theo_pitch_no_nitpick`, `theo_compare` | Pitch-Bewertung erst nach 1.0 (L-02 bis L-04). |
