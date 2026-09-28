# OTV2-20260928-npc-dialogue-transcripts

```yaml
task_id: OTV2-20260928-npc-dialogue-transcripts
title: NPC dialogue D10 - break Canary/Crystal dialogue conflicts with Tibia Global transcripts
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1095
jira: KAN-16
base_sha: 737a2dbb7c0ef393988dbc24cf45f770afa1f330
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-npc-dialogue-transcripts.md
  - docs/agents/tasks/active/OTV2-20260928-npc-dialogue-wave-a.md
  - docs/agents/tasks/archive/OTV2-20260928-npc-dialogue-wave-a.md
  - docs/agents/evidence/OTV2-20260928-npc-dialogue-wave-a-staged.json
  - docs/agents/evidence/OTV2-20260927-npc-admission-wave-a-staged.json
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - tools/content-migration/npc_dialogue_stage.py
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/**
  - content/manifest.json
  - content/content.lock.json
  - content/project.json
  - content/npcs/**
  - content/dialogues/**
  - content/services/**
  - content/cosmetics/mounts/index.json
  - imports/tibiawiki/**
public_contracts: []
depends_on:
  - OTV2-20260928-npc-dialogue-wave-a
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Decision D10 settles most dialogue conflicts held in wave A. When Canary and Crystal stage different
dialogues for an NPC, the Tibia Global in-game transcripts of `s2ward/tibia` (commit `8824eb38`) break the
tie. The source whose differing texts match more transcript lines, and at least one, is admitted whole. A
transcript never supplies text itself. Every decision records its scores and transcript, and the digest of
all transcripts used is stored in the staged evidence.

84 conflicts are resolved (75 Canary, 9 Crystal), so 701 NPCs now have dialogue (6,313 keyword nodes).
45 conflicts stay held: 44 whose transcript matches neither side better, and one without a transcript.
Captain Dreadnought stays held as `TEXT_BUNDLE_UNVERIFIED`.

Authority: owner request in this session to search the wiki and the internet so NPCs reproduce Tibia Global
as closely as possible.

## Acceptance and evidence

- Without `--transcripts`, the stage reproduces the previous wave A evidence byte for byte. With it, re-runs
  are byte-identical.
- `materialize_content_world_project_v2` loads and verifies all 2,157 NPC-side declarations, including 701
  Dialogues, against the pinned evidence.
- `content_world_project_repository` pins the new documents and the tree digest.
- The tree generator, its validator and tests, `validate_materialized_game_tree`, governance and the
  repository policy all pass.
