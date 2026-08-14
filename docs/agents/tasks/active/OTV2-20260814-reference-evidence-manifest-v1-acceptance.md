# OTV2-20260814-reference-evidence-manifest-v1-acceptance

```yaml
task_id: OTV2-20260814-reference-evidence-manifest-v1-acceptance
title: Accept and pin Reference evidence/parity manifest v1
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/reference-evidence-manifest-v1-acceptance
pr: 252
base_sha: 76d65d8bbd2a8eaca46b671fcd5d71a9d6382fa3
head_sha: 7d00b1cd22be10c774e598fa93bdd6536a6b3425
final_head_sha: null
final_head_frozen_at: null
owner: architecture continuation agent
created_at: 2026-08-14T10:15:00+02:00
updated_at: 2026-08-14T10:35:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260814-reference-evidence-manifest-v1-acceptance.md
  - docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
public_contracts:
  - docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
depends_on:
  - docs/architecture/GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md
  - docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json
  - docs/architecture/GAME-ABILITY-01_REFERENCE_MECHANIC_CATALOGUE_ENTRY_PARITY_FIXTURE_CONTRACT.md
blocks:
  - reviewed mechanic-level Reference evidence/parity population
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Promote the already-delivered v1 Reference evidence/parity registry to an owner-accepted semantic pin without rewriting the historical candidate contract, changing schema v1, adding mechanic cases or authorizing executable work, and reconcile the canonical current-status overlay so continuation cannot rebuild the accepted manifest.

## Architecture and source of truth

- **PROVEN:** PR #220/#221 delivered and lifecycle-closed the candidate manifest/schema package; schema v1 remains blob `208506f461231eb3ed8966ae16dade0764eb39b8`.
- **PROVEN:** pre-promotion manifest revision 1 was `CANDIDATE_NOT_ACCEPTED`, with empty `cases`, all nine domains `NO_MECHANIC_CASES_REGISTERED`, and `canonical_digest=null`.
- **PROVEN:** PR #249/#250 defines catalogue/fixture binding and blocks trustworthy parity promotion until an accepted manifest/schema revision is pinned.
- **PROVEN:** PR #252 now changes exactly this task, a later owner-acceptance baseline, the existing manifest JSON, and the canonical current-status overlay; schema v1 is not changed.
- **DERIVED:** preserving the candidate contract as history and adding a later acceptance baseline is safer than editing historical candidate status prose in place.
- **UNKNOWN:** no mechanic-level Reference behavior is promoted by this task.
- **CONFLICT:** none introduced; PR #191 remains separate GAME-CHAR provenance work.

## Acceptance criteria

- [x] Preserve the immutable first Reference target ID and 2026-07-28 boundary.
- [x] Keep schema version 1 byte-for-byte unchanged and record its exact repository blob identity.
- [x] Promote the machine manifest to revision 2 with `status=ACCEPTED`.
- [x] Keep `cases` empty and every domain coverage entry fail-closed.
- [x] Preserve independent evidence / implementation / parity axes and all existing fail-closed policy flags.
- [x] Keep `canonical_digest=null` truthful until accepted digest tooling exists; do not hand-compute a digest.
- [x] Add an explicit acceptance/pin contract that supersedes only candidate-status/pinning clauses, not evidence semantics.
- [x] Reconcile `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` so it no longer instructs successors to build an already accepted manifest.
- [x] Select the first reviewed representative `ABILITY_COMBAT` mechanic evidence case(s) + bounded parity fixture(s) as the next paper-only programme action after lifecycle closeout.
- [x] Do not create a new stable architecture gate ID.
- [x] Do not authorize runtime/client/protocol/DDL/Platform/production/external-repository work.
- [ ] Complete repaired exact-head full-diff self-review, repository validation, Codex finding resolution, merge and lifecycle closeout.

## Excluded scope

No mechanic cases or factual parity promotions; no evidence acquisition/official-client automation; no runtime/client/server/protocol implementation; no persistence/DDL/migrations; no Platform/external-repository writes; no production changes; no proprietary assets/code/protocol acquisition; no canonical digest tooling; no new stable gate ID. Shared overlays other than `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` remain for lifecycle reconciliation rather than expanding this semantic delivery further.

## Implementation / findings

Issue #251 owns this acceptance promotion. Schema v1 is intentionally untouched. Manifest revision 2 binds the same immutable target, sets `status=ACCEPTED`, keeps every domain inventory entry fail-closed and appends acceptance history without adding cases.

Self-review finding 1 on superseded head `f2836941e76cf2cfe43e53edb916653fc1ccc6f1`: the machine manifest listed the downstream GAME-ABILITY catalogue contract as its own normative dependency. Commit `5f5c9e89b5daf0947a3e4e32edfe8b4056cdf660` removed that reverse dependency. GAME-ABILITY remains a consumer described by the acceptance baseline; the evidence manifest remains owned by GAME-VISION/evidence semantics.

Owner-authorized Codex review of exact head `18936e1518ae197471da78f6e9d5d41157ff6034` found one P2: the canonical current-status overlay still selected “build the manifest” as next work and could cause duplicate delivery. Commit `7d00b1cd22be10c774e598fa93bdd6536a6b3425` reconciled only that canonical progression overlay: it records manifest v1 accepted/pinned and selects reviewed `ABILITY_COMBAT` evidence cases/fixtures next. The review finding is not considered resolved until the repaired diff is rechecked and the review thread is explicitly resolved.

Until canonical digest tooling is separately accepted, executable consumers remain unauthorized. Paper evidence may bind target ID + schema version + manifest revision + exact repository delivery evidence; mutable `latest` lookup is not authorized.

## Validation

### Focused

- manifest JSON parse: **PASS**.
- changed-field/schema review: **PASS** — unchanged schema v1 permits `manifest_revision >= 1`, `status=ACCEPTED`, `canonical_digest=null`, nonempty normative-contract strings and the appended `manifest_revision/date/summary/issue` history entry.
- acceptance pin invariants: **PASS** — immutable target unchanged; revision 2; status accepted; `cases=[]`; nine unique fail-closed domains; all fail-closed policy flags preserved; digest remains null.
- schema identity: **PASS** — schema path is absent from PR diff and remains blob `208506f461231eb3ed8966ae16dade0764eb39b8` on trusted base.
- layering review: **PASS after repair 1** — no downstream GAME-ABILITY dependency remains in machine manifest `normative_contracts`.
- canonical progression review: **PASS at content head `7d00b1cd22be10c774e598fa93bdd6536a6b3425`** — current-status no longer selects duplicate manifest construction; final resulting-head recheck still required.

### Component/integration

- command/run: `NOT_APPLICABLE` — paper-only contract/registry/status promotion.
- result: `NOT_APPLICABLE`.

### E2E

- scenario: `NOT_APPLICABLE` — no executable behavior changes.
- result: `NOT_APPLICABLE`.

### Exact-head CI

- all CI from `18936e1518ae197471da78f6e9d5d41157ff6034` is stale after the P2 repair.
- trigger source: pull_request.
- fresh resulting-head generation: required.
- result: pending.

## Self-review

- superseded reviewed heads: `5f5c9e89b5daf0947a3e4e32edfe8b4056cdf660`, `18936e1518ae197471da78f6e9d5d41157ff6034`.
- method/reviewer: implementing/coordinating architecture continuation agent.
- material findings: 2 total — one layering self-review finding and one Codex P2 progression finding; both materially repaired.
- verdict: pending final full-diff recheck on the resulting head.

## Independent review

- required by base risk classification: NO; paper-only acceptance/pinning with no security/protocol/durable mutation/production authority expansion.
- owner-funded Codex/OpenAI use: owner explicitly authorized Codex review for this exact PR #252; permission does not carry to a different PR/task.
- Codex review on `18936e1518ae197471da78f6e9d5d41157ff6034`: one material P2, repaired by current-status reconciliation.
- repaired-head re-review/thread resolution: pending.

## PR and closeout

- PR: #252 (ready for review).
- changed-file review: four intended paths after P2 repair.
- unresolved review threads: one Codex P2 pending repaired-head verification/resolution.
- related/superseded PRs: #220/#221 historical candidate delivery; #249/#250 downstream catalogue contract; #191 separate and untouched.
- protected auto-merge: pending.
- merge commit/result: pending.
- ownership release: pending.

## Context checkpoint

```yaml
last_progress: Repaired Codex P2 by reconciling the canonical current-status progression so accepted manifest v1 cannot be rebuilt from the canonical continuation path.
status: validating
branch: docs/reference-evidence-manifest-v1-acceptance
head_sha: 7d00b1cd22be10c774e598fa93bdd6536a6b3425
pr: 252
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: fresh resulting head required
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: false
blocker: null
next_action: Perform full-diff self-review on the resulting head, verify the Codex P2 is materially fixed, resolve only that repaired thread, and inspect fresh exact-head Merge gate generation.
```
