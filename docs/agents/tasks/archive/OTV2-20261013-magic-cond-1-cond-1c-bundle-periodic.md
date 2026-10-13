# OTV2-20261013-magic-cond-1-cond-1c-bundle-periodic

```yaml
task_id: OTV2-20261013-magic-cond-1-cond-1c-bundle-periodic
title: MAGIC-COND-1 COND-1c -- open periodic player damage on a bundle World
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/magic-cond-1-cond-1-20261013
pr: null
base_sha: 39f77b53
owner: "Oteryn: worker for control plane session_01XYC239givrMonrYoBwwECV"
created_at: 2026-10-13
updated_at: 2026-10-13
owned_paths:
  - apps/game-server/src/gameplay_transport/spell_periodic.rs
  - docs/agents/tasks/archive/OTV2-20261013-magic-cond-1-cond-1c-bundle-periodic.md
public_contracts:
  - CONDITIONS0-ACTOR-CONDITIONS-V1 (§3.2)
depends_on:
  - "BUNDLE-COMBAT-ENABLE-1 (#2025) merged"
```

## Outcome

COND-1a/1b and the owner wiring are on main. The remaining COND-1c gap was the bundle gate:
`tick_spell_periodic` read `qualified_room`, which is `None` on a bundle World, so a player's
damage-over-time and field ticks never ran there (BUNDLE-COMBAT-ENABLE-1 left "periodic player
damage" closed for MAGIC-COND-1). It now reads `combat_room`, which is the fixture room on a
fixture World and the room's bundle view (bundle-pin identities) on a bundle World. Every other
`qualified_room` consumer stays closed (monster AI, world item and parameter casts, field step
ingress, source map initialization).

## Excluded scope

COND-WIRE-1, protocol, migration, `foundation/**` (no change was needed).

## Validation

- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- PostgreSQL-backed tests: not run (no Docker daemon); no PG-owned code changed.
