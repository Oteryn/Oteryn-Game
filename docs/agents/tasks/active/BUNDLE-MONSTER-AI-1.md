# BUNDLE-MONSTER-AI-1

```yaml
task_id: BUNDLE-MONSTER-AI-1
title: "BUNDLE-MONSTER-AI-1: server-authoritative monster AI on a World Bundle Channel"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/bundle-monster-ai-1-20261013
issue: 1622
governing_issue: 1622
pr: null
created_at: 2026-10-13
updated_at: 2026-10-13
depends_on: ["BUNDLE-COMBAT-ENABLE-1 (#2025)", "SPAWN-1b (#1981)"]
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs
  - crates/gameplay/dispatch/src/gameplay_transport/channel_owner_auto_attack_tests.rs
  - docs/agents/tasks/active/BUNDLE-MONSTER-AI-1.md
public_contracts: []
external_repositories: []
```

## Outcome

Monsters spawned on a World Bundle Channel (SPAWN-1b) run the existing server-authoritative think and melee pass: target selection, chase step and melee swing. No wire, protocol, persistence or workflow change.

## Root cause

`drain_monster_melee` read `qualified_room`, which is `None` on a bundle World, so the pass returned before any think. #2025 added `combat_room`, the entry room's bundle view over the bundle cells, map revision and frame binding.

## Change

- `drain_monster_melee` reads `combat_room`: the player-target tiles (protection zone), the creature step rule (`creature_cell_admissible`: Walkable, not a protection-zone or floor-change tile, tile facts known) and the navigation revision now come from the bundle's cells.
- On a bundle World the entry door's collision set (entry frame) is not applied; bundle doors are Unknown tiles and already refuse a step.
- The fixture World is unchanged (`combat_room` is the entry room there).

## Not changed

World item and parameter casts, field step ingress, periodic player damage and the source item cycle keep reading `qualified_room`. Ranged and spell monster attacks stay disabled as before (melee-only first playable dispatch).

## Validation

- cargo fmt --all --check, git diff --check: pass
- cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings: pass
- cargo test --locked -p oteryn-game-server --lib: 2897 passed, 0 failed, 29 ignored
- new: `a_bundle_admits_creature_steps_only_onto_ordinary_walkable_cells`, `a_bundle_player_is_a_monster_target_outside_a_protection_zone_only`
- python tools/agents/validate_governance.py: pass
- integration and PostgreSQL targets: CI
