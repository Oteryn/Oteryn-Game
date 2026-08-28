# OTV2-20260826-reconnect-attempt-registry

```yaml
task_id: OTV2-20260826-reconnect-attempt-registry
title: Historical completed FND-04 reconnect-attempt retention hard bound
mode: CONTRACT
status: COMPLETED_ARCHIVED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
historical_branch: docs/reconnect-attempt-bound-193
issue: 193
pr: 195
allocation_pr: 195
allocation_merge_sha: 9878d42a21815027ef88067bfc59f8b40e78b473
base_sha: 1063caf409af6cd4b25fa844e17a483b87e76ad6
head_sha: null
final_head_sha: null
delivery_merge_sha: 9878d42a21815027ef88067bfc59f8b40e78b473
owner: ChatGPT registry worker for Issue #193
created_at: 2026-08-26T15:02:00+02:00
updated_at: 2026-08-28T14:05:00Z
archived_at: 2026-08-28
execution_budget_minutes: 60
large_budget_reason: null
owned_paths: []
released_paths:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20260826-reconnect-attempt-registry.md
public_contracts:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - DUR-RECONNECT-AUTHORITY-V1
depends_on:
  - issue:187 resolved by pr:190 / main:2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
blocks: []
write_authority: none
serialized_lease: none
external_repositories: []
```

## Coordinated terminal archive

PR #195 merged the completed registry delivery as protected `main@9878d42a21815027ef88067bfc59f8b40e78b473`. This record is immutable historical evidence, owns no path, and grants no dispatch, validation, registry-write, or blocker authority.

## Outcome

Append exactly one accepted registry row: `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` distinct retained attempts per open ControlLossEpoch per GameSession. Same-attempt retry consumes no extra slot; attempt 9 fails before persistent allocation or authority mutation.

## Acceptance criteria

- [x] RED proves the exact ID is absent on the allocation merge base.
- [x] Exactly one new registry object is added; every existing object is byte/semantic-equivalent except unavoidable trailing delimiter context.
- [x] `hard_maximum` is 8 and is not derived from NET03/DUR03 resources.
- [x] Max, max+1, same-ref retry and overflow obligations are recorded.
- [x] Registry JSON uniqueness/required-field/round-trip checks pass.
- Historical only: the registry delivery is terminal and merged; no validation or merge action remains on this archived record.

## Excluded scope

No runtime/Foundation/Durability implementation, Cargo/workflow, other resource value, wire/error numeric ID, production tuning or external-repository mutation.

## Validation

- RED on `main@1063caf409af6cd4b25fa844e17a483b87e76ad6`: target ID absent, expected failure observed.
- GREEN: exact ID count 1, hard maximum 8, fixed range 8..8, required fields/uniqueness/JSON round-trip PASS.
- `python tools/agents/validate_governance.py`: PASS.
- `cargo +1.94.0 run -q -p oteryn-architecture-check -- workspace .`: PASS.
- repository-policy validator retains the pre-existing canonical LICENSE mismatch; `git diff origin/main -- LICENSE` is empty.
- `git diff --check`: PASS.
- independent review: NOT_REQUIRED for exact transcription of the already accepted/reviewed PR #190 value into the serialized registry.

## Context checkpoint

```yaml
last_progress: delivery PR #195 is merged as protected main 9878d42a21815027ef88067bfc59f8b40e78b473; ownership is released
status: COMPLETED_ARCHIVED
branch: null
historical_branch: docs/reconnect-attempt-bound-193
head_sha: null
pr: 195
final_head_sha: null
delivery_merge_sha: 9878d42a21815027ef88067bfc59f8b40e78b473
owned_paths: []
owner_action_required: none — terminal delivery archived and paths released
blocker: none
next_action: retain this record as historical evidence only
```
