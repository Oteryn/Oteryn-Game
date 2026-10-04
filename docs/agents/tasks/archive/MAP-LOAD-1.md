# MAP-LOAD-1

```yaml
task_id: MAP-LOAD-1
title: "MAP-LOAD-1: World Bundle reader and compact base model"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/map-load-1-20261004
pr: PENDING
base_sha: 3af9994
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01D4rjLMf4CTUB5vZ3uYbWxe (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "docs/architecture/reviews/OTERYN_GAME_MAP_LOAD_PACKET1_BUNDLE_LOADER_DECISION_2026-10-04.md §2.2 with §1.1-§1.4; ADR-0021 §4.1, §4.2, §4.8 and the ITEM-PACKETS-AMEND-1 ground-speed amendment"
leases: none (no migration, no wire change)
owned_paths:
  - crates/world-bundle/**
  - tools/world-bundle-compiler/**
  - Cargo.toml
  - Cargo.lock
  - apps/game-server/Cargo.toml
  - apps/game-server/src/map/**
  - apps/game-server/src/movement/speed.rs
  - apps/game-server/src/world_runtime.rs
  - apps/game-server/tests/map_load_*.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/agents/evidence/MAP-LOAD-1-*.md
  - docs/agents/tasks/archive/MAP-LOAD-1.md
depends_on:
  - "MAP-BUNDLE-2 merged (format v3)"
  - "SPEED-1 merged"
public_contracts: []
external_repositories: []
```

## Outcome

- **`oteryn-world-bundle` (new crate, `crates/world-bundle`).** The v3 layout, the sector and
  spawn codecs (moved from the compiler with history) and the reader.
  - Caps are checked before any allocation.
  - The reader rejects the whole bundle on the first failure.
  - The compiler re-exports the crate and keeps its writer; its existing tests pass unchanged.
- **`WorldBase` (`apps/game-server/src/map`).** The compact read-only base, shared by `Arc`
  (`WorldBaseHandle` in `world_runtime.rs`, not wired).
  - Per tile: positions, palette-resolved compact ids with depths, the ground item, the walkable
    flag and the ground speed, from the palette `terrain` field only (§1.3, §1.4).
  - Sector occupancy is a bitset with a per-word rank, and a sector-row index sits over the sector
    table.
  - `load` takes only the bundle bytes and the pins (digest, schema versions, content revision,
    production flag). A production World refuses a non-production bundle.
- **`MapGroundSpeed`** (`movement/speed.rs`): the map-backed `GroundSpeedSource`. It returns 0 for
  no tile, no ground, or a non-walkable ground. Production keeps `EngineeringGroundSpeed`.
- **Budgets (§4.8).** All three rows are confirmed at their ADR values on the real map; see
  `docs/agents/evidence/MAP-LOAD-1-base-budgets.md`.

## Tests

- `apps/game-server/tests/map_load_base.rs`:
  - tile-by-tile equivalence;
  - terrain from the palette;
  - no other file opened;
  - pin refusals;
  - corrupt and unknown content;
  - ground-speed bounds;
  - every tile and bundle cap at its maximum and at maximum + 1;
  - a bounded seeded property test of the reader, which never panics;
  - two ignored budget tests that are run by hand.
- `movement::speed` unit tests: the map source with a non-150 ground and each 0 case, and a step
  paced on the map ground speed and refused at 0.

## Review

The security review of the bundle reader (ADR-0021 §4.8) runs on the final frozen head; the
control plane requests it.
