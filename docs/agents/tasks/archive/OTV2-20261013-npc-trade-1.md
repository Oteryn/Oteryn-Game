# OTV2-20261013-npc-trade-1

```yaml
task_id: NPC-TRADE-1
title: "NPC-TRADE-1: the NPC BUY/SELL commit as one item-only DUR-03 transaction"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-trade-1-20261013
issue: 1622
pr: 2040
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
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json      # Type 2 note: operation tag 9
  - docs/agents/tasks/archive/OTV2-20261013-npc-trade-1.md
excluded_paths:
  - apps/game-server/src/gameplay_transport/npc_command.rs   # NPC-CMD-1b owns the domain-8 window
public_contracts: [docs/contracts/game-events/v1/native_one_item_transaction.proto]
depends_on: ["NPC-TALK-1", "GOLD-FEE-1b (#1410)"]
external_repositories: []
next_action: "NPC-TRADE-2: wire command 8 in npc_command.rs to this writer after NPC-CMD-1b (#2035) merges, reusing project_offers for the offer index"
```

## Assumptions sent to the control plane

1. Migration `0082` (0081 is taken by PVP-1, #2032).
2. Split (a): this PR is the durable writer, its migration, audit, RL rows and PostgreSQL tests;
   command 8 is wired to it once NPC-CMD-1b supplies the bound catalogue revision.
3. Coins only: the BANK-FEE-0 bank part of NPC-0 §5.1 stays pending its acceptance.

CP answers (2026-10-13): 1. lease 0082 approved; 2. option (a), command-8 wiring is NPC-TRADE-2;
3. coins only.

## Validation

- `cargo fmt --check -p oteryn-game-server`: pass
- `cargo clippy -p oteryn-game-server --all-targets`: pass
- `cargo test -p oteryn-game-server --lib`: pass (2904 passed)
- `cargo test -p oteryn-game-server --test character_authority_postgres npc_trade` (PostgreSQL 17): pass (12 passed)
- `cargo test -p oteryn-game-server --test character_authority_postgres` (PostgreSQL 17.11, 17.6 pins relaxed locally only): 1416 passed, 1 failed on a src-side 17.6 version pin; CI runs 17.6
- `cargo run --locked -p oteryn-architecture-check -- workspace .`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Deviations

- `FeeBurnCause::NpcTrade` is the BUY coin BURN lines under the trade cause record, not a
  `game_item_fee_burns` row (a fee record needs a CharacterRevision advance; a trade has none).
- Charged and fluid offers (§5.3/§5.4, `count` as charges or sub-type) are not traded by this
  writer: no durable fluid sub-type exists and a charged item needs the timed-state proof arms.
  The caller refuses them; the follow-up is raised to the CP (Codex thread on `npc_trade.rs`).
- Over the ~500-line batch guideline: the migration, writer, encoder and tests are one unit.

## PR and closeout

- merge commit/result: squash merge of #2040 (pending)
- review: Codex review on d085fe6b found no issues; independent persistence review requested from the CP
- Codex review on 3b2ec4e8: two P2 findings fixed in one commit. (1) The Type 2 note of
  `GAME_EVENT_FOUNDATION_REGISTRY.json` names operation tag 9 `npc_trade`. (2) A price of 0 is
  a valid trade (NPC-0 §5.3 `F = 0` burns nothing, §5.4 `V = 0` mints nothing; travel route price
  0 is free): the writer, the audit check and 0082 (`unit_price`/`total_gold_units` from 0,
  `line_count` from 1, a BUY without a coin BURN only at total 0) admit it, with unit, audit and
  PostgreSQL cases. No admitted offer has price 0 today (0 of 12,234).
