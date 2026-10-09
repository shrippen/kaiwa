# Architekturentscheidungen (ADR)

Jede ADR hat einen Status:

- **Vorgeschlagen:** Empfehlung steht, Spike oder Freigabe fehlt.
- **Angenommen:** entschieden.
- **Ersetzt:** durch eine neuere ADR abgelöst.

Offene Punkte tragen eine Frage-ID (`F-xx`) aus dem jeweiligen Fragenpaket. Die Pakete werden als Artifact vorgelegt, die Antworten fließen hier ein.

| ADR | Thema | Status | Empfehlung |
|---|---|---|---|
| [001](001-qt-bindung.md) | Rust ↔ Qt/QML | Vorgeschlagen | cxx-qt |
| [002](002-stt.md) | Spracherkennung | Vorgeschlagen, Spike 0.3 | lokal whisper.cpp, Kandidat ReazonSpeech; Cloud optional |
| [003](003-tts.md) | Sprachausgabe | Vorgeschlagen, Spike 0.4 | lokal Kokoro (Apache-2.0), VOICEVOX optional; Cloud optional |
| [004](004-llm.md) | KI-Tutor und Korrektur | Vorgeschlagen, Spike 0.5 | zwei Aufrufe (Antwort gestreamt, Analyse parallel), Trait für OpenAI-kompatibel und Anthropic |
| [005](005-aussprache.md) | Aussprache-Pipeline | Vorgeschlagen, Spike 0.6 | eigenes Alignment + F0 lokal, Azure-Bewertung optional |
| [006](006-latenzbudget.md) | Latenzbudget | Vorgeschlagen, Spike 0.7 | 1,5 s ab erkanntem Sprechende, aufgeteilt nach Stufen |
| [007](007-daten-lizenzen.md) | Sprachdaten und Lizenzen | Angenommen | Code MIT, Daten getrennt; JMdict/KANJIDIC2, UniDic; JLPT- und Akzentlisten prüfen |
| [008](008-plasmoid-backend.md) | Backend für App und Plasmoid | Vorgeschlagen | Dienst `kaiwad` (D-Bus), dünner Client im Plasmoid |
