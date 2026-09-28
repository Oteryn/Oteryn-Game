# OTV2-20260928-tibiacom-snapshot-tool

```yaml
task_id: OTV2-20260928-tibiacom-snapshot-tool
title: Official tibia.com manual/spell-library capture tool (owner-run fetch, offline verify)
mode: BUILD
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/tibiacom-snapshot-tool
issue: 1077
pr: 1083
base_sha: 8d320703ec39b39c44c0b38268a07e4b0579e060
head_sha: 7b7f75eeaf0bff834fb9690f5ea069c3e38f94fd
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn content-import worker (Claude Code)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - tools/official-capture/tibiacom_capture.py
  - imports/official/tibia-com/README.md
  - imports/official/index.json
  - .github/workflows/tibiacom-snapshot-verify.yml
  - docs/agents/tasks/active/OTV2-20260928-tibiacom-snapshot-tool.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

A single-file stdlib Python 3 tool, `tools/official-capture/tibiacom_capture.py`, modelled on
`tools/content-schema/monster-authoring/wiki_br_capture.py`, that lets the repository owner take an
official tibia.com snapshot exactly once, on their own machine, and lets CI check that snapshot
forever after without any network access:

- `fetch --out <dir>`: fetches the six tibia.com manual sections (`controls`, `characters`,
  `combat`, `world`, `controls_trading`, `starting`) from
  `https://www.tibia.com/gameguides/?subtopic=manual&section=<name>`, one request at a time with a
  2-second delay and an honest `User-Agent`. Writes `<dir>/manifest.json` (per page: `url`,
  `fetched_at` UTC, `http_status`, `sha256` of the raw body, plus a top-level `spells` status) and
  `<dir>/facts.json` (deterministic `{section, anchor, key, value}` facts extracted from headings
  and the block text under them with `html.parser`, each value capped at 300 characters). Raw page
  text is never written to disk. If a response is a Cloudflare challenge (HTTP 403/503 or a known
  challenge marker in the body), `fetch` stops immediately, prints a clear message, and writes
  nothing.
- Spell library (S15, #1077): if `tools/content-schema/spell-authoring/tibiacom_spells.py` exists
  on the checkout, `fetch` imports it and calls its `parse_spell_library()`/`parse()` entry point
  on the fetched `https://www.tibia.com/library/?subtopic=spells` page instead of parsing it
  itself. It does not exist on `main` yet (only in the open #1077 PR), so this run sets
  `manifest.json["spells"] = "PENDING_1077"` and makes no spell-library request.
- `verify <dir>...`: offline. Checks both files' schema, that every `sha256` is a 64-hex-digest
  string, that every fact `value` is at most 300 characters and the file's total fact-value bytes
  stay under a fixed cap (the no-full-text rule), and that every fact's `section` names a page in
  that snapshot's `manifest.json`.
- `self-test`: offline, against an embedded tiny HTML fixture; also exercises `verify` against
  passing and deliberately broken snapshots in a temp directory.

`.github/workflows/tibiacom-snapshot-verify.yml` runs `self-test` plus `verify` on every
`imports/official/tibia-com/*/` directory on PRs touching `tools/official-capture/**` or
`imports/official/tibia-com/**`; it has no network step. `imports/official/index.json` documents
the new `tibia-com/<YYYY-MM-DD>/` snapshot convention (owner-run, immutable once committed, a new
date directory per new capture); `imports/official/tibia-com/README.md` gives the exact owner
command.

## Architecture and source of truth

- `PROVEN`: PR #1077 (open) — tibia.com blocks both the build container and GitHub-hosted runners
  with a Cloudflare challenge; the fix is to run `fetch` once from an owner machine and never
  attempt to bypass the challenge. `tools/content-schema/spell-authoring/tibiacom_spells.py` exists
  only in that PR's branch, not on `main`, confirmed by listing `tools/content-schema/spell-authoring/`
  at this task's base commit.
- `PROVEN`: `tools/content-schema/monster-authoring/wiki_br_capture.py` is the modelled style: a
  single stdlib file, an honest descriptive `User-Agent`, raw text never leaving the machine that
  fetched it, and an embedded-fixture `self-test` subcommand.
- `PROVEN`: `.github/workflows/monster-wiki-capture.yml` and `spell-wiki-capture.yml` are the
  capture-workflow conventions followed here (pinned `actions/checkout`/`actions/setup-python` SHAs,
  `permissions: contents: read`, a path-scoped `pull_request` trigger plus `workflow_dispatch`,
  `concurrency` group) — this workflow differs from them by design in having **no** fetch/network
  step, since it only runs the offline `self-test`/`verify` subcommands.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Evidence-capture tooling only: no production mutation, no session/lease/generation/authority
  fencing, no PREPARE/COMMIT, no persisted-recovery interpretation. `fetch` is a read-only HTTP GET
  against a public site run manually by the owner; `verify`/`self-test` are pure offline JSON/HTML
  checks with no I/O beyond reading the snapshot directory the caller names.
```

## Acceptance criteria

- [x] `tibiacom_capture.py fetch` covers the six named manual sections and the spell-library
      delegation/`PENDING_1077` fallback, writes only `manifest.json`/`facts.json` (never raw page
      text), and stops cleanly on a detected Cloudflare challenge.
- [x] `tibiacom_capture.py verify` checks schema, sha256 format, the 300-char per-value cap, a
      total-size cap (no-full-text rule), and that every fact references a manifest page.
- [x] `tibiacom_capture.py self-test` passes offline with an embedded fixture.
- [x] `.github/workflows/tibiacom-snapshot-verify.yml` runs `self-test` + `verify` on PRs touching
      the owned tool/snapshot paths, with no network step.
- [x] `imports/official/index.json` documents the `tibia-com/<YYYY-MM-DD>/` convention; keeps the
      file's existing schema (`OTERYN_GAME_TREE_DIRECTORY/v1`).
- [x] `imports/official/tibia-com/README.md` gives the exact 2-3 line owner command.
- [x] `tools/content-schema/spell-authoring/**` and `spell-wiki-capture.yml` (owned by #1077)
      untouched.

## Excluded scope

- No tibia.com data is captured by this task; no `imports/official/tibia-com/<date>/` snapshot
  directory is committed here (that is the owner's separate `fetch` run).
- No change to `tools/content-schema/spell-authoring/**`, `tibiacom_spells.py` itself, or
  `.github/workflows/spell-wiki-capture.yml` — those belong to #1077.
- No bypass, retry, UA spoofing or challenge-solving against tibia.com's Cloudflare block, in this
  tool or anywhere else.
- No runtime, protocol or persistence change.

## Implementation / findings

- Confirmed at authoring time that `tools/content-schema/spell-authoring/tibiacom_spells.py` is
  absent from `main` (present only on the open #1077 PR branch); `fetch` therefore takes the
  `PENDING_1077` branch and makes no request to the spell-library URL in this environment. The
  dynamic-import branch (calling `parse_spell_library()`/`parse()` when the module exists) is
  written defensively — it never copies #1077's parsing logic, only calls into it — but is
  exercised only once that module lands on `main`.
- `verify`'s no-full-text rule is two checks: a hard 300-character cap per fact value (matching
  `wiki_br_capture.py`'s field cap convention, this tool's cap is fixed rather than configurable),
  and a 200,000-byte cap on the sum of all fact-value bytes in one snapshot, which a genuine
  six-section-manual-plus-spell-library fact set stays far under, but a full page dump would not.
- The extractor (`ManualSectionParser`, stdlib `html.parser`) is deliberately generic: it reads
  h1-h4 headings (using the tag's `id` attribute when present, else a slug of its text, as the
  anchor) and the plain text of every `p`/`li`/`td`/`th`/`dd` under each heading, with no
  CSS-class-based guess at tibia.com's real page structure (which this environment cannot fetch to
  inspect, since it is itself blocked by the same Cloudflare challenge).

## Validation

### Focused

- command/run: `python3 tools/official-capture/tibiacom_capture.py self-test`;
  `python3 tools/agents/validate_governance.py`;
  `python3 tools/repository/validate_repository_policy.py`
- result: self-test PASS; governance validator PASS; repository-policy validator PASS (see PR for
  exact-head confirmation).

### Component/integration

- command/run: `NOT_APPLICABLE` — no component beyond the single tool; its own `self-test` covers
  `extract_facts`/`verify_snapshot` end-to-end offline.
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — the one live-network path (`fetch` against tibia.com) is owner-run
  outside CI/this session by design; nothing in this repository can exercise it end-to-end.
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: see PR for the exact pushed head and its checks.
- trigger source: push to `claude/tibiacom-snapshot-tool`.
- workflow/run/job: `tibiacom-snapshot-verify.yml` (new; runs only when a snapshot directory
  exists) plus the repository's standard PR gates.
- runner assignment: ordinary hosted runner (this job makes no tibia.com request).
- classification: pending PR checks.
- result: pending PR checks.

## Self-review

- exact head: see PR
- method/reviewer: implementing agent (this session)
- material findings: none found in self-review; `self-test` exercises both the happy-path
  extraction/verification and several deliberately-broken `verify` cases (oversized value,
  fact-references-unknown-section, malformed sha256, missing directory).
- verdict: ready for independent review

## Independent review

- required: NO — evidence/tooling-only change, no public contract, protocol, persistence or
  authority surface touched; owned paths are new files plus a documentation-only edit to
  `imports/official/index.json`'s `notes` field (schema/path/kind/owner unchanged).
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: all changed files fall within `owned_paths` above; `tools/content-schema/spell-authoring/**`
  and `.github/workflows/spell-wiki-capture.yml` untouched.
- unresolved review threads: none at open.
- related/superseded PRs: none superseded; this task's PR is #1083, which references #1077 (the
  tibia.com block report and the S15 spell library, which this task's `PENDING_1077` fallback
  defers to).
- protected auto-merge: not requested by this task; left to owner/control-plane review.
- merge commit/result: pending.
- ownership release: pending merge.

## Context checkpoint

```yaml
last_progress: PR #1083 opened against main on head 7b7f75e; awaiting CI
status: validating
branch: claude/tibiacom-snapshot-tool
head_sha: 7b7f75eeaf0bff834fb9690f5ea069c3e38f94fd
pr: 1083
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: push
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: wait for PR #1083 checks, then owner runs `tibiacom_capture.py fetch` once and commits the resulting imports/official/tibia-com/<date>/ snapshot
```
