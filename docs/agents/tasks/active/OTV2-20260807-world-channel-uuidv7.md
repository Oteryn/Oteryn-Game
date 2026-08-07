# OTV2-20260807-world-channel-uuidv7

```yaml
task_id: OTV2-20260807-world-channel-uuidv7
title: Record WorldId and ChannelId UUIDv7 representation
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-world-channel-uuidv7
pr: 64
base_sha: 6804f5d67b63f1374a9efa3710bcaad10805c801
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T10:49:00+02:00
updated_at: 2026-08-07T10:53:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-world-channel-uuidv7.md
  - docs/architecture/FND-ID-01_WORLD_CHANNEL_UUIDV7_REPRESENTATION_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/FND-ID-01_WORLD_CHANNEL_UUIDV7_REPRESENTATION_OWNER_BASELINE.md
depends_on:
  - FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md
  - FND-ID-01_WORLD_CHANNEL_ID_ISSUANCE_OWNER_BASELINE.md
  - ADR-0003 Platform World Registry authority
blocks:
  - complete FND-ID-01 contract until this owner-accepted representation is consumed
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Record the owner-accepted decision that the canonical target representation of `WorldId` and `ChannelId` is strongly typed UUIDv7 preserving all 128 bits, while canonical channel identity remains `WorldId + ChannelId` and Platform World Registry remains the issuer.

## Architecture and source of truth

- `PROVEN`: the durable-identity baseline already selects UUIDv7 for Oteryn-owned durable cross-boundary identity and previously left `WorldId`/`ChannelId` conditional on their authoritative cross-repository contract.
- `PROVEN`: the owner-accepted issuance baseline assigns canonical `WorldId` and `ChannelId` issuance to Platform World Registry / topology authority.
- `OWNER_ACCEPTED`: on 2026-08-07 the repository owner accepted UUIDv7 as the canonical target representation for both `WorldId` and `ChannelId`.
- `CONSTRAINT`: this Oteryn-v2 decision does not claim current Platform implementation conformance and cannot silently re-key externally owned identifiers; coordinated Platform adoption/migration remains required.

## Acceptance criteria

- [x] Record `WorldId = strongly typed UUIDv7`.
- [x] Record `ChannelId = strongly typed UUIDv7`.
- [x] Preserve all 128 bits across canonical boundaries.
- [x] Preserve semantic channel identity as `WorldId + ChannelId`.
- [x] Preserve Platform World Registry issuance authority.
- [x] Keep names/slugs/channel ordinals as labels rather than canonical identity.
- [x] Preserve strong type separation and reject UUID timestamp ordering as authority/causality.
- [x] Require explicit coordinated Platform adoption/migration and prohibit silent re-keying.
- [x] Leave exact wire byte order/text form to `FND-02` and physical persistence layout to `DUR-01`/`DUR-02`.
- [x] Do not implement runtime/protocol/persistence/Platform changes.
- [ ] Review complete diff and exact-head checks before merge readiness.

## Excluded scope

- no writes to `blakinio/Oteryn-Platform`;
- no Platform schema/data migration;
- no Rust implementation;
- no protocol IDL/framing/serialization choice;
- no database DDL/index choice;
- no production activation;
- no completion of the full `FND-ID-01` catalogue.

## Implementation / findings

A dedicated owner baseline records the representation decision without rewriting earlier evidence. The later decision resolves only the previous conditional representation wording for `WorldId` and `ChannelId`; all prior issuer, scope, fencing and cross-repository authority rules remain in force.

PR #64 contains only the two declared documentation paths and intentionally does not edit `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`, avoiding overlap with the still-open PR #63.

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

- final head: recorded in immutable PR/check evidence after final content commit
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
- related/superseded PRs: PR #63 is separate `FND-02` reconciliation scope and owns no files in this task
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Owner decision is recorded in a dedicated baseline and PR #64 is open with only the two declared paths.
status: validating
branch: docs/OTV2-20260807-world-channel-uuidv7
head_sha: null
pr: 64
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
next_action: Review the exact two-file diff and exact-head CI for PR #64 without moving the head unless a material defect is found.
```
