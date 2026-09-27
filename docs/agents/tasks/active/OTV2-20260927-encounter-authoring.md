# OTV2-20260927-encounter-authoring

```yaml
task_id: OTV2-20260927-encounter-authoring
title: Encounter authoring format and first Canary encounter (D20, D26-D28)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 960
jira: KAN-16
base_sha: bfcbe853db95aa7f56c0354f7aa986912381164f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01UiMEDawVAZ3kxLCLepWZnG
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-encounter-authoring.md
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

Owner decisions D20 (draft an Encounter format), D26 (encounters instanced per party by default),
D27 (encounters emit outcomes; reward and quest domains own cooldowns, rewards and quest steps) and
D28 (closed v1 vocabulary; start with the Soul War fourth-taint event). The format is a CANDIDATE
architecture document with offline tooling and a first transcribed encounter.
Authority: direct owner requests in this session; runtime implementation stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` (CANDIDATE): shape, triggers, conditions, actions,
  vocabulary rules and owner decisions D26-D28, grounded in the 131 unresolved Canary events.
- `tools/content-schema/encounter-authoring/`: schema, semantic validator,
  `verify_encounter_schema.py` 29/29, Canary transcription with a line-by-line manifest.
- `samples/soul_war_taint_zones/`: 15 participants, valid, manifest fully mapped.
- Monster converter records transcribed events as relocated; `population_census.py` 1,383 of 1,656
  fully resolved (1,377 before).
