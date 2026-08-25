# OTV2-20260825-next-wave-registry-allocation

```yaml
task_id: OTV2-20260825-next-wave-registry-allocation
title: Allocate accepted next-wave registry mutation
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/allocate-next-wave-registry-142
issue: 142
pr: null
base_sha: 88ad620169d6d08ebad6e49886ba1098da728480
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT coordinator for Issue #131
created_at: 2026-08-25T10:12:00+02:00
updated_at: 2026-08-25T10:12:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260825-close-next-wave-blockers.md
  - docs/agents/tasks/active/OTV2-20260825-next-wave-registry-allocation.md
  - docs/agents/tasks/active/OTV2-20260825-next-wave-registry.md
public_contracts:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
```

depends_on:
  - issue:131
  - issue:142
  - pr:140
blocks:
  - task:OTV2-20260825-next-wave-registry
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Merge one docs-only coordinator allocation that grants exactly one subsequent worker authority to mutate `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` and its own task record, copying the 24 accepted values from PR #140 without reinterpretation.

## Proven state

- Issue #133 is terminal and PR #140 merged as `88ad620169d6d08ebad6e49886ba1098da728480`.
- The canonical registry does not yet contain the 24 accepted candidate IDs.
- The shared runtime/Cargo lease remains released; this allocation does not acquire it.
- Registry authority is `none` until this allocation PR merges.
- Parent Issue #131 remains active.

## Acceptance criteria

- [x] Parent #131 remains active.
- [x] Worker branch is fixed but not created before allocation merge.
- [x] Worker authority after merge is limited to `RESOURCE_LIMITS_REGISTRY.json` and its task record.
- [x] No runtime/Cargo/workspace/production/Platform/external authority is granted.
- [ ] Governance, diff review and exact-head `game-gate` pass before merge.

## Validation

- focused: `python tools/agents/validate_governance.py` — `PASS`
- component: `git diff --check` — `PASS`
- E2E: `NOT_APPLICABLE` — docs-only allocation
- independent review: `NOT_REQUIRED` — allocation narrows authority and changes no executable semantics

## Context checkpoint

```yaml
last_progress: Issue #142 allocation is exact-base; governance/diff pass and complete allocation diff is ready for PR.
status: implementing
branch: coord/allocate-next-wave-registry-142
head_sha: null
pr: null
blocker: null
owner_action_required: null
next_action: Commit/push the allocation, open its PR, require exact-head CI and merge before creating the registry worker branch.
```