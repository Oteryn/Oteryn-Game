# OTV2-20261005-map-cutover-1a

```yaml
task_id: OTV2-20261005-map-cutover-1a
title: "MAP-CUTOVER-1a world bundle boot behind config, bundle collision index, boot refusals"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: map
base_branch: main
branch: agent/map-cutover-1a-20261005
pr: 1825
base_sha: 0ec917e6
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01LqD6MrGPFwRYfhzXGNAGDB (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md §2.3 MAP-CUTOVER-1a (at 76a0d9f6)"
owned_paths:
  - apps/game-server/src/node/config.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/map/boot.rs
  - apps/game-server/src/map/mod.rs
  - apps/game-server/src/content/native_cell_lookup.rs
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/tests/map_cutover_boot.rs
  - docs/agents/tasks/active/OTV2-20261005-map-cutover-1a.md
  - docs/agents/tasks/archive/OTV2-20261005-map-cutover-1a.md
public_contracts: []
external_repositories: []
```

## Outcome

- **Config.** Optional `[world_bundle]` in the node config: absolute path, 64-hex digest, the
  pinned project format, world schema and content revision, `production` (only `false` accepted)
  and the start tile (floor -15..=0). Absent, the node serves the fixture entry room as before.
- **Boot** (`map/boot.rs`). `boot` runs `map::load` with the pins, compares the readiness
  `map_revision` with the bundle's, derives the blocked tiles (a wall terrain, a missing palette
  entry, or an item whose definition is not known non-solid) and refuses a start that is missing,
  not walkable or blocked. A restart with the same pins rebuilds an equal `BundleWorld`.
- **Collision.** `BundleCollisionIndex` is a new sealed `NativeStaticCellLookup` variant; the
  Channel movement owner steps on it under the bundle scope (frame
  `oteryn:frame/world-bundle-v3`, the bundle `map_revision`). Spell and house tiles keep the room
  scope and so refuse under the bundle scope (fail closed).
- **block_solid.** The item definitions are not served yet, so `serve` boots with no known
  definition and every item tile blocks (fail closed) until MAP-CUTOVER-1b serves them.
- **Serve.** With `[world_bundle]` the node reads and boot-checks the bundle right after the
  configuration is accepted, before durability, registration, assignment or fixture content
  activation, and then stops with exit 21 (`WorldBundleUnserved`); a refusal exits 20. The node
  never activates both map sources. Serving the bundle World (its movement cells under its own
  content pin) is MAP-CUTOVER-1b; `BundleWorld::movement_cells` is the seam, tested here.
- Ground speed stays the engineering 150 behind `BundleWorld::ground_speed` until MAP-CLIENT-1.

## Tests

- `map::boot::tests`: a Channel walk steps onto grass and is refused onto lava, a wall, a solid
  item and off the map; an unknown item definition blocks.
- `tests/map_cutover_boot.rs`: boot; refusals for digest, schema, content revision,
  map_revision and each non-enterable start; restart equality.
- `node::config::tests`, `node::serve::tests`: the bundle section checks; a fixture config goes
  on to serve; a bundle config refuses after boot; boot refusals map to exit 20.

## Validation

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo test --locked -p oteryn-game-server map_cutover`: pass (3 unit, 3 integration).
- `cargo test --locked -p oteryn-game-server native_cell`: pass.
- `cargo test --locked -p oteryn-game-server node::config`: pass.
- `cargo test --locked -p oteryn-game-server node::serve`: pass.
- `cargo run --locked -p oteryn-architecture-check -- workspace .`: pass.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.
- `git diff --check`: pass.
