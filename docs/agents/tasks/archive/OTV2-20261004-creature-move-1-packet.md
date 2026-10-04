# OTV2-20261004-creature-move-1-packet

```yaml
task_id: OTV2-20261004-creature-move-1-packet
title: "CREATURE-MOVE-1-PACKET-1: packet creature steps, paths and movement goals"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/creature-move-1-packet-20261004
issue: 162
pr: 1765
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_MOVE1_STEPS_PATHS_AND_GOALS_PACKET_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261004-creature-move-1-packet.md
public_contracts: []
depends_on: [CREATURE-AI-0, CONDITIONS-0, SPEED-1, CREATURE-AI-1-PACKET-1, SPAWN-1A-PACKET-1]
blocks: [CREATURE-MOVE-1, CHASE-1, NPC-ACTOR-1, SUMMON-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- CREATURE-MOVE-1 builds CREATURE-AI-0 §5 and the path part of §7: the creature step timer, the
  path search on the writer inside its window budget, the movement goals, walk back and leash
  (packet §2.1).
- Creature speed is the content `speed`; the spawn speed draw has no Canary source and
  `MONSTER_SPEED_DRAW` is retired, with amendment notes in CONDITIONS-0 and CREATURE-AI-0 (§1.2).
- The near-target ×2 follows Canary's near counter exactly (§1.3).
- Paths use the Movement owner's own step-admission predicate; harmful field cost waits for a field
  owner behind a stub term (§1.4).
- Rows `CREATUREAI0-RL-09`, `-10`, `-11` and `-17`; `RL-17` is measured evidence, not a CI gate
  (§1.6, §1.7).
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
