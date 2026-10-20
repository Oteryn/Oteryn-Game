# REGEN-FOOD-1

```yaml
task_id: REGEN-FOOD-1
title: Player fed-time health/mana regeneration
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/regen-food-1-20261010
pr: null
owner: REGEN-FOOD-1 writer (CP coordination Oteryn/Oteryn-Game#1622)
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/food_regeneration.rs
  - apps/game-server/src/spell/actor_conditions.rs
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/spell/mod.rs
  - docs/reference/regeneration-food-20261010/
  - docs/agents/tasks/archive/REGEN-FOOD-1.md
public_contracts: []
```

## Outcome

A fed player regenerates health and mana at the owner-decided per-vocation rates; food time is
capped at 1,200 s; protection zones suppress regeneration. Evidence:
`docs/reference/regeneration-food-20261010/README.md`.

High-risk authority/recovery qualification: NOT_APPLICABLE (no fence, session, persistence or wire
change; runtime-only accumulator on the existing actor vitals successor).

## Validation

`cargo test -p oteryn-game-server food_regeneration`, clippy and fmt.

## Open

Item-use ingress (eat an item) and accumulator persistence are separate slices.
