# Rückverfolgung: Anforderung → Code → Prüfregel

Erzeugt mit `cargo run -p kaiwa-sim -- trace --write`; ein Test in der CI prüft, dass die Datei aktuell ist. Nicht von Hand bearbeiten.

- **Code:** Crates (`core` = `kaiwa-core` …) und `app`, deren Quelltext die ID nennt.
- **Stand:** *geprüft* = mindestens eine Prüfregel läuft automatisch und ist grün; *Code* = der Code nennt die ID, aber keine Prüfregel läuft automatisch; *offen* = weder noch; *Vorgabe* = Plattform- oder Architekturvorgabe (/LL5xx/) ohne Prüfregel, geprüft über CI und Pakete; *rot* = eine automatische Prüfregel scheitert. *Geprüft* heißt nicht, dass die Anforderung ganz erfüllt ist: Prüfregeln für Audio, GUI und den echten Tutor laufen noch nicht automatisch. Die Prüfregeln an der gebauten App (`hinnerk_narrow`, `hinnerk_font`, `lena_idle_memory`, `lena_zero_egress`) laufen im Qt-Job der CI (`kaiwa-sim run --app`) und zählen darum als grün.
- Was bewusst ruht (Plasmoid auf dem Zweig `later/plasmoid`), steht in [`STAND.md`](STAND.md).

| Priorität | geprüft | Code | offen | Vorgabe | rot |
|---|---|---|---|---|---|
| Muss | 53 | 19 | 5 | 3 | 0 |
| Soll | 13 | 6 | 5 | 0 | 0 |
| Kann | 0 | 0 | 2 | 1 | 0 |

| ID | Prio | Anforderung | Code | Stand | Prüfregeln |
|---|---|---|---|---|---|
| /LD010/ | M | Profil | store | Code | – |
| /LD020/ | M | Lernermodell | core, store | Code | – |
| /LD030/ | M | Sitzungen | core, store | Code | – |
| /LD040/ | M | Curriculum | core | Code | – |
| /LD050/ | M | Sprachdaten | app, backend, lang | Code | – |
| /LD060/ | S | Widersprüche | core, store | Code | – |
| /LF100/ | M | Sprechen per Push-to-Talk (Taste, Klick, Tastenkürzel) und per … | app, core, speech, store | geprüft | 1 von 1 automatisch grün: `mara_ptt` |
| /LF101/ | M | Die Sprechpause, nach der ein Zug als beendet gilt, ist einstellbar … | app, core, store | geprüft | 1 von 1 automatisch grün: `hinnerk_no_cutoff` |
| /LF102/ | M | Zögern und Füllwörter (えーと, あの) brechen den Zug nicht ab und werden … | core | geprüft | 1 von 1 automatisch grün: `hinnerk_no_cutoff` |
| /LF103/ | S | Unterbrechen: Fällt man der KI ins Wort, verstummt die TTS, und der … | app | Code | – |
| /LF104/ | S | Das Mikrofon ist in den Einstellungen wählbar (Standard: das des … | app | Code | – |
| /LF110/ | M | Die Antworten der KI werden gesprochen (TTS) und als Text angezeigt. | app, backend, core, lang, store | geprüft | 2 von 3 automatisch grün: `mara_listen_tools`, `mara_own_furigana`, `mara_word_lookup` |
| /LF111/ | M | Jede KI-Äußerung lässt sich wiederholen, langsamer abspielen (0,7×) … | ai, app, core, speech | geprüft | 1 von 1 automatisch grün: `mara_listen_tools` |
| /LF112/ | M | Ein Klick auf ein Wort zeigt Lesung, Bedeutung und Wortart und spielt … | app, backend, core, lang, store | geprüft | 1 von 1 automatisch grün: `mara_word_lookup` |
| /LF113/ | M | Das Transkript der eigenen Äußerung wird angezeigt (was die KI … | app, backend, core | geprüft | 1 von 2 automatisch grün: `emi_misrecognition`, `jonas_noise` |
| /LF114/ | M | Wer feststeckt, bekommt eine Hilfe in drei Stufen: Stichwort → … | core | geprüft | 1 von 1 automatisch grün: `mara_hint_offered` |
| /LF115/ | M | Es gibt Szenarien mit Rolle, Ort, Ziel und Erfolgsbedingung (z. B. … | core | Code | 0 von 1 automatisch grün: `jonas_follow_up` |
| /LF116/ | S | Freies Gespräch zu einem gewählten Thema auf dem eigenen Niveau. | app, backend, core | geprüft | 1 von 1 automatisch grün: `theo_free_talk` |
| /LF117/ | S | KI-Figuren mit festem Charakter, Sprechstil und Stimme (z. B. … | app, core, speech | geprüft | 1 von 1 automatisch grün: `selin_register_flag` |
| /LF120/ | M | Tippen statt Sprechen (Kana-IME oder Rōmaji mit Umwandlung). | app, core | geprüft | 1 von 1 automatisch grün: `jonas_typed_marked` |
| /LF121/ | M | Ein Gespräch lässt sich unterbrechen und später an derselben Stelle … | app, backend, core, store | geprüft | 1 von 1 automatisch grün: `mara_resume` |
| /LF122/ | S | Lernende dürfen in die UI-Sprache wechseln („Wie sagt man …?“). | core | Code | 0 von 2 automatisch grün: `hinnerk_german_word_help`, `mara_codeswitch` |
| /LF200/ | M | Einstufung per Gespräch (5–8 min) mit steigender Schwierigkeit. | app, core, store | geprüft | 2 von 2 automatisch grün: `emi_ceiling`, `selin_profile_split` |
| /LF201/ | M | Das Lernermodell führt getrennte Fähigkeiten: Hörverstehen (Tempo … | app, backend, core, store | geprüft | 2 von 2 automatisch grün: `emi_ceiling`, `selin_profile_split` |
| /LF202/ | M | Das Lernermodell ist für Lernende einsehbar (Profil-Ansicht in … | app, backend, core | geprüft | 1 von 1 automatisch grün: `selin_profile_visible` |
| /LF210/ | M | Der Schwierigkeitsregler zielt auf eine Erfolgsquote von etwa 80 % … | core, store | geprüft | 3 von 3 automatisch grün: `emi_no_baby_talk`, `hinnerk_pace`, `mara_success_band` |
| /LF211/ | M | Stellgrößen: Anteil unbekannter Wörter (≤ 5 %), Satzlänge … | app, core | geprüft | 2 von 2 automatisch grün: `hinnerk_tts_slow`, `selin_settings_split` |
| /LF212/ | M | Hilfen (Wiederholung, Zeitlupe, Übersetzung, Hilfe-Stufen) fließen in … | app, core | geprüft | 1 von 1 automatisch grün: `mara_hints_silent` |
| /LF213/ | M | Die Anpassung wirkt innerhalb einer Sitzung (sofort leichter nach … | core | geprüft | 1 von 1 automatisch grün: `jonas_scenario_stuck` |
| /LF214/ | M | Die Schwierigkeit steigt schrittweise: Pro Sitzung ändert sich … | core | geprüft | 2 von 2 automatisch grün: `hinnerk_pace`, `mara_gradual` |
| /LF215/ | S | Starke Unterforderung (z. B. Erfolg ≥ 95 % über drei Sitzungen) führt … | app, backend, core | geprüft | 2 von 2 automatisch grün: `emi_skip`, `emi_skip_once` |
| /LF220/ | M | Das Curriculum ist ein Graph aus Einheiten. | app, backend, core, store | geprüft | 1 von 1 automatisch grün: `mara_path` |
| /LF221/ | M | Inhalte (Szenarien, Beispielsätze, Erklärungen) werden geprüft … | core, lang | Code | – |
| /LF230/ | M | Wiederholung (SRS) für Wörter, Wendungen und Grammatikmuster aus … | app, backend, core, store | geprüft | 2 von 2 automatisch grün: `emi_srs_keigo`, `mara_srs_errors` |
| /LF231/ | S | Wer eine Pause einlegt (≥ 7 Tage), steigt mit einer Auffrischung … | app, core, store | geprüft | 1 von 1 automatisch grün: `mara_comeback` |
| /LF300/ | M | Nach jedem eigenen Zug bewertet das System Grammatik, Wortwahl … | – | geprüft | 2 von 3 automatisch grün: `emi_keigo_detect`, `jonas_analysis_gaps`, `selin_register_flag` |
| /LF301/ | M | Jeder Vorschlag zeigt: eigene Form → bessere Form (als Diff), eine … | app | geprüft | 1 von 1 automatisch grün: `mara_diff_audio` |
| /LF302/ | M | Vorschläge haben einen Schweregrad: *verhindert Verständnis* … | ai, core | geprüft | 1 von 2 automatisch grün: `emi_keigo_detect`, `selin_register_flag` |
| /LF303/ | M | Bei korrekten Äußerungen gibt es keinen Verbesserungsvorschlag. | ai, core | geprüft | 1 von 3 automatisch grün: `selin_casual_ok`, `theo_fp`, `theo_pitch_no_nitpick` |
| /LF304/ | M | Wie viel Feedback angezeigt wird, ist einstellbar: *wenig* (nur … | app, core | geprüft | 1 von 1 automatisch grün: `mara_little_feedback` |
| /LF305/ | M | Recast: Die KI greift Fehler im Gespräch natürlich auf, indem sie die … | core | geprüft | 1 von 1 automatisch grün: `selin_recast` |
| /LF306/ | M | Nur Fehler mit dem Schweregrad *verhindert Verständnis* unterbrechen … | core | geprüft | 1 von 1 automatisch grün: `mara_little_feedback` |
| /LF307/ | S | Lernende können einem Vorschlag widersprechen („Das ist korrekt“). | app, backend, core, store | geprüft | 1 von 1 automatisch grün: `theo_dispute` |
| /LF310/ | M | Sitzungsbilanz: erreichte Ziele, die drei wichtigsten Verbesserungen … | app, core | geprüft | 1 von 1 automatisch grün: `mara_summary` |
| /LF320/ | S | Aussprache-Feedback für die Moren-Struktur: Langvokale (おばさん/おばあさん) … | app, speech, store | Code | 0 von 1 automatisch grün: `mara_mora_hint` |
| /LF321/ | S | Pitch-Akzent: Soll- und Ist-Kontur eines Wortes oder Satzes als Kurve. | – | offen | 0 von 2 automatisch grün: `theo_pitch_detect`, `theo_pitch_no_nitpick` |
| /LF330/ | S | Nachsprechen und Shadowing: Satz hören, nachsprechen, eigene Aufnahme … | – | offen | 0 von 1 automatisch grün: `theo_compare` |
| /LF500/ | M | Plasmoid für Panel und Desktop. | – | offen | 0 von 1 automatisch grün: `mara_plasmoid_compact` |
| /LF501/ | M | Kurzgespräch (3–5 min): ein Mini-Szenario aus der aktuellen Einheit … | backend | Code | 0 von 1 automatisch grün: `mara_plasmoid_short` |
| /LF502/ | M | Fünf Wiederholungen als Sprech-Mini-Übung (Hören → Antworten oder … | backend | Code | 0 von 1 automatisch grün: `mara_plasmoid_reviews` |
| /LF503/ | M | Das Plasmoid zeigt dieselben Verbesserungsvorschläge wie die App … | backend | Code | 0 von 1 automatisch grün: `mara_plasmoid_feedback` |
| /LF504/ | M | Das Plasmoid folgt den Plasma-Widget-Regeln von Kante: Standard ist … | – | offen | 0 von 1 automatisch grün: `mara_plasmoid_compact` |
| /LF505/ | M | Push-to-Talk funktioniert im Plasmoid per Klick und per globalem … | – | offen | 0 von 1 automatisch grün: `mara_ptt_shortcut` |
| /LF506/ | M | Das Plasmoid funktioniert ohne geöffnete App. | – | offen | 0 von 1 automatisch grün: `mara_plasmoid_fast` |
| /LF507/ | S | Eine Plasmoid-Sitzung lässt sich in der App fortsetzen und umgekehrt. | backend | geprüft | 1 von 2 automatisch grün: `jonas_sync`, `mara_resume` |
| /LF510/ | S | App und Plasmoid teilen Modelle, Profil und Mikrofon (ein Dienst … | – | offen | – |
| /LF520/ | S | Erinnerungen über das Benachrichtigungssystem (KNotification), zur … | backend, store | Code | – |
| /LF521/ | S | Globales Tastenkürzel für Push-to-Talk und „Kurzsitzung starten“ … | – | offen | 0 von 1 automatisch grün: `mara_ptt_shortcut` |
| /LF522/ | K | KRunner-Plugin: Wort nachschlagen, vorsprechen lassen, in die … | – | offen | – |
| /LF600/ | M | Lernpfad mit Einheiten, Szenarien und Wiederholungsknoten. | app, backend, core | geprüft | 1 von 1 automatisch grün: `mara_path` |
| /LF601/ | M | Tagesziel in Minuten Sprechzeit oder Szenarien, selbst wählbar … | app, core, store | geprüft | 1 von 1 automatisch grün: `mara_daily_goal` |
| /LF602/ | M | Serie (Tage mit erreichtem Tagesziel), mit Freeze (zwei pro Monat … | app, backend, core, store | geprüft | 5 von 5 automatisch grün: `jonas_freeze`, `jonas_midnight`, `jonas_sick_pause`, `jonas_timezone`, `mara_comeback` |
| /LF603/ | S | XP für Sprechzeit, erreichte Ziele und Wiederholungen. | app, core | geprüft | 1 von 1 automatisch grün: `jonas_xp_fair` |
| /LF604/ | S | Wochenrückblick: Sprechzeit, neue Wörter, Fortschritt pro Fähigkeit. | app, backend, core, store | geprüft | 1 von 1 automatisch grün: `mara_progress` |
| /LF605/ | M | Gamification lässt sich ganz abschalten (keine XP, keine Serie, nur … | app, backend, core, store | geprüft | 1 von 1 automatisch grün: `hinnerk_gamification_off` |
| /LF606/ | M | Es gibt keine Herzen, Leben oder Sperren nach Fehlern. | core | geprüft | 1 von 1 automatisch grün: `hinnerk_no_hearts` |
| /LF610/ | S | Statistik: Heatmap der Übungstage, Wortschatz pro Niveau, Sprechzeit … | app, backend | geprüft | 1 von 1 automatisch grün: `mara_progress` |
| /LF620/ | M | Benachrichtigungen sind abschaltbar, höchstens eine pro Tag und ohne … | app, backend, core, store | geprüft | 1 von 1 automatisch grün: `hinnerk_no_pressure` |
| /LF700/ | K | Sync zwischen Geräten über eine Ende-zu-Ende-verschlüsselte Datei auf … | – | offen | 0 von 1 automatisch grün: `jonas_sync` |
| /LF710/ | M | Export aller eigenen Daten (Profil, Verlauf, Wiederholung) als offene … | app, backend, store | geprüft | 3 von 3 automatisch grün: `lena_delete`, `lena_export`, `lena_import` |
| /LF711/ | M | Die Ansicht „Was verlässt das Gerät?“ zeigt je Baustein (STT, LLM … | ai, app, backend | geprüft | 2 von 2 automatisch grün: `lena_transparency`, `selin_win_mixed` |
| /LF712/ | M | Aufnahmen werden standardmäßig nicht gespeichert. | app, backend, store | geprüft | 2 von 2 automatisch grün: `lena_default_no_audio`, `lena_retention` |
| /LF713/ | M | API-Schlüssel liegen im System-Schlüsselbund (KWallet, Secret … | ai, app, backend | Code | 0 von 1 automatisch grün: `selin_win_keys` |
| /LF714/ | S | Kostenbremse für Cloud-Anbieter: geschätzte Kosten pro Sitzung und … | ai, app, backend, store | geprüft | 1 von 1 automatisch grün: `jonas_cost_limit` |
| /LF720/ | S | Mehrere Profile pro Gerät (z. B. Familie), jeweils getrennt. | app, backend, store | geprüft | 1 von 1 automatisch grün: `hinnerk_own_profile` |
| /LL010/ | M | Antwortlatenz (Sprechende → erster Ton der KI): p50 ≤ 1,5 s und p95 ≤ … | app, backend | geprüft | 1 von 3 automatisch grün: `lena_responsive`, `lena_suggest_smaller`, `theo_latency` |
| /LL011/ | M | Das Plasmoid öffnet in ≤ 300 ms (Dienst läuft) und ist ≤ 2 s nach … | – | offen | 0 von 1 automatisch grün: `mara_plasmoid_fast` |
| /LL012/ | S | Speicher im Leerlauf ≤ 150 MB PSS (ohne geladene Modelle; PSS teilt … | app, lang, store | geprüft | 1 von 1 automatisch grün: `lena_idle_memory` |
| /LL020/ | M | Profil-Ansicht, Lernpfad und Statistik laden in ≤ 500 ms bei der … | store | geprüft | 1 von 1 automatisch grün: `lena_stats_fast` |
| /LL100/ | M | Die STT-Zeichenfehlerrate (CER) für Lernendensprache auf dem … | speech | geprüft | 1 von 2 automatisch grün: `emi_misrecognition`, `jonas_noise` |
| /LL110/ | M | Fehlalarmrate der Verbesserungsvorschläge (Vorschlag bei korrekter … | core | Code | 0 von 1 automatisch grün: `theo_fp` |
| /LL111/ | M | Trefferquote für eingebaute Fehler der Personas (Kap. 9) ≥ 85 % bei … | – | geprüft | 1 von 3 automatisch grün: `emi_keigo_detect`, `mara_recall`, `theo_tp` |
| /LL112/ | M | Erklärungen stehen immer in der UI-Sprache und sind im Niveau der … | ai, app, backend | geprüft | 2 von 4 automatisch grün: `emi_explain_examples`, `hinnerk_plain_words`, `mara_explain_de`, `theo_ui_en` |
| /LL200/ | M | Vollständig per Tastatur bedienbar, mit sichtbarem Fokus (Kante … | – | geprüft | 1 von 1 automatisch grün: `theo_keyboard` |
| /LL201/ | M | Screenreader: Alle Bedienelemente haben Accessible-Namen und -Rollen … | – | geprüft | 1 von 1 automatisch grün: `hinnerk_a11y_names` |
| /LL202/ | M | Die Schriftgröße folgt dem System und ist zusätzlich in der App … | app, store | geprüft | 2 von 2 automatisch grün: `hinnerk_font`, `hinnerk_targets` |
| /LL203/ | M | Kontraste erreichen WCAG AA in System, Kante und Kante Light. | – | geprüft | 1 von 1 automatisch grün: `hinnerk_targets` |
| /LL204/ | S | Untertitel für alles Gesprochene. | app, speech, store | geprüft | 1 von 1 automatisch grün: `hinnerk_tts_slow` |
| /LL300/ | M | Keine Dark Patterns: keine künstliche Verknappung, keine Schuld- oder … | backend, core | geprüft | 1 von 1 automatisch grün: `hinnerk_no_pressure` |
| /LL301/ | M | Die GUI wird aus Kante erzeugt (siehe Kap. 8). | app | Code | 0 von 1 automatisch grün: `selin_win_style` |
| /LL302/ | M | Die KI-Figuren bleiben im Lernkontext. | core | Code | 0 von 1 automatisch grün: `jonas_inappropriate` |
| /LL303/ | M | Das Fenster ist mindestens 533 px breit, logische Pixel (800 auf … | app, backend | geprüft | 1 von 1 automatisch grün: `hinnerk_narrow` |
| /LL400/ | M | Lokal zuerst: Alle Muss-Funktionen sind mit lokalen Bausteinen … | ai | geprüft | 1 von 1 automatisch grün: `lena_zero_egress` |
| /LL401/ | M | Im Nur-lokal-Modus gibt es keine ausgehenden Verbindungen außer zu … | app, speech | geprüft | 2 von 2 automatisch grün: `lena_download_announced`, `lena_zero_egress` |
| /LL402/ | M | DSGVO-Konformität: Zweckbindung, Löschung, Export und Auskunft über … | store | geprüft | 1 von 1 automatisch grün: `lena_delete` |
| /LL500/ | M | Hauptplattform: KDE Plasma 6 (aktuelle und vorige Minor-Version) … | – | Vorgabe | – |
| /LL501/ | S | Windows 10/11 und macOS (aktuelle und vorige Version): App ohne … | – | offen | 0 von 1 automatisch grün: `selin_win_style` |
| /LL502/ | K | Android (Kirigami), Mindestversion nach ADR. | – | Vorgabe | – |
| /LL503/ | M | Eine Codebasis: Rust-Kern und Qt-6-QML-Oberfläche. | – | Vorgabe | – |
| /LL504/ | M | Anbieter für STT, LLM und TTS sind austauschbar (Trait). | app, speech | geprüft | 1 von 1 automatisch grün: `jonas_local_switch` |
| /LL505/ | M | Der Kern ist ohne GUI testbar (Headless-API, `kaiwa-sim`). | core | Vorgabe | – |
| /LL506/ | M | Mehrsprachigkeit der UI über die KDE-Infrastruktur (ki18n/gettext). | store | geprüft | 1 von 1 automatisch grün: `theo_ui_en` |
| /LL600/ | M | Jede Persona hat ein Startprofil, ein Lernverhalten, typische Fehler … | sim | Code | – |
| /LL601/ | M | `kaiwa-sim` simuliert Lernende in drei Stufen: Text (Headless-API) … | sim | Code | – |
| /LL602/ | M | Langzeitläufe mit beschleunigter Uhr (30 simulierte Tage) prüfen … | core, sim | Code | – |
| /LL603/ | M | Die Läufe sind reproduzierbar (fester Seed, aufgezeichnete Antworten … | sim | Code | – |
| /LL604/ | M | Der Bericht pro Lauf enthält: bestandene und gescheiterte Prüfregeln … | sim | Code | – |
