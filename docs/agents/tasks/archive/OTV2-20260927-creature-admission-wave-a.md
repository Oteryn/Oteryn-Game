# OTV2-20260927-creature-admission-wave-a

```yaml
task_id: OTV2-20260927-creature-admission-wave-a
title: Admit Canary wave A monsters (1,315) into WorldProject/v2 and the content tree (admission slice 3)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 990
jira: KAN-16
base_sha: 017b69404070f5f4547e9102cbd98985e9aa1082
head_sha: b9359874f07425c63a7435a48480330955c67be9
final_head_sha: b9359874f07425c63a7435a48480330955c67be9
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-creature-admission-wave-a.md
  - docs/agents/tasks/active/OTV2-20260927-creature-admission-profiles.md
  - docs/agents/tasks/archive/OTV2-20260927-creature-admission-profiles.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/src/content/project/v2/creature.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - apps/game-server/tests/content_world_project_v2_creature_admission.rs
  - tools/content-migration/**
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
  - imports/canary/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Slice 3 of `OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md`: the 1,315 wave A Canary monsters
enter the protected WorldProject/v2 project as linked Reference records (Creature, Presentation,
Behavior, Loot, Ability, Effect, Formula) with their declarative candidate-only profiles, a
`oteryn:source.canary` import batch and one `canary/monster-file` source identity binding per
creature. The successor content tree gains the seven creature families and `imports/canary/`.
Nothing spawns: no placement, spawn, asset or runtime behaviour is admitted.
Authority: direct owner request in this session ("druga droga"); runtime behaviour stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: declarative content admission; no production mutation, fence, session, authority or
persisted-recovery evidence is touched. `content/world/**` changes take one independent exact-head
review under the standing authorization of `OWNER_FUNDED_AI_POLICY.md`.

## Acceptance and evidence

- The materializer pins the staged evidence and the staging tool by SHA-256 and reproduces
  `content/world/**` byte for byte. Two runs give the same tree digest.
- `content_world_project_repository` captures, rewrites and links the project:
  - 38,157 Items plus 18,280 creature records;
  - 1,315 Creature definitions;
  - 17,315 profiles;
  - 1,315 Canary bindings.
- The content tree round-trips each creature family and its profiles and bindings
  (`validate_world_project_v2_to_tree.py`). The Item and Mount shards are unchanged.
- The v2 presentation fix: attachment and visual effect slots accept several bindings (the two
  addons of an outfit), unique by slot and binding. Palette slots stay one per slot. This is tested.
- `cargo fmt`, `clippy -D warnings`, the full `oteryn-game-server` suite and the governance, policy
  and semantic validators pass.

## Completion

Merged as PR #990 (`b0241e5c4a8a5e473a2592255a31ce4c047d220c`) from final head `b935987`; required checks passed on that head.
The Codex exact-head review of `123a0ed` raised one P2 finding (the `imports/canary/` marker), which was fixed in `9e41c65`.
The merge of `main` then applied the R7 P04 gold coin rekey, and CI passed on `b935987`.
Owner released.
