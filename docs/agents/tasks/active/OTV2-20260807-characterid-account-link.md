# OTV2-20260807-characterid-account-link

```yaml
task_id: OTV2-20260807-characterid-account-link
title: Record CharacterId global identity and Platform account linkage
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-characterid-account-link
pr: 68
base_sha: 6804f5d67b63f1374a9efa3710bcaad10805c801
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T11:45:00+02:00
updated_at: 2026-08-07T11:49:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-characterid-account-link.md
  - docs/architecture/FND-ID-01_CHARACTER_ID_ACCOUNT_LINK_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/FND-ID-01_CHARACTER_ID_ACCOUNT_LINK_OWNER_BASELINE.md
depends_on:
  - FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md
  - ADR-0003 Platform Identity / Gateway boundary
blocks:
  - complete FND-ID-01 identifier catalogue until CharacterId issuer/scope/account linkage is consumed
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Record the owner-accepted decision that `CharacterId` is a globally unique, strongly typed full-128-bit UUIDv7 issued by the game-domain Character authority; it is not semantically scoped by `WorldId`, survives rename and world transfer, is never reused after deletion, and is linked explicitly to Platform-owned `AccountId` so Platform WWW can consume an authorized AccountId-to-CharacterId projection.

## Architecture and source of truth

- `PROVEN`: the identifier baseline classifies CharacterId as durable cross-boundary identity.
- `PROVEN`: the UUIDv7 baseline selects UUIDv7 for CharacterId and names the character domain as logical generator.
- `PROVEN`: ADR-0003 keeps AccountId/Identity authority in Oteryn Platform.
- `OWNER_ACCEPTED`: on 2026-08-07 the project owner accepted the global CharacterId model, stable identity across rename/world transfer, non-reuse after deletion and direct Platform-WWW account linkage with separated ownership.

## Acceptance criteria

- [x] Define `CharacterId` as strongly typed UUIDv7 preserving all 128 bits.
- [x] Define CharacterId as global semantic identity rather than WorldId + CharacterId.
- [x] Assign canonical CharacterId issuance to game-domain Character authority.
- [x] Preserve AccountId ownership in Platform Identity.
- [x] Define explicit validated AccountId -> CharacterId[] relationship for Platform WWW projections.
- [x] Preserve CharacterId across rename and world transfer.
- [x] Keep character name and current WorldId as mutable state rather than character identity.
- [x] Forbid CharacterId reuse after deletion and preserve tombstone/audit direction.
- [x] Keep identity separate from session/lease/fencing authority.
- [x] Defer exact Platform API, projection schema, wire encoding, persistence and character lifecycle implementation to later contracts.
- [x] Do not implement Rust runtime, protocol, database or Platform behavior.
- [ ] Exact-head checks and independent audit before merge readiness.

## Excluded scope

No Rust runtime, character service implementation, account database mutation, protocol schema, database DDL, Platform WWW implementation, Bazaar implementation, production activation or completion of the full `FND-ID-01` catalogue.

No write to `blakinio/Oteryn-Platform` is authorized by this task.

## Implementation / findings

PR #68 owns only the two declared documentation paths. PRs #63 through #67 are separate architecture packages with no overlapping paths.

The accepted model deliberately separates three concerns: Platform-owned `AccountId`, globally stable game-domain `CharacterId`, and mutable current-world membership. Platform WWW consumes an authorized projection rather than acquiring gameplay authority.

## Validation

### Focused
- command/run: pending exact-head documentation/governance workflow
- result: pending

### Component/integration
- command/run: `NOT_APPLICABLE` — architecture documentation only
- result: `NOT_APPLICABLE`

### E2E
- scenario: `NOT_APPLICABLE` — no executable runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI
- final head: recorded in immutable PR/check evidence after this final content commit
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending immutable PR/check evidence
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending final diff review
- unresolved review threads: pending
- related PRs: #63 through #67 are separate non-overlapping FND-ID-01/reconciliation packages
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Owner CharacterId decision is recorded and PR #68 is open with only two declared documentation paths.
status: validating
branch: docs/OTV2-20260807-characterid-account-link
head_sha: null
pr: 68
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
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
next_action: Review the exact two-file diff and exact-head CI for PR #68 without moving the head unless a material defect is found.
```
