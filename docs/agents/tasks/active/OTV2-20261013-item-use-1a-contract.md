# OTV2-20261013-item-use-1a-contract

```yaml
task_id: OTV2-20261013-item-use-1a-contract
title: "ITEM-USE-1a: DUR-03 item-use audit contract (operation tag 9, rows, encoder)"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/use-item-1-20261013
issue: 1622
pr: 0
depends_on: []
merge: "pending"
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
next_action: "push WIP, self-review, wait for the complete Codex review, one fix commit, archive, squash auto-merge; then ITEM-USE-1b (migration and store) under its own record"
owned_paths:
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - apps/game-server/src/durability/item_use_audit.rs
  - apps/game-server/src/durability/item_mint_audit.rs
  - apps/game-server/src/durability/mod.rs
  - docs/agents/tasks/active/OTV2-20261013-item-use-1a-contract.md
```

## Scope

CP allocation of ITEM-USE-1 (#1622), split by the CP into 1a contract, 1b migration and store,
1c runtime and ingress, sequential. 1a is the contract only: decision ITEM-USE-0 §4. No runtime, SQL,
wire or authority change; registration grants none.

## Outcome

- **Proto and codec.** `OneItemTransactionV1` operation tag 9 `item_use` (`OneItemUseV1`), closed
  `OneItemUseCauseV1 { Food, Potion }` whose identity is the CommandRef, `OneItemUseFlaskV1` and
  the `PARITY_PENDING` `flask_missing` burn. `item_use_audit.rs` encodes and decodes it with the
  shared envelope gate (player command, rule 2 fence); unit conservation, closed shapes and every
  bound are checked before encode.
- **Rows.** `DUR03-RL-01/-02/-04/-06/-07-ITEM-USE*` registered, the byte rows measured on the worst
  case; the ITEMUSE0-RL-01, -02 and -04 rows belong to 1b/1c (runtime and measurement).
- **Contracts.** Foundation registry entry for tag 9; DUR-03 item-use amendment names the tag and rows.

## Validation

pending (filled before freeze)
