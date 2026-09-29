# OTV2-20260928-tibiacom-spells-hardening

```yaml
task_id: OTV2-20260928-tibiacom-spells-hardening
title: tibia.com capture - spell library through the real #1077 parser, literal YYYY-MM-DD hardening
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1181
allocation_comment: "#162, owner decision D100"
base_branch: main
branch: claude/tibiacom-spells-hardening
base_sha: df2dd464ceb97287fea0bdcc717911c4c8ef182c
head_sha: f31eb5a3f182b6d4f444fbbbb843ff18260ad433
final_head_sha: f31eb5a3f182b6d4f444fbbbb843ff18260ad433
final_head_frozen_at: 2026-09-28
owner: "Oteryn: content import" (Claude Code)
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/official-capture/**
  - imports/official/tibia-com/README.md
  - docs/agents/tasks/active/OTV2-20260928-tibiacom-spells-hardening.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

1. **Spells.** `tibiacom_capture.py fetch` no longer looks for guessed `parse_spell_library()`/`parse()`
   names (neither exists). #1077's parser is `tools/content-schema/spell-authoring/tibiacom_spells.py`:
   `list_facts(text, captured)` over the list view (`LIST_COLUMNS`), the function that produced
   `samples/tibiacom-spell-list-2026-09-28.json`. The explicit adapter
   `spell_facts_from_library_html(module, body, captured_date)` cuts the fetched page's list table
   (the one whose header is exactly `LIST_COLUMNS`) into tab-separated text, calls `list_facts`, and
   writes one `spells` fact per row: anchor `list`, key `spells.list.<slug>`, value a compact JSON
   object of the row's fields. The field mapping stays in #1077's module; the adapter never copies it.
   A module without `list_facts`/`LIST_COLUMNS`, a page without the list table, a malformed row, an
   over-long row or more than `SPELL_RECORDS_CAP` rows aborts `fetch` with no output. With the module
   absent the snapshot is still `PENDING_1077`.
2. **Bounds that fit a table of facts.** The manual pages' 25% prose ratio cannot hold for a page that
   is itself a table of facts (the committed 193-row sample gives fact chars 3.25 x visible chars).
   `verify` now holds the `spells` section to `SPELL_FACT_TO_VISIBLE_TEXT_RATIO_LIMIT` (6.0), to the
   exact row schema `list_facts` emits (five required and two optional string fields, no extras; canonical JSON value; key equal to `spells.list.<slug of name>` (repeats numbered `-2`, `-3` in fact order), so each fact could have come from the adapter) and to the
   unchanged count, value and absolute-byte caps. Manual sections keep 25%. `fetch` now verifies what it
   wrote and removes the new directory if verification fails, so a run cannot leave a snapshot CI rejects.
3. **#1083 hardening.** Every date shape is literal ASCII `[0-9]` with `fullmatch` and a calendar check
   (`parse_iso_date`), not dependent on `date.fromisoformat`. `fetch` checks the `--out` directory name
   before any request (literal date or UTC run form, and equal to the run's own stamp); `verify` and
   `verify-root` apply the same rule to directory names, `captured_at` and `fetched_at`.
4. **README.** `imports/official/tibia-com/README.md` documents the owner command that adds spells in a
   NEW run directory. Existing snapshots stay immutable and no snapshot data is committed here.

## Architecture and source of truth

- `PROVEN`: `tibiacom_spells.py` on main defines `LIST_COLUMNS`, `list_facts`, `page_facts`, `facts`,
  `fetch` (Playwright); no `parse_spell_library`/`parse`.
- `PROVEN`: the adapter reproduces all 193 rows of the committed S15 sample field-for-field when the
  sample is rendered as an HTML table (checked once in scratch, not committed).
- `PROVEN`: the 2026-09-28 manual snapshot plus those 193 spell facts verifies with no errors.
- `UNKNOWN`: the live tibia.com list page HTML. tibia.com was not fetched. The adapter assumes a
  server-rendered `<table>` with the header `Name, Group, Type, Exp Lvl, Mana, Premium` and cells such as
  `Find Person (exiva "name")`, as in the owner's list copy behind S15. If the live page differs, `fetch`
  aborts with a clear reason and writes nothing.
- Detail pages (`page_facts`, per-spell cooldown/soul/amount) are not captured; the list view is what
  S15 already consumes.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Offline evidence tooling and a README. No runtime, session, lease, generation, authority or
  persisted-recovery semantics; no network access from CI or from this worker.
```

## Validation

### Focused

- run: `python3 tools/official-capture/tibiacom_capture.py self-test`;
  `python3 tools/official-capture/tibiacom_capture.py verify-root imports/official/tibia-com`;
  `python3 tools/official-capture/tibiacom_capture.py verify imports/official/tibia-com/2026-09-28
  imports/official/tibia-com/2026-09-28-160207Z`;
  `python3 tools/official-capture/tibiacom_capture.py check-immutability --base <base> --head <head>`;
  `python3 tools/agents/validate_governance.py`;
  `python3 tools/repository/validate_repository_policy.py`; `git diff --check`
- result: recorded against the frozen final head of the PR (see the PR body and checks).
- fixtures: `SPELL_LIST_FIXTURE_HTML` (a small recorded excerpt, not a full page: decoy table, script,
  list table with a quoted-words row, a `-` level row and an HTML entity) and
  `SPELL_LIST_EXPECTED_FACTS`; the self-test runs it through the real #1077 module, through a stubbed
  `cmd_fetch` (manifest `captured`, facts equal, snapshot verifies), at 200-row scale, and with the
  negatives: no list table, header-only, short row, missing/legacy entry points, over-long row, over-cap
  rows, and spells facts with an extra field (`payload`), a missing required field, a key/name slug
  mismatch, a wrong value type, a non-canonical value or a wrong anchor.
- negatives for the date check: non-padded, no-dash, ISO week/ordinal, fullwidth and Arabic-Indic digits,
  surrounding whitespace/newline, impossible date; in `parse_iso_date`, `parse_snapshot_directory_name`,
  `parse_utc_timestamp`, `verify`, `verify-root` and `fetch` (no network touched, nothing written).

### Component/integration/E2E

- `NOT_APPLICABLE`: no runtime component. CI job `tibia.com Snapshot Verify` runs the same self-test and
  verification offline. The real capture needs the owner's machine and is not run here.

### Exact-head CI

- candidate: the frozen final head of the PR; its live checks govern.
- result: pending

## Self-review

- exact head: the frozen final head of the PR
- method/reviewer: implementing agent (this session)
- findings: none open. The visible-text figure used to size the spells ratio omits page chrome, so the
  real margin is larger than measured (fact chars 3.25 x, raw bytes 5.35 x visible against 6.0).
- verdict: ready for independent review

## Independent review

- required: YES - changes the snapshot verifier bounds (a provenance/no-page-copy guard).
- verdict: pending (requested through the control plane, no `@codex` from this worker)

## Excluded scope

`.github/**` (the game-gate fan-in is batched separately), `tools/repository/**`, every existing
snapshot directory, `tools/content-schema/spell-authoring/**`: unchanged. No new snapshot data. No
`@codex`, no auto-merge.

## Context checkpoint

```yaml
last_progress: PR #1181 merged via Merge Queue as 59604a2; protected-main readback matched f31eb5a; record archived in the 2026-09-29 batch
status: completed
branch: claude/tibiacom-spells-hardening
pr: 1181
final_head_sha: f31eb5a3f182b6d4f444fbbbb843ff18260ad433
owner_action_required: run the spells capture command from imports/official/tibia-com/README.md after merge
blocker: null
next_action: none for this task
```

## Closeout

- merge commit/result: `59604a2` on protected `main` (#1181); the changed files are byte-identical to `f31eb5a`
- ownership release: all leases released at merge
- archived in the batch archive of 2026-09-29
