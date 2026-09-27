# OTV2-20260927-npc-admission-pilot

```yaml
task_id: OTV2-20260927-npc-admission-pilot
title: NPC admission slice 3 - staging writer and 20-NPC pilot into WorldProject/v2
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: null
jira: KAN-16
base_sha: b48d61c687b11110de6df5c60fb5e3c166f117c6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-pilot.md
  - docs/agents/tasks/active/OTV2-20260927-npc-promotion-candidates.md
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-v2-services.md
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-presentation-behavior.md
  - docs/agents/tasks/archive/OTV2-20260927-npc-promotion-candidates.md
  - docs/agents/tasks/archive/OTV2-20260927-npc-admission-v2-services.md
  - docs/agents/tasks/archive/OTV2-20260927-npc-admission-presentation-behavior.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - docs/agents/evidence/OTV2-20260927-npc-admission-pilot-staged.json
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
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
  - imports/tibiawiki/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Slice 3 of `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md`. `tools/content-migration/npc_admission_stage.py`
stages the NPC promotion candidates as v2 records. The first 20 NPCs enter the protected WorldProject/v2
project through the materializer, which fully validates them in Rust:

- 20 NPC declarations;
- 40 Generic Presentation/Behavior records with their profiles;
- 4 trade Services with 219 offers (a non-gold currency, count and sub-type rows);
- 2 travel Services with 6 routes (including a premium or minimum-level route);
- 44 source identity bindings (Canary, Crystal, and TibiaWiki pages where the wiki decided a fact or
  confirmed a single-source NPC).

The pilot covers every shape the admission must prove: travel, a non-gold currency, count and sub-type
offers, a Crystal-only NPC, wiki-decided placements, travel and trade, an item look, a mount and a
standing NPC. The successor tree's Presentation and Behavior shards carry the 20 new profiles.

Authority: owner request in this session ("kontynuuj"; D1-D7). Nothing runs: no placement, dialogue,
trade settlement or travel execution is admitted.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: declarative content admission; no production mutation, fence, session, authority or
persisted-recovery evidence is touched. `content/world/**` changes take one independent exact-head
review under the standing authorization of `OWNER_FUNDED_AI_POLICY.md`.

## Acceptance and evidence

- The staging tool is pure and byte-repeatable. It also stages the full wave (983 NPCs, 10,723 offers and
  189 routes, every Item registered); one NPC with no walk configuration in either source is deferred
  (`MOVEMENT_UNDECLARED`).
- The materializer pins the staged pilot and the tool by SHA-256 and reproduces `content/world/**`.
- `content_world_project_repository` captures, rewrites and links the project with the pilot.
- `validate_world_project_v2_to_tree.py` and `validate_materialized_game_tree.py` pass.
- Full `oteryn-game-server` tests, `fmt` and `clippy -D warnings` pass.

## Excluded scope

Wave A in bulk and the NPC and Service families of the successor tree (`content/npcs`,
`content/services`) are slice 4. Placements wait for a World; dialogue waits for Oteryn-authored text;
travel discounts, gated rows and held NPCs stay in the candidates.
