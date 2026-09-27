# OTV2-20260927-npc-admission-wave-a

```yaml
task_id: OTV2-20260927-npc-admission-wave-a
title: NPC admission slice 4 - wave A (983 NPCs) into WorldProject/v2 and the NPC and Service tree families
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1024
jira: KAN-16
base_sha: e2e2038b0f1020751f87b75df174cdfadc9b430c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-wave-a.md
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-pilot.md
  - docs/agents/tasks/archive/OTV2-20260927-npc-admission-pilot.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - docs/agents/evidence/OTV2-20260927-npc-admission-wave-a-staged.json
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-migration/**
  - tools/content-schema/validate_materialized_game_tree.py
  - content/world/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - content/items/index.json
  - content/cosmetics/mounts/index.json
  - content/creatures/definitions/**
  - content/presentations/definitions/**
  - content/behaviors/**
  - content/loot/**
  - content/abilities/**
  - content/npcs/**
  - content/services/**
  - imports/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task is slice 4 of `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md`. The staging tool from slice 3 stages the
full wave A, and the materializer pins it, replacing the pilot. The protected WorldProject/v2 project then
admits:

- 983 NPC declarations;
- 1,966 Generic Presentation/Behavior records with their profiles;
- 289 trade Services with 10,723 offers;
- 53 travel Services with 189 routes;
- 2,035 source identity bindings.

The successor content tree gains the `content/npcs/definitions` and `content/services/{trade,travel}`
families.

One candidate is deferred: Dragon Ancestor Spirit (`MOVEMENT_UNDECLARED`). Neither source declares a walk
configuration for it, and the engine default is not treated as evidence.

Authority: the owner chose option 1 in this session ("opcja 1", slice 4). Nothing runs: no placement,
dialogue, trade settlement or travel execution is admitted.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: declarative content admission; no production mutation, fence, session, authority or
persisted-recovery evidence is touched. `content/world/**` changes take one independent exact-head
review under the standing authorization of `OWNER_FUNDED_AI_POLICY.md`.

## Acceptance and evidence

- The materializer pins the staged wave A and the tool by SHA-256 and reproduces `content/world/**`.
- `content_world_project_repository` captures, rewrites and links the project with wave A.
- The tree validators round-trip the NPC and Service declarations and the NPC bindings.
- The full `oteryn-game-server` test suite passes, as do `fmt` and `clippy -D warnings`.

## Excluded scope

- Placements wait for a World, and dialogue waits for Oteryn-authored text.
- Travel discounts, gated rows, held NPCs and the deferred NPC stay in the candidates.
- The bank, blessings, crafting and tasks service markers stay unpopulated.
