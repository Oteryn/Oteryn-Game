# OTV2-20260927-thirteenth-encounter-slice

```yaml
task_id: OTV2-20260927-thirteenth-encounter-slice
title: Monster spawn callbacks (thirteenth encounter slice)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: null
jira: KAN-16
base_sha: 52b983faad527247dfcd7cdc7a3dc41b834df9b1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-thirteenth-encounter-slice.md
  - docs/agents/tasks/active/OTV2-20260927-twelfth-encounter-slice.md
  - docs/agents/tasks/archive/OTV2-20260927-twelfth-encounter-slice.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - tools/content-schema/encounter-authoring/**
  - tools/content-schema/monster-authoring/samples/population-canary-47dfd51f.json
  - tools/content-schema/monster-authoring/samples/population-bundles-canary-47dfd51f.json
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This thirteenth encounter slice transcribes the `mType.onSpawn` callbacks of the Iron Servant Replica, the three Cobra Bastion
humans and the two Drume commanders as `creature_spawned` rules (D29). No vocabulary is added. The census rises from
1,517 to 1,523 fully resolved monsters. The completed task record of #1022 is archived. `content/world` and the Item
registry are untouched.

Authority: owner direction in this session ("kontynuuj dalej to co możesz"). Runtime behaviour stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: authoring-format evidence tooling and documentation only.

## Acceptance and evidence

- The samples are regenerated. 79 encounters validate, and 73 manifests resolve fully.
- `verify_encounter_schema.py` passes 91/91.
- `population_census.py` reports 1,523 resolved, 127 blocked and 6 not converted.
- Crystal (D30) carries none of these callbacks, so Canary is transcribed.
- The governance and policy validators pass.
