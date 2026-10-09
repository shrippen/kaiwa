# Architekturentscheidungen (ADR)

Jede ADR hat einen Status:

- **Vorgeschlagen:** Empfehlung steht, Spike oder Freigabe fehlt.
- **Angenommen:** entschieden.
- **Ersetzt:** durch eine neuere ADR abgelöst.

Offene Punkte tragen eine Frage-ID (`F-xx`) aus dem jeweiligen Fragenpaket. Die Pakete werden als Artifact vorgelegt, die Antworten fließen hier ein.

| ADR | Thema | Status | Empfehlung |
|---|---|---|---|
| [001](001-qt-bindung.md) | Rust ↔ Qt/QML | Angenommen (Spike 0.2) | cxx-qt 0.10; `cxx_name` in camelCase; Kante als Kopie |
| [002](002-stt.md) | Spracherkennung | Spike 0.3 gemessen (H-03) | ReazonSpeech Zipformer: CER 1,3 %, RTF 0,07; whisper als Option |
| [003](003-tts.md) | Sprachausgabe | Spike 0.4 gemessen (H-01, H-02) | Kokoro trifft den Akzent nicht; VOICEVOX im lokalen Test |
| [004](004-llm.md) | KI-Tutor und Korrektur | Angenommen, Modellwahl im Spike 0.5 | zwei Aufrufe (Antwort gestreamt, Analyse parallel); Vorlagen OpenAI-kompatibel und Anthropic; lokales Modell per Download oder eigener Server |
| [005](005-aussprache.md) | Aussprache-Pipeline | Vorgeschlagen, Spike 0.6 | eigenes Alignment + F0 lokal, Azure-Bewertung optional |
| [006](006-latenzbudget.md) | Latenzbudget | Vorgeschlagen, Spike 0.7 | 1,5 s ab erkanntem Sprechende, aufgeteilt nach Stufen |
| [007](007-daten-lizenzen.md) | Sprachdaten und Lizenzen | Angenommen | Code MIT, Daten getrennt; JMdict/KANJIDIC2, UniDic; JLPT- und Akzentlisten prüfen |
| [008](008-plasmoid-backend.md) | Backend für App und Plasmoid | Angenommen | Dienst `kaiwad` (D-Bus), dünner Client im Plasmoid; nur über Pakete |
