# OTV2-20260927-npc-admission-v2-services

```yaml
task_id: OTV2-20260927-npc-admission-v2-services
title: NPC admission slice 2 - typed WorldProject/v2 service offers and travel routes (D7)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 994
jira: KAN-16
base_sha: 3f29e7d9384b3e21b27032aaf8bdfbc27dae5a09
head_sha: null
final_head_sha: c6a23eccc285a08dcbffd411ed58246894e7d631
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-v2-services.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_v2_npc_admission.rs
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Slice 2 of `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md` (merged in PR #983), owner decision D7:
WorldProject/v2 Service offers gain `count` and `sub_type`, and Services gain typed travel `routes`,
with canonical ordering and structural validation. Declarative and candidate-only; no content change.
Authority: direct owner request in this session ("kontynuuj").

## Architecture and source of truth

- `docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md` §6 (PROVEN by the tests below).
- `docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md`: NPC/Service stay declarative.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: a declarative project-schema extension with no runtime reader; no production
mutation, fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `cargo +1.94.0 test --locked -p oteryn-game-server --test content_world_project_v2_npc_admission`:
  round-trip and canonical rewrite, canonical order of offers and routes, nine rejected invariants,
  unknown route fields rejected; declarations never lower to executable Reference records.
- Existing `content_world_project_v2` literals gain the new optional fields; serialized documents
  without them are unchanged (serde defaults, skipped when empty).
- `cargo fmt --all --check`, `cargo clippy -p oteryn-game-server --all-targets -D warnings` and the full
  `cargo test -p oteryn-game-server` pass.

## Next action

Slice 3: writer and pilot (about 20 NPCs) from `promotion-candidates-v1.json` into v2 documents.
