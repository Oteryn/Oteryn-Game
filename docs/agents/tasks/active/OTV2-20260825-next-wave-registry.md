# OTV2-20260825-next-wave-registry

```yaml
task_id: OTV2-20260825-next-wave-registry
title: Register accepted next-wave first-slice limits
mode: CONTRACT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/next-wave-registry-142
issue: 142
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT registry worker for Issue #142
created_at: 2026-08-25T10:12:00+02:00
updated_at: 2026-08-25T10:12:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260825-next-wave-registry.md
public_contracts:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
```

depends_on:
  - issue:142
  - pr:140
  - main:88ad620169d6d08ebad6e49886ba1098da728480
blocks:
  - issue:93
  - issue:116
  - issue:123
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Copy exactly the 24 accepted candidate records from the merged Issue #133 evidence into the canonical resource registry. Do not duplicate the inherited `FND02-WIRE-FRAME-BYTES` entry and do not register any fail-closed-excluded row.

## Acceptance criteria

- [ ] RED assertion proves all 24 accepted IDs are absent before mutation.
- [ ] GREEN assertion proves exactly those IDs/values and required fields after mutation.
- [ ] JSON uniqueness and round-trip validation pass.
- [ ] No production default, wire/error numeric identifier, excluded-row value, or implementation change appears.
- [ ] Governance, architecture checks, whole-diff review and exact-head `game-gate` pass.

## Excluded scope

No product/runtime/Cargo/workspace change; no Ability/Interaction/AI/Movement/Durability/listener implementation; no production tuning, secrets, deployment, Platform or external-repository write.

## Validation

- focused registry assertion: pending RED then GREEN
- governance / architecture / diff: pending
- E2E: `NOT_APPLICABLE`
- independent review: `NOT_REQUIRED` unless the registry diff introduces semantics beyond exact #140 copy

## Context checkpoint

```yaml
last_progress: Worker task prepared but has no write authority until the allocation PR merges.
status: waiting
branch: docs/next-wave-registry-142
head_sha: null
pr: null
blocker: allocation_not_merged
owner_action_required: null
next_action: After allocation merge, create the worker branch from the exact merge SHA and run the RED registry assertion before mutation.
```