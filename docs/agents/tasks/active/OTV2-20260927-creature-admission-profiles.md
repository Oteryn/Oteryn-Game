# OTV2-20260927-creature-admission-profiles

```yaml
task_id: OTV2-20260927-creature-admission-profiles
title: Typed WorldProject/v2 creature admission profiles and the staging tool (admission slice 2)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 986
jira: KAN-16
base_sha: 12d96e0e6dff227145267deadddb0b085806e293
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-creature-admission-profiles.md
  - docs/agents/tasks/active/OTV2-20260927-creature-admission-design.md
  - docs/agents/tasks/archive/OTV2-20260927-creature-admission-design.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/src/content/project/v2/creature.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_v2_creature_admission.rs
  - tools/content-migration/creature_admission_stage.py
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Slice 2 of `OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md`: WorldProject/v2 gains typed,
declarative, candidate-only authoring profiles for Creature (extended), Behavior, Presentation,
Ability (extended), Effect, Formula and Loot, with structural validation and canonicalization.
The staging tool `creature_admission_stage.py` turns the census bundles into Reference records,
profiles and source identity bindings, closed over their references. No `content/**` change.
Authority: direct owner request in this session ("druga droga"); runtime behaviour stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: declarative content schema and an offline tool; no production mutation, fence,
session, authority or persisted-recovery evidence is touched, and no runtime path reads the profiles.

## Acceptance and evidence

- `content_world_project_v2_creature_admission` tests: a complete dragon round-trips, lowers and
  links; authored sequences keep their order while sets are canonicalized; 12 negative cases fail
  with their specific error; unknown fields are rejected.
- The staging tool admits 1,315 wave A monsters (18,280 records, 17,315 profiles) and defers 101
  (Encounter), 59 (68 unregistered Items), 1 (zero-count loot) and 14 (unresolved references).
  The pilot (25 monsters) is byte-identical across runs.
- `cargo fmt`, `cargo clippy -p oteryn-game-server --all-targets -D warnings` and the full
  `oteryn-game-server` test suite pass; governance, repository policy and semantic audits pass.
