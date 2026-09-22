> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #666 merged as `c0ab26554669e696ead11526f9977ccfa05a83ec`, and the canonical task branch is deleted. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260918-fnd02-terminal-outcome-retention-resource-registration-663

```yaml
task_id: OTV2-20260918-fnd02-terminal-outcome-retention-resource-registration-663
title: FND-02 terminal-outcome retention resource registration
mode: CONTRACT
status: ready
issue: 663
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/fnd02-terminal-outcome-retention-resource-registration-663
pr: null
base_sha: 3d3e31c288e4a91199e8863a24fd41ed9aea727c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn impl foundation
created_at: 2026-09-18
updated_at: 2026-09-18
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260918-fnd02-terminal-outcome-retention-resource-registration-663.md
public_contracts:
  - FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
depends_on:
  - "#663 comment 5735153528"
  - "#162 resource-registration allocation / 2026-09-18"
blocks:
  - CONTENT_WORLD_CW4_LOCAL_OBJECT_RUNTIME_COMPONENT_504
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Register exactly the two architect-frozen FND-02 terminal-retention resource rows and no runtime behavior:

- `FND02-RETAINED-TERMINAL-RECORDS = 1 record/GameSession`;
- `FND02-RETAINED-TERMINAL-CHARGED-BYTES = 3116 charged bytes/GameSession`.

This child grants no Foundation runtime, protocol, CW4, persistence or production-deployment authority.

## Architecture and source of truth

FACT

- Protected admission source is `main@3d3e31c288e4a91199e8863a24fd41ed9aea727c`.
- #663 comment `5735153528` is the controlling architecture authority.
- Protected #665 evidence establishes one retained terminal semantic record per GameSession and an aggregate charged-byte bound of 3116.
- Retained binding identity remains exact active Content-generation identity plus stable `TransitionKey`.
- No wall-clock TTL is selected.

CONSTRAINT

- The two registry IDs, units, hard maxima, fixed configurable ranges and semantics are copied without widening or reinterpretation.
- Existing `FND02-OUTSTANDING-COMMANDS`, CommandResult payload limits, NET03 limits and fixture bounds remain unrelated and unchanged.

## Preserved invariants

- Pending commands are not terminal-retention entries and cannot be evicted to satisfy terminal-retention capacity.
- Terminal eviction never lowers `next_command_id` and never makes a `CommandRef` reusable.
- A second simultaneously retained terminal record is not admitted unless deterministic eligible-terminal eviction first restores the count bound.
- Charged-byte admission uses checked arithmetic and rejects overflow or over-bound retention before gameplay mutation requiring replayability.
- Expired duplicates remain reconciliation / `COMMAND_OUTCOME_EXPIRED`, never fresh work.
- No full `TransitionBinding` or `policy_guard_refs` retention is authorized.

## Excluded scope

No writes to `apps/game-server/**`, other contracts, architecture decisions, protocol/schema/stable-ID registries,
CW3/CW4, Durability/persistence, Cargo/workspace/lock, workflows/governance, external repositories,
production environments or protected branches.

## Acceptance criteria

- [x] Exactly two new registry rows are added.
- [x] Count row hard maximum and fixed range are exactly 1.
- [x] Charged-byte row hard maximum and fixed range are exactly 3116.
- [x] Row semantics match #663 comment `5735153528`.
- [x] Existing registry rows are otherwise unchanged.
- [x] Changed paths remain inside the two-path custody.
- [x] No runtime implementation is added.

## Validation

- Registry JSON parse: PASS.
- Exact architect-row semantic comparison: PASS against #663 comment `5735153528`.
- `python tools/agents/validate_governance.py`: PASS — 26 required policy documents and 9 project lanes validated.
- `git diff --check`: PASS.
- Exact changed-path readback: PASS — exactly the two allocated custody paths.
- Whole-diff adversarial self-review: PASS — zero material findings; no runtime or unrelated registry drift.

## PR and closeout

- protected merge/direct merge: NOT_AUTHORIZED_BY_WORKER.
- integration route: coordinator-owned governed exact-head Merge Queue lifecycle.
- next serialized step after protected registry readback: bounded Foundation runtime child from #663 comment `5735153528`.
- CW4 resume remains forbidden until that later Foundation runtime child is protected-integrated and read back.

## Context checkpoint

```yaml
last_progress: two architect-frozen FND-02 terminal-retention resource rows registered on canonical branch
status: ready
branch: agent/fnd02-terminal-outcome-retention-resource-registration-663
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
owner_action_required: null
blocker: null
next_action: validate exact diff, commit, normal push, open PR, then hand off READY_FOR_INTEGRATION
```
