# Working rules for this repository

## What lives here

Kaiwa (会話), a Japanese conversation trainer: Rust core, Qt 6 / Kirigami app, Plasma 6 widget.

| Path | What |
|---|---|
| `ROADMAP.md` | Milestones M0–M7, Definition of Done, risks |
| `spec/LASTENHEFT.md` | Requirements (`/LFxxx/`, `/LDxxx/`, `/LLxxx/`); every feature links its IDs |
| `spec/adr/` | Architecture decisions (ADR-001 … from M0) |
| `qa/personas/` | Executable QA personas for `kaiwa-sim` (schema in `schema.json`, see `qa/README.md`) |

## Working here

- A feature is done when its requirement is linked, tests cover it and a persona journey checks it (`qa/README.md`).
- Personas change only together with their checks; never adjust `start_level` or planted errors to make a run pass.
- Persona names and ids from the demo world (`shrippen.github.io/demo/world`) stay stable.
- Merging (decisions P-01, G-07): Claude may merge its own PRs on Gitea once CI is green, in every shrippen repository; where a repository has no CI, after its local checks (build, tests). Open questions go to the owner in bundles, as an artifact with proposals; answers are recorded where they apply (ADRs, spec).
- Cloud sessions: `tools/ccr-setup.sh` is the environment's setup script (G-05): tea always, Qt 6.8 and Kirigami with `KAIWA_QT=1`. The tea login stays OAuth (`tea login add --oauth`, then deliver the redirect to the local callback).
- Voice recordings live in Git LFS under `qa/corpus/audio/` (G-03).

## GUI rule

- Every GUI of this project is generated from Kante, not inspired by it: landing pages,
  web apps, Qt Quick / Kirigami apps, Plasma widgets, dialogs, e-mail and print layouts.
  Source: https://github.com/shrippen/Kante (checkout `../Kante`).
  Web: link `https://shrippen.github.io/v1/shrippen.css` and `shrippen.js`, or vendor them
  unchanged. Apps: copy `qml/Kante` (and `KantePlasma` for Plasma widgets) unchanged.
- Use Kante's tokens, roles, components, classes, QML components and motion as they are.
  No own colours, fonts, sizes, radii, cuts, shadows, animation timings, no own copy or
  variant of a component that Kante has. Raw values (`#hex`, `px` for controls) are a bug;
  use roles (`--primary`, `--focus`, `--warn`, `KanteStyle.*`).
- A missing element is added to Kante first (CSS or QML, docs, catalogue), then used here.
  Never solve it locally in this project and never wait with a "temporary" copy.
- Exception: Kimai plugins take their GUI from Knust (`kimai/knust/` in Kante) and
  the kit (`kimai/kit/`), the Kante spinoff that adapts Kante to Kimai's look. The same rule applies to Knust: use it
  as it is, and add missing elements to Knust.
- Exception: Kintsugi (`shrippen/kintsugi`) uses Kante Gold (`<html data-kante="gold">`),
  the noble variant defined in Kante itself. The same rule applies: use it as it is, and
  add a missing element or Gold detail to Kante (its Gold block) first. No other project
  uses Kante Gold without a decision recorded here.
- A project without a GUI (library, CLI, scripts) has nothing to do here.

## Release rule

- Every release gets a detailed release log with graphics: a lead, the changes grouped as New, Improved, Fixed and Note, each point with what changed for the user and why; a demo screenshot per visible change (every language of the page), a diagram for changes inside.
- The log goes into the release notes on Gitea and as the newest entry of `#changes` on the landing page (Kante `.changelog`). A release is done when the landing page shows it.

## Repository rule

- This repository lives on Gitea (`git.arianw.de`). GitHub is only a push mirror of it.
- Changes arrive as pull requests: work on a branch, open a PR, merge it on Gitea (the mirror follows).
- Never merge a PR, push to `main` (or any default branch), push tags or publish releases on GitHub. A merge there is overwritten by the next Gitea push.
- Never force-push a branch that someone else's PR depends on.
