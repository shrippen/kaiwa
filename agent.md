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
- Merging (decisions P-01, G-07): Claude may merge its own PRs on Gitea once CI is green and the review leaves no **Wichtig** open (see below), in every shrippen repository; where a repository has no CI, after its local checks (build, tests). The review rule (owner, 18.10. and 22.10.2026) applies in every shrippen repository too. Open questions go to the owner in bundles, as an artifact with proposals; answers are recorded where they apply (ADRs, spec).
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
- Review (all shrippen repositories, owner 18.10. and 22.10.2026): every PR is reviewed before it is merged. The owner does not read code, so this is the second reader.
  - PR-Agent first (owner, 22.10.2026): where `.gitea/workflows/pr-agent.yml` is set up, the session comments `/review` on the PR for each round with new code or a new base (the first one included; a rebase, a merge of the base or a change of the base branch, see below) and waits for PR-Agent's comment. That comment reviews the whole diff and covers the commit that was the PR's head when `/review` was posted. PR-Agent labels its findings as `REVIEW.md` says; a finding without a label counts as **Wichtig**. A run cancelled by a newer `/review` on the same PR (the concurrency group holds the PR number) is replaced, not failed.
  - If PR-Agent fails (the run fails or ends without a review comment, reports a key or cost limit, or no run is created within 10 minutes), the session posts `/review` once more. If that fails too, a new Claude agent reviews the round as below, and the session says in the PR why. Where PR-Agent is not set up, the new Claude agent always reviews.
  - The Claude agent: the session that wrote the PR starts a new agent (Agent tool, no shared context) and gives it only the PR's title and body, the diff against its base (`git diff <base>...HEAD`), the repository and `REVIEW.md`. It does not tell it what it thinks of its own change. The agent checks every finding it would post against the code (file, line, the input that goes wrong) and drops what it cannot show. It reports as `REVIEW.md` says: count first, labels **Wichtig**, **Kleinigkeit**, **Bestand**. The session posts the report unchanged as a PR comment on Gitea. Above it goes the heading `## Claude-Review (neuer Agent) zu <commit>`, below it the footer `---` and `_Generated by [Claude Code](https://claude.ai/code)_`.
  - Each **Wichtig** is fixed, or answered in the PR with a reason.
  - PR-Agent sees no earlier round. So when an earlier round had a **Wichtig** (open or closed), after PR-Agent's comment (or with answers only, alone) a new Claude agent closes the round: it gets the PR's title and body, the repository, `REVIEW.md`, the open **Wichtig** with the answers, PR-Agent's findings of this round, the closed **Wichtig** and how they were closed, and the new commits (`git diff <last reviewed commit>..HEAD`; without new commits, the code at HEAD). It says for each open **Wichtig** whether it is fixed or its answer is accepted, and whether a **Wichtig** PR-Agent raises again is one already closed (fixed, answer accepted or decided by the owner; it stays closed unless the agent shows it is not fixed). Its count holds every **Wichtig** still open, PR-Agent's new ones included. It posts like a review, heading `## Claude-Review (offene Wichtig) zu <commit>`. An answer it does not accept goes to the owner, who decides in words. With the fallback, one new agent does both: the review of the new commits and the open **Wichtig**.
  - After a rebase, a merge of the base or a change of the PR's base branch a new round is due, also without a new commit, against the new base; a Claude agent of that round (closing or fallback) gets the whole diff against the new base (`git diff <base>...HEAD`) in place of the new commits. Every round counts, also one with answers only: after three rounds that still leave a **Wichtig** open, stop and rethink the approach with the owner.
  - Merge only when the last round covers the PR's latest commit against its current base branch (the branch by name; the base moving on alone needs no new round) and leaves no **Wichtig** open (its closing agent's count, or PR-Agent's comment when no earlier round had a **Wichtig**), and CI is green on the PR's latest commit: the Gitea checks of `.gitea/workflows/ci.yml`, `CI / rust (pull_request)` and `CI / qt (pull_request)`, show success there. No status there yet means wait, not merge; if it never comes, check the Gitea runner or ask the owner. PR-Agent's status does not count here. Where CI runs only for some paths, the repository rule says which commits need it. Without CI, the local checks of the merging rule count.
  - Before the merge, the owner confirms in words, not by reading code, a PR that changes CI or release workflows (`.gitea/`, `.github/`, `packaging/`), this review rule (`agent.md`, `REVIEW.md`), secrets or what is published (a release, a tag, a new public page, site or package; rebuilding what is already published does not count).
  - `REVIEW.md` in the repository root says what failure looks like here, what to check in particular, what to skip and how to report. A repository without one gets it in its next PR.
- PR-Agent's status is not a CI check. The owner may post `/review` at any time (PR-Agent takes no other command here); the merge waits for that run and handles its findings like a round's. Owner and session post from the same account, so a run that fails is handled the same whoever posted `/review`: as above, once more, then the fallback.
