# OTV2-20261013-bundle-combat-enable-1

```yaml
task_id: OTV2-20261013-bundle-combat-enable-1
title: "BUNDLE-COMBAT-ENABLE-1: player auto-attack and native combat casts on a World Bundle Channel"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/bundle-combat-enable-1-20261013
issue: 1622
governing_issue: 1622
terminal_pr_policy: archive_pending
pr: 2025
created_at: 2026-10-13
updated_at: 2026-10-13
depends_on: ["SPAWN-1b (#1981)", "ENTRY-SPELL-TILES-1 (#2005)", "CHAR-POSITION-1 (#2006)", "BUNDLE-TILES-1 (#2012)"]
next_action: "BUNDLE-MONSTER-AI-1 opens the monster AI gate for a bundle World; BUNDLE-TILES-PROJECTILE-1 and BUNDLE-TILES-IMMOVABLE-1 source the facts that keep ranged and relocation casts mostly refused"
owned_paths:
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/attack.rs
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs
  - apps/game-server/src/gameplay_transport/spell_timer_callbacks.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs
  - apps/game-server/src/foundation/channel_owner_auto_attack_tests.rs
  - apps/game-server/src/map/spell_tiles.rs   # test helpers made pub(crate) only
  - apps/game-server/src/spell/world_execution.rs
  - docs/agents/tasks/active/OTV2-20261013-bundle-combat-enable-1.md
  - docs/agents/tasks/archive/OTV2-20261013-bundle-combat-enable-1.md
public_contracts: []
external_repositories: []
```

## Root cause

`node/serve.rs` gave the gameplay seam `qualified_room: None` for a bundle World. The qualified entry room carries the entry room's cells, frame binding and map revision, and every room consumer compares those with the Channel pin, which on a bundle World carries the bundle's frame binding and map revision. So every consumer was closed, `ATTACK_TARGET` and the auto-attack drain (`attack.rs`) and the native combat cast (`native_combat_cast.rs`) included.

## Result

- `QualifiedNativeEntryRoom::bundle_view` (`content/project/native_entry.rs`): the qualified room over the bundle's cells (with the BUNDLE-TILES-1 spell tiles), the bundle frame binding, map revision and start, as `bundle_channel_pin` puts them on the pin. The compiled Content, its digests and the entry door content stay the room's own; the spawn source is dropped. It binds only bundle cells of the same World and compiled generation, in a frame other than the entry frame.
- `node/serve.rs` builds the combat room (the room itself, or its bundle view; event `bundle_combat state=enabled|disabled`) and hands it to the seam as the new `combat_room`. `qualified_room` is unchanged.
- `combat_room` is read by `ATTACK_TARGET`, the auto-attack drain, the native combat cast (both gate sites) and the ordinary-combat timer continuation. Every room-vs-pin identity check still runs and now binds to the bundle pin; owner fence, session-generation fencing and the spell item authority are unchanged.
- `spell/world_execution.rs`: a bundle static owner. The bundle's static blockers are already tile facts (`block_solid`, `block_projectile`); its runtime blockers (usable doors, pickupable Items) are Unknown tiles and are refused before the owner is asked. Entry door bindings keep the entry frame and refuse a bundle World.
- Kills: a lethal swing or cast enters the existing kill sink and `drain_kill_rewards` (loot, corpse); that path reads the Channel pin, not the room, and is unchanged.
- No wire-format, protocol, persistence or workflow change.

## Not changed (stays closed for a bundle World)

Monster AI, world item and parameter casts, field step ingress, periodic player damage and the source map initialization keep reading `qualified_room` (`None` on a bundle World). Tiles the bundle leaves Unknown (a coin or other pickupable Item, a usable door, an Item without a source id) refuse a swing or cast tile read; most bundle Items have no projectile profile (BUNDLE-TILES-PROJECTILE-1), so ranged and area casts stay mostly refused, and relocation casts refuse every bundle tile (BUNDLE-TILES-IMMOVABLE-1).

## Validation

- cargo fmt --all --check: pass
- git diff --check: pass
- cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings: pass
- cargo test --locked -p oteryn-game-server --lib --quiet: 2869 passed, 0 failed, 29 ignored
- new tests in `foundation/channel_owner_auto_attack_tests.rs`: `a_bundle_view_carries_the_bundle_pin_identities_and_binds_only_bundle_cells`, `a_bundle_rat_is_held_and_attackable_outside_a_bundle_protection_zone`, `a_bundle_protection_zone_refuses_the_target_and_the_attacker`: pass
- integration and PostgreSQL targets of oteryn-game-server: CI (local disk budget)
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass
