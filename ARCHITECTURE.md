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
| `crates/kaiwa-backend` | The one interface a front end uses (ADR-008): the commands (start a talk, take a turn, finish, change a setting, listen, speak) and the content of every view as JSON. |
| `app/src` | The Qt objects of the app: they pass what the learner does to the backend and show what it answers. |
| `app/qml` | The pages. `Kante/` is the vendored Kante copy (`tools/sync-kante.sh`), never edited here. |
| `crates/kaiwad`, `plasmoid/` | From M4 (branch `later/plasmoid`): the D-Bus service that hosts the backend under Plasma, and the widget as its thin client. |
| `crates/kaiwa-sim` | The QA driver: plays the personas in `qa/personas` against the same code the app uses. |
| `data/` | Content without code: curriculum, scenarios, grammar, dictionary excerpt. |
| `qa/` | Personas, recorded tutor answers, the speech corpus. |
| `spec/` | Requirements (`LASTENHEFT.md`), ADRs, traceability. |
| `spikes/` | Experiments. Nothing here is shipped; `s02-qt` is built in CI as a probe. |
| `packaging/` | Arch package, Flatpak, desktop file. |

## Layers

```
 front ends     app/qml ─► app/src         plasmoid (M4)          kaiwa-sim
                              │                  │                    │
                  in-process (every OS)   or the service kaiwad (Plasma, M4)
                              ▼                  ▼                    ▼
 interface      kaiwa-backend   ◄── the only crate a front end calls
                      │
          ┌───────────┼──────────────┬───────────────┐
          ▼           ▼              ▼               ▼
 below    kaiwa-store ─► kaiwa-core ─► kaiwa-ai      kaiwa-speech ─► kaiwa-lang
                                                     (the backend uses kaiwa-lang too)
```

- A front end calls `kaiwa-backend` and nothing below it: not the store, not the core, not
  the tutor, not speech. The backend keeps its store private.
- Microphone, voice and their models sit below the interface (ADR-008, /LF510/). Under
  Plasma the service `kaiwad` hosts the backend, so app and widget share models, session
  and microphone; elsewhere the app runs the same backend in its own process.
- A front end holds no rule. Which scenario comes next, what a finished talk changes,
  limits of a setting, what happens when a setting changes: all of that is the backend or
  below. A front end only shows what the backend answers.
- The flow of a conversation (start, turn, finish, save) exists once, in the backend. The
  app's talk and the widget's short talk both use it.
- A learning rule exists once, in `kaiwa-core`. The store books what the core decided; it
  does not decide. `kaiwa-sim` is a front end: it reaches the rules through the backend
  and never recomputes one.
- A list the learner sees (error kinds, pause steps, levels) is defined once in the core
  and reaches the pages through a view; no page and no other crate keeps its own copy.
- A failure to save or load reaches the learner in every front end the same way; it is
  never only logged.
- `kaiwa-core`, `kaiwa-ai` and `kaiwa-lang` know nothing of Qt or audio; core and lang
  know nothing of the profile's files either. Dependencies only run downwards in the
  drawing.
- A persona check tests behaviour through the backend or the running app, not text in a
  source file.

## Where new things go

| Change | Touch |
|---|---|
| Content: a scenario, a lesson, grammar | `data/` and, where a journey covers it, a persona in `qa/personas`. No code. |
| A learning rule (difficulty, review, points, path) | `kaiwa-core` with its tests; a persona journey in `qa/`. |
| A kind of correction or another list the learner sees | The one list in `kaiwa-core`; the view in `kaiwa-backend` passes it on. |
| A setting | Stored in the profile (`kaiwa-store`, with a migration if the shape changes); its limits and effects as a backend command; the settings view; `app/qml/SettingsPage.qml` or one of its tiles (`app/qml/Settings*.qml`). |
| A page or a part of one | Its content as a view in `kaiwa-backend`; the page in `app/qml`; a QML test (`tools/qml-tests.sh`). |
| A step in the conversation | The flow in `kaiwa-backend`; the app only gets a new signal or field. |
| A tutor provider | `kaiwa-ai`: provider, preset, a test against a recorded payload. |
| Speech: a model, a voice, a check | `kaiwa-speech`; a front end reaches it through a backend command. |
| A change to stored data | A migration in `kaiwa-store` and `spec/export.schema.json`. |
| A requirement | Its id in `spec/LASTENHEFT.md`; the feature links it. |
| A GUI component | In the Kante repository first, then `tools/sync-kante.sh`. |

A new command that writes the profile journals its event first (`kaiwa-core` `Event`,
`kaiwa-backend` `journal.rs`, ADR-010).

A change that needs more places than its row lists is a finding for the next audit.

## Known deviations

- The app still depends on `kaiwa-core`, `kaiwa-ai` and `kaiwa-lang` directly, all in
  `conversation.rs`: the input kind, the tutor's error kinds, the lookup's words.
- `kaiwa-sim` sets up and reads test state through the backend's `seed` (feature `sim`,
  only the sim turns it on) and still depends on `kaiwa-store`, `kaiwa-core`, `kaiwa-ai`
  and `kaiwa-lang` directly for its own checks. The main path of every persona (`run.rs`,
  `steps.rs`) sets up the persona's profile (amount of corrections, language, placement) through
  `seed` and starts its talk with the backend's flow (`Talk`, unstored, on the recorded or the
  live tutor, handed in through `seed().talk_with`), so difficulty, learner level, unit grammar,
  amount of corrections and language apply. The talk-recording checks of `engine.rs`
  (marks after the end) play through `Talk` too. Still on a bare core `Session` (some with
  their own tutor): the other checks in `engine.rs` (`talk.rs`), the profile checks in
  `profile.rs` and the day simulation (`days.rs`, `simulate`). The load-time check (`fast`,
  `profile.rs`) still opens a profile in `kaiwa-store`: it needs `batch` and a timed load. The dispute check reads the QA list
  through `seed`, which no backend command offers.
- The conversation page does not take the backend's background troubles: a chosen tutor
  that cannot be set up (no key) reads there as "no tutor"; the main pages say why after
  the talk. Found in the review of PR 167.
- Microphone, recognition, voice and downloads are backend types (`ears`, `voice`,
  `models`), but the app holds them and drives when to load, listen, speak and unload. For
  `kaiwad` (M4) that flow still has to move into the backend.
- `kaiwa-sim`'s day simulation still adds up points per turn itself instead of the core's
  `turn_xp`.
- Some persona checks still read source text: scans of all pages for strings, names,
  handlers and gamification text, the palette's colours, the voice's pace in
  `app/src/conversation.rs` and the one file that downloads (`kaiwa-backend/src/models.rs`).
  What a page does is checked in QML tests (`app/qml/tests`).
- `kaiwad` and the plasmoid are not on `main` yet (M4, branch `later/plasmoid`); until
  then the app is the only front end and runs the backend in its own process. On that
  branch `kaiwad` drives microphone and recognition itself; M4 moves that into the
  backend.
