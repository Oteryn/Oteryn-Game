# OTV2-20261006-map-viewport-perf-2

```yaml
task_id: OTV2-20261006-map-viewport-perf-2
title: "MAP-VIEWPORT-PERF-2 flat handle table and cheaper plan stage for the MAP01-VIEWPORT-US gate"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: map
base_branch: main
branch: claude/map-viewport-perf-2-20261006
pr: 1859
base_sha: 86f704d8
owner: claude-code-session-01Uju7uZDKoK1KBi6dg1Chph (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-06
updated_at: 2026-10-06
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md §2.2"
review: on the frozen head, requested by the control plane
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owned_paths:
  - apps/game-server/src/gameplay_transport/item_view.rs
  - apps/game-server/src/gameplay_transport/world_map.rs
  - apps/game-server/src/map/view.rs
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - docs/agents/evidence/MAP-VIEWPORT-PERF-2-viewport.md
  - docs/agents/tasks/archive/OTV2-20261006-map-viewport-perf-2.md
```

## Scope

A hashed slot table replaces the `BTreeMap`/`BTreeSet` handle maps of `ItemHandleTable`; the plan stage
cuts stacks by index range, reuses the view's buffers between updates and moves tiles into the snapshot
instead of cloning them. No wire change, no budget, registry or protocol change. Handle issuance unchanged.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server world_map`: pass (20 passed, golden bytes unchanged)
- `cargo test --locked -p oteryn-game-server item_view`: pass (22 passed); `container_view`: pass (14 passed)
- `cargo test --locked -p oteryn-game-server map::`: pass (25 passed)
- `cargo test --release --locked -p oteryn-game-server --lib map_viewport_measure -- --ignored --nocapture`: OK, snapshot p99 1.683 ms (main 3.264 ms); the 100 us gate is not met (BLOCKER, see evidence)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (54 tests)
- `git diff --check`: pass

Evidence: `docs/agents/evidence/MAP-VIEWPORT-PERF-2-viewport.md`.
