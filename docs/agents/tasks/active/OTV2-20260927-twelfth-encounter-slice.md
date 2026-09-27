# OTV2-20260927-twelfth-encounter-slice

```yaml
task_id: OTV2-20260927-twelfth-encounter-slice
title: Heart of Destruction minion forms and servant replicas (twelfth encounter slice)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1022
jira: KAN-16
base_sha: e422fc9d2962f63ccb6f0af9ee80ec5f449c2a31
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-twelfth-encounter-slice.md
  - docs/agents/tasks/active/OTV2-20260927-eleventh-encounter-slice.md
  - docs/agents/tasks/archive/OTV2-20260927-eleventh-encounter-slice.md
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

This twelfth encounter slice continues the complex monster events while the Item import is in progress. It transcribes:
- the Heart of Destruction minion forms: `DisruptionTransform`, `ChargedDisruptionTransform`, `CracklerTransform` and `DepolarizedTransform`;
- `ReplicaServantDeath`, the Forgotten Knowledge servant replicas.

No vocabulary is added. The census rises from 1,511 to 1,517 fully resolved monsters. The completed task record of #1016 is
archived. `content/world` and the Item registry are untouched.

Authority: owner direction in this session ("kontynuuj dalej to co możesz"). Runtime behaviour stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: authoring-format evidence tooling and documentation only.

## Acceptance and evidence

- The samples are regenerated. 78 encounters validate, and 72 manifests resolve fully.
- `verify_encounter_schema.py` passes 91/91.
- `population_census.py` reports 1,517 resolved, 133 blocked and 6 not converted.
- Crystal (D30) carries identical scripts.
- The governance and policy validators pass.
