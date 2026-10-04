# OTV2-20261004-arch-root-packets-1

```yaml
task_id: OTV2-20261004-arch-root-packets-1
title: "ARCH-ROOT-PACKETS-1: root packets for MAP-LOAD-1, ITEM-USE-WIRE-1, BANK-RET-0, BANK-1 and GOLD-FEE-2; NPC-BEHAVIOUR-0 acceptance; #513 disposition"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-root-packets-20261004
issue: 162
pr: 1733
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ROOT_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-root-packets-1.md
public_contracts: []
depends_on: []
blocks: [MAP-LOAD-1, ITEM-USE-WIRE-1, BANK-RET-0, BANK-1, GOLD-FEE-2, NPC-VIS-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Packets: MAP-LOAD-1 (hard, security review of the reader), ITEM-USE-WIRE-1 (impl, protocol
  review), BANK-RET-0 (control plane, privacy review), BANK-1 (hard, persistence review) and
  GOLD-FEE-2 (hard, persistence review), in that dependency order (§0.2, §2).
- Proposed leases: capability 15 `ITEM_USE_V1`; migrations 0068 (BANK-1) and 0070 (GOLD-FEE-2);
  event type 3 `BANK_OPERATION`; profile `ECONOMY_LEDGER_RETENTION_V1` (§0.1).
- Owner answers 2026-10-04: BANK-0 Q1 = b, recorded in BANK-0; batch scope 2a (§1.1).
- Bundle staging: CI-built artifact pinned by digest, the server refuses any other (§1.2).
- NPC-BEHAVIOUR-0 accepted when this merges, with its Amends line (§1.5).
- #513: limits still correct; recommend closing it and moving `DUR03-RL-08` to stage C (§1.6).
- Next wave (MAP-OVERLAY-1, ITEM-USE-1, NPC-ACTOR-1) goes in the next batch.
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
