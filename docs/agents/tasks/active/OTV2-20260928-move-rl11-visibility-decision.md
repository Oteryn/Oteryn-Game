# OTV2-20260928-move-rl11-visibility-decision

```yaml
task_id: OTV2-20260928-move-rl11-visibility-decision
title: "MOVE-RL-11 visibility decision (D84-D87)"
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: null
base_sha: 8e2e474a3d82a6bfbc21be409f444cfd64d42961
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-move-rl11-visibility-decision.md
  - docs/agents/tasks/active/OTV2-20260928-b3-inventory-destination-decision.md   # archive move after #1137
  - docs/agents/tasks/archive/OTV2-20260928-b3-inventory-destination-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the durable text for the `MOVE-RL-11` visibility re-decision: owner decisions
D84-D87 (#162 comment 5875312123). It defines the Global 18 × 14 interest area with a configurable
test range, the Global floor rule, the visible entity kinds, a 256-entity ceiling with a
deterministic nearest-first order, the companion rows `MOVE-RL-08`, `-09`, `-10`, and routes the
protocol schema revision to the protocol lane.

No registry, protocol-registry, runtime or migration change.

## Architecture and source of truth

- `PROVEN`: the registry `MOVE-RL-*` rows; the wave-2 limits packet; VSL-MOVE-01 §13, §15, §17;
  `world_spatial_v1.proto` and the protocol registry; FND-02 rows; D57 and D78 scope counts.
- `DERIVED`: the Global 15 × 11 view, 18 × 14 area and floor rule (OTS protocol evidence).
- `UNKNOWN`: players per Channel; dense-scene cost.

## High-risk authority/recovery qualification

Not applicable. The decision selects projection bounds; visibility derives only from committed
server state and changes no authority, persistence or identity. VIS-1 and VIS-2 carry their own
qualification when allocated.

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Registry rows, protocol schema, runtime code, line of sight, invisibility, spectators.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored
status: implementing
branch: claude/gifted-rubin-a0axzx
pr: null
owner_action_required: null
blocker: null
next_action: open the PR, bind this record to it, freeze and route one external review
```
