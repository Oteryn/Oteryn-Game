# OTV2-20260927-forgotten-knowledge-fights

```yaml
task_id: OTV2-20260927-forgotten-knowledge-fights
title: Forgotten Knowledge fight encounters (tenth encounter slice)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1012
jira: KAN-16
base_sha: 420bdf0d4a0940e1417eab8b352ed65b62420c28
head_sha: bd74f550bfec8859fae4feee1d8459639c1372ae
final_head_sha: bd74f550bfec8859fae4feee1d8459639c1372ae
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-forgotten-knowledge-fights.md
  - docs/agents/tasks/active/OTV2-20260927-heart-of-destruction-encounters.md
  - docs/agents/tasks/archive/OTV2-20260927-heart-of-destruction-encounters.md
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

This tenth encounter slice continues the complex monster events while the Item import is in progress. It
transcribes the Forgotten Knowledge fight scripts:
- `HealthForgotten` doubles the primary part of every health change of Lady Tenebris and the Thorn Knight forms
  when their guard creature is not within 7 tiles.
- `ThornKnightDeath` turns the mounted knight into the shielded one, then the shielded one into the enraged one.
- `LloydPrepareDeath` and the energy prism scripts make Lloyd survive four times while the prisms are killed one by
  one.

One D31 vocabulary use is documented: in a `heal_received` rule, a `this_hit` damage modifier scales the heal.
The census rises from 1,498 to 1,505 fully resolved monsters. The completed task record of #1011 is archived.
`content/world` and the Item registry are untouched.

Authority: owner direction in this session (D31 consent for vocabulary additions). Runtime behaviour stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: authoring-format evidence tooling and documentation only.

## Acceptance and evidence

- The samples are regenerated. 72 encounters validate, and 66 manifests resolve fully.
- `verify_encounter_schema.py` passes 88/88.
- `population_census.py` reports 1,505 resolved, 145 blocked and 6 not converted.
- Crystal (D30) matches, except for nil guards in the Lloyd script.
- The governance and policy validators pass.

## Completion

Merged as PR #1012 (`2bbeaaeb9857f22199dc585387339f4f658400bc`) from final head `bd74f55`; required checks passed on that head.
Owner released.
