# OTV2-20260928-encounter-admission-rust

```yaml
task_id: OTV2-20260928-encounter-admission-rust
title: Typed WorldProject/v2 Encounter profile (encounter admission slice 3)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1142
jira: KAN-16
base_sha: 812930a7c338aa079650b7db6ea0ec25bae3106d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-encounter-admission-rust.md
  - docs/agents/tasks/active/OTV2-20260928-encounter-admission-design.md
  - docs/agents/tasks/archive/OTV2-20260928-encounter-admission-design.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/src/content/project/v2/creature.rs
  - apps/game-server/src/content/project/v2/encounter.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_v2_creature_admission.rs
  - apps/game-server/tests/content_world_project_v2_encounter_admission.rs
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Slice 3 of `OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md` (owner decision E1-E5, 2026-09-28):

- `ProjectV2EncounterAuthoring.details`: the typed encounter authoring vocabulary v1 (E1). It is candidate-only and structurally validated.
- `ProjectV2CreatureAuthoring.encounters`: a covered creature's encounters (E3).
- `ProjectV2AbilityDetails.encounter`: the D45 summon spells.

There are no content changes. It also archives the task record of #1136.

## Acceptance and evidence

- `cargo fmt --check`, `cargo clippy --all-targets -D warnings` and `cargo test --locked -p oteryn-game-server` pass.
- The encounter admission tests cover a round trip, canonical sets and authored order, 25 broken invariants, and unknown fields and kinds failing closed.
- Exact-head review before the Merge Queue (protected v2 contract surface).
