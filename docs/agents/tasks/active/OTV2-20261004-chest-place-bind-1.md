# OTV2-20261004-chest-place-bind-1

```yaml
task_id: OTV2-20261004-chest-place-bind-1
title: "CHEST-PLACE-BIND-1: bind RewardClaim placements to world bundle entries"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: OTV2-20261004-chest-place-bind-1
issue: 1622
pr: null
owner: claude-code worker for control plane session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/world_reward_claims.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/map/mod.rs
  - apps/game-server/tests/world_reward_claims_*.rs
  - docs/agents/tasks/archive/OTV2-20261004-chest-place-bind-1.md
public_contracts: []
depends_on: [MAP-LOAD-1]
blocks: [WORLD-CONTENT-SERVE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Scope

ARCH-WORLD-CONTENT-SERVE-1 §1.4-§1.6 and §2.3 on `main` (839432f8): the sparse top-level
`unique` table of `WorldBase` with `TileView::unique`, and the pure binder in
`content/world_reward_claims.rs`. Not called from the node.
