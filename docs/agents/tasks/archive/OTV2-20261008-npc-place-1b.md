# OTV2-20261008-npc-place-1b

```yaml
task_id: OTV2-20261008-npc-place-1b
title: "NPC-PLACE-1b: World Bundle v4 NPC frame, placement realization and travel destination holds"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-place-1b-20261008
issue: 1622
pr: 1983
head_sha: "exact frozen head in the PR FREEZE comment"
final_head_sha: "exact frozen head in the PR FREEZE comment"
owner: single writer session (control plane session_01CwP6d84eCPvpgoEuyci8Tx)
created_at: 2026-10-08
updated_at: 2026-10-11
execution_policy: continuous_progress
packet: "docs/architecture/reviews/OTERYN_GAME_NPC_PLACE1_NPC_PLACEMENTS_DECISION_2026-10-06.md §3, §5, §6, §7"
owned_paths:
  - crates/world-bundle/
  - tools/world-bundle-compiler/
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - apps/game-server/tests/npc_placement_parity.rs
  - apps/game-server/src/map/boot.rs
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - apps/game-server/src/movement/speed.rs
  - apps/game-server/tests/map_cutover_boot.rs
  - apps/game-server/tests/map_cutover_view.rs
  - apps/game-server/tests/map_load_base.rs
  - apps/game-server/tests/map_overlay_channel.rs
  - apps/game-server/tests/support/map_item_mint_postgres_cases.rs
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
- Control plane Q3 (a) (D607): `tests/map_cutover_boot.rs` and `tests/map_cutover_view.rs`, the
  same single `npcs: Default::default(),` line only.
- Control plane Q2 (a): `apps/game-server/tests/npc_placement_parity.rs`, a new file on existing
  pub APIs only.
- Control plane D967 (4a 5a): `gameplay_transport/world_map_tests.rs` also for its `GOLDEN` digest
  (the v4 bundle moves `map_revision`); `tests/map_load_base.rs`, `tests/map_overlay_channel.rs`
  and `tests/support/map_item_mint_postgres_cases.rs` for their hand-built bundle assemblers' v4
  update only (NPC row and frame, format 4, v4 digest domain), assertions unchanged.
- Control plane decision (b): the CI routing change is not in this PR, because any
  `tools/repository/**` change runs the merge-authority audit, which stays red until the Rust 1.95
  gate migration lands (#1978 then #1944).

## Outcome

`OTERYN_WORLD_BUNDLE/v4` carries the NPC row and frame and the manifest `npcs` block. The compiler
admits placements for service-model NPCs only, realizes them onto enterable cells or holds them
with a typed reason, and holds unrealizable travel destinations. The reader enforces the §14
checks and limits, and the `oteryn` pin is rebuilt for v4 with NPC-PLACE-1a's placements. Finding
for a separate map/runtime packet: on the real tree all 25 placements and all 195 travel routes
are held as `BlockSolid`, because Item-family ground without a native profile resolves as solid.

Follow-ups:

- WORLD-BUNDLE-ROUTING-1, which waits for #1944: add `content/npcs/definitions/`,
  `content/dialogues/definitions/` and `content/services/` to `WORLD_BUNDLE_INPUT_PREFIXES` in
  `tools/repository/classify_pr_test_lanes.py`, so that NPC catalogue edits run the world_bundle lane.
  Constraint: it must land with or before the next `tools/repository/**` PR. Until then,
  `test_classify_pr_test_lanes.py` fails its parity check against the compiler's `INPUT_PATHS`,
  which already lists these three directories.
- MAP-CELL-FACTS-1: compile-time cell facts taken from the first top-level ground, with contained
  entries ignored, for spawns and NPC placements alike (the Codex review finding on `compile.rs`).

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy -p oteryn-world-bundle -p oteryn-world-bundle-compiler -p oteryn-game-server -p oteryn-client --all-targets -- -D warnings`: pass
- `cargo test -p oteryn-world-bundle -p oteryn-world-bundle-compiler`: pass
- `cargo test -p oteryn-game-server -p oteryn-client --no-fail-fast`: pass
- `world-bundle-compiler pin-check . <out>`: pass
- `python3 tools/repository/validate_repository_policy.py`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Review

Review state on the frozen head is the control plane's; none requested by this worker.
