# Kaiwa – Roadmap

*Arbeitstitel: **Kaiwa** (会話, „Gespräch“). Japanisch üben durch Gespräche mit einer KI, also Sprechen und Hörverstehen mit Verbesserungsvorschlägen. Die Anforderungen stehen im [Lastenheft](spec/LASTENHEFT.md) (IDs `/LFxxx/`, `/LLxxx/`). Die QA-Personas liegen in [`qa/personas/`](qa/personas/).*

## Leitlinien

1. **Vom ersten Tag an sprechen** (Jumpspeak): Jede Lektion ist ein Gespräch. Lückentexte und Karten gibt es nur zur Vorbereitung und Wiederholung.
2. **Kleine, regelmäßige Schritte** (Duolingo): ein sichtbarer Lernpfad, ein Tagesziel und eine Serie. Dazu gehört keine Erpressung, also keine Schuld-Benachrichtigungen und keine Herzen, die das Üben sperren.
3. **Adaptiv statt linear:** Das Programm zielt auf etwa 80 % Erfolg pro Gespräch (/LF210/). Ist es zu leicht, steigt das Niveau, ist es zu schwer, fängt das Programm auf.
4. **Lokal zuerst:** Alles läuft auch ohne Cloud. Cloud-Anbieter sind ein Opt-in pro Baustein (/LL400/).
5. **QA durch Personas ab Meilenstein 0:** Jede Funktion bekommt eine Reise in mindestens einer Persona, bevor sie als fertig gilt.
6. **Die GUI kommt aus Kante:** Fehlende Bausteine werden zuerst in Kante ergänzt (Proposal, QML-Komponente, Test, Galerie) und erst dann hier benutzt.

## Architektur in einem Bild

```
 ┌──────────── Kirigami-App ────────────┐   ┌──── Plasmoid (KantePlasma) ────┐
 │ QML + qml/Kante                      │   │ Kurzsession, Wiederholung,     │
 │ Lernpfad · Gespräch · Statistik      │   │ Serie, Push-to-Talk            │
 └───────────────┬──────────────────────┘   └───────────────┬────────────────┘
                 │ cxx-qt (QML-Modul org.shrippen.kaiwa)    │
                 └──────────────┬───────────────────────────┘
          in-process (alle OS)  │  oder D-Bus-Dienst kaiwad (Plasma, M4 vor 1.0)
 ┌──────────────────────────────┴───────────────────────────────────────────┐
 │ kaiwa-core   Lernmodell · Curriculum · Schwierigkeitsregler · Sessions   │
 │ kaiwa-lang   Morphologie (vibrato/lindera + UniDic) · Furigana · JLPT-Tags│
 │ kaiwa-speech Audio (cpal) · VAD · STT (whisper.cpp | Cloud)              │
 │ kaiwa-tts    TTS (lokal | Cloud) · Sprechtempo                           │
 │ kaiwa-pron   Alignment · Moren · Pitch-Akzent (F0)                       │
 │ kaiwa-ai     LLM-Anbieter (lokal OpenAI-kompatibel | Cloud) · Schemas    │
 │ kaiwa-store  SQLite · Export/Import · optionaler Sync                    │
 └──────────────────────────────────────────────────────────────────────────┘
 kaiwa-sim  QA-Läufer: Personas → simulierte Lernende (Text + synthetische Sprache)
```

Das Workspace-Layout (Cargo) und die Entscheidungen dazu stehen als ADRs in `spec/adr/`. ADR-001 bis ADR-008 sind Teil von M0.

## Meilensteine

Größen: **S** ≈ 1–2 Wochen, **M** ≈ 3–5 Wochen, **L** ≈ 6+ Wochen (eine Person mit KI-Unterstützung). Die Reihenfolge ist verbindlich, die Größen sind Schätzungen.

### M0 – Fundament und Spikes (0.0.x) · M

Hier fallen die riskanten Entscheidungen, bevor Funktionen gebaut werden.

| # | Arbeitspaket | Ergebnis |
|---|---|---|
| 0.1 | Repo, `agent.md` (GUI-Regel aus Kante, Repository-Regel, Release-Regel), CI (fmt, clippy, test, QML-Tests) | grüne Pipeline auf Gitea |
| 0.2 | Spike **cxx-qt + Kirigami + Kante**: Fenster mit `KanteScope`, `KanteButton` und Stilumschaltung System/Kante | ADR-001 Qt-Bindung (cxx-qt vs. qmetaobject-rs) |
| 0.3 | Spike **STT Japanisch**: whisper.cpp (small/medium/large-v3-turbo) gegen Cloud mit 50 Testsätzen, Lernenden-Akzent synthetisch und echt | ADR-002 STT, Messwerte CER und Latenz |
| 0.4 | Spike **TTS Japanisch**: lokale Engines (z. B. VOICEVOX Core, Piper-ja) und Cloud, mit Prüfung der Lizenzen und Stimmrechte | ADR-003 TTS, Lizenztabelle |
| 0.5 | Spike **LLM-Tutor**: strukturierte Ausgabe (Antwort + Korrekturen + Niveau-Tags) lokal (7–14B) gegen Cloud | ADR-004 LLM-Anbieter-Schnittstelle, Prompt-Format |
| 0.6 | Spike **Aussprache**: Moren-Alignment und F0-Kontur an 20 Minimalpaaren (おばさん/おばあさん, きて/きって, はし/はし) | ADR-005 Aussprache-Pipeline, Machbarkeit |
| 0.7 | **Latenzbudget** messen: Sprechende → Tutor spricht | ADR-006, Ziel /LL010/ |
| 0.8 | **Daten und Lizenzen**: JMdict/KANJIDIC2 (CC BY-SA), UniDic, JLPT-Listen, Akzentdaten | ADR-007, `NOTICE` |
| 0.9 | **Kante-Lücken** als Proposal in Kante (`proposals/2026-1x-kaiwa/`): Furigana-Text, Push-to-Talk-Taste mit Pegel, Korrektur-Diff, Pitch-Kurve, Lernpfad-Knoten, Transkript-Zeile auf `KanteMessage` | Proposal-PR in Kante |
| 0.10 | **kaiwa-sim Gerüst**: Personas laden und validieren (`qa/personas/schema.json`), Textmodus gegen ein Mock-Backend | `cargo run -p kaiwa-sim -- --persona all --dry-run` |

**Exit:** Alle ADRs sind entschieden, das Latenzbudget ist auf Referenzhardware erreichbar (oder mit Cloud-Fallback dokumentiert), und der Kante-Proposal ist eingereicht.

### M1 – Erstes Gespräch (0.1) · L

Ein Szenario funktioniert durchgehend: **Selbstvorstellung (自己紹介)**.

- Push-to-Talk und automatisches Sprechende per VAD (/LF100/, /LF101/)
- STT → LLM-Tutor → TTS mit Untertitel in Kana/Kanji, Furigana und Übersetzung auf Wunsch (/LF110/–/LF115/)
- **Korrektur nach jedem Zug** auf Textebene: Grammatik, Wortwahl, Höflichkeitsform, mit Erklärung in der UI-Sprache (/LF300/–/LF304/)
- **Recast im Gespräch:** Der Tutor wiederholt die korrigierte Form natürlich (/LF305/)
- Tippen statt Sprechen als Fallback (/LF120/)
- Lokales Profil in SQLite (/LD010/)
- App-GUI: Gesprächsseite mit `KanteMessage`, den neuen Kante-Bausteinen aus 0.9 und Einstellungen (Anbieter, Mikrofon)
- **QA:** Mara (Anfängerin) und Emi (Herkunftssprecherin) spielen „Selbstvorstellung“ im Textmodus durch.

**Exit:** In 10 Durchläufen pro Persona gibt es keine Abbrüche, die Korrektur-Fehlalarmrate liegt bei Emis korrekten Sätzen unter 10 % (/LL110/), und die Latenz erreicht das Ziel.

### M2 – Lernmodell und Adaptivität (0.2) · L

- **Einstufung** per Gespräch, 5–8 Minuten, statt Test (/LF200/). Überspringen ist möglich, dann beginnt das Programm bei Null.
- **Lernermodell:** Fähigkeiten pro Grammatikpunkt, Wort, Hören (Tempo), Sprechen (Flüssigkeit, Latenz), Lesen (Kana, Kanji) (/LF201/)
- **Schwierigkeitsregler:** Er steuert Wortschatz außerhalb des Niveaus (i+1, ≤ 5 % unbekannt), Satzlänge, Sprechtempo der TTS, Furigana-Dichte, Hilfen und Register (/LF210/–/LF215/).
- **Curriculum N5**, Einheiten 1–12 als Graph mit Szenarien (Konbini, Bahnhof, Restaurant, Wegbeschreibung, Arbeit, Hobbys …) (/LF220/)
- **Wiederholung (SRS)** mit FSRS für Wörter und Wendungen aus den eigenen Fehlern (/LF230/)
- **Sitzungsbilanz:** die drei wichtigsten Punkte, neue Wörter und der Fortschritt (/LF310/)
- **QA:** Alle Personas laufen 30 simulierte Tage mit beschleunigter Uhr. Prüfregeln: Selin wird im Lesen herab- und im Hören heraufgestuft, Emi überspringt Einheiten, und Hinnerk bekommt keinen Zeitdruck.

### M3 – Aussprache (0.3) · L

- Erkennung von **Langvokalen, Doppelkonsonanten (っ), ん, Vokalentstimmung und Moren-Timing** (/LF320/)
- **Pitch-Akzent:** Soll-Kontur aus Akzentdaten, Ist-Kontur aus F0, als Kurve dargestellt (Kante-Baustein aus 0.9) (/LF321/)
- **Shadowing und Nachsprechen:** Satz hören, nachsprechen und vergleichen, mit Wiedergabe der eigenen Aufnahme (/LF330/)
- Gewichtung: Nur Fehler, die die Verständlichkeit gefährden, unterbrechen. Der Rest landet in der Bilanz (/LF306/).
- **QA:** Theo (Pitch-Akzent-Perfektionist) prüft Fehlalarme, Hinnerk prüft langsames Sprechen und lange Pausen, Jonas prüft Störgeräusche (Zug).

### M5 – Motivation und Lernpfad (0.5) · M

- Lernpfad-Ansicht (Einheiten, Szenarien, Wiederholungsknoten), XP, Tagesziel, Serie mit Freeze und Wochenrückblick (/LF600/–/LF606/)
- Statistik: Heatmap (`KanteHeatmap`), Wortschatz, Sprechzeit, Fehlerarten im Verlauf (/LF610/)
- Ruhige Benachrichtigungen, abschaltbar, ohne Schuldformulierungen (/LF620/, /LL300/)
- **QA:** Jonas (versucht das System auszutricksen: Einwort-Antworten, Wiederholungen, Rōmaji tippen), Hinnerk (keine Gamification, falls abgeschaltet)

### M6 – Cross-Plattform und Daten (0.6–0.8) · L

- **0.6 Linux breit:** Flatpak (App; das Plasmoid kommt aus Distro-Paket und AUR), GNOME und andere Desktops ohne Plasmoid (/LL500/)
- **0.7 Windows und macOS:** Installer, in-process-Backend ohne D-Bus, Audio-Backends (/LL501/)
- **0.8 Android:** Kirigami, Mikrofonrechte, Cloud-Anbieter als Standard auf schwacher Hardware (/LL502/)
- **Sync** (optional): Ende-zu-Ende-verschlüsselte Datei per WebDAV/Nextcloud, ohne eigenen Server (/LF700/)
- Export und Löschung aller Daten (DSGVO), Datenschutzansicht „Was verlässt das Gerät?“ (/LF710/, /LF711/)
- **QA:** Lena (nur lokal, prüft den Netzwerkverkehr), Jonas (Gerätewechsel), Selin (Windows)

### M4 – Plasma-Integration und Plasmoid (vor 1.0) · M

Zeitlich nach M6, wenn die App nahe 1.0 ist (Entscheidung im Chat, 2026-10-10). Der Stand von `kaiwad` und Plasmoid liegt auf dem Zweig `later/plasmoid` (Kurzgespräch, fünf Wiederholungen, Sprechtaste über den Dienst, Erinnerungen) und wird dann auf den aktuellen Stand gebracht. Dazu gehören Erinnerungen mit frei wählbarer Uhrzeit (I-07, KanteTimeField statt der festen Auswahl) und die ruhenden Fragen I-02 bis I-04.


- **Plasmoid** im Panel und auf dem Desktop: Kurzgespräch (3–5 min), 5 Wiederholungen, Tagesziel und Serie, Push-to-Talk (/LF500/–/LF506/)
- `kaiwad` als D-Bus-Dienst mit Session-Aktivierung: Er lädt die Modelle einmal, App und Plasmoid teilen sich Sitzung und Mikrofon (/LF510/, ADR-008).
- Globales Tastenkürzel für Push-to-Talk (KGlobalAccel), Erinnerungen über KNotification, optional KRunner („kaiwa 駅“ → Wort nachschlagen und vorsprechen) (/LF520/–/LF522/)
- Eine im Plasmoid begonnene Sitzung lässt sich in der App fortsetzen (/LF507/).
- **QA:** Mara (5-Minuten-Sitzungen zwischen Terminen, Unterbrechung durch Anruf) und Jonas (Plasmoid + Android)

### M7 – 1.0 · M

- Inhalte **N5 und N4** vollständig, mit Abnahme durch ein Muttersprachler-Review pro Einheit (/LF221/)
- Barrierefreiheit: Tastatur, Screenreader (Accessible-Namen auf allen Kante-Bausteinen), Schriftgröße, Kontrast in Kante und Kante Light (/LL200/–/LL203/)
- **Alle Personas** sind über 30 simulierte Tage grün und alle Abnahmekriterien erfüllt (Lastenheft Kap. 10).
- Landing Page nach Kante-Vorlage, Release-Log mit Screenshots pro Sprache (Release-Regel), Eintrag in `overview/projects.json`. Vorher erscheint Kaiwa nicht auf der Übersicht (G-09).

### Nach 1.0

N3–N1, Kanji-Schreiben (Strichfolge, Stift oder Touch), Rollenspiele mit mehreren KI-Figuren, Lehrkraft-Modus (eigene Szenarien teilen), weitere UI-Sprachen und andere Zielsprachen. Die Architektur lässt die Sprache als Parameter zu, aber 1.0 ist nur Japanisch.

## Definition of Done (jede Funktion)

- Die Anforderung im Lastenheft ist verlinkt, und Tests decken sie ab (Unit, QML, `kaiwa-sim`-Reise).
- Die GUI besteht nur aus Kante-Bausteinen, ohne Rohwerte (`#hex`, eigene Größen). Ein Check wie `check-tokens` läuft über `app/` und `plasmoid/`.
- Die Funktion funktioniert in System, Kante und Kante Light sowie in de und en.
- Ist sie sichtbar, kommt ein Demo-Screenshot pro Sprache für das Release-Log dazu.

## Risiken

| Risiko | Wirkung | Gegenmaßnahme |
|---|---|---|
| STT „korrigiert“ Lernendenfehler weg (Whisper glättet) | Fehler werden nicht erkannt | Rohhypothesen mit niedriger Temperatur und Wort-Konfidenz, Alignment gegen Erwartung, ADR-002 |
| Lokale Latenz zu hoch | Gespräch wirkt zäh | Streaming-STT/-TTS, kleinere Modelle, Cloud-Fallback, kaiwad hält Modelle warm |
| LLM-Fehlkorrekturen | Vertrauensverlust | strukturierte Ausgabe, Gegenprüfung mit Morphologie und Regeln, Fehlalarm-Budget in QA (/LL110/) |
| Lizenzen von Stimmen und Daten | Release blockiert | ADR-003, ADR-007 vor M1 |
| cxx-qt-Reife (Plasmoid-Plugin) | Integrationsaufwand | Spike 0.2, Fallback D-Bus plus reines QML-Plasmoid |
| Kante fehlen Bausteine | Wartezeit | Proposal in M0, parallel zu M1 umsetzen |
| Noch niemand für das Muttersprachler-Review (F-05: später) | Abnahme 1.0 (/LF221/) verzögert sich | spätestens zu Beginn von M6 klären, sonst bezahltes Lektorat pro Einheit einplanen |
