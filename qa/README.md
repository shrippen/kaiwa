# QA mit Personas

Die Personas in [`personas/`](personas/) sind **ausführbare Spezifikationen** (Lastenheft Kap. 9, /LL600/–/LL604/). Jede Persona beschreibt eine simulierte lernende Person:

- Startniveau als Wahrheit, gegen die die Einstufung geprüft wird
- Lernverhalten: wann, wie lange, auf welchem Gerät, Pausen, Abbruchgründe
- Sprechprofil für die synthetische Stimme: Akzent, Tempo, Pausen, Störgeräusch, Aussprachefehler
- eingebaute Sprachfehler mit dem erwarteten Schweregrad und einer Lernkurve (`fades_after`)
- Reisen mit Schritten und Prüfregeln, die auf Anforderungs-IDs verweisen
- `never`: Dinge, die dieser Persona nie passieren dürfen

Format: [`personas/schema.json`](personas/schema.json) (JSON Schema 2020-12). Freitexte sind Deutsch, Schlüssel und Enums Englisch. Fünf der sieben Personas stammen aus der gemeinsamen Demo-Welt *Studio Weber* (`shrippen.github.io/demo/world`, Feld `demo_world_id`), damit Screenshots und Daten zu den anderen shrippen-Projekten passen.

| Persona | Schwerpunkt der Prüfung | Oberflächen |
|---|---|---|
| [Mara Weber](personas/mara.json) | Anfängerin, Kurzsitzungen, Unterbrechen und Fortsetzen, wenig Feedback | Plasmoid, App |
| [Selin Aydın](personas/selin.json) | ungleichmäßiges Profil, Register, Windows | App (Windows, Plasma) |
| [Theo Lindqvist](personas/theo.json) | Fehlalarme, Pitch-Akzent, Widerspruch, nur Tastatur, lokale Latenz | App, Plasmoid |
| [Hinnerk Aalders](personas/hinnerk.json) | Barrierefreiheit, langsames Sprechen, einfache Sprache, keine Gamification | App |
| [Emi Hartmann](personas/emi.json) | Herkunftssprecherin, Obergrenze, Überspringen, 敬語 | App, Android |
| [Jonas Brandt](personas/jonas.json) | Anti-Gaming, Serie und Freeze, Störgeräusche, Gerätewechsel, Sync | Android, Plasmoid, App |
| [Lena Kraus](personas/lena.json) | Nur lokal, Netzwerkmitschnitt, DSGVO, schwache Hardware | App, Plasmoid |

## Wie `kaiwa-sim` eine Persona spielt

```
persona.json ──▶ Lernenden-Agent ──▶ Äußerung (Text) ──┬─[text]──▶ Headless-API ─┐
   │              (LLM, an Wortschatz,  + markierte      │                         │
   │               Grammatik und Fehler  Fehler          └─[audio]─▶ TTS mit Akzent│
   │               der Persona gebunden)                        + Störgeräusch    │
   │                                                            ▶ echte STT ──────┤
   │                                                    [gui] AT-SPI-Treiber ─────┤
   │                                                                              ▼
   └──────────── Prüfregeln (checks, never) ◀──── Ereignisprotokoll ◀── Kaiwa (App, Plasmoid, kaiwad)
                         │
                         ▼
                Bericht: bestanden/gescheitert je ID, Fehlalarm- und Trefferquote,
                Latenzen, Stellgrößen-Kurven, Screenshots (de, en)
```

1. **Lernenden-Agent:** Ein LLM spielt die Persona. Es darf nur Wörter und Grammatik aus `start_level` plus dem bisher Gelernten verwenden (Wortliste aus dem Lernermodell der Simulation). Eingebaute Fehler setzt es mit ihrer `rate` ein und markiert sie, mit sinkender Rate nach `fades_after` Rückmeldungen. So bleibt das Verhalten glaubwürdig und auswertbar.
2. **Modi:**
   - `text` ruft die Headless-API auf, schnell, für CI.
   - `audio` erzeugt die Äußerung mit einer TTS-Stimme nach `speech.voice_hint`, verändert Tempo und Pausen, mischt Störgeräusche auf `snr_db` und gibt sie in die echte STT-Pipeline.
   - `gui` bedient App und Plasmoid über den Accessibility-Baum (AT-SPI, z. B. [selenium-webdriver-at-spi](https://invent.kde.org/sdk/selenium-webdriver-at-spi)) und speist Audio über ein virtuelles PipeWire-Mikrofon ein.
3. **Zeit:** `sim_days` läuft mit einer beschleunigten Uhr (Kern-API `Clock`), die Sitzungen folgen `learning.sessions` und `breaks`.
4. **Prüfregeln:** `checks[].assert` ist die lesbare Regel, ihre Maschinenform steht in `crates/kaiwa-sim/checks/<id>.rs`. `never` gilt während des ganzen Laufs.
5. **Determinismus:** fester Seed pro Lauf. In der CI werden die Antworten der Anbieter aus Fixtures abgespielt, nächtlich laufen sie live gegen lokale Modelle und einen Cloud-Anbieter.

## Lange Reisen über simulierte Tage

Reisen mit `sim_days` haben ein Tages-Skript (`qa/scripts/<journey>.json` mit `"days"`): Startdatum, Seed, Tagesziel und Sonderfälle (Sitzung über Mitternacht, Zeitzonenwechsel). `kaiwa-sim` spielt die Persona dann auf der Ebene der Züge: Sie hat pro Fähigkeit ein wahres Niveau (`start_level`), ein Zug gelingt mit 80 % Wahrscheinlichkeit, wenn das Material auf diesem Niveau liegt, öfter darunter, seltener darüber. Eingebaute Fehler kommen mit ihrer `rate` und lassen nach `fades_after` Korrekturen nach. Alles andere ist der echte Lernkern: Lernermodell, Schwierigkeitsregler, Curriculum, FSRS, Tagesziel, Serie und XP.

```sh
cargo run -p kaiwa-sim -- days --persona mara --journey mara_30_days   # Sitzung für Sitzung ansehen
```

## Befehle (Zielbild)

```sh
cargo run -p kaiwa-sim -- validate                       # Schema und Verweise auf Anforderungs-IDs prüfen
cargo run -p kaiwa-sim -- run --persona mara --mode text # eine Persona, alle Reisen im Textmodus
cargo run -p kaiwa-sim -- run --all --mode audio --days 30 --report out/
cargo run -p kaiwa-sim -- run --persona lena --journey lena_local_only --netns --capture
```

## Regeln

- Jede Muss-Anforderung aus Kap. 4 und 6.1–6.5 des Lastenhefts steht in mindestens einer Prüfregel; `validate` meldet Lücken. Ausgenommen sind Daten (Kap. 5, über Unit-Tests und `lena_export`), Plattform- und Architekturvorgaben (6.6), die QA selbst (Kap. 9) und das Review /LF221/.
- Eine neue Funktion ist erst fertig, wenn eine Reise sie prüft (Definition of Done, Roadmap).
- Personas ändern sich nur zusammen mit ihren Prüfregeln. Wahrheit (`start_level`) und Fehlerlisten dürfen nicht an die Ergebnisse angepasst werden, damit ein Lauf grün wird.
- Namen und Ids aus der Demo-Welt bleiben stabil. Ändert sich dort etwas, wird hier nachgezogen.
- Screenshots aus `gui`-Läufen (de und en) sind das Material für das Release-Log (Release-Regel).
