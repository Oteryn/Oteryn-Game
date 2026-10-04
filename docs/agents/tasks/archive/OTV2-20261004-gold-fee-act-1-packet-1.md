# OTV2-20261004-gold-fee-act-1-packet-1

```yaml
task_id: OTV2-20261004-gold-fee-act-1-packet-1
title: "GOLD-FEE-ACT-PACKET-1: the event type 2 activation packets (GOLD-FEE-ACT-1, GOLD-FEE-ACT-2)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gold-fee-act-1-packet-1-20261004
issue: 162
pr: null
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_GOLD_FEE_ACT1_TYPE2_ACTIVATION_DECISION_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-gold-fee-act-1-packet-1.md
public_contracts: []
depends_on: []
blocks: [GOLD-FEE-ACT-1, GOLD-FEE-ACT-2]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Control plane D497 (6a): the type-2 activation switch moves out of #1733
  (ARCH-BATCH-ROOT-PACKETS-V1 §1.7 phase 2) into this decision. #1733 keeps phase 1, GOLD-FEE-2.
- Two packets: GOLD-FEE-ACT-1 (readiness: an empty activation table, the fence, the outbox trigger
  and a transaction-scoped activation read in every writer) and GOLD-FEE-ACT-2 (the activation row
  and the registry binding to V2 at revision 2), applied only once every node runs
  GOLD-FEE-ACT-1 code (§1.1, §2).
- #1733 P1 4177113877: activation and every type-2 insert are serialized by one advisory key.
  Writers take it shared as their first statement, the trigger takes it shared for any unfenced
  insert, and activation takes it exclusive, so the activation commit cannot overtake a V1
  transaction (§1.2, §1.3).
- #1733 P1 4177113872: GOLD-FEE-ACT-1 owns all six type-2 writers (`item_mint.rs`,
  `item_transfer.rs`, `reward_claim_mint.rs`, `item_decay_retire.rs`, `item_timed_state.rs`,
  `item_fee_burn.rs`) and `item_mint_audit.rs`. Each takes its tuple from one activation read in
  its transaction, and a coverage test checks every insert site (§1.4, §2.1).
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
