# OTV2-20260928-encounter-admission-design

```yaml
task_id: OTV2-20260928-encounter-admission-design
title: Encounter admission into WorldProject/v2 (decision E1-E5) and anchor locations
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1136
jira: KAN-16
base_sha: 1a2d1bd0da0ce87ea7d172fdaef97a0a651bfb7a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-encounter-admission-design.md
  - docs/agents/tasks/active/OTV2-20260928-vortex-conditions.md
  - docs/agents/tasks/archive/OTV2-20260928-vortex-conditions.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - tools/content-schema/encounter-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision ("tak", 2026-09-28) to E1-E5 of `OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md`: encounters enter
WorldProject/v2 as typed declarative profiles together with their creatures. The creatures are guarded by their
encounter bindings, and admission is closed over references and anchor locations.

This change is slices 1 and 2: the decision, and the anchor locations of E2 in the encounter authoring format. A point
is `{x, y, floor}` and an area is a list of boxes of whole tiles, each on one floor. 141 of the 142 anchors are located; the Soul War taint
zones stay unlocated. It also archives the task record of #1127.

## Acceptance and evidence

- `verify_encounter_schema.py` 148/148; 83 encounters validate and 78 manifests resolve fully.
- No manifest changes, so the census, the staging and `content/world` are unchanged.
- `validate_governance.py` passes.
