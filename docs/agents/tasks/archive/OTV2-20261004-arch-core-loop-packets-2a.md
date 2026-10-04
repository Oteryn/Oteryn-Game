# OTV2-20261004-arch-core-loop-packets-2a

```yaml
task_id: OTV2-20261004-arch-core-loop-packets-2a
title: "ARCH-CORE-LOOP-PACKETS-2 part A: ATTACK-1 and CHAT-1b-2 splits, ATTACK-WIRE-1 lease, spawn rulings"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-core-loop-packets-20261004
issue: 162
pr: "the one named in the #162 FREEZE_SHA entry"
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_CORE_LOOP_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-core-loop-packets-2a.md
public_contracts: []
depends_on: []
blocks: [ATTACK-WIRE-1, ATTACK-1a, ATTACK-1b, CHAT-1b-2a, CHAT-1b-2b, SPAWN-CONTENT-1, SPAWN-1a]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **D486 item 1:** ATTACK-1 splits into a pure ATTACK-1a (`combat/attack/**`) and the wiring
  ATTACK-1b. CHAT-1b-2 splits into a pure CHAT-1b-2a (`chat/**`) and the wiring CHAT-1b-2b, which
  reuses CAP-NEG-1 instead of CHARM-5-COMP. ATTACK-WIRE-1 holds capability 17 `ATTACK_V1`
  (requires 6), command types 11 and 12 and domain 10 (§0.1, §1.1, §1.2).
- **D486 item 2:** SPAWN-CONTENT-1 needs MAP-BUNDLE-1 only, because spawns are a base bundle
  family. SPAWN-1 splits into SPAWN-1a, the D116 fixture spawn before MAP-LOAD-1, and SPAWN-1b, the
  bundle family and its measurements (§1.3, §1.4).
- Packets: ATTACK-WIRE-1, ATTACK-1a, ATTACK-1b, CHAT-1b-2a, CHAT-1b-2b, SPAWN-CONTENT-1 and SPAWN-1a
  (§2). Shared-file order (§0.3).
- Brief amendments in ATTACK-0, CHAT-0 and CREATURE-AI-0.
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
