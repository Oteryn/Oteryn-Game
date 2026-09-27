# OTV2-20260927-encounter-boss-deaths

```yaml
task_id: OTV2-20260927-encounter-boss-deaths
title: Transcribe Canary boss death events into encounters (Dream Courts, Forgotten Knowledge, Ferumbras Ascension)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: null
jira: KAN-16
base_sha: fcd965b30f96a98de3384ff845d7919dbb6f7a86
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01UiMEDawVAZ3kxLCLepWZnG
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-encounter-boss-deaths.md
  - docs/agents/tasks/archive/OTV2-20260927-encounter-authoring.md
  - docs/agents/tasks/archive/OTV2-20260927-monster-behaviour-patterns.md
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

Continue the owner-decided Encounter transcription (D26-D28) with the largest Canary boss death
events. Each boss becomes its own `instance_per_party` encounter that emits a
`damage_contributors` outcome; quest storages and cooldowns are listed as outcome evidence for the
quest and reward domains. Mechanics outside the v1 vocabulary stay unresolved for the owner.
Authority: direct owner requests in this session; runtime implementation stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- Task records of merged PRs #959 and #960 archived as completed.
- Schema: `emit_outcome.credited`, condition `has_master`, `map_item` `destination`,
  `revert_destination` and `effect`; `verify_encounter_schema.py` 35/35.
- `dreamCourtsDeath`, `ForgottenKnowledgeBossDeath`, `AscendantBossesDeath` transcribed: 24
  encounters valid, 21 manifests fully resolved; unresolved: Alptramun dream escalation, Melting
  Frozen Horror fixed tiles, Ferumbras Mortal Shell crystal reset.
- `population_census.py` 1,389 of 1,656 fully resolved (1,383 before).
