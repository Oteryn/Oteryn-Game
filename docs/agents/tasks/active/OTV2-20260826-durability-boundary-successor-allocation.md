# OTV2-20260826-durability-boundary-successor-allocation

```yaml
task_id: OTV2-20260826-durability-boundary-successor-allocation
title: Allocate Durability architecture successors
mode: COORDINATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/allocate-durability-boundary-successors
issue: 162
architecture_issue: 187
foundation_issue: 192
registry_issue: 193
pr: null
base_sha: 2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
head_sha: null
final_head_sha: null
owner: Oteryn: work coordinator
created_at: 2026-08-26T15:02:00+02:00
updated_at: 2026-08-26T15:02:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
  - docs/agents/tasks/active/OTV2-20260825-impl-durability.md
  - docs/agents/tasks/active/OTV2-20260826-durability-boundary-successor-allocation.md
  - docs/agents/tasks/active/OTV2-20260826-impl-foundation-reconnect-durability.md
  - docs/agents/tasks/active/OTV2-20260826-reconnect-attempt-registry.mdpublic_contracts:
  - DUR-RECONNECT-AUTHORITY-V1
depends_on:
  - pr:190 / main:2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
blocks:
  - issue:192 first write
  - issue:193 registry mutation
  - issue:167 dependency reconciliation
write_authority: coordinator_docs_only_until_this_allocation_merges
external_repositories: []
```

## Outcome

Convert resolved architecture Issue #187 into two non-overlapping executable successors: Foundation #192 and serialized registry #193. Preserve the existing Durability branch/task but change it to `WAITING_DEPENDENCY`; it receives no Foundation or registry authority.

## Acceptance criteria

- [ ] Protected main and PR #190 merge are read back.
- [ ] #192 receives only the five exact Foundation paths plus its own task packet after this PR merges.
- [ ] #193 receives only `RESOURCE_LIMITS_REGISTRY.json` plus its own task packet after this PR merges.
- [ ] #167 remains no-write and depends on merged #192 before a fresh resume allocation.
- [ ] No path overlap exists between #192, #193 and current #167 worker-owned paths.
- [ ] Governance and `git diff --check` pass; runtime/E2E are `NOT_APPLICABLE` for allocation-only docs.

## Independent review

`NOT_REQUIRED` for this allocation-only authority narrowing; #192 remains XHigh and requires genuine exact-head independent security review.
## Context checkpoint

```yaml
last_progress: PR #190 merged and successor Issues #192/#193 exist; exact docs-only allocation is being validated
status: implementing
branch: coord/allocate-durability-boundary-successors
head_sha: null
pr: null
final_head_sha: null
owner_action_required: null
blocker: null
next_action: validate and merge this exact allocation, then release write authority to #192/#193 only
```
