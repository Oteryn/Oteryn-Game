# OTV2-20261005-map-viewport-perf-1

```yaml
task_id: OTV2-20261005-map-viewport-perf-1
title: "MAP-VIEWPORT-PERF-1 domain-17 snapshot to the MAP01-VIEWPORT-US gate"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: map
base_branch: main
branch: agent/map-viewport-perf-1-20261005
pr: 1839
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
  - apps/game-server/src/gameplay_transport/item_view.rs   # CP D757
```

## Scope

Flat window-offset arrays, reused buffers and an in-place delta replace the per-call maps and
vectors of the domain-17 plan. No wire change, no budget change, no new registry row.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server world_map`: pass (20 passed; item_view 22 passed)
- `cargo test --locked -p oteryn-game-server map::`: pass (23 passed)
- `cargo test --release --locked -p oteryn-game-server map_viewport_measure -- --ignored --nocapture`: OK, snapshot p99 2.098 ms (round 2); the 100 us gate is not met (BLOCKER, see evidence)
- `python tools/agents/validate_governance.py`: pass
- `git diff --check`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (54 tests)

Evidence: `docs/agents/evidence/MAP-VIEWPORT-PERF-1-viewport.md`.
