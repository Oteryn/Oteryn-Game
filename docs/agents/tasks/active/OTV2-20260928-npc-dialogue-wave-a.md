# OTV2-20260928-npc-dialogue-wave-a

```yaml
task_id: OTV2-20260928-npc-dialogue-wave-a
title: NPC dialogue wave A - admit static Canary/Crystal NPC dialogue into WorldProject/v2 and the content tree
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1078
jira: KAN-16
base_sha: 3b41c0f4c0b3d4480a392d1c0ea4c4b40f7c16ea
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-npc-dialogue-wave-a.md
  - docs/agents/tasks/active/OTV2-20260928-npc-dialogue-schema.md
  - docs/agents/tasks/archive/OTV2-20260928-npc-dialogue-schema.md
  - imports/tibiawiki/**
  - content/npcs/**
  - content/services/**
  - content/cosmetics/mounts/index.json
  - docs/agents/evidence/OTV2-20260928-npc-dialogue-wave-a-staged.json
  - docs/agents/evidence/OTV2-20260927-npc-admission-wave-a-staged.json
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - tools/content-migration/npc_dialogue_stage.py
  - tools/content-migration/npc_admission_stage.py
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - tools/content-schema/validate_materialized_game_tree.py
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/**
  - content/manifest.json
  - content/content.lock.json
  - content/project.json
  - content/dialogues/**
public_contracts: []
depends_on:
  - OTV2-20260928-npc-dialogue-schema
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Wave A NPCs gain their static dialogue. `npc_dialogue_stage.py` reads the Canary `47dfd51f` and Crystal
`ff7ede59` NPC text bundles and keeps only static text:

- greet, farewell, walk-away and send-trade messages, as lists of parts;
- ambient voices with their cadence and say/yell mode;
- `say` keyword replies with no condition and no effect, in source sibling order, with their conversation
  flags and fallback nodes.

A dialogue is taken when both sources agree, or when only one source has the NPC. 137 NPCs whose sources
disagree are held with `DIALOGUE_CONFLICT`.

`npc_admission_stage.py --dialogues` links 610 Dialogue declarations (4,511 keyword nodes) to their NPCs.
The deferred Dragon Ancestor Spirit keeps none. The materializer pins both staged files, and the successor
tree gains `content/dialogues/definitions`. Tibia NPC text is used as reference data under D9 and the
repository's asset policy (#1050).

Authority: owner request in this session ("kontynuuj 1,2,3 i 4", points 2 and 4).

## Acceptance and evidence

- Both stage tools are deterministic: re-running them is byte-identical. Without `--dialogues`, the
  admission stage reproduces the previous wave A file byte for byte.
- `materialize_content_world_project_v2` loads and validates all 2,066 NPC-side declarations, including
  610 Dialogues.
- `content_world_project_repository` pins the new documents and the tree digest.
- The tree generator, its validator and tests, `validate_materialized_game_tree`, governance and the
  repository policy all pass.
