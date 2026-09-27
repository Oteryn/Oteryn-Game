# OTV2-20260927-fifteenth-encounter-slice

```yaml
task_id: OTV2-20260927-fifteenth-encounter-slice
title: D34 boss attribute for Burning Hatred (fifteenth encounter slice)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1043
jira: KAN-16
base_sha: 961c74ab573d87807ed24cbd414a46d8072f72d3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-fifteenth-encounter-slice.md
  - docs/agents/tasks/active/OTV2-20260927-fourteenth-encounter-slice.md
  - docs/agents/tasks/archive/OTV2-20260927-fourteenth-encounter-slice.md
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

This fifteenth encounter slice adds the D34 boss attribute change the owner consented to ("Wszystkie 7"):

- the `attribute` action (outgoing damage percent and defense);
- with it the `item_used` trigger and the timer `add` operation, both needed for the Sorrow of Burning Hatred.

Goshnar's Hatred, its four Burning Hatred forms and the Mighty Splinter of Madness resolve. The census rises from
1,533 to 1,539 fully resolved monsters, compared with the 2026-09-27 wiki. The completed task record of #1037 is
archived. `content/world` and the Item registry are untouched.

Authority: owner answer in this session ("Wszystkie 7" for the D34 vocabulary). Runtime behaviour stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: authoring-format evidence tooling and documentation only.

## Acceptance and evidence

- The samples are regenerated. 82 encounters validate, and 77 manifests resolve fully.
- `verify_encounter_schema.py` passes 114/114.
- `population_census.py` reports 1,539 resolved, 111 blocked and 6 not converted, with the 2026-09-27 wiki samples.
- The wiki (Fandom, Soul War Quest spoiler, read 2026-09-27) decides (D25) that an unremoved Mighty Splinter is absorbed.
- The governance and policy validators pass.
