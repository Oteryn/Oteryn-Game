# OTV2-20260927-npc-admission-wiki-completion

```yaml
task_id: OTV2-20260927-npc-admission-wiki-completion
title: NPC admission D8 - complete held NPCs from TibiaWiki (wiki positions and variant base names)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1036
jira: KAN-16
base_sha: 63de6f9ab0c4f22abad1c81c996d525bdc00c1dc
head_sha: null
final_head_sha: 45b6cc73d8dbd2988ec0153cfa5ad03318367d51
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-wiki-completion.md
  - docs/agents/tasks/active/OTV2-20260927-npc-admission-wave-a.md
  - docs/agents/tasks/archive/OTV2-20260927-npc-admission-wave-a.md
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - docs/agents/evidence/OTV2-20260927-npc-admission-wave-a-staged.json
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-schema/npc-authoring/**
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

Owner request (2026-09-27): "masz dostep do internetu to uzupelnij te npc", which asks to use the wiki to
complete the held NPCs. This is owner decision D8, which adds two rules to the promotion candidates:

- An NPC that neither source places is promoted when its TibiaWiki page has a position. That position
  becomes its candidate placement, and placements still wait for a World.
- A single-source NPC whose name is a Day/Night or stage variant is wiki-confirmed through the page of its
  base name.

Variants that share one page keep only their Canary or Crystal source binding, because v2 binds a page to a
single NPC. Custom server NPCs, quest-only characters and names the wiki knows only under a corrected
spelling stay held.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this is a declarative content admission. `content/world/**` changes take one independent
exact-head review under the standing authorization of `OWNER_FUNDED_AI_POLICY.md`.

## Acceptance and evidence

- The regenerated candidates pass `validate_promotion.py` and the promotion tests.
- The staged wave and the materializer admit the added NPCs.
- `content_world_project_repository` passes, and so do the tree validators.
- The full `oteryn-game-server` tests pass, along with `fmt` and `clippy -D warnings`.
