# OTV2-20260929-crystal-1530-monsters

```yaml
task_id: OTV2-20260929-crystal-1530-monsters
title: Game version 15.30 monsters from the CrystalServer summer-update branch (Canary lacks them)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: null
jira: KAN-16
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260929-crystal-1530-monsters.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
  - tools/content-schema/monster-authoring/**
  - tools/content-migration/creature_admission_stage.py
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/**
  - imports/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - "zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f (branch summer-update, read-only reference)"
```

## Outcome

The game version is 15.30 (`OTERYN_GAME_VERSION_1530_AND_OTS_BRANCHES_DECISION_20260928.md`). No Canary branch has the
Summer Update 2026 monsters, so they come from CrystalServer `summer-update` at `00ce02a5`, pinned by commit.

- `crystal_batch.py` selects the 49 files under `data-global/monster/summer_update_2026` whose name no Canary file
  creates (8 of them bosses). It converts them with the normal converter and the Crystal 15.30 item tables.
- The reference-date wiki values (45 pages at the cut) and the library values (21 monsters) are adopted over the Crystal
  values, per D15 and D47.
- Census: 48 resolved, 1 blocked (Phosphorus final form, an unresolved attack).
- Admission: 13 creatures enter `content/world` under the new import batch `g4-creature-crystal-1530-r1`
  (`oteryn:source.crystalserver`, bindings `crystalserver/monster-file`).
  - 33 wait for their 15.30 loot items in the Oteryn Item registry (#1179 registers the Crystal `00ce02a5` items).
  - The 2 Energy Cannons wait as `initial_health`: they spawn at 1 of 100 health, and the profile has one health value.

## Acceptance and evidence

- The census, the staging, the materializer, the tree writer and its validators, and the Rust tests pass.
- Exact-head review before the Merge Queue, because `content/world` changes.
