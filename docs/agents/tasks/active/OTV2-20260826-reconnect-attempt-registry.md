# OTV2-20260826-reconnect-attempt-registry

```yaml
task_id: OTV2-20260826-reconnect-attempt-registry
title: Register FND-04 reconnect-attempt retention hard bound
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/reconnect-attempt-bound-193
issue: 193
pr: null
allocation_pr: 194
allocation_merge_sha: 1063caf409af6cd4b25fa844e17a483b87e76ad6
base_sha: 1063caf409af6cd4b25fa844e17a483b87e76ad6
head_sha: null
final_head_sha: null
owner: ChatGPT registry worker for Issue #193
created_at: 2026-08-26T15:02:00+02:00
updated_at: 2026-08-26T15:29:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260826-reconnect-attempt-registry.md
public_contracts:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - DUR-RECONNECT-AUTHORITY-V1
depends_on:
  - issue:187 resolved by pr:190 / main:2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
blocks:
  - issue:192 final acceptance
  - issue:167 downstream resume
write_authority: exact_allocated_registry_and_task_paths
serialized_lease: docs/contracts/RESOURCE_LIMITS_REGISTRY.json only
external_repositories: []
```

## Outcome

Append exactly one accepted registry row: `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` distinct retained attempts per open ControlLossEpoch per GameSession. Same-attempt retry consumes no extra slot; attempt 9 fails before persistent allocation or authority mutation.

## Acceptance criteria

- [x] RED proves the exact ID is absent on the allocation merge base.
- [x] Exactly one new registry object is added; every existing object is byte/semantic-equivalent except unavoidable trailing delimiter context.
- [x] `hard_maximum` is 8 and is not derived from NET03/DUR03 resources.
- [x] Max, max+1, same-ref retry and overflow obligations are recorded.
- [x] Registry JSON uniqueness/required-field/round-trip checks pass.
- [ ] Governance, repository policy, architecture check, diff review and exact-head CI pass before expected-head merge.

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
last_progress: one-row registry mutation is GREEN; governance, architecture check and diff checks pass; repository-policy LICENSE mismatch is baseline-only
status: validating
branch: docs/reconnect-attempt-bound-193
head_sha: null
pr: null
final_head_sha: null
owner_action_required: null
blocker: null
next_action: freeze the delivery head, open the exact-head PR and require protected CI before merge
```
