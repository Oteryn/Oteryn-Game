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
  `https://www.tibia.com/gameguides/?subtopic=manual&section=<name>`, one request at a time, a
  2-second delay, an honest `User-Agent`, only an HTTP 200 accepted. Writes `<dir>/manifest.json`
  (per page: `url`, `fetched_at` UTC, `http_status`, `sha256`, `visible_text_chars`, plus a
  top-level `spells` status) and `<dir>/facts.json` (bounded, deterministic
  `{section, anchor, key, value}` facts with a factual signal, extracted with `html.parser`, each
  value ≤300 chars, ≤`FACTS_PER_SECTION_CAP` per section). Raw page text is never written. A
  non-200 status or a Cloudflare challenge stops `fetch` immediately and writes nothing.
- Spell library (S15, #1077): if `tools/content-schema/spell-authoring/tibiacom_spells.py` exists
  on the checkout, `fetch` calls its `parse_spell_library()`/`parse()` instead of parsing itself.
  Absent on `main` (only on the open #1077 PR), so this sets `manifest.json["spells"] =
  "PENDING_1077"` and makes no spell-library request.
- `verify <dir>...`: offline. Schema, sha256 format, per-value/per-section/total-size caps, the
  fact-to-visible-text ratio, exact six-section completeness (URL, HTTP 200, ≥1 fact), and the
  `spells` enum/page-consistency.
- `self-test`: offline, embedded HTML fixture plus passing/broken `verify` snapshots.

`.github/workflows/tibiacom-snapshot-verify.yml` runs `self-test` plus `verify` on every
`imports/official/tibia-com/*/` directory on PRs touching `tools/official-capture/**` or
`imports/official/tibia-com/**`; it has no network step. `imports/official/index.json` documents
the new `tibia-com/<YYYY-MM-DD>/` snapshot convention (owner-run, immutable once committed, a new
date directory per new capture); `imports/official/tibia-com/README.md` gives the exact owner
command.

## Architecture and source of truth

- `PROVEN`: PR #1077 (open) — tibia.com blocks the build container and GitHub-hosted runners with
  a Cloudflare challenge; `fetch` runs once from an owner machine, never bypassing the challenge.
  `tools/content-schema/spell-authoring/tibiacom_spells.py` exists only on that PR's branch, not on
  `main` (confirmed at this task's base commit).
- `PROVEN`: `tools/content-schema/monster-authoring/wiki_br_capture.py` is the modelled style —
  single stdlib file, honest `User-Agent`, raw text never leaves the fetching machine, embedded
  `self-test`.
- `PROVEN`: `.github/workflows/monster-wiki-capture.yml`/`spell-wiki-capture.yml` set the
  capture-workflow conventions followed here (pinned action SHAs, `permissions: contents: read`,
  path-scoped `pull_request` + `workflow_dispatch`, `concurrency`); this workflow differs by design
  in having no fetch/network step.

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

- [x] `fetch` covers the six named manual sections and the spell-library delegation/`PENDING_1077`
      fallback, writes only `manifest.json`/`facts.json`, only HTTP 200 accepted, stops cleanly on
      a Cloudflare challenge.
- [x] `verify` checks schema, sha256 format, per-value/per-section/ratio/total-size caps, six-section
      completeness, and `spells` consistency.
- [x] `self-test` passes offline with an embedded fixture.
- [x] `.github/workflows/tibiacom-snapshot-verify.yml` runs `self-test` + `verify` (no network) and
      rejects edits to already-committed dated snapshot directories.
- [x] `imports/official/index.json` documents the `tibia-com/<YYYY-MM-DD>/` convention (schema
      unchanged); `imports/official/tibia-com/README.md` gives the owner command.
- [x] `tools/content-schema/spell-authoring/**` and `spell-wiki-capture.yml` (owned by #1077)
      untouched.

## Excluded scope

- No tibia.com data captured here; no `imports/official/tibia-com/<date>/` snapshot committed
  (owner's separate `fetch` run).
- No change to `tools/content-schema/spell-authoring/**` or `spell-wiki-capture.yml` (#1077).
- No bypass, retry, UA spoofing or challenge-solving against tibia.com's Cloudflare block.
- No runtime, protocol or persistence change.

## Implementation / findings

- `tools/content-schema/spell-authoring/tibiacom_spells.py` is absent from `main` (only on the
  open #1077 PR branch), so `fetch` takes the `PENDING_1077` branch; the dynamic-import branch
  calling its `parse_spell_library()`/`parse()` never copies #1077's code, only calls into it.
- Extractor: `ManualSectionParser` (stdlib `html.parser`) reads h1-h4 headings (anchor = `id` attr
  or a slug of the text) and the text of `p`/`li`/`td`/`th`/`dd` blocks under each, with no
  CSS-class guess at tibia.com's real structure (unreachable here — same Cloudflare block).

## Repair: Codex review findings (PR #1083, return to AUTHORING; two rounds)

Round 1, head `1a82643`, 4 findings, all repaired:

- P1 r4120578784 (no page copy): bounded factual-signal extraction (digit/`key: value`/named
  control key), `FACTS_PER_SECTION_CAP`=40; `manifest.json` records `visible_text_chars`; `verify`
  rejects a page over the 25% fact-to-visible-text ratio or the per-section cap.
- P2 r4120578795 (bad status): `fetch`/`verify` require HTTP 200 exactly.
- P2 r4120578800 (completeness): `verify` requires exactly the six sections at their exact URL,
  ≥1 fact each, and `spells` enum consistency.
- P2 r4120578808 (immutability): the workflow diffs PR base→head under
  `imports/official/tibia-com/`, rejecting `M`/`D`/`R` of an already-committed dated dir.

Round 2, head `a90b128`, 5 more P2s, all repaired:

- r4120758016: the spell library gets its own `SPELL_RECORDS_CAP`=600 (`facts_cap_for_section`),
  not the 40-fact manual cap; `fetch` caps delegated spell facts at write time too.
- r4120758045: `verify` rejects `visible_text_chars` ≤0 for any page with ≥1 fact.
- r4120758054: `fetch_url` now returns raw response bytes; `sha256` hashes those raw bytes, with a
  separate decode used only for parsing.
- r4120758029: the immutability logic moved into a tested `check-immutability` tool subcommand
  (`find_immutability_violations`) and now also rejects an `A` landing inside an already-committed
  dated directory, not just `M`/`D`/`R`.
- r4120758056: `verify` rejects a duplicate manifest page section or url instead of silently
  overwriting `pages_by_section`.

All nine review threads replied to before their respective pushes; self-test covers all nine.

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
- material findings: none in self-review beyond the four Codex findings above, all repaired;
  `self-test` covers the happy path plus many deliberately-broken `verify` cases.
- verdict: ready for independent review

## Independent review

- required: it happened regardless of the `NO` self-assessment above (evidence/tooling-only, no
  public contract/protocol/persistence/authority surface): automated PR review is unconditional in
  this repository.
- exact head: `1a82643` (round 1), `a90b128` (round 2)
- method/auditor: Codex, automated PR review (not triggered by this worker)
- material findings: 9 total across two rounds (see Repair section above) — all accepted and
  repaired
- verdict: findings addressed; a fresh review of the repaired head is for the control plane to
  request, not this worker (no `@codex` trigger from this task)

## PR and closeout

- changed-file review: all changed files fall within `owned_paths`; `tools/content-schema/spell-authoring/**`
  and `spell-wiki-capture.yml` untouched.
- unresolved review threads: none — see Repair/Independent review above.
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
