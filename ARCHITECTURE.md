# Architecture

How this repository is meant to be built. Where the code differs today is listed under
"Known deviations". Change this file in the PR that changes the structure. The decisions
behind it are the ADRs in `spec/adr/`; ADR-008 sets the main line.

## What it does

Kaiwa is a Japanese conversation trainer: the learner speaks with a tutor character, gets
help and corrections, and a learning model picks what comes next. One Rust core serves the
Qt app today and, from M4, the service `kaiwad` with the Plasma widget.

## Building blocks

| Path | Its one task |
|---|---|
| `crates/kaiwa-lang` | Language data: the words of a sentence, the dictionary, pitch accent. |
| `crates/kaiwa-ai` | The tutor's chat providers behind `ChatProvider` (OpenAI-compatible, Anthropic, recorded answers), their presets, cost and key handling. |
| `crates/kaiwa-core` | The conversation engine and the learning model without GUI and without storage: scenario, session, corrections, learner model, difficulty, curriculum, review, daily goal. Every learning rule lives here. |
| `crates/kaiwa-store` | The profile as one SQLite file with versioned migrations, export and import. It stores what the core decides. |
| `crates/kaiwa-speech` | Microphone, speech recognition, the character's voice, the mora check; models live outside the binary. |
| `crates/kaiwa-backend` | The one interface a front end uses (ADR-008): the commands (start a talk, take a turn, finish, change a setting) and the content of every view as JSON. |
| `app/src` | The Qt objects of the app: they pass what the learner does to the backend, show what it answers, and drive microphone and voice. |
| `app/qml` | The pages. `Kante/` is the vendored Kante copy (`tools/sync-kante.sh`), never edited here. |
| `crates/kaiwa-sim` | The QA driver: plays the personas in `qa/personas` against the same code the app uses. |
| `data/` | Content without code: curriculum, scenarios, grammar, dictionary excerpt. |
| `qa/` | Personas, recorded tutor answers, the speech corpus. |
| `spec/` | Requirements (`LASTENHEFT.md`), ADRs, traceability. |
| `spikes/` | Experiments. Nothing here is shipped; `s02-qt` is built in CI as a probe. |
| `packaging/` | Arch package, Flatpak, desktop file. |

## Layers

```
 front ends     app/qml ─► app/src            (M4: kaiwad + plasmoid)      kaiwa-sim
                             │      │                    │                     │
                             │      └─► kaiwa-speech     │                     │
                             ▼                           ▼                     ▼
 interface      kaiwa-backend   ◄── the only crate a front end calls for learning
                             │
                 ┌───────────┼────────────┐
                 ▼           ▼            ▼
 storage, core  kaiwa-store ─► kaiwa-core ─► kaiwa-ai        kaiwa-lang (used by
                                                              backend and speech)
```

- A front end calls `kaiwa-backend` and nothing below it: not the store, not the core, not
  the tutor. The backend keeps its store private.
- A front end holds no rule. Which scenario comes next, what a finished talk changes,
  limits of a setting, what happens when a setting changes: all of that is the backend or
  below. The app only shows, records and plays.
- The flow of a conversation (start, turn, finish, save) exists once, in the backend. The
  app's talk and the widget's short talk both use it.
- A learning rule exists once, in `kaiwa-core`. The store books what the core decided; it
  does not decide. `kaiwa-sim` calls the same functions and never recomputes a rule.
- A list the learner sees (error kinds, pause steps, levels) is defined once in the core
  and reaches the pages through a view; no page and no other crate keeps its own copy.
- A failure to save or load reaches the learner in every front end the same way; it is
  never only logged.
- `kaiwa-core`, `kaiwa-ai` and `kaiwa-lang` know nothing of Qt, files of the profile or
  audio. Dependencies only run downwards in the drawing.
- A persona check tests behaviour through the backend or the running app, not text in a
  source file.

## Where new things go

| Change | Touch |
|---|---|
| Content: a scenario, a lesson, grammar | `data/` and, where a journey covers it, a persona in `qa/personas`. No code. |
| A learning rule (difficulty, review, points, path) | `kaiwa-core` with its tests; a persona journey in `qa/`. |
| A kind of correction or another list the learner sees | The one list in `kaiwa-core`; the view in `kaiwa-backend` passes it on. |
| A setting | Stored in the profile (`kaiwa-store`, with a migration if the shape changes); its limits and effects as a backend command; the settings view; `app/qml/SettingsPage.qml`. |
| A page or a part of one | Its content as a view in `kaiwa-backend`; the page in `app/qml`; a QML test (`tools/qml-tests.sh`). |
| A step in the conversation | The flow in `kaiwa-backend`; the app only gets a new signal or field. |
| A tutor provider | `kaiwa-ai`: provider, preset, a test against a recorded payload. |
| Speech: a model, a voice, a check | `kaiwa-speech`. |
| A change to stored data | A migration in `kaiwa-store` and `spec/export.schema.json`. |
| A requirement | Its id in `spec/LASTENHEFT.md`; the feature links it. |
| A GUI component | In the Kante repository first, then `tools/sync-kante.sh`. |

A change that needs more places than its row lists is a finding for the next audit.

## Known deviations

- The app reaches past the backend: it uses the store at about 30 places and depends on
  `kaiwa-store`, `kaiwa-core`, `kaiwa-ai` and `kaiwa-lang` directly; rules for settings
  live in `app/src/app.rs`. `kaiwa-sim` depends on the same four crates. `AUDIT.md`
  finding 7.
- The conversation flow exists twice, in `app/src/conversation.rs` and in the backend's
  short talk, and the two differ. Finding 6.
- What a finished talk changes is written twice in the store and a third time in
  `kaiwa-sim`'s day simulation. Finding 4.
- Error kinds are named in the core and again, differently, in the backend's views; pause
  steps stand at five places. Findings 1 and 3.
- A failed save in a conversation is only logged. Finding 5.
- Some persona checks search for text in source files. Finding 8.
- `kaiwad` and the plasmoid do not exist yet (M4); until then the app is the only front end.
