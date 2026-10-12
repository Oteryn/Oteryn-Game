# OTV2-20261012-entry-spell-tiles-1

```yaml
task_id: OTV2-20261012-entry-spell-tiles-1
title: "ENTRY-SPELL-TILES-1: qualified tile facts for every cell of the accepted entry room"
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/entry-spell-tiles-1-20261012
pr: 2005
base_sha: 08e13f7dea83c43c13df85f901caf1e032ef3cd9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Claude Code session_01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-12T06:30:00Z
updated_at: 2026-10-12T07:20:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/project/native_entry_room.json
  - apps/game-server/src/content/native_gameplay.rs
  - apps/game-server/src/gameplay_transport/monster_ai_cycle.rs
  - tools/monster-lab/arena_map.py
  - docs/agents/tasks/active/OTV2-20261012-entry-spell-tiles-1.md
  - docs/agents/tasks/archive/OTV2-20261012-entry-spell-tiles-1.md
public_contracts:
  - docs/architecture/reviews/OTERYN_GAME_SPAWN1A_FIXTURE_SPAWN_ROOM_R2_DECISION_2026-10-04.md
depends_on:
  - "CREATURE-MOVE-1b (#1987)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The accepted entry room (`native_entry_room.json`, `OTERYN_NATIVE_ENTRY_ROOM_SOURCE/v1`, `entry-r2`) gets a `native_first_entry.spell_tiles` document (`OTERYN_NATIVE_SPELL_TILES/v1`). CREATURE-MOVE-1b (#1987) found that the room had no tile document, so every tile lookup answered `Unknown`. A creature's melee perception then dropped any player it could not place on a qualified tile, so rats in the entry room could not perceive a player (owner choice (a)).

- One tile per placed cell, in placement order, six in all: `entry-start`, `entry-east`, `entry-north`, `entry-den`, `entry-den-north`, `entry-door`. `QualifiedSpellTiles::qualify` requires exactly one tile per cell.
- Ground: `terrain_key` equals the placed terrain (`oteryn:terrain/stone-floor`) and `source_binding` is `null`. `ground_speed` is `null` and `top_items` is empty.
- Flags: `entry-north` (the `Blocked` cell) sets `block_solid`, `block_projectile` and `immovable_block_solid`. Every other flag on every tile is `false`, including `protection_zone` and `floor_change`.
- A lookup for a placed cell now answers `Ok`, and a cell outside the room answers `Absent`.

## Rotation

- The room bytes changed (sha256 `b0d5d034…1308940` → `5f7ba6cd…27f6`). The server digest, frame binding digest and map revision digest are computed at runtime from those bytes. No literal pins of them were found in tests or goldens.
- `tools/monster-lab/arena_map.py` `SOURCE_SHA256` is rotated to the new bytes.
- `.github/workflows/native-entry-room-qualification.yml` needs no change: `EXPECTED_SHA` is the commit under test, and `_platform` pins the Platform checkout, not the room.
- Two tests asserted `Unknown` tiles for the room and now assert the qualified tile (`monster_ai_cycle.rs`, `native_gameplay.rs`).

## Behavior changes (DERIVED from code)

- When native gameplay is active, `field_step_ingress` takes the durable field-ingress path for entry-room player steps instead of the plain `step_in_channel`, as the spell candidate room already does.
- `world_execution` no longer refuses world/tile-metadata spells in this room with `MissingTileMetadata`.
- In `monster_ai_cycle::creature_cell_admissible`, the `Unknown` fail-open arm is no longer reached for the entry room. It is kept for any accepted room without a tile document; removing it is a possible follow-up.
- Player pacing still uses `EngineeringGroundSpeed` (150). With `ground_speed: null`, companion haste still refuses in this room.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task adds authored content facts to an accepted room overlay and updates two test assertions. It does not touch session, lease, generation or fence evidence, PREPARE/COMMIT, controllers or persisted recovery evidence.

## Open questions

- Amendment 01 §3 and Amendment 02 list a closed set of room overlays that does not name `spell_tiles`. The code (`accepted_shape`) already admits `spell_tiles` for both rooms. Should a docs follow-up amend those tables?
- Should `ground_speed` stay `null` or be authored as a value? `null` was chosen so that no speed is invented.

## Validation

- cargo fmt --all --check: pass
- git diff --check: pass
- cargo test --locked -p oteryn-game-server --lib -- native_entry native_spell_tiles monster_ai_cycle native_gameplay content::: pass (352 passed, 1 ignored)
- cargo test --locked -p oteryn-game-server --test content_native_entry_room --test content_native_entry: pass (11 passed; 2 passed, 1 ignored)
- python3 -m unittest discover -p 'test_*.py' (tools/monster-lab): pass (51 tests OK)
- cargo clippy --all-targets, the full workspace tests and the native entry room qualification workflow (`tools/qualification/native_entry_room/run.sh`, Platform checkout and PHP image): CI (local disk full)
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass (60 tests OK)
