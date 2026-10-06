# OTV2-20261006-map-viewport-measure-1

```yaml
task_id: OTV2-20261006-map-viewport-measure-1
title: "MAP-VIEWPORT-MEASURE-1 real-map measurement of the domain-17 snapshot and delta"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: map
base_branch: main
branch: claude/map-viewport-measure-1
pr: 1873
base_sha: 22539da5
owner: claude-code-session-019Zv18wHYVNSkBGu82dMt3f (oteryn-impl-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-06
updated_at: 2026-10-06
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md §2, amended by CP decision D823"
review: Codex, on the frozen head, requested by the control plane
head_sha: "exact frozen head in the FREEZE report to the control plane"
owned_paths:
  - apps/game-server/src/gameplay_transport/world_map_real_tests.rs
  - apps/game-server/src/gameplay_transport/world_map.rs   # one #[cfg(test)] mod line
  - apps/game-server/tests/support/real_map_bundle.rs
  - docs/agents/evidence/MAP-VIEWPORT-MEASURE-1-viewport.md
  - docs/agents/tasks/archive/OTV2-20261006-map-viewport-measure-1.md
```

## Scope

A release `#[ignore]` measurement over the real map (snapshot and delta p50/p99/max, stage split, bytes,
snapshot share, CPU per player-second at 1, 4 and 8 threads, 504 views). D823: the harness is an in-crate test
module because the map types are crate-private; the support file is a copy of the `map_load_base.rs` compile,
which is untouched. No visibility, wire, registry, budget or runtime change. The Channel writer line is
`not composed` (`observe_world_map` is the trait default on `main`).

## Result

Snapshot p99 646-791 us and delta p99 388-445 us at 1 and 4 threads; 163-174 us CPU per update, 0.41-0.44 core
at 500 players and 5 steps per second, against the 20% default (0.20). Does not fit at 5 steps per second; the
evidence proposes production rows and the next step (§1.2).

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --release --locked -p oteryn-game-server --lib map_viewport_real -- --ignored --nocapture`: ok (57 s)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

Evidence: `docs/agents/evidence/MAP-VIEWPORT-MEASURE-1-viewport.md`.
