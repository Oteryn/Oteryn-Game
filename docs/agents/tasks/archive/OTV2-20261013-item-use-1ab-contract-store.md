# OTV2-20261013-item-use-1ab-contract-store

```yaml
task_id: OTV2-20261013-item-use-1ab-contract-store
title: "ITEM-USE-1a+1b: DUR-03 item-use audit contract, migration 0083 and durable store"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/use-item-1-20261013
issue: 1622
pr: 2039
depends_on: []
merge: "pending"
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
next_action: "none; ITEM-USE-1c (runtime and ingress) follows under its own record"
owned_paths:
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - apps/game-server/src/durability/item_use_audit.rs
  - apps/game-server/src/durability/item_mint_audit.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/item_use.rs
  - apps/game-server/migrations/0083_item_use.sql
  - apps/game-server/tests/item_use_postgres.rs
  - apps/game-server/tests/support/item_use_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20261013-item-use-1ab-contract-store.md
```

## Scope

CP allocation of ITEM-USE-1 (#1622). The owner combined 1a (contract) and 1b (migration and durable
store) into one PR; 1c (runtime and ingress) follows as its own PR. Decision ITEM-USE-0 §4-§5. No
wire change; the runtime role gains no item write outside a use record.

## Outcome

- **Proto and codec.** `OneItemTransactionV1` operation tag 9 `item_use` (`OneItemUseV1`), closed
  `OneItemUseCauseV1 { Food, Potion }` whose identity is the CommandRef, `OneItemUseFlaskV1` and
  the `PARITY_PENDING` `flask_missing` burn. `item_use_audit.rs` encodes and decodes it with the
  shared envelope gate (player command, rule 2 fence); unit conservation, closed shapes and every
  bound are checked before encode.
- **Rows.** `DUR03-RL-01/-02/-04/-06/-07-ITEM-USE*` registered, the byte rows measured on the worst
  case; the ITEMUSE0-RL-01, -02 and -04 rows belong to 1b/1c (runtime and measurement).
- **Migration 0083 and store.** `game_item_use_records` (one record per CommandRef, forever) with a
  deferred consistency guard; the four shared item guards each re-issued with their 0075 body
  verbatim plus one arm admitting the change only with a record of the same physical transaction.
  `DurabilityRoot::commit_item_use` is the item-only rule 2 writer: replay by CommandRef and
  request binding, plan check with `StaleState`, the four shapes, one type 2 audit event.
- **Contracts.** Foundation registry entry for tag 9; DUR-03 item-use amendment names the tag and rows.

## Codex review (head d11e131)

Three P2 findings, all fixed in the final authoring commit: request revisions are bound to the
fenced root (`StaleState` on mismatch); the decoder uses the registered item-use payload and
envelope ceilings (5,366 / 5,754 B); a replay requires the committed EventId. Each has a test.

## Codex review (head cdfadf3)

Two P2 findings, fixed in the next commit: the item-only use no longer takes or checks an expected
CharacterRevision (composition decision §3; the store takes `CurrentCharacterItemFence`, the
`CharacterRevisionMismatch` error is removed); a replay also requires the committed occurrence
time. Each has a Postgres case.

## Codex review (head e6d7c04)

Two P2 findings. Fixed: the audit decoder rejects a flask entry at the source entry's ordinal (unit
case). Not changed: the ABA `state_revision` fence for prepared stacks. A stack restored to the same
definition, quantity and location yields the same use result, the item state carries no revision and
sibling item transactions do not fence by it; the path is not worth a new request field (decision
taken to the CP). Migration renumbered 0081 to 0083 by CP direction (PVP-RT-1 holds 0081,
NPC-TRADE-1 0082); main merged.

## Validation

### Focused

- `cargo test --lib durability::item_use`: pass
- `cargo clippy --all-targets -- -D warnings`: pass
- `cargo fmt --check`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK

### Component/integration

- `cargo test --test item_use_postgres` (8 item-use cases): pass on PostgreSQL 17.11 locally; the
  protected 17.6 target is CI's.

### E2E

- `NOT_APPLICABLE`: no runtime or ingress in this slice (ITEM-USE-1c).
