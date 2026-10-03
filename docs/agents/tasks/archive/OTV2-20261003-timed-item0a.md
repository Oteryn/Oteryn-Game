# OTV2-20261003-timed-item0a

```yaml
task_id: OTV2-20261003-timed-item0a
title: "TIMED-ITEM-0A: bank payment for the NPC soft boots repair"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/timed-item0a-bank-repair
pr: 1633
base_sha: 061e1e4e263b2faf946e7693a51d00932a21c83e
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/agents/tasks/archive/OTV2-20261003-timed-item0a.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on:
  - "BANK-FEE-0 acceptance (docs/agents/tasks/archive/OTV2-20260930-bank-fee0.md)"
  - "GOLD-FEE-2 (bank FEE_DEBIT writer, ledger, schema and audit; gates TIMED-REPAIR-1 bank path)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner answer 2 (#1622 comment 5968564302) admits bank payment for the NPC soft boots repair, as in
Tibia. TIMED-ITEM-0 §7 replaces its coins-only exclusion with the BANK-FEE-0 §3 bank part
(coins first, then the (Account, World) balance; a junior pays with coins only). NPC-0 §6.2 and
the DUR-03 §39.3 `NpcRepair` paragraph follow. The repair rows are unchanged: the worn item takes
one of the 20 main-backpack entries, so at most 19 coin inputs exist, and the bank path mints no
change. No new fee source or sink is added (D178). The bank part is pending on acceptance of
BANK-FEE-0, and TIMED-REPAIR-1 implements it only after GOLD-FEE-2. Allocation: D286 (#1622 comment 5968568302), after #1471 merged.

Edits are limited to the NPC-0 repair paragraph and the DUR-03 `NpcRepair` sentence.

## Architecture and source of truth

- `PROVEN`: owner answer 2; BANK-FEE-0 §3-§5; TIMED-ITEM-0 §7 as merged in #1471.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. TIMED-REPAIR-1 implements and tests the shape.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, economy).
