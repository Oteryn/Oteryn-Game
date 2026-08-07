# OTV2-20260807-nodeid-incarnation

```yaml
task_id: OTV2-20260807-nodeid-incarnation
title: Record NodeId process-incarnation semantics
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-nodeid-incarnation
pr: 65
base_sha: 6804f5d67b63f1374a9efa3710bcaad10805c801
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T10:56:00+02:00
updated_at: 2026-08-07T11:00:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-nodeid-incarnation.md
  - docs/architecture/FND-ID-01_NODE_ID_PROCESS_INCARNATION_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/FND-ID-01_NODE_ID_PROCESS_INCARNATION_OWNER_BASELINE.md
depends_on:
  - ADR-0001 native Rust multichannel architecture
  - ADR-0009 GameNode execution/capacity/recovery baseline
  - UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md
blocks:
  - complete FND-ID-01 identifier catalogue until NodeId lifecycle is consumed
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Record the owner-accepted decision that canonical `NodeId` identifies one concrete GameNode process incarnation, uses strongly typed UUIDv7, and changes on every process restart. Stable host, VM, pod/container, orchestrator-node or logical deployment-slot identity is separate and must never be overloaded into `NodeId`.

## Architecture and source of truth

- `PROVEN`: ADR-0001 defines `NodeId` as the identity of one GameNode process runtime rather than physical infrastructure.
- `PROVEN`: ADR-0009 separates host, container, process, GameNode and ChannelRuntime and allows a channel to move between nodes without changing channel identity.
- `OWNER_ACCEPTED`: on 2026-08-07 the repository owner accepted `NodeId = strongly typed UUIDv7 process incarnation`, with a fresh `NodeId` on every process restart/replacement.
- `CONSTRAINT`: exact UUID generator/registration handshake and any stable placement-ID type remain deliberately unresolved.

Canonical project terminology remains `NodeId`; historical/candidate `GameNodeId` wording must not create a second identifier for the same concept.

## Acceptance criteria

- [x] `NodeId` identifies one GameNode process incarnation.
- [x] `NodeId` uses strongly typed UUIDv7 preserving 128 bits.
- [x] Every start/restart/replacement creates a new `NodeId`.
- [x] Infrastructure and stable placement identity remain separate concepts.
- [x] `NodeId` is not part of semantic `WorldId + ChannelId` identity.
- [x] `NodeId` grants no mutation authority without current assignment/fencing.
- [x] Exact generator/registration handshake is not prematurely frozen.
- [x] No runtime/orchestrator/protocol/persistence implementation is included.
- [ ] Exact-head documentation/governance checks and independent audit before merge readiness.

## Excluded scope

No Rust runtime, orchestrator integration, heartbeat/registration API, persistence, protocol encoding, stable placement-ID final contract, production deployment or completion of the full `FND-ID-01` catalogue.

## Implementation / findings

The dedicated owner baseline records the accepted distinction between process incarnation and durable topology identity. A replacement GameNode gets a new `NodeId`, while a recovered channel may retain the same `WorldId + ChannelId` under a newer ownership generation/fence.

PR #65 owns only the two declared documentation paths. PRs #63 and #64 are separate architecture packages and do not overlap these paths.

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
- related/superseded PRs: #63 and #64 are separate and non-overlapping
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Owner NodeId decision is recorded in the dedicated baseline and PR #65 is open with only two declared documentation paths.
status: validating
branch: docs/OTV2-20260807-nodeid-incarnation
head_sha: null
pr: 65
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
next_action: Review the exact two-file diff and exact-head CI for PR #65 without moving the head unless a material defect is found.
```
