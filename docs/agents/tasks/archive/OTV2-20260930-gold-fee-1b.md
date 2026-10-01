# OTV2-20260930-gold-fee-1b

```yaml
task_id: OTV2-20260930-gold-fee-1b
title: GOLD-FEE-1b - platinum and crystal fee inputs and the change MINT (migration 0031)
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gold-fee-1b
issue: 162
allocation: "#162 5916023254 (lease 0031)"
pr: null   # set on the PR; a commit cannot hold its own PR number
base_sha: a6a054e3   # main at authoring
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
owner: "GOLD-FEE-1b hard worker (claude-code-session-01KHBMhdyXg4ZtugYkB2FucZ)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
size_exception: "about 1,490 added lines (about 150 of them the verbatim 0012/0013 guard bodies 0031 must restate), one atomic unit (the 0031 guards, the writer, the audit shape and their PostgreSQL cases are only sound together); same exception as GOLD-FEE-1a"
owned_paths:
  - apps/game-server/migrations/0031_character_gold_fee_change_mint.sql
  - apps/game-server/src/durability/item_fee_burn.rs
  - apps/game-server/src/durability/item_fee_burn_audit.rs
  - apps/game-server/src/domain/currency.rs
  - apps/game-server/tests/support/item_fee_burn_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json   # the two RL-07 fee byte rows only
  - docs/contracts/game-events/v1/native_one_item_transaction.proto   # additive fields 11-12
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json   # event type 2 note
  - docs/agents/tasks/archive/OTV2-20260930-gold-fee-1b.md
public_contracts: [DUR-03, GAME-ITEM-01]
depends_on: ["#1318 (GOLD-FEE-1a, migration 0023)"]
blocks: [CHARM-6, GOLD-FEE-2, NPC-TRADE-1]
cross_repository_coordination_id: null
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

`burn_fee_in_transaction` now plans over gold, platinum and crystal stacks of the equipped main
backpack (decision §4.2: worth ascending, then display order) and mints the change back in the
same physical transaction: `change / 100` platinum and `change % 100` gold, each a fresh item in a
new direct entry after the burn lines (the highest ordinal before the burn plus one, then the
next), under the two output slots the source fixes with the TransactionId. The change must fit
the backpack's declared capacity after the burn (`ChangeDoesNotFit` otherwise, nothing written).
Occurrence replay returns the change with the rest of the first outcome.

Migration 0031 replaces the fee consistency guard (every 0023 arm kept; plan order and the
untouched-stack check generalised to worth ascending then display order; the change must be below
the last line's coin worth; the change MINT must be exactly the planned outputs in their entries)
and adds one admitting clause each to the item MINT guard (0013 body) and the backpack placement
guard (0012 body). The fee event gains the additive fields 11-12; the RL-07 fee rows are
re-measured (payload 25,398 B, envelope 25,712 B) and the stored envelope CHECK follows.

## Interpretations (for the independent review)

- **Change placement.** "Placed after the burn lines, each taking the next ordinal" is read as the
  highest ordinal the backpack held before the burn plus one, so a change entry never reuses the
  ordinal a burn line of the same transaction names. The guard recomputes it from the surviving
  entries and the lines.
- **Uniqueness.** Both change items carry the fee TransactionId as `minted_transaction_id` and
  `placed_transaction_id`. 0031 therefore makes items unique per (minted TransactionId, key) and
  drops the unique placed TransactionId of backpack entries. Every other MINT and placement path
  stays bound one-to-one to its own receipt (unique TransactionId and ItemInstanceId) by the
  unchanged guard arms, and a later MINT reusing a committed fee's TransactionId is refused
  (PostgreSQL case).
- **Capacity.** The source supplies the backpack's current Content facts
  (`FeeChangeFacts.backpack`), as the reward-claim MINT does; the writer refuses facts that do not
  match the equipped backpack. The database still enforces the 20-entry ceiling.
- **Coin definitions.** The source supplies the compatible gold, platinum and crystal definition
  revisions. A live coin stack of the backpack at another revision refuses the fee
  (`InvalidInput`, nothing written) rather than being skipped: decision §4.2 makes only the
  compatible revision eligible, and the database plan guard counts every coin stack. The change is
  minted at the platinum and gold revisions; the database checks the key and family, not the
  revision (as for every other MINT).
- **Content write.** D176's `semantics.stack` = 100 content edit for i3031/i3035/i3043 is queued on
  the content train (#162 5916023254, "the GOLD-FEE-1b coin edit") and is not in this PR. Runtime
  behaviour does not depend on it: `COIN_STACK_MAXIMUM` = 100 is the typed maximum the planner and
  guards enforce, and a stackable definition without a proven maximum already gets 100 (D82).
- **Bank.** The bank part of a fee (BANK-FEE-0) is GOLD-FEE-2, not this task.

## Closeout

- Validation: fmt, clippy (all targets, `-D warnings`), the package tests, and on PostgreSQL 17.6
  `item_fee_burn_postgres` (7 cases) plus the item, corpse, reward-claim, charm and authority
  targets. Guard mutations of 0031 (minimality arm, output count, change ordinal, change quantity)
  each turn a case red.
- CHARM-6 composes this writer: it supplies `FeeChangeFacts` and replaces
  `game_item_fee_burn_consistency_guard` from the 0031 body (decision §6).
- Repair (Codex review of `de8ee89`, three P2 findings): coin stacks must be at the compatible
  revision (above); the change fit counts entries already over the declared capacity
  (`entries - whole + outputs <= capacity`); the event audit requires the first change ordinal
  above every burn line's ordinal. Each has a unit or PostgreSQL case. New candidate, new freeze.
- Review: required independent exact-head review (persistence, value conservation), triggered by
  the control plane on the frozen head.
