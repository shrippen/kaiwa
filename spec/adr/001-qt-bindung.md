# ADR-001: Rust ↔ Qt/QML

**Status:** Angenommen (Spike 0.2) · **Bezug:** /LL503/, Roadmap 0.2

## Kontext

Der Kern ist in Rust, die Oberfläche in QML mit Kirigami und Kante (`qml/Kante`, `qml/KantePlasma`). Kante verlangt echtes Qt Quick. Toolkits, die QML nur nachbilden, scheiden daher aus.

## Optionen

| Option | Für | Gegen |
|---|---|---|
| **cxx-qt** (KDAB) | Qt 6, QML-Module aus Rust (`#[qobject]`, Signale, Properties), CMake- und Cargo-Build, aktiv gepflegt, in KDE-Umfeld im Einsatz | Build-Komplexität (CMake + Cargo), Lernkurve |
| qmetaobject-rs | rein Cargo, einfacher Einstieg | weniger aktiv, Qt-6-Unterstützung lückenhaft |
| Slint, egui, Tauri | reines Rust | kein Qt Quick, Kante-Regel verletzt |

## Empfehlung

**cxx-qt.** Ein QML-Modul `org.shrippen.kaiwa` stellt `Session`, `Learner`, `Path` und `Settings` als QObjects bereit. Kirigami und Kante kommen als QML dazu, Kante als unveränderte Kopie in `app/qml/Kante` (eine Importart pro Projekt, Kante-README).

## Spike 0.2 (Abnahme)

Ein Kirigami-Fenster mit `KanteScope`, `KanteButton`, `KanteMessage` und Stilumschaltung System/Kante/Kante Light. Ein Rust-Signal aktualisiert die Oberfläche. Der Build läuft unter Linux und in der CI.

## Ergebnis Spike 0.2

Siehe [`spikes/s02-qt/`](../../spikes/s02-qt/):

- Mit cxx-qt 0.10, Qt 6.8 und Kirigami 6.10 läuft ein Kirigami-Fenster mit Kante 1.25 (System, Kante, Kante Light), gesteuert von einem Rust-QObject. Ergebnisse kommen aus einem Worker-Thread über `qt_thread().queue`.
- **Festlegungen:**
  - Properties tragen `cxx_name` in camelCase.
  - Kante wird unverändert nach `app/qml/Kante` kopiert, mit Sync-Skript und CI-Gleichstandsprüfung.
  - Die App-CI braucht Qt ≥ 6.6 und KF6. Ubuntu 24.04 hat nur Qt 6.4. Das Image ist eine offene Frage im Fragenpaket 2.
