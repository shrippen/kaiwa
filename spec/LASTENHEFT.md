# Lastenheft – Kaiwa

| | |
|---|---|
| Produkt | Kaiwa (会話), Arbeitstitel |
| Auftraggeber | shrippen |
| Stand | 2026-10-12, Version 0.3 (Entwurf, Entscheidungen der Fragenpakete eingearbeitet) |
| Bezug | [Roadmap](../ROADMAP.md), [QA-Personas](../qa/personas/), [Rückverfolgung](RUECKVERFOLGUNG.md), [Stand](STAND.md) |

Das Lastenheft beschreibt, **was** das Produkt leisten soll und **warum**. Das **Wie** steht im Pflichtenheft und in den ADRs (`spec/adr/`). Die Nummerierung folgt Balzert:

| Präfix | Bedeutung |
|---|---|
| `/LFxxx/` | Funktion |
| `/LDxxx/` | Daten |
| `/LLxxx/` | Leistung und Qualität |

Jede Anforderung hat eine Priorität:

- **M** = Muss (1.0)
- **S** = Soll (1.0, verschiebbar)
- **K** = Kann (nach 1.0)

Entscheidungen des Auftraggebers aus den Fragenpaketen (IDs wie `K-01`) stehen in den ADRs. Wo eine Entscheidung eine Anforderung einschränkt oder genauer fasst, steht sie hier kursiv bei der Anforderung. Wie weit jede Anforderung umgesetzt und geprüft ist, zeigt die erzeugte Tabelle [`RUECKVERFOLGUNG.md`](RUECKVERFOLGUNG.md).

---

## 1. Zielbestimmung

### 1.1 Ausgangslage

Wer Japanisch lernt, übt mit Apps meist Lesen und Wortschatz (Duolingo). Sprechen und Hörverstehen im echten Gespräch kommen zu kurz. Jumpspeak setzt auf KI-Gespräche, ist aber ein Cloud-Abo ohne Desktop-Integration und ohne lokale Option. Auf KDE Plasma gibt es kein Werkzeug, mit dem man zwischendurch fünf Minuten Japanisch sprechen kann.

### 1.2 Ziel

Kaiwa ist ein Trainer für **Sprechen und Hörverstehen** im Japanischen. Man übt in Gesprächen mit einer KI, bekommt Verbesserungsvorschläge zu Grammatik, Wortwahl, Höflichkeit und Aussprache, und die Schwierigkeit passt sich dem eigenen Können an. Hauptplattform ist KDE Plasma 6, mit einer schlanken Plasmoid-Version für Kurzsitzungen. Die App läuft außerdem unter anderen Linux-Desktops, Windows, macOS und Android.

### 1.3 Musskriterien (Auswahl, vollständig in Kap. 4–7)

- Gespräche per Sprache in Szenarien von N5 bis N4, mit Korrekturen nach jedem Zug und einer Bilanz am Ende
- Einstufung und laufende Anpassung der Schwierigkeit
- Wiederholung eigener Fehler und neuer Wörter (SRS)
- Plasmoid für Sitzungen von 3–5 Minuten
- Vollständig lokal nutzbar, Cloud nur als Opt-in
- GUI aus Kante (System, Kante, Kante Light), UI auf Deutsch und Englisch

### 1.4 Wunschkriterien

Aussprachefeedback zum Pitch-Akzent, KRunner-Integration, Sync zwischen Geräten, Android-App, Statistiken über Monate.

### 1.5 Abgrenzungskriterien (was Kaiwa nicht ist)

- Kein Kanji-Schreibtrainer und kein Lesekurs (Lesen nur als Stütze des Gesprächs)
- Kein Ersatz für Unterricht mit Menschen und keine Prüfungsvorbereitung mit Zertifikat
- Keine soziale Plattform: keine Ranglisten mit Fremden, kein Chat zwischen Nutzerinnen und Nutzern
- Keine Inhalte von Lehrbüchern Dritter (Genki, Minna no Nihongo, Marugoto). Kaiwa orientiert sich nur an den Kann-Beschreibungen (JF-Standard, JLPT-Niveaus).
- Keine Monetarisierung im Produkt (keine Käufe, keine Werbung)

---

## 2. Produkteinsatz

### 2.1 Anwendungsbereiche

Selbststudium zu Hause, am Arbeitsplatz (Kurzsitzung im Panel), unterwegs (Laptop, Android) und zur Reisevorbereitung.

### 2.2 Zielgruppen

Erwachsene und Jugendliche ab 14 mit Deutsch oder Englisch als UI-Sprache, von absoluten Anfängern bis N4, ab 1.x bis N3+. Die Personas in [`qa/personas/`](../qa/personas/) stehen für die Zielgruppe und sind zugleich Prüflinge der QA (Kap. 9).

| Persona | Steht für |
|---|---|
| Mara Weber | Berufstätige Anfängerin mit wenig Zeit, Plasmoid |
| Selin Aydın | Anime-geprägte „falsche Anfängerin“, Hören stark, Lesen schwach, Register zu salopp |
| Theo Lindqvist | Fortgeschrittener (N4) mit hohem Anspruch an Aussprache und Pitch-Akzent |
| Hinnerk Aalders | Senior, wenig Technikerfahrung, hört und sieht schlechter, spricht langsam |
| Emi Hartmann | Herkunftssprecherin: flüssig in Alltagssprache, keine Höflichkeitsformen, kaum Kanji |
| Jonas Brandt | Spielerisch, motiviert durch die Serie, testet Grenzen, wechselt oft das Gerät |
| Lena Kraus | Datenschutzbewusst, nur lokal, durchschnittliche Hardware |

### 2.3 Betriebsbedingungen

Einzelplatz mit einem Profil pro Systemkonto (mehrere Profile: /LF720/). Mikrofon und Lautsprecher oder Headset. Offline-Betrieb ist mit lokalen Modellen möglich, Cloud-Anbieter brauchen Internet und einen eigenen API-Schlüssel.

---

## 3. Produktübersicht

```
            ┌────────────┐  Sprache   ┌────────┐  Text  ┌─────────────┐
 Lernende ─▶│ Mikrofon   │──────────▶│  STT   │──────▶│  Tutor (LLM) │──┐
            └────────────┘            └────────┘       │ + Korrektur  │  │
                 ▲                         │ Audio      └─────┬───────┘  │
                 │                         ▼                  │ Antwort  │ Korrekturen,
            ┌────┴──────┐            ┌──────────┐             ▼          │ Niveau-Tags
            │ Lautspr.  │◀───────────│   TTS    │◀── Antwort-Text        │
            └───────────┘            └──────────┘                        ▼
                                     ┌──────────┐            ┌──────────────────┐
                                     │Aussprache│──────────▶│   Lernermodell    │
                                     └──────────┘            │ Schwierigkeit·SRS │
                                                             └──────────────────┘
```

Oberflächen:

| Oberfläche | Zweck |
|---|---|
| **App** (Kirigami) | Lernpfad, Gespräch, Wiederholung, Statistik, Einstellungen |
| **Plasmoid** | Kurzgespräch, Wiederholung, Tagesziel |
| **CLI und Headless-API** | QA und Simulation (`kaiwa-sim`) |

---

## 4. Funktionale Anforderungen

### 4.1 Gespräch (Sprechen und Hören)

| ID | Prio | Anforderung | Begründung |
|---|---|---|---|
| /LF100/ | M | Sprechen per **Push-to-Talk** (Taste, Klick, Tastenkürzel) und per **Freisprechen** mit automatischer Erkennung des Sprechendes (VAD). | Push-to-Talk für laute Umgebungen und Anfänger, Freisprechen für natürliche Gespräche. |
| /LF101/ | M | Die Sprechpause, nach der ein Zug als beendet gilt, ist einstellbar (Standard 1,2 s, bis 5 s). Ohne Zeitlimit für den ganzen Zug. | Langsames Sprechen und Nachdenken dürfen nicht bestraft werden (Persona Hinnerk). |
| /LF102/ | M | Zögern und Füllwörter (えーと, あの) brechen den Zug nicht ab und werden nicht als Fehler gewertet. | Füllwörter gehören zum natürlichen Japanisch. |
| /LF103/ | S | **Unterbrechen**: Fällt man der KI ins Wort, verstummt die TTS, und der neue Zug beginnt. | Natürliche Gesprächsführung. |
| /LF104/ | S | Das **Mikrofon** ist in den Einstellungen wählbar (Standard: das des Systems). Ein **Test** nimmt einen Satz auf, zeigt den Pegel, sagt, ob er zu leise, zu laut oder zu verrauscht ist, spielt ihn vor und zeigt, was die Erkennung verstanden hat; die Aufnahme wird nicht gespeichert. Kommt im Gespräch **kein Ton** an (Stille oder nur leises Rauschen), geht nichts an die Erkennung; die App sagt es und verweist auf den Test. | Die häufigste Ursache schlechter Erkennung ist das falsche oder falsch eingestellte Mikrofon. |
| /LF110/ | M | Die Antworten der KI werden gesprochen (TTS) und als Text angezeigt. Die Schrift ist wählbar: Kana und Kanji mit Furigana, nur Kana oder ausgeblendet (reines Hören). Auch die eigenen Sätze tragen Furigana: sofort die des Wörterbuchs, mit der Antwort die Lesungen des Tutors, der den Satz im Zusammenhang liest. *Bis eine Stimme den Pitch-Akzent trifft (VOICEVOX im lokalen Test, H-01), spricht Kokoro als Übergangsstimme nur die Antworten der Figur, mit Hinweis in der App (H-02, ADR-003).* | Hörverstehen gezielt trainieren, Lesen nur als Stütze. |
| /LF111/ | M | Jede KI-Äußerung lässt sich **wiederholen**, **langsamer abspielen** (0,7×) und **übersetzen** (UI-Sprache). Jede Nutzung wird als Hilfe gezählt (/LF212/). Was die Stimme im Gespräch schon erzeugt hat, bleibt bis zum Ende des Gesprächs zwischengespeichert (höchstens etwa fünf Minuten Ton, Älteres zuerst verworfen); „Nochmal“ spielt dann sofort. | Verstehen sichern, ohne das Gespräch zu verlassen. |
| /LF112/ | M | Ein Klick auf ein Wort zeigt Lesung, Bedeutung und Wortart und spielt die Aussprache ab. Das Wort lässt sich in die Wiederholung übernehmen. | Wortschatz im Kontext. |
| /LF113/ | M | Das **Transkript** der eigenen Äußerung wird angezeigt (was die KI verstanden hat). Liegt die Erkennung daneben, kann man den Zug wiederholen oder ihn als „falsch erkannt“ markieren. | Transparenz, Erkennungsfehler sind keine Lernfehler. |
| /LF114/ | M | Wer feststeckt, bekommt eine **Hilfe** in drei Stufen: Stichwort → Satzanfang → vollständiger Beispielsatz zum Nachsprechen. | Gespräche dürfen nicht an Anfängern scheitern. |
| /LF115/ | M | Es gibt **Szenarien** mit Rolle, Ort, Ziel und Erfolgsbedingung (z. B. „Bestelle zwei Getränke und frag nach der Toilette“). Die Erfolgsbedingung wird vom System erkannt. | Jumpspeak-Prinzip: ein Ziel statt freiem Plaudern. |
| /LF116/ | S | **Freies Gespräch** zu einem gewählten Thema auf dem eigenen Niveau. | Für Fortgeschrittene (Theo, Emi). |
| /LF117/ | S | KI-Figuren mit festem Charakter, Sprechstil und Stimme (z. B. Kellnerin, Kollege, Großmutter). Das Register (です/ます, Umgangssprache, 敬語) folgt der Rolle. | Register üben, Wiedererkennung. |
| /LF120/ | M | **Tippen statt Sprechen** (Kana-IME oder Rōmaji mit Umwandlung). Getippte Züge werden im Lernermodell als „geschrieben“ markiert und zählen nicht als Sprechübung. | Barrierefreiheit, ruhige Umgebung, ohne das Sprechziel zu verwässern (Jonas). |
| /LF121/ | M | Ein Gespräch lässt sich unterbrechen und später an derselben Stelle fortsetzen. | Kurze Zeitfenster (Mara). |
| /LF122/ | S | Lernende dürfen in die UI-Sprache wechseln („Wie sagt man …?“). Die KI antwortet mit der japanischen Form und bittet um Nachsprechen. | Code-Switching ist echtes Lernerverhalten. |

### 4.2 Einstufung und Adaptivität

| ID | Prio | Anforderung | Begründung |
|---|---|---|---|
| /LF200/ | M | **Einstufung** per Gespräch (5–8 min) mit steigender Schwierigkeit. Ergebnis ist ein Profil je Fähigkeit, keine Note. Überspringbar (Start bei Null) und jederzeit wiederholbar. | Startniveau ohne Prüfungsgefühl. |
| /LF201/ | M | Das **Lernermodell** führt getrennte Fähigkeiten: Hörverstehen (Tempo, Wortschatz), Sprechen (Grammatik, Wortschatz, Flüssigkeit, Antwortlatenz), Aussprache (Moren, Akzent), Lesen (Kana, Kanji als Stütze), Register. Jedes Wort und jeder Grammatikpunkt hat einen eigenen Zustand. | Profile sind ungleichmäßig (Selin: Hören hoch, Lesen niedrig). |
| /LF202/ | M | Das Lernermodell ist für Lernende **einsehbar** (Profil-Ansicht in Worten, nicht als Rohwerte) und lässt sich korrigieren („Das kann ich schon“, „Zu schwer“). | Vertrauen und Kontrolle. |
| /LF210/ | M | Der **Schwierigkeitsregler** zielt auf eine Erfolgsquote von etwa 80 % pro Gespräch: verstandene KI-Züge und erreichte Ziele ohne Hilfe. | Der Lernbereich liegt knapp über dem Können (i+1). |
| /LF211/ | M | Stellgrößen: Anteil unbekannter Wörter (≤ 5 %), Satzlänge, Nebensätze, Sprechtempo der TTS (0,7–1,1×), Furigana-Dichte, Register, Hilfe-Angebot und Zahl der Ziele pro Szenario. | Schwierigkeit hat viele Dimensionen. |
| /LF212/ | M | Hilfen (Wiederholung, Zeitlupe, Übersetzung, Hilfe-Stufen) fließen in die Bewertung ein, aber nie als Strafe sichtbar. | Ehrliches Signal ohne Abschreckung. |
| /LF213/ | M | Die Anpassung wirkt **innerhalb** einer Sitzung (sofort leichter nach zwei unverstandenen Zügen) und **zwischen** Sitzungen (Niveau der nächsten Einheit). | Frustration früh abfangen. |
| /LF214/ | M | Die Schwierigkeit steigt schrittweise: Pro Sitzung ändert sich höchstens eine Stellgröße um eine Stufe, außer wenn man selbst eingreift oder das Programm stark unterfordert. | Der Anstieg soll sich gleichmäßig anfühlen. |
| /LF215/ | S | Starke Unterforderung (z. B. Erfolg ≥ 95 % über drei Sitzungen) führt zum **Angebot**, Einheiten zu überspringen (Duolingo „Jump ahead“), mit kurzem Prüfgespräch. | Herkunftssprecher und falsche Anfänger nicht langweilen (Emi). |
| /LF220/ | M | Das **Curriculum** ist ein Graph aus Einheiten. Jede Einheit hat Kann-Beschreibungen, Grammatikpunkte, Kernwortschatz und Szenarien. Für 1.0 gilt N5 und N4 (orientiert an JLPT und JF-Standard A1–A2). | Klarer Pfad wie bei Duolingo, offen genug für Adaptivität. |
| /LF221/ | M | Inhalte (Szenarien, Beispielsätze, Erklärungen) werden **geprüft**: automatisch (Niveau-Check gegen das Wörterbuch, Grammatik-Lint) und vor dem Release durch Muttersprachler-Review pro Einheit. | KI-erzeugte Inhalte enthalten Fehler. |
| /LF230/ | M | **Wiederholung (SRS)** für Wörter, Wendungen und Grammatikmuster aus Gesprächen, Fehlern und /LF112/. Die Wiederholung findet als Mini-Gespräch oder als Nachsprechen statt, nicht als Karteikarte. Planung nach FSRS. | Langfristiges Behalten, das Sprechen bleibt im Mittelpunkt. |
| /LF231/ | S | Wer eine Pause einlegt (≥ 7 Tage), steigt mit einer **Auffrischung** wieder ein, statt mit einem Berg fälliger Wiederholungen. | Wiedereinstieg ohne Frust. |

### 4.3 Verbesserungsvorschläge

| ID | Prio | Anforderung | Begründung |
|---|---|---|---|
| /LF300/ | M | Nach jedem eigenen Zug bewertet das System Grammatik, Wortwahl, Partikeln, Konjugation, Register und Natürlichkeit. | Kernnutzen. |
| /LF301/ | M | Jeder Vorschlag zeigt: eigene Form → bessere Form (als Diff), eine kurze Erklärung in der UI-Sprache und die bessere Form als Audio. *Das Audio kommt erst mit einer Stimme, die den Akzent trifft (H-02).* | Verständlich und zum Nachsprechen. |
| /LF302/ | M | Vorschläge haben einen **Schweregrad**: *verhindert Verständnis*, *falsch*, *unnatürlich*, *Stil*. | Gewichtung statt Rotstift. |
| /LF303/ | M | Bei korrekten Äußerungen gibt es **keinen** Verbesserungsvorschlag. Alternativen, die nur anders wären, erscheinen höchstens als „Auch möglich“. | Fehlalarme zerstören das Vertrauen (/LL110/). |
| /LF304/ | M | Wie viel Feedback angezeigt wird, ist einstellbar: *wenig* (nur Verständnis und Fehler), *normal*, *viel* (inklusive Stil). Der Standard richtet sich nach dem Niveau. | Anfänger nicht erschlagen, Fortgeschrittene fordern. |
| /LF305/ | M | **Recast**: Die KI greift Fehler im Gespräch natürlich auf, indem sie die richtige Form in ihrer Antwort verwendet, ohne den Gesprächsfluss zu stoppen. | Jumpspeak- und Unterrichtsprinzip, implizites Lernen. |
| /LF306/ | M | Nur Fehler mit dem Schweregrad *verhindert Verständnis* unterbrechen das Gespräch mit einer Rückfrage. Alle anderen stehen in der Seitenleiste und in der Bilanz. | Flüssigkeit vor Genauigkeit. |
| /LF307/ | S | Lernende können einem Vorschlag **widersprechen** („Das ist korrekt“). Widersprüche werden gesammelt und fließen in die QA ein. | Fehlkorrekturen finden. |
| /LF310/ | M | **Sitzungsbilanz**: erreichte Ziele, die drei wichtigsten Verbesserungen, neue Wörter, Sprechzeit, Vergleich mit der letzten Sitzung. Neue Lerninhalte gehen in die Wiederholung. | Abschluss und Lernerfolg sichtbar machen. |
| /LF320/ | S | **Aussprache-Feedback** für die Moren-Struktur: Langvokale (おばさん/おばあさん), Doppelkonsonanten (きて/きって), ん, Vokalentstimmung (です, した), Silbenrhythmus. *Für 1.0 (ADR-005): Hinweise nur in Sätzen (J-05) und nur für Kürzungen, also Langvokal zu kurz und っ weggelassen (K-01); sie erscheinen ab 60 % Erkennung insgesamt und 55 % je Art bei höchstens 5 % Fehlalarmen auf echten Stimmen (K-02); 59,7 % bei 4,8 % auf dem App-Weg gelten als erreicht, die Hinweise sind standardmäßig an (L-01). Das eigene Modell `kaiwa-kana-ctc` wird heruntergeladen (K-04 bis K-06).* | Typische Fehler deutschsprachiger Lernender. |
| /LF321/ | S | **Pitch-Akzent**: Soll- und Ist-Kontur eines Wortes oder Satzes als Kurve. Bewertet werden nur Akzente, die Bedeutung unterscheiden oder die Verständlichkeit stören. *1.0 erscheint ohne Pitch-Bewertung (F-09, Vorstudie, L-02). Mit einer akzenttreuen Stimme (H-02) zeigt die App die eigene Kurve neben der Kurve der Figur, ohne Urteil (L-03); die Bewertung folgt nach 1.0 mit eigenem Akzentmodell und eigenen Aufnahmen (L-04).* | Feinschliff ohne Überforderung (Theo). |
| /LF330/ | S | **Nachsprechen und Shadowing**: Satz hören, nachsprechen, eigene Aufnahme und Vorbild vergleichen (Wiedergabe, Kurve, Moren). *Erst mit einer akzenttreuen Stimme (H-02).* | Aussprache gezielt üben. |

### 4.4 Plasmoid und Plasma-Integration

| ID | Prio | Anforderung | Begründung |
|---|---|---|---|
| /LF500/ | M | **Plasmoid** für Panel und Desktop. Kompakt im Panel zeigt es ein Symbol mit Tagesziel-Ring und Serie, ausgeklappt eine Kurzsitzung. | Hauptplattform KDE, Lernen nebenbei. |
| /LF501/ | M | **Kurzgespräch** (3–5 min): ein Mini-Szenario aus der aktuellen Einheit oder der Wiederholung. Start mit einem Klick. | Wenig Zeit (Mara). |
| /LF502/ | M | **Fünf Wiederholungen** als Sprech-Mini-Übung (Hören → Antworten oder Nachsprechen). | Fällige Wiederholungen nebenbei abbauen. |
| /LF503/ | M | Das Plasmoid zeigt dieselben Verbesserungsvorschläge wie die App, aber knapp (eine Zeile pro Vorschlag) und mit Link „In der App ansehen“. | Konsistenz bei kleiner Fläche. |
| /LF504/ | M | Das Plasmoid folgt den Plasma-Widget-Regeln von Kante: Standard ist das Plasma-Theme, Kante und Kante Light sind Opt-in (Einstellung „Stil“). | Kante-Regel. |
| /LF505/ | M | Push-to-Talk funktioniert im Plasmoid per Klick und per globalem Tastenkürzel. | Bedienung ohne Fensterwechsel. |
| /LF506/ | M | Das Plasmoid funktioniert ohne geöffnete App. | Eigenständig nutzbar. |
| /LF507/ | S | Eine Plasmoid-Sitzung lässt sich in der App **fortsetzen** und umgekehrt. | Nahtloser Wechsel. |
| /LF510/ | S | App und Plasmoid teilen Modelle, Profil und Mikrofon (ein Dienst, kein doppeltes Laden). | Speicher und Konsistenz. |
| /LF520/ | S | Erinnerungen über das Benachrichtigungssystem (KNotification), zur selbst gewählten Uhrzeit, mit Aktionen „Jetzt 5 min“ und „Später“. *Freie Uhrzeit (I-07), kommt mit dem Plasmoid (Roadmap M4).* | Gewohnheit aufbauen. |
| /LF521/ | S | Globales Tastenkürzel für Push-to-Talk und „Kurzsitzung starten“ (KGlobalAccel). | Plasma-Integration. |
| /LF522/ | K | KRunner-Plugin: Wort nachschlagen, vorsprechen lassen, in die Wiederholung übernehmen. | Plasma-Integration. |

### 4.5 Motivation und Lernpfad

| ID | Prio | Anforderung | Begründung |
|---|---|---|---|
| /LF600/ | M | **Lernpfad** mit Einheiten, Szenarien und Wiederholungsknoten. Erledigtes, Aktuelles und Gesperrtes ist erkennbar, und zwar nie nur über die Farbe. | Orientierung wie bei Duolingo. |
| /LF601/ | M | **Tagesziel** in Minuten Sprechzeit oder Szenarien, selbst wählbar (Standard 5 min). | Kleine, machbare Ziele. |
| /LF602/ | M | **Serie** (Tage mit erreichtem Tagesziel), mit Freeze (zwei pro Monat, automatisch) und Pausenmodus (Urlaub, Krankheit). | Motivation ohne Erpressung. |
| /LF603/ | S | **XP** für Sprechzeit, erreichte Ziele und Wiederholungen. Getippte Züge zählen weniger, Einwort-Antworten und wiederholte Floskeln zählen nicht voll. | Anreize, die Sprechen belohnen und nicht das Ausnutzen (Jonas). |
| /LF604/ | S | **Wochenrückblick**: Sprechzeit, neue Wörter, Fortschritt pro Fähigkeit. | Fortschritt sichtbar machen. |
| /LF605/ | M | Gamification lässt sich ganz abschalten (keine XP, keine Serie, nur der Pfad). | Nicht alle wollen das (Hinnerk). |
| /LF606/ | M | Es gibt keine Herzen, Leben oder Sperren nach Fehlern. | Fehler gehören zum Sprechenlernen. |
| /LF610/ | S | Statistik: Heatmap der Übungstage, Wortschatz pro Niveau, Sprechzeit, häufigste Fehlerarten im Verlauf. | Selbstreflexion. |
| /LF620/ | M | Benachrichtigungen sind abschaltbar, höchstens eine pro Tag und ohne Schuldformulierungen („Dein Japanisch vermisst dich“ ist verboten). | Respekt vor der Aufmerksamkeit (/LL300/). |

### 4.6 Daten, Profile, Datenschutz

| ID | Prio | Anforderung | Begründung |
|---|---|---|---|
| /LF700/ | K | **Sync** zwischen Geräten über eine Ende-zu-Ende-verschlüsselte Datei auf einem selbst gewählten Speicher (WebDAV, Nextcloud, Ordner), ohne Kaiwa-Server. | Geräte wechseln, ohne die Daten abzugeben (Jonas, Lena). |
| /LF710/ | M | **Export** aller eigenen Daten (Profil, Verlauf, Wiederholung) als offene Datei (JSON) sowie **Import** und **vollständige Löschung**. | DSGVO, Datenhoheit. |
| /LF711/ | M | Die Ansicht **„Was verlässt das Gerät?“** zeigt je Baustein (STT, LLM, TTS) den Anbieter, ob er lokal oder in der Cloud läuft, und welche Daten (Audio, Text) übertragen werden. | Transparenz (Lena). |
| /LF712/ | M | Aufnahmen werden standardmäßig **nicht** gespeichert. Speichern ist ein Opt-in für Vergleich und Fortschritt, mit eigener Aufbewahrungsdauer. | Datensparsamkeit. |
| /LF713/ | M | API-Schlüssel liegen im System-Schlüsselbund (KWallet, Secret Service, Windows Credential Manager, macOS Keychain), nie im Klartext. | Sicherheit. |
| /LF714/ | S | **Kostenbremse** für Cloud-Anbieter: geschätzte Kosten pro Sitzung und Monat, mit einem Limit, das man selbst setzt. *Kein Limit voreingestellt (I-06); die Preise prüft Claude zu jedem Release (I-05).* | Keine Überraschungen. |
| /LF720/ | S | Mehrere Profile pro Gerät (z. B. Familie), jeweils getrennt. *Ohne Schutz der Profile gegeneinander (I-08).* | Gemeinsamer Rechner. |

---

## 5. Produktdaten

| ID | Prio | Daten | Inhalt |
|---|---|---|---|
| /LD010/ | M | Profil | Name, UI-Sprache, Ziele, Einstellungen, Einstufung, Datenschutz-Entscheidungen |
| /LD020/ | M | Lernermodell | Zustand je Fähigkeit, Wort (FSRS: Stabilität, Schwierigkeit, fällig), Grammatikpunkt, Fehlerart; Verlauf der Stellgrößen |
| /LD030/ | M | Sitzungen | Szenario, Züge (Transkript, Text der KI, Korrekturen, Hilfen, Latenzen), Ergebnis; Audio nur bei Opt-in (/LF712/) |
| /LD040/ | M | Curriculum | Einheiten, Kann-Beschreibungen, Grammatik, Wortschatz, Szenarien, Figuren; versioniert und mitgeliefert |
| /LD050/ | M | Sprachdaten | Wörterbuch (JMdict), Kanji (KANJIDIC2), Morphologie (UniDic), Akzent, JLPT-Zuordnung, jeweils mit Lizenzangabe. *Als eigenes Paket `kaiwa-data`, GitHub-Release (L-06); JLPT-Listen von tanos.co.uk nur mit im Webarchiv bestätigter Lizenz, sonst eigene Zuordnung (L-09).* |
| /LD060/ | S | Widersprüche | Vorschläge, denen widersprochen wurde (/LF307/), lokal und nur auf Wunsch exportiert |

Mengengerüst: bis 10 000 Sitzungen und 20 000 Lernobjekte pro Profil ohne spürbare Verzögerung (/LL020/).

---

## 6. Nichtfunktionale Anforderungen

### 6.1 Leistung

| ID | Prio | Anforderung |
|---|---|---|
| /LL010/ | M | **Antwortlatenz** (Sprechende → erster Ton der KI): p50 ≤ 1,5 s und p95 ≤ 3 s mit Cloud-Anbietern; p50 ≤ 3 s lokal auf Referenzhardware (Framework Laptop 13, AMD Ryzen 7 7840U, 32 GB, ohne dGPU). Bis zur Antwort zeigt das System sichtbar „hört zu / denkt nach“. |
| /LL011/ | M | Das Plasmoid öffnet in ≤ 300 ms (Dienst läuft) und ist ≤ 2 s nach Start bereit zum Sprechen. |
| /LL012/ | S | Speicher im Leerlauf ≤ 150 MB (ohne geladene Modelle). Modelle werden nach Inaktivität entladen (einstellbar). |
| /LL020/ | M | Profil-Ansicht, Lernpfad und Statistik laden in ≤ 500 ms bei der Datenmenge aus Kap. 5. |

### 6.2 Qualität der Sprachverarbeitung

| ID | Prio | Anforderung |
|---|---|---|
| /LL100/ | M | Die STT-Zeichenfehlerrate (CER) für Lernendensprache auf dem Testkorpus liegt ≤ 12 % (lokal) und ≤ 8 % (Cloud). Erkennungsfehler werden nicht als Lernfehler gewertet, wenn das System unsicher ist (Konfidenz). *Lokal ist ReazonSpeech Standard (H-03, ADR-002); Cloud-STT wird erst gemessen, wenn ein Konto dazukommt (F-03).* |
| /LL110/ | M | **Fehlalarmrate** der Verbesserungsvorschläge (Vorschlag bei korrekter Äußerung, Schweregrad ≥ *falsch*) ≤ 5 % auf dem Referenzkorpus und in den Persona-Läufen. In M1 ist ein Zwischenziel von ≤ 10 % zulässig. |
| /LL111/ | M | **Trefferquote** für eingebaute Fehler der Personas (Kap. 9) ≥ 85 % bei *falsch* und *verhindert Verständnis*. |
| /LL112/ | M | Erklärungen stehen immer in der UI-Sprache und sind im Niveau der Lernenden formuliert. Japanische Beispiele stehen mit Lesung dabei. |

### 6.3 Barrierefreiheit

| ID | Prio | Anforderung |
|---|---|---|
| /LL200/ | M | Vollständig per Tastatur bedienbar, mit sichtbarem Fokus (Kante: cyan, 2 px). |
| /LL201/ | M | Screenreader: Alle Bedienelemente haben Accessible-Namen und -Rollen (AT-SPI unter Linux). Das ist zugleich die Grundlage der GUI-Automatisierung in der QA. |
| /LL202/ | M | Die Schriftgröße folgt dem System und ist zusätzlich in der App skalierbar (bis 160 %, Furigana mitskaliert; Eigentümer, 10.11.2026: 160 % ist vorerst die obere Grenze). |
| /LL203/ | M | Kontraste erreichen WCAG AA in System, Kante und Kante Light. Zustände sind nie nur an der Farbe erkennbar. |
| /LL204/ | S | Untertitel für alles Gesprochene. Für Menschen mit Hörminderung gibt es eine Lautstärkeanhebung und eine Klangoption „klar“ (höhere Sprachverständlichkeit). |

### 6.4 Gestaltung und Ethik

| ID | Prio | Anforderung |
|---|---|---|
| /LL300/ | M | Keine Dark Patterns: keine künstliche Verknappung, keine Schuld- oder Angst-Formulierungen, keine Belohnung für Ausnutzen. |
| /LL301/ | M | Die GUI wird aus **Kante** erzeugt (siehe Kap. 8). |
| /LL302/ | M | Die KI-Figuren bleiben im Lernkontext. Bei unangemessenen Eingaben lenkt die Figur freundlich zurück, ohne zu moralisieren. Inhalte sind ab 14 geeignet. |
| /LL303/ | M | Das Fenster ist mindestens 800 px breit (Eigentümer, 10.11.2026); jede Ansicht ist bei dieser Breite vollständig bedienbar. |

### 6.5 Datenschutz und Sicherheit

| ID | Prio | Anforderung |
|---|---|---|
| /LL400/ | M | **Lokal zuerst**: Alle Muss-Funktionen sind mit lokalen Bausteinen nutzbar. Cloud-Anbieter sind Opt-in pro Baustein (STT, LLM, TTS getrennt). |
| /LL401/ | M | Im Nur-lokal-Modus gibt es **keine** ausgehenden Verbindungen außer zu selbst gewählten Diensten (Update-Prüfung nur mit Zustimmung). Das ist in der QA nachweisbar (Persona Lena). |
| /LL402/ | M | DSGVO-Konformität: Zweckbindung, Löschung, Export und Auskunft über die Anbieter (/LF710/, /LF711/). |

### 6.6 Plattformen und Wartbarkeit

| ID | Prio | Anforderung |
|---|---|---|
| /LL500/ | M | **Hauptplattform:** KDE Plasma 6 (aktuelle und vorige Minor-Version), Wayland und X11. Pakete: Flatpak (App), AUR, openSUSE/Fedora/Debian-Pakete (App + Plasmoid). *App-ID `org.shrippen.kaiwa` (G-08); `kaiwa-git` kommt mit 1.0 ins AUR (I-09).* |
| /LL501/ | S | Windows 10/11 und macOS (aktuelle und vorige Version): App ohne Plasmoid. |
| /LL502/ | K | Android (Kirigami), Mindestversion nach ADR. |
| /LL503/ | M | Eine Codebasis: Rust-Kern und Qt-6-QML-Oberfläche. Plattformspezifisches liegt hinter Schnittstellen (Audio, Schlüsselbund, Benachrichtigung, Dienst). |
| /LL504/ | M | Anbieter für STT, LLM und TTS sind austauschbar (Trait). Ein neuer Anbieter braucht keine Änderung an Kern oder GUI. |
| /LL505/ | M | Der Kern ist ohne GUI testbar (Headless-API, `kaiwa-sim`). Die Testabdeckung der Kern-Crates liegt bei ≥ 80 % Zeilen. |
| /LL506/ | M | Mehrsprachigkeit der UI über die KDE-Infrastruktur (ki18n/gettext). Für 1.0 gibt es de und en. |

---

## 7. Benutzungsoberfläche

- **App-Ansichten:** Start (heute: Tagesziel, nächstes Szenario, Wiederholung), Lernpfad, Gespräch, Wiederholung, Bilanz, Profil und Statistik, Einstellungen (Anbieter, Mikrofon, Feedback-Menge, Schrift, Stil, Datenschutz).
- **Gesprächsansicht:**
  - links Szenario und Ziele (`KanteSteps`)
  - Mitte Gesprächsverlauf (`KanteMessage` mit Furigana)
  - rechts oder unten Verbesserungsvorschläge (Diff, Schweregrad als Tier)
  - unten Push-to-Talk-Taste mit Pegel, Hilfe-Taste und Tippen
- **Plasmoid:**
  - kompakt: Symbol mit Ring (Tagesziel) und Seriezahl
  - ausgeklappt: Szenario-Titel, zwei bis drei letzte Züge, Push-to-Talk, eine Zeile Feedback, „In der App fortsetzen“
- **Zustände** der Push-to-Talk-Taste: bereit, hört zu, denkt nach, spricht, Fehler. Jeder Zustand ist an Form und Text erkennbar, nicht nur an der Farbe.
- **Stil:** System (Standard), Kante, Kante Light, nach den „Rules for apps“ und „Plasma widget rules“ in Kante.

---

## 8. Technische Rahmenbedingungen (Vorgaben)

| Bereich | Vorgabe |
|---|---|
| Sprache | **Rust** (stable, Edition 2024) für Kern, Sprachverarbeitung, Dienst und QA-Läufer |
| GUI | **Qt 6 / QML** mit **Kirigami**, Brücke Rust ↔ QML über **cxx-qt** (Entscheidung in ADR-001) |
| Designsystem | **Kante**: `qml/Kante` und `qml/KantePlasma` unverändert übernommen (Kopie oder qrc), keine eigenen Farben, Schriften, Größen oder Varianten. Fehlende Bausteine kommen zuerst nach Kante (GUI-Regel, `AGENT-RULE.md`). |
| Plasma | Plasmoid als Plasma-6-Paket (QML), Backend über das QML-Modul `org.shrippen.kaiwa` oder den D-Bus-Dienst `kaiwad` (ADR-008). Zeitlich nach den anderen Plattformen, wenn die App nahe 1.0 ist (Roadmap M4) |
| Sprache (Modelle) | Spracherkennung ReazonSpeech über sherpa-onnx (ADR-002), Übergangsstimme Kokoro (ADR-003), Moren-Prüfung mit dem eigenen Modell `kaiwa-kana-ctc` (ADR-005). Modelle werden heruntergeladen, wie das lokale Sprachmodell mit Quelle, Größe und Lizenz vorher angezeigt (F-08), nie im Repository oder Programm |
| Daten | SQLite, ein Profil = eine Datei, Migrationen versioniert |
| KI-Anbieter | Lokal: OpenAI-kompatible Schnittstelle (llama.cpp-Server, Ollama). Cloud: z. B. Anthropic Claude. Strukturierte Ausgabe per JSON-Schema. |
| Lizenz | MIT (wie Kante) für den Code. Datenlizenzen in `NOTICE`, Daten als getrenntes Paket (ADR-007). Abhängigkeiten müssen mit MIT verträglich sein: keine (A)GPL-Bibliotheken im ausgelieferten Programm, LGPL nur dynamisch gelinkt (Qt, Kirigami). |
| Repository | Gitea `git.arianw.de/shrippen/kaiwa`, GitHub nur als Spiegel, Änderungen per PR |

---

## 9. Qualitätssicherung mit Personas

Die Personas in [`qa/personas/`](../qa/personas/) sind **ausführbare Spezifikationen**. `kaiwa-sim` spielt sie automatisch durch. Ablauf und Format: [`qa/README.md`](../qa/README.md).

| ID | Prio | Anforderung |
|---|---|---|
| /LL600/ | M | Jede Persona hat ein Startprofil, ein Lernverhalten, typische Fehler (eingebaut und markiert), ein Sprechprofil (für synthetische Sprache) und Reisen mit Prüfregeln, die auf Anforderungs-IDs verweisen. |
| /LL601/ | M | `kaiwa-sim` simuliert Lernende in drei Stufen: **Text** (Headless-API), **Audio** (TTS mit Akzentprofil und Störgeräusch → echte STT-Pipeline), **GUI** (AT-SPI-Automatisierung von App und Plasmoid, z. B. mit selenium-webdriver-at-spi). |
| /LL602/ | M | Langzeitläufe mit beschleunigter Uhr (30 simulierte Tage) prüfen Adaptivität, SRS und Serie. |
| /LL603/ | M | Die Läufe sind reproduzierbar (fester Seed, aufgezeichnete Antworten der Anbieter als Fixtures für CI, Live-Läufe nächtlich). |
| /LL604/ | M | Der Bericht pro Lauf enthält: bestandene und gescheiterte Prüfregeln mit IDs, Fehlalarm- und Trefferquote, Latenzen, Kurven der Stellgrößen und Screenshots pro Sprache. Die Screenshots sind zugleich Material für das Release-Log. |

---

## 10. Abnahmekriterien (1.0)

1. Alle Muss-Anforderungen sind umgesetzt und mindestens einer Persona-Reise zugeordnet.
2. Alle Personas bestehen ihre Reisen im Text- und Audio-Modus und die GUI-Reisen der Hauptplattform, jeweils über 30 simulierte Tage.
3. /LL010/, /LL100/, /LL110/ und /LL111/ sind auf Referenzhardware und mit einem Cloud-Anbieter erreicht.
4. Muttersprachler-Review aller Einheiten N5 und N4 ist abgeschlossen (/LF221/). Wer es übernimmt, ist noch offen (F-05).
5. Der Nur-lokal-Modus ist ohne ausgehende Verbindungen nachgewiesen (/LL401/).
6. Die Landing Page zeigt das Release-Log 1.0 nach der Release-Regel.

---

## 11. Glossar

| Begriff | Bedeutung |
|---|---|
| **Recast** | Die KI verwendet die korrigierte Form in ihrer Antwort, ohne den Fehler ausdrücklich zu benennen. |
| **i+1** | Input knapp über dem aktuellen Können (Krashen). |
| **FSRS** | Free Spaced Repetition Scheduler, Algorithmus für Wiederholungsintervalle. |
| **More** | Japanische Zeit- und Silbeneinheit (z. B. と・う・きょ・う = 4 Moren). |
| **Pitch-Akzent** | Tonhöhenverlauf, der im Japanischen Bedeutungen unterscheidet (はし↗ Brücke / は↘し Essstäbchen). |
| **Register** | Sprechstil: Umgangssprache, です/ます-Form, 敬語 (Höflichkeitssprache). |
| **VAD** | Voice Activity Detection, erkennt Sprechbeginn und -ende. |
| **Plasmoid** | Widget für KDE Plasma (Panel, Desktop). |
| **Kante** | Designsystem der shrippen-Projekte (`shrippen/Kante`). |
| **Persona-Reise** | Ablauf einer Persona mit Schritten und Prüfregeln, von `kaiwa-sim` ausgeführt. |

---

## 12. Änderungen

| Version | Datum | Änderung |
|---|---|---|
| 0.1 | 2026-10-09 | Erster Entwurf, Lizenz MIT (F-02) |
| 0.2 | 2026-10-11 | Entscheidungen der Fragenpakete bei den Anforderungen vermerkt (H-02, H-03, F-03, F-05, F-08, F-09, G-08, I-05 bis I-09, J-05, K-01 bis K-06); Modelle in Kap. 8; Rückverfolgung erzeugt |
| 0.3 | 2026-10-12 | Fragenpaket 7: Moren-Hinweise standardmäßig an (L-01, /LF320/), Pitch nach 1.0 und Kurve ohne Urteil (L-02 bis L-04, /LF321/), Datenpaket und JLPT-Listen (L-06, L-09, /LD050/) |
