# OTV2-20260928-vortex-conditions

```yaml
task_id: OTV2-20260928-vortex-conditions
title: Stepped-on trigger and caster condition (owner decision D46)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: null
jira: KAN-16
base_sha: a311c4eb2b16d88b45b24f20a12f7f2b76fb78c3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-vortex-conditions.md
  - docs/agents/tasks/active/OTV2-20260928-more-summons.md
  - docs/agents/tasks/archive/OTV2-20260928-more-summons.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
  - tools/content-schema/monster-authoring/**
  - tools/content-schema/encounter-authoring/**
  - tools/content-migration/test_world_project_v2_to_tree.py
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - content/imports/**
  - imports/**
  - content/creatures/definitions/**
  - content/presentations/definitions/**
  - content/behaviors/**
  - content/loot/**
  - content/abilities/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision D46 ("tak", 2026-09-28): two vocabulary additions for the events that need them.

- `stepped_on(role, ItemRef)` trigger: the Heart of Destruction vortex. The Hunger or the World Devourer opens its
  vortex tile; a Greed stepping on an open vortex disappears and lowers both summon counters. With it, `hunger summon`
  (a Greed while fewer than three are out, 15 s apart) becomes a rule of the `world_devourer` encounter.
- `has_condition(role, conditions, present)` condition and the `relative(x, y)` position: `soulcatcher summon` (a
  Corrupted Soul north of the Soulcatcher while it is poisoned or bleeding; Canary evidence, to be verified).
- The census rises from 1,555 to 1,557 (The Hunger and Soulcatcher); both wait in the creature staging with the other
  encounter-covered monsters. This PR also archives the remaining summons task record of #1121.

## Acceptance and evidence

- `verify_encounter_schema.py` 137/137; 83 encounters validate and 78 manifests resolve fully.
- The census, the staging, the tree regeneration and its validators pass.
- The Rust tests pass.
- Exact-head review before the Merge Queue because staged creature evidence changes.
