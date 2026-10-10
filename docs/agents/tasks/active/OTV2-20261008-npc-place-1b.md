# OTV2-20261008-npc-place-1b

```yaml
task_id: OTV2-20261008-npc-place-1b
title: "NPC-PLACE-1b: World Bundle v4 NPC frame, placement realization and travel destination holds"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-place-1b-20261008
issue: 1622
pr: PENDING
head_sha: "exact frozen head in the PR FREEZE comment"
final_head_sha: "exact frozen head in the PR FREEZE comment"
owner: single writer session (control plane session_01CwP6d84eCPvpgoEuyci8Tx)
created_at: 2026-10-08
updated_at: 2026-10-10
execution_policy: continuous_progress
packet: "docs/architecture/reviews/OTERYN_GAME_NPC_PLACE1_NPC_PLACEMENTS_DECISION_2026-10-06.md §3, §5, §6, §7"
owned_paths:
  - crates/world-bundle/
  - tools/world-bundle-compiler/
  - tools/repository/classify_pr_test_lanes.py
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - apps/game-server/tests/npc_placement_parity.rs
  - apps/game-server/src/map/boot.rs
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - apps/game-server/src/movement/speed.rs
  - content/world/pins/oteryn.json
  - content/world/pins/README.md
  - docs/agents/tasks/active/OTV2-20261008-npc-place-1b.md
  - docs/agents/tasks/archive/OTV2-20261008-npc-place-1b.md
public_contracts: [OTERYN_WORLD_BUNDLE_FORMAT_V1]
depends_on: [NPC-PLACE-1, NPC-PLACE-1a]
```

## Scope notes

- Control plane decision (a): `map/boot.rs` and `gameplay_transport/world_map_tests.rs` for the
  cfg(test) `npcs: Default::default()` manifest line only; the pin files for a re-pin only.
- Control plane Q1 (a): `movement/speed.rs`, the same single cfg(test) line only.
- Control plane Q2 (a): `apps/game-server/tests/npc_placement_parity.rs`, a new file on existing
  pub APIs only.
