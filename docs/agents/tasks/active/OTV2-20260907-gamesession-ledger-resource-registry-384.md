# OTV2-20260907-gamesession-ledger-resource-registry-384

```yaml
task_id: OTV2-20260907-gamesession-ledger-resource-registry-384
title: Register durable GameSession nonreuse lifetime bound
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
issue: 384
branch: agent/gamesession-ledger-resource-registry-384
pr: null
base_sha: 2dce2162ef62ee75a390b84c1a4d97fab704f97f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: work-controlled-registry-writer
created_at: 2026-09-07T14:00:52Z
updated_at: 2026-09-07T14:51:01Z
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
  - Oteryn/Oteryn-Game#385 protected as main@2dce2162ef62ee75a390b84c1a4d97fab704f97f
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
- `PROVEN`: allocation PR #385 integrated through full Merge Queue run `34134404703` and protected `main` readback is exactly `2dce2162ef62ee75a390b84c1a4d97fab704f97f`.
- `PROVEN`: Issue #384 comment `5572398472` grants the exclusive three-path registry lease from that immutable admission SHA.
- `PROVEN`: fresh open-PR reconciliation after #385 found no competing writer for the allocated registry/task/plan paths.

The worker copies protected authority; it does not select resource values, error codes or retry behavior.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to executable authority mutation: this task changes a contract registry only and performs no session, PREPARE/COMMIT, persistence or production operation. Independent review is nevertheless required because the registry row controls future session/recovery capacity behavior.

## Acceptance criteria

- [x] Work records one exact protected admission SHA and exclusive registry lease before the worker branch exists or registry bytes change.
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

Protected allocation #385 is terminally integrated and read back. The worker branch was created exactly from `main@2dce2162ef62ee75a390b84c1a4d97fab704f97f` only after the exclusive lease was recorded. The implementation is limited to copying Decision A Section 8 into the existing registry while preserving all admitted entry objects unchanged; no value selection remains open.

## Validation

### Focused

- command/run: pending registry mutation
- result: pending

### Component/integration

- command/run: repository governance/contract validation after registry mutation
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
- related/superseded PRs: #383 architecture prerequisite protected; #385 allocation protected
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: protected allocation #385 read back and exclusive registry lease admitted
status: implementing
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
blocker: null
next_action: append the exact protected Decision A Section 8 row and preserve all admitted registry entry objects
```
