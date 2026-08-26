# OTV2-20260826-reconnect-attempt-registry

```yaml
task_id: OTV2-20260826-reconnect-attempt-registry
title: Register FND-04 reconnect-attempt retention hard bound
mode: CONTRACT
status: waiting_allocation_merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/reconnect-attempt-bound-193
issue: 193
pr: null
allocation_pr: pending
base_sha: null
head_sha: null
final_head_sha: null
owner: Oteryn: work coordinator
created_at: 2026-08-26T15:02:00+02:00
updated_at: 2026-08-26T15:02:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260826-reconnect-attempt-registry.md
public_contracts:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - DUR-RECONNECT-AUTHORITY-V1
depends_on:
  - issue:187 resolved by pr:190 / main:2394f6f4633b8c6662d8d79a84110cc2ae13dcb7blocks:
  - issue:192 final acceptance
  - issue:167 downstream resume
write_authority: none_until_allocation_pr_merges
serialized_lease: docs/contracts/RESOURCE_LIMITS_REGISTRY.json only
external_repositories: []
```

## Outcome

Append exactly one accepted registry row: `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` distinct retained attempts per open ControlLossEpoch per GameSession. Same-attempt retry consumes no extra slot; attempt 9 fails before persistent allocation or authority mutation.

## Acceptance criteria

- [ ] RED proves the exact ID is absent on the allocation merge base.
- [ ] Exactly one new registry object is added; every existing object is byte/semantic-equivalent except unavoidable trailing delimiter context.
- [ ] `hard_maximum` is 8 and is not derived from NET03/DUR03 resources.
- [ ] Max, max+1, same-ref retry and overflow obligations are recorded.
- [ ] Registry JSON uniqueness/required-field/round-trip checks pass.
- [ ] Governance, repository policy, architecture check, diff review and exact-head CI pass before expected-head merge.

## Excluded scope

No runtime/Foundation/Durability implementation, Cargo/workflow, other resource value, wire/error numeric ID, production tuning or external-repository mutation.

## Context checkpoint

```yaml
last_progress: PR #190 accepted the exact hard maximum; serialized registry delivery awaits allocation merge
status: waiting_allocation_mergebranch: docs/reconnect-attempt-bound-193
head_sha: null
pr: null
final_head_sha: null
owner_action_required: null
blocker: allocation PR must merge before registry mutation
next_action: after allocation merge, create the named branch from that exact protected-main SHA and append the single accepted registry row
```
