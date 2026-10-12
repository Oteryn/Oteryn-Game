# OTV2-20261012-bundle-start-tile-1

```yaml
task_id: OTV2-20261012-bundle-start-tile-1
title: "BUNDLE-START-TILE-1: the real World Bundle boots on preproduction (E2012 BOOT_WORLD_BUNDLE_REFUSED, start tile not enterable)"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/bundle-start-tile-20261012
pr: 2017
next_action: "redeploy the Synology preproduction Game node from this merge and set node.toml and ROOT_BASE/ops/preprod-topology.toml [world_bundle] start_x = 10229; BUNDLE-SOLID-COMPILER-1 aligns the compiler's NPC/route BlockSolid fact at the next planned World reset"
owned_paths:
  - apps/game-server/src/map/facts.rs
  - apps/game-server/src/map/boot.rs
  - apps/game-server/src/map/spell_tiles.rs
  - tools/world-bundle-compiler/src/main.rs
  - tools/world-bundle-compiler/tests/pin_check.rs
  - content/world/pins/oteryn.json
  - content/world/pins/README.md
  - deploy/synology-game/preprod-topology.toml
  - docs/agents/tasks/archive/OTV2-20261012-bundle-start-tile-1.md
```

## Cause

- The preproduction node boots the pinned bundle (digest `5c32d7ac…`) with `[world_bundle] start = (10228, 10032, 0)`. `map::boot::check` passes (the cell has a walkable ground). `CheckedBundle::boot` refuses with `StartNotEnterable` (`apps/game-server/src/map/boot.rs`, the `enterable(&self.base, &blocked, self.start)` check), which `node/serve.rs` reports as E2012 `BOOT_WORLD_BUNDLE_REFUSED`.
- `blocked` comes from `palette_facts` (`apps/game-server/src/map/facts.rs`): an Item entry whose served definition has no solidity fact blocks. In the real World every palette entry is an Item key (`oteryn:item.tibia.i<N>`), grounds and borders included; they carry their `ground`/`border` Terrain record in the palette (format v2 Terrain route). The generation's Item profiles (`spell-native-profiles.json`) cover none of the World's grounds or borders, so every ground tile was blocked at boot.
- The declared start (10228, 10032, 0) also holds the plain Item i1788 (part of a 2x2 object, no Terrain record, no profile), which blocks even with grounds fixed.

## Result

- `facts.rs`: an Item entry routed to a `ground` Terrain record does not block when its served definition has no solidity fact (`terrain_floor`); the base tile's ground still decides walkability. An Item routed to a `border` does not block only when the terrain catalogue proves the border walkable (`walkable` KNOWN true): `UNWALKABLE_BORDERS` lists the 244 `border` records whose fact is not KNOWN true (shorelines, cliffs, for example i30614, i4633–i4652), and a test keeps it equal to `content/world/terrain`, so an unwalkable or unknown border keeps blocking (review finding on #2017). A `wall` still blocks; a stated `blocks_movement` still decides; every other Item without the fact still blocks (as the MAP-FLOOR-1 floor-change exception).
- The pin's and the preproduction topology's `entry_start` move to (10229, 10032, 0): walkable ground i4408 and border i1109 (catalogue `walkable` KNOWN true), no plain Item, no wall, no door, no NPC, spawn or floor change. The bundle bytes and digest (`5c32d7acdb5c41552a8081edbc09b8009a9b52a1b282c254e0b7254e0a4965e7`) are unchanged, so the staged artifact stays valid; `inputs_digest` is recomputed.
- `world-bundle-compiler pin-check` refuses an `entry_start` whose top-level entries hold a plain Item (an Item entry without a Terrain record), so a pin cannot again name a start the node refuses; `pin_check.rs` covers the former start.
- Not changed: the bundle format, the compiler's bundle output and its NPC/route `BlockSolid` fact (`compile.rs` `cell_facts`). That fact still calls an Item-routed ground without a profile solid, which is stricter than the node; aligning it changes the bundle bytes (travel routes held), so it waits for a planned World reset (follow-up BUNDLE-SOLID-COMPILER-1).

## Host follow-up (no host was changed)

- The owner's proposed start (Tibia 32544, 32515, 7 = bundle (32544, 32515, -7), the Newhaven teleport landing) is not enterable in this bundle: besides ground i870 and walkable border i4539 it holds the plain Item i19394 ("RESERVED SPRITE", no solidity fact), as do 68 tiles around it. The start stays (10229, 10032, 0) until the owner decides.

- After this merges and the node is redeployed: node.toml `[world_bundle] start_x = 10229` (start_y 10032, start_floor 0 unchanged) and the same in `ROOT_BASE/ops/preprod-topology.toml`; digest, path and `[readiness] map_revision` unchanged.

## Validation

- cargo fmt --all --check: pass
- git diff --check: pass
- cargo clippy --locked -p oteryn-world-bundle-compiler --all-targets --quiet -- -D warnings: pass
- cargo clippy --locked -p oteryn-game-server --lib --tests --quiet -- -D warnings: pass
- cargo test --locked -p oteryn-game-server --lib -- map::: pass (65 passed, 4 ignored)
- the ignored `pinned_bundle_tiles_answer_every_base_tile` with the pin-check artifact `5c32d7ac…`: the boot at (10229, 10032, 0) with the generation's Items succeeds with this change and fails with `StartNotEnterable` without it; the test then stops on its pre-existing rope-spot `floor_change` assertion, which this task does not touch
- cargo run --release -p oteryn-world-bundle-compiler -- pin-check: pass (digest 5c32d7ac… unchanged)
- cargo test --locked -p oteryn-world-bundle-compiler --quiet: pass (39 passed, 1 ignored); the release-only `pin_check_accepts_the_reviewed_pin_and_refuses_every_drift` was not run locally (the shared disk filled while it copied the World), CI runs it
- bash deploy/synology-game/test-preprod-topology.sh: pass
- full cargo test -p oteryn-game-server (integration and PostgreSQL targets): CI (local disk budget)
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass (60 tests OK)
