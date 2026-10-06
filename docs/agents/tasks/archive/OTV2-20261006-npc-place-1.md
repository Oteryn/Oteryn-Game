# OTV2-20261006-npc-place-1

```yaml
task_id: OTV2-20261006-npc-place-1
title: "NPC-PLACE-1: NPC placements as a World Project family and a World Bundle v4 NPC frame"
mode: ARCHITECTURE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: cand/npc-place-1
issue: 162
pr: null
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_018KZRHnq8bTd3FMhoCoh37U (Sol Supervising Architect)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_NPC_PLACE1_NPC_PLACEMENTS_DECISION_2026-10-06.md
  - docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_NPC_PACKETS_2026-10-05.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - docs/agents/tasks/archive/OTV2-20261006-npc-place-1.md
public_contracts: []
depends_on: []
blocks: [NPC-PLACE-1a, NPC-PLACE-1b, NPC-ACTOR-1, NPC-TALK-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Answers the #1622 escalation 6021673251 (NPC-PLACE-1 was never packeted; the bundle has no NPC
  shape) and the placement part of 6021806127 (Doctor Marrow `POSITION_CONFLICT`).
- Shape: a separate World Project family `Npc.Placement` and a separate World Bundle NPC frame;
  the spawn grammar is untouched.
- Format: `OTERYN_WORLD_BUNDLE/v4` with an NPC row and frame, the manifest member `npcs`, limits
  `NPCPLACE1-RL-01..03`; the format contract text is amended by NPC-PLACE-1b with the code.
- Source holds (`NO_PLACEMENT_SOURCE`, `POSITION_CONFLICT`, `SCHEDULE_VARIANT`, `SHARED_CELL`) and
  compiler holds (`UnboundNpc`, the cell reasons, `SpawnPoint`, `SharedCell`); a production build
  stops on any compiler-held placement or travel destination.
- Doctor Marrow is held `POSITION_CONFLICT` once its NPC key is admitted; only a map-owner record
  resolves it.
- Packets NPC-PLACE-1a (content lane) and NPC-PLACE-1b (impl worker, format and CI-routing
  review). Amends NPC-0, NPC-BEHAVIOUR-0, ARCH-NPC-PACKETS-1 and NPC admission §5 as
  pending-on-acceptance notes.
- Owner questions: none.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
