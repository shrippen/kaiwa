# Working rules for this repository

The rules for all shrippen repositories are in `RULES.md` (writing, code, commits, bug
fixes, repository, GUI). It is a synced copy from the rules repository; do not edit it
here. This file holds only what is particular to this repository. A Claude Code session on
the owner's machine has the rules through the global `CLAUDE.md` already and reads
`RULES.md` only for its GUI rule, before work on a user interface.

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
- Merging (decisions P-01, G-07): Claude may merge its own PRs on Gitea once CI is green and the review leaves no **Wichtig** open (see the skill `pr-review`), in every shrippen repository; where a repository has no CI, after its local checks (build, tests). The review rule (owner, 18.10. and 22.10.2026) applies in every shrippen repository too. Open questions go to the owner in bundles, as an artifact with proposals; answers are recorded where they apply (ADRs, spec).
- Cloud sessions: `tools/ccr-setup.sh` is the environment's setup script (G-05): tea always, Qt 6.8 and Kirigami with `KAIWA_QT=1`. The tea login stays OAuth (`tea login add --oauth`, then deliver the redirect to the local callback).
- Voice recordings live in Git LFS under `qa/corpus/audio/` (G-03).

## Release

Release steps: `RELEASE.md`. Before a release, load the skill `release-changelog`. A release
is published, so the owner confirms it in words.

## Repository (this project)

- The merge waits for: CI is green on the PR's latest commit: the Gitea checks of `.gitea/workflows/ci.yml`, `CI / rust (pull_request)` and `CI / qt (pull_request)`, show success there. No status there yet means wait, not merge; if it never comes, check the Gitea runner or ask the owner. PR-Agent's status does not count here. Without CI, the local checks count.
- The owner also confirms: a PR that changes `packaging/`.
- Everything else: `RULES.md` (Repository) and the skill `pr-review`.
