# ADR-001: Rust ↔ Qt/QML

**Status:** Vorgeschlagen · **Bezug:** /LL503/, Roadmap 0.2

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
