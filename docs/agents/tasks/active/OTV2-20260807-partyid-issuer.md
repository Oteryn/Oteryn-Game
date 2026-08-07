# OTV2-20260807-partyid-issuer

```yaml
task_id: OTV2-20260807-partyid-issuer
title: Record PartyId issuer and world scope
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-partyid-issuer
pr: 67
base_sha: 6804f5d67b63f1374a9efa3710bcaad10805c801
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T11:21:00+02:00
updated_at: 2026-08-07T11:27:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-partyid-issuer.md
  - docs/architecture/FND-ID-01_PARTY_ID_ISSUER_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/FND-ID-01_PARTY_ID_ISSUER_OWNER_BASELINE.md
depends_on:
  - FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md
  - accepted cross-channel party semantics
blocks:
  - complete FND-ID-01 identifier catalogue until PartyId issuer/lifecycle is consumed
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Record the owner-accepted decision that `PartyId` is a strongly typed full-128-bit UUIDv7 issued by the authoritative world-level game-domain Party/Social authority, while canonical party identity remains `WorldId + PartyId` and one party may contain members currently placed on different channels of the same world.

## Architecture and source of truth

- `PROVEN`: the accepted identity baseline already defines semantic party identity as `WorldId + PartyId`.
- `PROVEN`: accepted social/instance architecture permits a party to span channels inside one logical world.
- `PROVEN`: the UUIDv7 baseline selects UUIDv7 for durable cross-boundary party identity.
- `OWNER_ACCEPTED`: on 2026-08-07 the project owner accepted world-level game-domain Party/Social authority as the `PartyId` issuer; Platform is not the issuer.

## Acceptance criteria

- [x] Define `PartyId` as strongly typed UUIDv7 preserving 128 bits.
- [x] Preserve semantic identity as `WorldId + PartyId`.
- [x] Assign issuer authority to world-level game-domain Party/Social authority.
- [x] State that Platform Identity, Game Gateway and World Registry do not mint canonical PartyId.
- [x] Preserve a party across member channel placement differences within one world.
- [x] Keep leader, current ChannelId, GameNode and invite tokens separate from party identity.
- [x] Keep PartyId as identity rather than membership/authorization proof.
- [x] Defer exact service/process placement, membership revision model, wire encoding and persistence layout to later gates.
- [x] Do not implement runtime/protocol/persistence behavior.
- [ ] Exact-head checks and independent audit before merge readiness.

## Excluded scope

No Rust runtime, Party service implementation, invite/membership protocol, database DDL, Platform changes, production activation or completion of the full `FND-ID-01` catalogue.

## Implementation / findings

PR #67 owns only the two declared documentation paths. PRs #63 through #66 are separate architecture packages with no overlapping paths.

The decision fixes world-level game-domain issuer authority and cross-channel identity semantics without prematurely fixing Party/Social service deployment or membership mechanics.

## Validation

### Focused
- candidate head `d4776f28aa5b671ac55265ed5d2b37f29fd0fc19`: Dependency Review succeeded.
- candidate head `d4776f28aa5b671ac55265ed5d2b37f29fd0fc19`: Agent governance run `31165692345` failed in `Verify pull request target and metadata` because the initial PR body omitted required `## Validation`.
- correction: PR #67 metadata was fixed without changing architecture content; this task checkpoint records that real validation event and causes a new exact-head generation rather than an empty retrigger commit.
- CodeQL for the prior candidate was still in progress when this checkpoint was written.

### Component/integration
- command/run: `NOT_APPLICABLE` — architecture documentation only
- result: `NOT_APPLICABLE`

### E2E
- scenario: `NOT_APPLICABLE` — no executable runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI
- final head: pending after this evidence commit
- trigger source: pull_request synchronize
- workflow/run/job: pending new generation
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending immutable PR/check evidence
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: prior candidate reviewed; repeat against new exact head
- unresolved review threads: pending
- related PRs: #63 through #66 are separate non-overlapping architecture packages
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: PartyId baseline is recorded; initial governance metadata defect was diagnosed and corrected, and a real evidence checkpoint created the next exact-head CI generation.
status: validating
branch: docs/OTV2-20260807-partyid-issuer
head_sha: null
pr: 67
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: 2
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Validate the new exact head, keep PR #67 draft until required checks and independent audit are satisfied, and do not change architecture content unless a material defect is found.
```
