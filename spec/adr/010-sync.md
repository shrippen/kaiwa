# ADR-010: Sync zwischen Geräten

**Status:** Vorgeschlagen (Schritt 1 bis 4 ohne Netz; Schritt 5 wartet auf die Bestätigung der Schichten durch den Eigentümer) · **Bezug:** /LF700/, /LF507/, /LF710/, /LF711/, /LF713/, `jonas_sync`, `jonas_local_switch`

## Kontext

Ein Profil soll auf mehreren Geräten aktuell sein, über eine Ende-zu-Ende-verschlüsselte Ablage, die der Lernende selbst wählt (WebDAV, Nextcloud, ein Ordner), ohne Kaiwa-Server (/LF700/). `jonas_sync` verlangt: Am Abend sind Tagesziel, Serie, Wiederholung und die offene Sitzung auf dem PC aktuell, Konflikte werden zusammengeführt, nicht überschrieben.

Das Profil ist eine SQLite-Datei. Lernstand (`learning`), Karten (`items`) und Tage (`days`) entstehen schrittweise aus Gesprächen, Wiederholungen, Einstufung und Überspringen. Sie lassen sich nicht Feld für Feld zusammenführen. Die Tageszähler (`DayRecord`: Wiederholungen, XP, Sprechzeit) gingen mit „das Größere gewinnt“ verloren und würden mit „Summe“ doppelt gezählt. Lernermodell und Regler hängen von der Reihenfolge ab (gleitende Mittel, Fenster der letzten Antworten).

## Optionen

| Option | Für | Gegen |
|---|---|---|
| A: Profildatei hochladen, der Neueste gewinnt | einfach | überschreibt, was das andere Gerät geübt hat; widerspricht `jonas_sync` |
| B: je Tabelle eine eigene Regel (Vereinigung, Neuestes, Maximum) | ohne Umbau | Tageszähler, Serie und Lernermodell werden falsch; jede Regel ist eine zweite Lernregel neben dem Kern |
| **C: Journal: jede Handlung des Lernenden ist ein Ereignis; der Zustand ist die Faltung des Journals durch dieselben Befehle wie live** | Zusammenführen heißt: Ereignisse vereinigen, ordnen, neu falten; jede Regel bleibt einmal im Kern | Umbau aller schreibenden Befehle; ein vergessener Schreibweg lässt Faltung und Live-Zustand auseinanderlaufen |

## Entscheidung

**C.**

- **Ereignisse.** Neue Tabelle `events` (Id `<Gerät>-<Nummer>`, Gerät, laufende Nummer, Zeitpunkt mit Ortszeit-Versatz, Inhalt als JSON). Die Geräte-Id entsteht einmal zufällig und steht in den Einstellungen. Die Ereignisarten (`Event` in `kaiwa-core`, reine Daten) folgen den schreibenden Befehlen des Backends: Gespräch begonnen, gespeichert, beendet, korrigiert; Wiederholung beantwortet; Einstufung; Überspringen; Wort in die Wiederholung; Einstellung; Pause; Widerspruch; Kosten gebucht. Das erste Ereignis ist `Base`, der Stand beim Beitritt.
- **Ein Weg.** Live schreibt ein Befehl erst das Ereignis und wendet es dann an (`journal::apply` im Backend, das Szenarien, Curriculum und Grammatik kennt). Das Neufalten ruft dasselbe `apply`. Der Test „das Journal neu gefaltet ergibt denselben Export wie live“ läuft in jedem Schritt.
- **Ordnung.** Nach Zeitpunkt, dann Gerät, dann Nummer. Kommen fremde Ereignisse dazu, werden die abgeleiteten Tabellen in einer Transaktion geleert und ab `Base` neu gefaltet, nie während eines laufenden Zuges.
- **Ids.** Gespräche bekommen die Id ihres Start-Ereignisses (Text statt fortlaufender Zahl). Karten (`norm(text)`) und Tage (Tagesnummer) sind schon geräteübergreifend eindeutig.
- **Konflikte.**
  - Lernstand, Karten, Tage, Serie, Kosten: keine eigene Regel, sie sind das Ergebnis der Faltung. Zwei Geräte am selben Tag ergeben die Summe ihrer Übung.
  - Einstellungen: je Schlüssel die späteste. Gerätebezogene Einstellungen (Tutor, Mikrofon, Entladezeit; welche genau, entscheidet der Eigentümer) gehen nicht ins Journal; sonst bricht `jonas_local_switch`.
  - Offene Sitzung: je Sitzung der späteste gespeicherte Stand; ein beendetes Gespräch schlägt ein offenes. Führen zwei Geräte dieselbe offene Sitzung getrennt weiter, gewinnt die spätere. Das ist der eine benannte Verlust.
  - Pausen und Widersprüche: Vereinigung über die Ereignis-Id.
- **Ablage.** Eine versiegelte Datei je Gerät im gewählten Ordner; jedes Gerät schreibt nur seine eigene und liest die der anderen. Damit braucht es keine Sperre und kein ETag, und ein Ordner, den ein anderes Programm abgleicht (Syncthing), geht genauso. Beitritt: Das erste Gerät legt `base` ab (sein Stand); ein weiteres tritt nur mit leerem Profil bei, wie beim Import.
- **Verschlüsselung.** Format `age` mit Passphrase (scrypt). Kein eigenes Format; der Lernende kann seine Dateien mit dem `age`-Werkzeug selbst öffnen (/LF710/). Passphrase und WebDAV-Passwort liegen im Schlüsselbund wie die Tutor-Schlüssel (/LF713/).
- **Transport.** Neuer Baustein `kaiwa-sync` unterhalb des Backends, nur Bytes: `Remote` mit `list`, `get`, `put`, umgesetzt für einen Ordner, für WebDAV (PROPFIND mit Tiefe 1, GET, PUT) und im Speicher für Tests. Das Backend reicht Passphrase und Zugangsdaten hinein; `kaiwa-sync` hängt nicht an `kaiwa-ai`.
- **Versionen.** Jede Datei trägt die Formatversion. Ein Gerät mit älterem Stand lehnt neuere Ereignisse ab und sagt es dem Lernenden, statt anders zu falten.

## Schritte

1. Diese ADR; Tabelle `events` mit Geräte-Id; `Event` im Kern; `journal::apply` für Wiederholung und Einstellung; Test „Faltung = live“.
2. Gespräche als Ereignisse über `Session::restore`; Gesprächs-Id als Text; Export `kaiwa-export/4`.
3. Die übrigen Ereignisse; danach schreibt nichts mehr am Journal vorbei.
4. `Backend::merge` und Beitritt, mit zwei Profilen getestet: beide üben am selben Tag getrennt, danach sind beide gleich und die Tageszähler die Summe.
5. Baustein `kaiwa-sync` mit Ordner, Speicher und `age`; `Backend::sync_now`; `jonas_sync` in `kaiwa-sim` über einen Ordner. Ändert die Schichten in `ARCHITECTURE.md`: der Eigentümer bestätigt.
6. WebDAV, Kachel in den Einstellungen, Zeile in „Was verlässt das Gerät?“ (/LF711/).

## Risiken

- Ein Schreibweg ohne Ereignis: Faltung und Live-Zustand laufen auseinander. Der Test aus Schritt 1 zeigt es.
- Geräte mit verschiedenem Programmstand falten verschieden: Formatversion, siehe oben.
- Eine falsch gehende Uhr verschiebt die Reihenfolge; das Ergebnis bleibt auf allen Geräten gleich, kann aber überraschen.
- Die Korrektur nach dem Ende (V-05) ersetzt im Journal das beendete Gespräch; die gespeicherte Aufzeichnung davor kann dann entfallen. Eigener Schritt.
- Größe: Transkripte liegen im Journal; verdichtet (neue `base`) wird erst, wenn es messbar stört.

## Offen für den Eigentümer

- Welche Einstellungen gelten je Gerät (Vorschlag: Tutor, Mikrofon, Entladezeit, Lauter, Klar, Freisprechen) und welche je Profil?
- Schritt 5 fügt den Baustein `kaiwa-sync` in die Schichten ein.
