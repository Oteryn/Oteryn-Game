# OTV2-20261012-bundle-tiles-1

```yaml
task_id: OTV2-20261012-bundle-tiles-1
title: "BUNDLE-TILES-1: derived spell tile facts for the bundle scope from the v4 World Bundle and the served Items"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/bundle-tiles-1-20261012
issue: 1622
pr: 2012
next_action: "BUNDLE-COMBAT-ENABLE-1 (after #1981, #2005 and #2006) opens the qualified_room gates for the bundle scope; BUNDLE-TILES-IMMOVABLE-1 and BUNDLE-TILES-PROJECTILE-1 source the facts this task leaves fail-closed"
owned_paths:
  - apps/game-server/src/map/spell_tiles.rs
  - apps/game-server/src/map/mod.rs
  - apps/game-server/src/map/boot.rs
  - apps/game-server/src/content/project/native_spell_tiles.rs
  - apps/game-server/src/content/project/native_entry.rs
  - docs/agents/tasks/archive/OTV2-20261012-bundle-tiles-1.md
```

## Result

- Owner decision: the spell and combat tile facts of the deployed World Bundle map are derived at boot from the format v4 bundle and the pinned content. There is no format v5, no new bundle field and no migration.
- New module `apps/game-server/src/map/spell_tiles.rs`. `SpellTilesBuilder` runs in the boot's one sector scan (`CheckedBundle::boot` and `scan` in `map/boot.rs`), so the bundle is decoded once. `BundleSpellTiles` stores one interned `QualifiedSpellTile` per distinct tile and a `u32` shape index per base tile (`WorldBase::tile_index`, split out of `WorldBase::tile`). It is shared with `BundleMap` through `Arc`.
- `QualifiedSpellTiles::lookup` answers under the bundle scope. `NativeEntryMovementCells::from_bundle` binds the bundle's tiles to the bundle scope (`QualifiedSpellTiles::from_bundle`). The scope check is unchanged: the entry scope does not open the bundle's tiles, and the bundle scope does not open the entry room's tiles.
- Sources, one per fact (the module documentation is the reference):
  - Ground: the base ground item, its id and its stored `ground_speed`.
  - Source ids: the Tibia id of the palette key.
  - `block_solid`: a non-walkable ground, or a tile the boot blocks (a wall, a solid or unprofiled Item, a sealed door).
  - `block_projectile`: a wall, or an Item whose served definition blocks projectiles. An unprofiled Item blocks, matching `boot_bundle_items`.
  - `floor_change`: the MAP-FLOOR-1 catalogue mask.
  - `protection_zone`: tile flag bit 0.
- Rope spots (386, 21965) keep their ground id and are not `floor_change`. The genuine Canary capture `canary-thalom-world.json` (item policy 386) records them as walkable ground with `floor_change` false.
- Some tiles are unknown (`SpellTileLookupError::Unknown`): a top-level entry without a source id, a pickupable entry, or a usable door (MAP-DOOR-1 `DoorBook`), because their state changes at runtime. A position without a bundle tile is `Absent`. The source step facts (height, floor-change kinds, PvP zones) are `None`.
- `immovable_block_solid`, `immovable_block_item` and `immovable_nonfield_block_item` have no source: neither the bundle nor the served definitions state `movable` or `block_pathfind`. They fail closed as `true`, so rope, levitate and house-kick relocation refuse every bundle tile as a destination.
- Canary and Crystal semantics beyond the repository documents and captures named in the module documentation are OTS_HYPOTHESIS_ONLY.
- Unchanged: the `qualified_room` gate in `node/serve.rs` and the 12 gate sites, `node/serve.rs`, `gameplay_transport/mod.rs`, the bundle format and the bundle compiler.

## Follow-ups (out of owned paths)

- BUNDLE-COMBAT-ENABLE-1 (after #1981, #2005, #2006): open the `qualified_room` gates for the bundle scope. Note: `QualifiedCellGroundSpeed` (companion execution) reads the tile's stored `ground_speed`, not the pacing switch point `BundleWorld::ground_speed`.
- BUNDLE-TILES-IMMOVABLE-1: source the three `immovable_*` facts. The candidates are an Item profile `movable` or `block_pathfind` fact, or a pinned capture.
- BUNDLE-TILES-PROJECTILE-1: most palette Items of the deployed generation have no Item profile, so their tiles block projectiles and ranged combat on the bundle stays mostly refused. The candidates are Item profiles, or appearance `flags.unsight` (OTS_HYPOTHESIS_ONLY until a pinned decision).

## Validation

- cargo fmt --all --check: pass
- git diff --check: pass
- cargo clippy --locked -p oteryn-game-server --lib --tests -- -D warnings: pass
- cargo test --locked -p oteryn-game-server --lib -- spell_tiles map::: not run locally (the shared build disk was full; the test link failed with a bus error), CI runs it
- the ignored `pinned_bundle_tiles_answer_every_base_tile` (OTERYN_PINNED_WORLD_BUNDLE, pinned pre-production artifact): not run, same reason
- clippy --all-targets and the full cargo test -p oteryn-game-server: CI (disk budget)
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass (60 tests OK)
