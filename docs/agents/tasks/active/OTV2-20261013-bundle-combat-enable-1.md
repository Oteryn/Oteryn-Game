# OTV2-20261013-bundle-combat-enable-1

```yaml
task_id: OTV2-20261013-bundle-combat-enable-1
title: "BUNDLE-COMBAT-ENABLE-1: player auto-attack and native combat casts on a World Bundle Channel"
mode: IMPLEMENTATION
status: authoring
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/bundle-combat-enable-1-20261013
issue: 1622
governing_issue: 1622
terminal_pr_policy: archive_pending
pr: none
created_at: 2026-10-13
updated_at: 2026-10-13
depends_on: ["SPAWN-1b (#1981)", "ENTRY-SPELL-TILES-1 (#2005)", "CHAR-POSITION-1 (#2006)", "BUNDLE-TILES-1 (#2012)"]
owned_paths:
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/attack.rs
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs
  - apps/game-server/src/gameplay_transport/spell_timer_callbacks.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs
  - apps/game-server/src/spell/world_execution.rs
  - docs/agents/tasks/active/OTV2-20261013-bundle-combat-enable-1.md
  - docs/agents/tasks/archive/OTV2-20261013-bundle-combat-enable-1.md
public_contracts: []
external_repositories: []
```

## Goal

On a World Bundle Channel, player `ATTACK_TARGET` (auto-attack melee) and native combat spell casts work, and their kills reach the existing kill reward (loot and corpse) path. No wire-format change.

## Root cause

`node/serve.rs` passes `qualified_room: None` to the gameplay seam for a bundle World, because the qualified entry room carries the entry room's cells, frame binding and map revision, which do not match the bundle Channel pin. Every room consumer compares the room with the pin, so the gate was closed wholesale.

## Plan

- `QualifiedNativeEntryRoom::bundle_view`: the room with the bundle's cells (and their BUNDLE-TILES-1 spell tiles), frame binding, map revision and start; compiled Content and digests unchanged.
- A separate seam field `combat_room` (the room itself, or the bundle view) read only by auto-attack, `ATTACK_TARGET`, the native combat cast and its ordinary-combat timer continuation. Every other `qualified_room` gate (monster AI, world item and parameter casts, field step, periodic damage, source map) stays closed for a bundle World.
- `spell/world_execution.rs`: a bundle static owner; bundle static blockers are already tile facts and runtime blockers are Unknown tiles.
