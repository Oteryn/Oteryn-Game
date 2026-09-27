# OTV2-20260927-encounter-slice-3

```yaml
task_id: OTV2-20260927-encounter-slice-3
title: Transcribe Cults of Tibia, Wrath of the Emperor, Rathleton and other Canary boss events into encounters
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 964
jira: KAN-16
base_sha: b5f4c9641d4fb1db08be36415cbfe4b0f31278dd
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01UiMEDawVAZ3kxLCLepWZnG
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-encounter-slice-3.md
  - docs/agents/tasks/archive/OTV2-20260927-encounter-boss-deaths.md
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

Continue the owner-decided Encounter transcription (D26-D28) with further Canary boss events. Where
a Canary script is broken, the reference-date wiki decides (D25). Mechanics outside the v1
vocabulary stay unresolved for the owner.
Authority: direct owner requests in this session; runtime implementation stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- Task record of merged PR #962 archived as completed.
- `map_item` accepts `at: death_position`; a revert restores the original item and attributes;
  `verify_encounter_schema.py` 39/39.
- Cults of Tibia, Wrath of the Emperor, Ghulosh, Hero of Rathleton, Dangerous Depth and Azerus
  events transcribed: 44 encounters valid, 40 manifests fully resolved.
- `GlowingRubbishAmuletDeath` classified as quest progress (D6).
- `population_census.py` 1,427 of 1,656 fully resolved (1,389 before).
