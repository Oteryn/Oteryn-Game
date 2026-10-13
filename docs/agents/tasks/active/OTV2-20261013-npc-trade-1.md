# OTV2-20261013-npc-trade-1

```yaml
task_id: NPC-TRADE-1
title: "NPC-TRADE-1: the NPC BUY/SELL commit as one item-only DUR-03 transaction"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-trade-1-20261013
issue: 1622
pr: none
head_sha: "the PR head; the control plane records the frozen SHA"
created_at: 2026-10-13
updated_at: 2026-10-13
migration_lease: "0082"   # 0081 is held by open PR #2032 (PVP-1); 0082_npc_trade.sql
decisions:
  - docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md   # §4, §5
resource_limits: [DUR03-RL-01, DUR03-RL-02, DUR03-RL-06, DUR03-RL-07]
owned_paths:
  - apps/game-server/migrations/0082_npc_trade.sql
  - apps/game-server/src/durability/npc_trade.rs
  - apps/game-server/src/durability/npc_trade_audit.rs
  - apps/game-server/src/durability/npc_trade_tests.rs
  - apps/game-server/src/durability/npc_trade_audit_tests.rs
  - apps/game-server/src/durability/item_mint_audit.rs      # operation oneof tag 9
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/tests/character_authority_postgres.rs   # one mod line for the cases
  - apps/game-server/tests/support/npc_trade_postgres_cases.rs
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20261013-npc-trade-1.md
excluded_paths:
  - apps/game-server/src/gameplay_transport/npc_command.rs   # NPC-CMD-1b owns the domain-8 window
public_contracts: [docs/contracts/game-events/v1/native_one_item_transaction.proto]
depends_on: ["NPC-TALK-1", "GOLD-FEE-1b (#1410)"]
external_repositories: []
next_action: "author the durable half; command-8 wiring after NPC-CMD-1b merges (assumption, CP question 2)"
```

## Assumptions sent to the control plane

1. Migration `0082` (0081 is taken by PVP-1, #2032).
2. Split (a): this PR is the durable writer, its migration, audit, RL rows and PostgreSQL tests;
   command 8 is wired to it once NPC-CMD-1b supplies the bound catalogue revision.
3. Coins only: the BANK-FEE-0 bank part of NPC-0 §5.1 stays pending its acceptance.
