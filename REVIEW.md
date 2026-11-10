# Review guide

Every PR is reviewed before it is merged (rule in `RULES.md`, procedure in the skill `pr-review`): by PR-Agent
(owner, 22.10.2026); a new Claude agent checks the **Wichtig** of earlier
rounds, and reviews when PR-Agent fails. The
reviewer knows nothing of the session that wrote the change: it gets the
PR's intent, its diff and the repository, and this file. The owner does not read code; this review is the second reader.

## What failure looks like here

Kaiwa teaches Japanese by conversation. A bug that matters is one a learner
notices or learns from:

- **Teaches wrong Japanese:** a wrong correction or explanation, a false
  reading or pitch, a word or grammar point on the wrong JLPT level, a
  dictionary hit that is not the word in the sentence (は → 葉).
- **Judges the learner wrongly:** placement, difficulty or mora hints that
  misfire (false alarms) or stay silent, streak, freeze or XP that count
  wrong, gamification shown where it is switched off.
- **Breaks the conversation:** no tutor answer, a hanging GUI, a dead page.
- **Leaks:** data leaves the machine where the learner chose local only, an
  API key in a file or log instead of the keyring, a model downloaded
  without a click or a checksum, the demo in a shipped binary (Q-15).
- **Breaks the contract:** an older profile or export no longer loads.
- **Lies:** STAND, ROADMAP, LASTENHEFT, ADRs or comments say what the
  code does not do.

## What to check in particular

- GUI rule: Kante components and `KanteStyle.*` roles as they are, no raw
  colours or sizes; `app/qml/Kante` changes only by `tools/sync-kante.sh`.
- A feature links its requirement IDs (`/LFxxx/`, `/LDxxx/`, `/LLxxx/`),
  has tests and a persona journey. Personas change only with their
  checks, never `start_level` or planted errors to make a run pass.
- Bug fixes: does the fix remove the cause? Is there a test that failed
  before it? A simulation check holds over several seeds, not just one.
- New text exists in German and English; no German in the English GUI.
- Persona names and ids from the demo world stay stable.
- Release: the release log with a screenshot per language; done only
  when the landing page shows the entry.
- CI: both copies of `ci.yml` (`.gitea/`, `.github/`) change together;
  `pr-agent.yml` is Gitea only.
- Content (scenarios, grammar, curriculum): Japanese and explanations are
  right, sources and licences match ADR-007 and `NOTICE`.

## Skip

- `Cargo.lock` files, `app/qml/Kante/` and `kante.qrc` (synced), the
  binary `data/dict/*.sqlite`, `app/shots/`, `qa/corpus/audio/` (LFS).
- Formatting and style that `cargo fmt` and `clippy` already enforce.

## How to report

- Every finding names file and line and the concrete input or state that
  goes wrong. No guess without evidence; if unsure, say so.
- Each finding gets one label: **Wichtig** (a bug from the list above,
  must be fixed or answered before the merge), **Kleinigkeit** (optional,
  at most five) or **Bestand** (the bug was there before this PR; not
  blocking).
- The report starts with the count, e.g. "2 Wichtig, 1 Kleinigkeit" (the
  session puts the heading above it). The count of **Wichtig** holds every
  one still open, also from an earlier round, e.g. "1 Wichtig (offen aus
  Runde 1)". Only when none is open it says plainly: "Nichts zu beheben."
- In a later round: say for each open **Wichtig** of the last round whether
  it is fixed, or whether its answer is accepted, with the reason. PR-Agent
  sees no earlier round: it reports only its own findings with their labels,
  and a new Claude agent closes the open ones (skill `pr-review`).
- Findings in German, code names as they are.
