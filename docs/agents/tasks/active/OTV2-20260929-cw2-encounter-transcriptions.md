# OTV2-20260929-cw2-encounter-transcriptions

```yaml
task_id: OTV2-20260929-cw2-encounter-transcriptions
title: Transcribe, type and restage the four section 12 encounters (Alptramun, Gorzindel, Melting Frozen Horror, The Sandking)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/friendly-albattani-rfzvku
pr: null
base_sha: 4ea220fa
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-29T12:00:00Z
updated_at: 2026-09-29T12:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/encounter-authoring/canary_encounters.py
  - tools/content-schema/encounter-authoring/samples/{alptramun,gorzindel,melting_frozen_horror,the_sandking}/**
  - tools/content-schema/monster-authoring/samples/population-*.json (census outputs)
  - apps/game-server/src/content/project/v2/encounter.rs
  - apps/game-server/tests/content_world_project_v2_encounter_admission.rs
  - docs/agents/tasks/archive/OTV2-20260929-cw2-encounter-transcriptions.md
public_contracts: []
depends_on:
  - "OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md section 12 (ACCEPTED), step 1 done in OTV2-20260929-cw2-encounter-vocabulary-impl"
  - "owner answers on #162 batch 2 (Q4, Q5, Q6, Q8 = section 12.6 Q1-Q4)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Section 12.5 steps 2 and 3 (step 4, the E4 restage, follows in its own PR):

- step 2, `canary_encounters.py`: Alptramun (Q1: four tier-replacement rules; the counter and the never-cast summon spell
  are `approved_omission`), Gorzindel (CW2-1 portal, Q2 per-instance rooms), Melting Frozen Horror (Q4: two death rules,
  dragon egg and solid horror roles, `MeltingDeath` covered), The Sandking (CW2-2..4, Q3: the vanish-and-brood cycle
  and the stage counter). 83 samples validate, 82 manifests resolve (was 78); `verify_encounter_schema.py` 172/172.
- census (`population_census.py`, Canary `47dfd51f`, Crystal `00ce02a5`): fully resolved 1557 -> 1560, blocked
  93 -> 90 (Melting Frozen Horror, Stolen Tome of Portals, The Sandking).
- step 3, Rust: the typed v2 Encounter profile mirrors CW2-1..4 with positive and negative tests.

## Validation

Recorded at freeze.
