# OTV2-20261001-npc-behaviour0

```yaml
task_id: OTV2-20261001-npc-behaviour0
title: "NPC-BEHAVIOUR-0 NPC presence, walking, voices and focus"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-npc-behaviour-0
pr: "assigned at PR creation; recorded in the #162 FREEZE_SHA entry"
base_sha: d3cfb246
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261001-npc-behaviour0.md
  - docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

NPC-BEHAVIOUR-0 decides how NPCs exist and behave between conversations, item 3 of the architect's
base-mechanics close-out plan (#162 5929069698).

- **Presence:** one runtime actor per admitted placement on every channel, nothing durable; not a
  combat target, not pushable; entity kind 5 `Npc` in capability 6 before it is offered.
- **Think:** 1,000 ms, only while a player perceives the NPC, with its own budget row.
- **Walking:** Canary's random step inside the walk square, never while talking, never onto floor
  changes or teleports, staying on its side of protection-zone boundaries.
- **Voices:** admitted lines as local speech at the content cadence and chance.
- **Focus:** a customer queue owned by GAME-NPC-SERVICE; the NPC faces its newest customer; a range
  check on every committed move ends a conversation beyond talk range with the walk-away line.
- **Owner questions:** none; R1-R3 are architect rulings.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: the NPC service boundary §3; NPC-0 §3.2, §4; the NPC authoring schema §2, D9, D11;
  MOVE-RL-11 §4.2; CREATURE-AI-0 §4.1, §5.1, §7; CONDITIONS-0 §4.2; CHAT-0 §3; WORLD-INTERACTION-0
  §7.1; `crates/protocol-oteryn/src/world_spatial_entities.rs`; the protocol registry.
- `DERIVED`: Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b` (`OTS_HYPOTHESIS_ONLY`; `npc.cpp`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. NPC-VIS-1 needs protocol review; NPC-ACTOR-1 movement and determinism
review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (protocol, movement, determinism).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code and content; NPC schedules, instances, sounds, scripted behaviour.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.
- `git diff --cached --check`: clean.

## Closeout

- PR: recorded in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Amendments, each pending on acceptance of NPC-BEHAVIOUR-0: MOVE-RL-11 §4.2, §4.3; CREATURE-AI-0
  §4.1, §7; CHAT-0 §3; NPC-0 §4.1.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-npc-behaviour-0
owner_action_required: null
blocker: null
next_action: null
```
