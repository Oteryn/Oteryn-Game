# OTV2-20260807-fnd-id01-minimum-scope

```yaml
task_id: OTV2-20260807-fnd-id01-minimum-scope
title: Freeze the minimum scope of the final FND-ID-01 contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-fnd-id01-minimum-scope
pr: 80
base_sha: 7194f510a09bb5aba6ceab94841f9a48d95e57da
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T19:57:00+02:00
updated_at: 2026-08-07T20:01:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-fnd-id01-minimum-scope.md
  - docs/architecture/FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md
depends_on:
  - ADR-0001 through ADR-0011
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - docs/architecture/UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_OWNER_DECISION_CHECKPOINT_2026-08-07.md
blocks:
  - complete FND-ID-01 contract must consume this scope decision before FND-02 freezes dependent protocol identity fields
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Record the owner-accepted rule that the final `FND-ID-01` contract is a **minimum cross-boundary foundation identifier contract**, not an exhaustive catalogue of every identifier that Oteryn may ever use.

The accepted baseline must keep the ordered foundation programme moving without prematurely designing durability, analytics, economy, social, quest, event, content or operations domains whose owning contracts have not yet been accepted.

## Architecture and source of truth

- `PROVEN` — `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` identifies `FND-ID-01` as the next ordered foundation gate and requires it before `FND-02` freezes dependent protocol identity fields.
- `PROVEN` — existing owner baselines already accept strong semantic types, UUIDv7 for adopted durable Oteryn-owned identities, explicit semantic scopes, and separation among durable identity, runtime-local handles, session-local handles and ordering/fencing values.
- `PROVEN` — `WorldId`, `ChannelId`, `NodeId`, `InstanceId`, `PartyId` and `CharacterId` already have substantial accepted semantics/ownership decisions.
- `DERIVED` — expanding `FND-ID-01` into every future domain ID would couple the foundation gate to later `DUR-*`, `ANL-*`, social, economy, content and gameplay contracts and would make the gate difficult to finish without improving foundation correctness.
- `PROVEN` — the project owner accepted the minimum-scope direction in the current architecture conversation on 2026-08-07.

## Acceptance criteria

- [x] State that the final `FND-ID-01` contract freezes only the minimum identifier semantics required by foundation cross-boundary contracts.
- [x] Define the minimum per-identifier dimensions that `FND-ID-01` must freeze.
- [x] Preserve every already accepted identifier semantic/representation/issuer decision.
- [x] Explicitly defer domain-specific durable identities to their owning later gates unless foundation compatibility requires an earlier semantic placeholder.
- [x] Keep protocol-specific sequencing/message encoding in `FND-02` and persistence/indexing in `DUR-*`.
- [x] Prevent a future implementer from interpreting the minimum catalogue as permission to use raw interchangeable UUIDs or unscoped IDs.
- [x] Preserve no-implementation mode.
- [ ] Review exact changed-file diff.
- [ ] Independent architecture audit against final exact head.
- [ ] Required exact-head documentation/governance checks.

## Excluded scope

This task does not:

- create the complete `FND-ID-01` contract;
- decide unresolved `GameSessionId`, admission, lease or reconnect implementation details owned by `FND-04`;
- decide final `CommandId`, message sequence, session-handle or wire encoding details owned by `FND-02`;
- decide PostgreSQL keys, indexes, partitions or migrations owned by `DUR-*`;
- freeze `ItemInstanceId`, `TradeId`, `MarketOfferId`, `RewardGrantId`, `EventId`, `TransactionId`, `BossAttemptId`, quest-run IDs or other later-domain catalogues merely because UUIDv7 is their likely representation direction;
- decide content-key/bundle-local ID representation owned by `DUR-04`;
- modify Rust code, runtime, protocol, Platform, persistence, database or production systems.

## Implementation / findings

The owner accepted a narrow final-gate boundary:

```text
FND-ID-01
    = minimum cross-boundary semantic identity contract required to make
      FND-02 / FND-03 / FND-04 unambiguous

not

FND-ID-01
    = exhaustive inventory of every durable/runtime/analytics/gameplay ID
      that the complete game may ever contain
```

The canonical architecture result is stored in `docs/architecture/FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md`.

## Validation

### Focused

- command/run: pending documentation/governance validation
- result: pending

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture documentation only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: to be recorded in immutable PR/check evidence after the content head is frozen
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending
- method/auditor: separate architecture consistency review against accepted ADRs and owner baselines
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none expected
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Owner-accepted minimum FND-ID-01 boundary is recorded in the canonical architecture baseline and draft PR #80 is open with only the two declared documentation paths.
status: validating
branch: docs/OTV2-20260807-fnd-id01-minimum-scope
head_sha: null
pr: 80
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
next_action: Review the exact final two-file diff, perform the independent architecture audit, and inspect exact-head documentation/governance checks without moving the head unless a material defect is found.
```
