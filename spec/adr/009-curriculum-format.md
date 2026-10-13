# ADR-009: Curriculum-Format v2

**Status:** Angenommen (Fragenpaket 8: Q-03, Q-04, Q-07, Q-10) · **Bezug:** /LF220/, /LF221/, /LF600/, /LD040/, /LD050/, `spec/INHALT-ENTWURF.md`

## Kontext

Bis Oktober 2026 gab es eine Datei `data/curriculum/n5.json` mit 12 Einheiten. Die Grammatik einer
Einheit war Freitext, teils deutsch („〜つ (Zählwort)“), teils Wortschatz („遠い・近い“). Für N4 und
später N3 bis N1 braucht Kaiwa mehrere Stufen, die aufeinander aufbauen, einen Katalog der
Grammatikpunkte (für Erklärungen und den Grammatik-Lint) und einen Nachweis des
Muttersprachler-Reviews.

## Entscheidung

1. **Eine Datei je Stufe** (Q-04): `data/curriculum/n5.json`, `n4.json`, … Jede Datei trägt
   `level: {jlpt, cefr}`. `Curriculum::merge` fügt die Dateien in der Reihenfolge N5, N4, N3, N2,
   N1 zu einem Graphen zusammen, jede Einheit behält die Stufe ihrer Datei. `requires` darf auf
   Einheiten anderer Stufen zeigen (`n4-01` → `n5-12`); `Curriculum::check` prüft den
   Gesamtgraphen (unbekannte Einheiten, doppelte Ids, Zyklen, Themen 1–15).
2. **Neue Felder je Einheit, alle optional** (Q-07): `topics` (JF-Themen 1–15),
   `jf_can_do` (Ids der JF-Can-do-Datenbank, Q-10: nur die Ids als Verweis, der Kann-Satz bleibt
   eigene Formulierung), `review` (`by`, `date`, `hash`, siehe INHALT-ENTWURF 3.3). Eine Datei im
   alten Format liest sich unverändert.
3. **Grammatik-Katalog** `data/grammar/<stufe>.json` (Q-07): je Punkt `id` (ASCII), `label` (so
   steht er in der Einheit, japanisch), eigene Erklärung de/en und Beispiele in Furigana-Markup.
   Die Stufe kommt aus der Datei. `Curriculum::check_grammar` verlangt: jeder Punkt einer Einheit
   steht im Katalog, und seine Stufe ist die Stufe der Einheit, die ihn zuerst nutzt. Jeder
   Katalogpunkt gehört zu einer Einheit (Test). Erklärungen schreibt Kaiwa selbst; Tae Kims Guide
   (CC BY-NC-SA) dient nur als Orientierung für Reihenfolge und Aufbau.
4. **Anzeige** (Q-03): Die Stufe heißt in der App „N5 · A1“, der Lernpfad hat einen Abschnitt je
   Stufe und den Satz „Die Stufen orientieren sich an JLPT und JF-Standard.“ Nie „JLPT-Kurs“, kein
   Bestehensversprechen.

## Folgen

- Grammatik-Labels in `n5.json` sind auf japanische Muster umgestellt (die deutschen Zusätze
  stehen im Katalog), „遠い・近い“ ist kein Grammatikpunkt mehr.
- App und `kaiwad` binden die Stufendateien ein (`kaiwa-backend::CURRICULUM`), `kaiwa-sim` liest
  `data/curriculum/` als Verzeichnis.
- Noch nicht umgesetzt: Morphologie-Muster je Punkt (für den Grammatik-Lint, INHALT-ENTWURF 3.2),
  Prüfung des Review-Hashs im Release-Gate (M7), Abdeckung der JF-Themen je Stufe.
