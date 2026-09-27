# OTV2-20260927-encounter-d29

```yaml
task_id: OTV2-20260927-encounter-d29
title: Encounter vocabulary extensions (D29) and the Urmahlullu, Alptramun and Splinter of Madness transcriptions
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 967
jira: KAN-16
base_sha: 76c2da68bde1928ab35e4e0f7828c675133cd297
head_sha: 6a121d78c9a38be596afd3386f5c70d284709dbe
final_head_sha: 6a121d78c9a38be596afd3386f5c70d284709dbe
final_head_frozen_at: null
owner: released
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-encounter-d29.md
  - docs/agents/tasks/archive/OTV2-20260927-monster-callbacks.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - tools/content-schema/encounter-authoring/**
  - tools/content-schema/monster-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision D29 (accepted 2026-09-27): the proposed Encounter vocabulary extensions 1-8 are
added; acting on whatever stands on a fixed tile is not added, scripted movement is deferred, and
state shared by all parties stays with the quest domain and is read through `world_state`.
Authority: direct owner decision in this session; runtime implementation stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- Task record of merged PR #966 archived as completed.
- Schema and validator: `creature_spawned`, `ability_cast`, `world_state`, fractional or absolute
  health thresholds, `creature_present` near a role, `{min, max}` ranges, `damage`,
  `component`, `map_item.interaction`; `verify_encounter_schema.py` 63/63.
- Urmahlullu, the Alptramun dream counter and the Splinters of Madness transcribed; inline callbacks
  covered by an encounter are relocated by the monster converter. 52 encounters valid, 46
  manifests fully resolved.
- `population_census.py` 1,463 of 1,656 fully resolved (1,453 before).

## Completion

Merged as PR #967 (`6f3da2f4079d2f668b8aa7aec5199857422cde6e`) from final head `6a121d7`; required checks passed on that head.
Owner released.
