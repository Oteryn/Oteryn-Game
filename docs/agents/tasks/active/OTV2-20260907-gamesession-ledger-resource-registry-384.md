# OTV2-20260907-gamesession-ledger-resource-registry-384

```yaml
task_id: OTV2-20260907-gamesession-ledger-resource-registry-384
title: Register durable GameSession nonreuse lifetime bound
mode: CONTRACT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/gamesession-ledger-resource-registry-384
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned_until_protected_allocation
created_at: 2026-09-07T14:00:52Z
updated_at: 2026-09-07T14:00:52Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260907-gamesession-ledger-resource-registry-384.md
  - docs/agents/programs/OTV2_GAMESESSION_LEDGER_RESOURCE_REGISTRY_20260907.md
public_contracts:
  - FND-DUR-GAMESESSION-NONREUSE-V1
  - FND-04C GameSession Ledger Capacity Error Amendment
depends_on:
  - Oteryn/Oteryn-Game#383 protected as main@6b07f96d47de37971bb54fed5bb9c12decd1be17
  - Oteryn/Oteryn-Game#384 allocation protected and explicit Work admission
blocks:
  - WP2 Decision A implementation amendment
  - WP4 Decision A persistence/reload amendment
  - downstream Server Seam re-evaluation
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Protected `RESOURCE_LIMITS_REGISTRY.json` contains exactly one new architecture-owned row, `FND04-GAMESESSION-USED-IDS-PER-CHARACTER`, with the protected Decision A bound and error semantics, while every pre-existing registry entry remains unchanged. The task is then archived and its exclusive registry lease released.

## Architecture and source of truth

- `PROVEN`: PR #383 is protected as `main@6b07f96d47de37971bb54fed5bb9c12decd1be17` after full Merge Queue success.
- `PROVEN`: protected Decision A Section 8 owns the exact row content and hard maximum `65536`.
- `PROVEN`: protected FND-04C amendment distinguishes permanent ledger exhaustion from retryable transient `ADMISSION_CAPACITY_EXCEEDED`.
- `PROVEN`: open-PR changed-path preflight found no current writer for `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`.
- `UNKNOWN`: this task's immutable admission SHA until its allocation PR is protected/read back.

The worker copies protected authority; it does not select resource values, error codes or retry behavior.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to executable authority mutation: this task changes a contract registry only and performs no session, PREPARE/COMMIT, persistence or production operation. Independent review is nevertheless required because the registry row controls future session/recovery capacity behavior.

## Acceptance criteria

- [ ] Work records one exact protected admission SHA and exclusive registry lease before the worker branch exists or registry bytes change.
- [ ] Add exactly one new ID: `FND04-GAMESESSION-USED-IDS-PER-CHARACTER`.
- [ ] Hard maximum and configurable minimum/maximum are exactly `65536`.
- [ ] Row semantics and boundary tests match protected Decision A Section 8 and its FND-04C amendment; no retryable transient-capacity substitution.
- [ ] Every pre-existing registry entry object is unchanged.
- [ ] JSON parses; required fields are present; IDs remain unique; configured ranges satisfy registry rules.
- [ ] Applicable governance/contract validation and changed-file/diff inspection pass.
- [ ] Whole-diff self-review reports zero open material findings.
- [ ] Genuinely independent exact-head review reports zero open P0/P1/P2 material findings.
- [ ] Exact-head repository CI and protected Merge Queue succeed.
- [ ] Protected-main readback matches the accepted row, task is archived, branch/lease are released, and #162 receives the exact next WP2 action.

## Excluded scope

No other registry row change; no Foundation/runtime Rust; no Durability/SQL/migration; no Cargo/lockfile; no workflow, ruleset or protection change; no Server Seam; no Platform/Atlas/META; no production, deployment, credential, secret or live-data mutation. Do not release #361, #335, #356 or #247 from this task alone.

## Implementation / findings

No registry implementation is admitted by this allocation candidate. The first legal implementation action after protected admission is to compare the protected registry object set against the protected Decision A row, then append only that exact row on the dedicated worker branch.

## Validation

### Focused

- command/run: pending after protected admission
- result: pending

### Component/integration

- command/run: repository governance/contract validation after protected admission
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — contract-registry-only delivery changes no executable runtime behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES — resource semantics govern future session/recovery capacity behavior
- exact head: pending
- method/auditor: genuinely independent non-author reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #383 architecture prerequisite protected; no replacement registry PR exists at allocation time
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: prospective registry task recorded in coordinator allocation candidate
status: waiting
branch: agent/gamesession-ledger-resource-registry-384
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
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
blocker: allocation_not_yet_protected
next_action: protect and read back the coordinator allocation before creating the worker branch
```
