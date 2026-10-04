# OTV2-20261004-creature-ai-1-packet

```yaml
task_id: OTV2-20261004-creature-ai-1-packet
title: "CREATURE-AI-1-PACKET-1: packet creature targeting, attacks, defences and the think budget"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-core-loop-packets-3-20261004
issue: 162
pr: 0
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_AI1_TARGETING_AND_THINK_PACKET_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_CORE_LOOP_PACKETS_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-creature-ai-1-packet.md
public_contracts: []
depends_on: [CREATURE-AI-0, ARCH-CORE-LOOP-PACKETS-2, SPAWN-1A-PACKET-1]
blocks: [CREATURE-AI-1, SPAWN-1a, CREATURE-MOVE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- CREATURE-AI-1 builds CREATURE-AI-0 §3, §4 and §7: idle and wake, perception through the VIS-1
  interest predicate, eligibility, target selection and change, attacks and defences as Ability
  proposals, flee, override slots and the think budget (packet §2.1).
- The movement split: steps stay one greedy cardinal step per think until CREATURE-MOVE-1 (§1.1).
- The behaviour profile is one runtime value projected from the v2 behaviour authoring. It is
  required at admission and fed by SPAWN-1a (§1.2).
- The D115 constants leave the think path (§1.3). Wander comes from `movement.wander` (§1.4).
- Melee stays on ATTACK-1's swing, with the melee entry's parameters (§1.5).
- Rows `CREATUREAI0-RL-05`, `-06`, `-07` and `-18`. Every other CREATURE-AI-0 row has a named
  owner packet (§1.7).
- The #1735 §0.3 writer list of the resource-limit registry gains CREATURE-AI-1.
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
