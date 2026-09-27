# OTV2-20260927-fourteenth-encounter-slice

```yaml
task_id: OTV2-20260927-fourteenth-encounter-slice
title: D34 vocabulary for Grave Danger (fourteenth encounter slice)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: null
jira: KAN-16
base_sha: be0492e53ed39361cbb65565b51255ee14fac06a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-fourteenth-encounter-slice.md
  - docs/agents/tasks/active/OTV2-20260927-target-date-20260927.md
  - docs/agents/tasks/archive/OTV2-20260927-target-date-20260927.md
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

This fourteenth encounter slice adds five of the seven D34 vocabulary additions the owner consented to:

- the `chance_from_amount` condition, for the Ugly Monster that Gaffir and Guard Captain Quaid may call;
- the `move_lock` action, for the turns of Sir Baeloc and Sir Nictros;
- for King Zelos and his four knights:
  - a `damage_modifier` scaled by the time left on a timer (`timer_remaining`);
  - the `shared_life` action;
  - death explosions authored by the encounter (`abilities` and `cast encounter_ability`).

It also states when `damage_accumulated` fires. The census rises from 1,523 to 1,533 fully resolved monsters. The completed task record of #1030 is archived.
`content/world` and the Item registry are untouched.

Authority: owner answer in this session ("Wszystkie 7" for the D34 vocabulary). Runtime behaviour stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: authoring-format evidence tooling and documentation only.

## Acceptance and evidence

- The samples are regenerated. 81 encounters validate, and 75 manifests resolve fully.
- `verify_encounter_schema.py` passes 104/104.
- `population_census.py` reports 1,533 resolved, 117 blocked and 6 not converted, with the 2026-09-27 wiki samples.
- The wiki (Fandom, Grave Danger Quest spoiler and creature pages) decides where Canary differs (D25):
  - the knight ritual of King Zelos;
  - the vampiric blood explosion.
- The governance and policy validators pass.
