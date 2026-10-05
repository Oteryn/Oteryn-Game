# OTV2-20261005-map-viewport-perf-1

```yaml
task_id: OTV2-20261005-map-viewport-perf-1
title: "MAP-VIEWPORT-PERF-1 domain-17 snapshot to the MAP01-VIEWPORT-US gate"
mode: IMPLEMENT
status: authoring
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: map
base_branch: main
branch: agent/map-viewport-perf-1-20261005
pr: null
base_sha: 0ec917e6
owner: claude-code-session-01TwFmqXUtpmbigH6Nv6dooF (oteryn-impl-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md §2.2 (commit 76a0d9f6, PR 1819)"
review: Codex, on the frozen head, requested by the control plane
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owned_paths:
  - apps/game-server/src/map/view.rs
  - apps/game-server/src/gameplay_transport/world_map.rs
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json   # MAP01-VIEWPORT-US notes only
  - docs/agents/evidence/MAP-VIEWPORT-PERF-1-viewport.md
  - docs/agents/tasks/archive/OTV2-20261005-map-viewport-perf-1.md
```

## Scope

Flat window-offset arrays, reused buffers and an in-place delta replace the per-call maps and
vectors of the domain-17 plan. No wire change, no budget change, no new registry row.

## Validation

Filled at the final authoring commit.
