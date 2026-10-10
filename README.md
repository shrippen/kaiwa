# Kaiwa

*Arbeitstitel.* Japanisch sprechen und verstehen üben, im Gespräch mit einer KI, mit Verbesserungsvorschlägen und einer Schwierigkeit, die mitwächst. Vorbilder sind Jumpspeak (Gespräch zuerst) und Duolingo (kleine Schritte, Lernpfad). Hauptplattform ist KDE Plasma 6 mit Plasmoid für Kurzsitzungen. Die App läuft auch auf Linux, Windows, macOS und Android. Rust, Qt 6 / Kirigami, GUI aus [Kante](https://github.com/shrippen/Kante).

**Status:** Planung (M0).

## Starten

```sh
./start.sh                # App mit dem eigenen Profil
./start.sh demo [de|en]   # App mit frischem Demo-Profil (temporär)
./start.sh sim            # QA-Personas im Textmodus (kaiwa-sim run)
./start.sh check          # fmt, clippy, Tests, Personas wie in der CI
```

Voraussetzungen und Sprachmodell: [app/README.md](app/README.md).

- [Roadmap](ROADMAP.md)
- [Lastenheft](spec/LASTENHEFT.md)
- [QA-Personas](qa/README.md)

Lizenz: [MIT](LICENSE). Sprachdaten (JMdict, KANJIDIC2 u. a.) stehen unter eigenen Lizenzen, siehe `NOTICE`.
