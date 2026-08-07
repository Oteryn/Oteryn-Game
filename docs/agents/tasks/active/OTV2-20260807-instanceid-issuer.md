# OTV2-20260807-instanceid-issuer

```yaml
task_id: OTV2-20260807-instanceid-issuer
title: Record InstanceId issuer and scope
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-instanceid-issuer
pr: 66
base_sha: 6804f5d67b63f1374a9efa3710bcaad10805c801
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T11:08:00+02:00
updated_at: 2026-08-07T11:12:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-instanceid-issuer.md
  - docs/architecture/FND-ID-01_INSTANCE_ID_ISSUER_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/FND-ID-01_INSTANCE_ID_ISSUER_OWNER_BASELINE.md
depends_on:
  - FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md
  - INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md
blocks:
  - complete FND-ID-01 identifier catalogue until InstanceId issuer is consumed
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Record the owner-accepted decision that `InstanceId` is a strongly typed full-128-bit UUIDv7 issued by the authoritative game-domain Instance/Activity allocator, while canonical instance identity remains `WorldId + InstanceId`.

## Architecture and source of truth

- `PROVEN`: the accepted instance baseline defines canonical instance identity as `WorldId + InstanceId`, separates concrete InstanceRuntime from activity/template and permits cross-channel same-world participation.
- `PROVEN`: the UUIDv7 baseline selects UUIDv7 for durable cross-boundary instance identity.
- `OWNER_ACCEPTED`: on 2026-08-07 the project owner accepted the authoritative game-domain Instance/Activity allocator as `InstanceId` issuer; Platform is not the issuer.

## Acceptance criteria

- [x] Define `InstanceId` as strongly typed UUIDv7 preserving 128 bits.
- [x] Preserve semantic identity as `WorldId + InstanceId`.
- [x] Assign issuer authority to the game-domain Instance/Activity allocator.
- [x] State that Platform Identity/Gateway/World Registry do not mint canonical InstanceId.
- [x] Keep activity/template, origin ChannelId and GameNode placement separate from concrete instance identity.
- [x] Preserve cross-channel same-world shared-instance semantics.
- [x] Keep identity separate from admission/ownership/fencing authority.
- [x] Defer allocator deployment/API, exact wire encoding and persistence layout to later gates.
- [x] Do not implement runtime/protocol/persistence behavior.
- [ ] Exact-head checks and independent audit before merge readiness.

## Excluded scope

No Rust runtime, allocator implementation, admission flow, protocol schema, database DDL, Platform changes, production activation or completion of the full `FND-ID-01` catalogue.

## Implementation / findings

PR #66 owns only the two declared documentation paths. PRs #63, #64 and #65 are separate architecture packages with no overlapping paths.

The logical issuer is fixed, while exact allocator process/service placement remains deliberately unresolved for `FND-03` and operations design.

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
- related PRs: #63, #64 and #65 are separate non-overlapping architecture packages
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Owner InstanceId decision is recorded and PR #66 is open with only two declared documentation paths.
status: validating
branch: docs/OTV2-20260807-instanceid-issuer
head_sha: null
pr: 66
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
next_action: Review the exact two-file diff and exact-head CI for PR #66 without moving the head unless a material defect is found.
```
