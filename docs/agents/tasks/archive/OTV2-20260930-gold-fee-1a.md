# OTV2-20260930-gold-fee-1a

```yaml
task_id: OTV2-20260930-gold-fee-1a
title: GOLD-FEE-1a - in-transaction gold fee BURN (gold coins), fee-shape rows, audit operation
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gold-fee-1a
issue: 162
pr: null   # set on the PR; a commit cannot hold its own PR number
base_sha: 5b2c5197   # main at authoring, with claude/gold-fee-boundary-t0 (#1313, 09e59858) merged in
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
owner: "GOLD-FEE-1a hard worker (claude-code-session-01EsJZSgyZhrwgqkWVcJVkVi)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
split: "1a = gold-coin-only burn (change is always 0); 1b = platinum/crystal inputs with the change MINT"
size_exception: "accepted: about 2,300 hand-written lines, one atomic unit (migration, writer, audit, guards and their PostgreSQL cases are only sound together); the split did not reduce it"
owned_paths:
  - apps/game-server/migrations/0023_character_gold_fee_burn.sql
  - apps/game-server/src/durability/item_fee_burn.rs
  - apps/game-server/src/durability/item_fee_burn_audit.rs
  - apps/game-server/src/durability/mod.rs   # two mod lines and the linkage test
  - apps/game-server/src/domain/currency.rs
  - apps/game-server/src/domain/mod.rs   # one mod line
  - apps/game-server/tests/item_fee_burn_postgres.rs
  - apps/game-server/tests/support/item_fee_burn_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json   # seven new fee-shape rows only
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md   # review hardening
  - docs/agents/tasks/archive/OTV2-20260930-gold-fee-1a.md
owned_path_extensions:   # needed by decision §4.5 (the audit schema); flagged for the control plane
  - apps/game-server/src/durability/item_mint_audit.rs   # oneof arm tag 6; envelope gate under a shape ceiling
  - docs/contracts/game-events/v1/native_one_item_transaction.proto   # fee_burn = 6 and its messages
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json   # event type 2 note
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §15 pointer line (hardening 1)
  - apps/game-server/tests/support/item_transfer_postgres_cases.rs   # least-privilege row: runtime DELETE on backpack entries
public_contracts: [DUR-03, GAME-ITEM-01]
depends_on: ["#1313 (GOLD-FEE T0 decision)"]
blocks: [GOLD-FEE-1b, CHARM-6]
cross_repository_coordination_id: null
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

`burn_fee_in_transaction` runs inside a fee source's fenced Character transaction: replay by
cause occurrence (same binding returns the retained outcome, a changed binding conflicts), the
root re-locked at the expected or next revision, the gold stacks of the equipped main backpack
planned by `domain::currency::plan_fee` (decision §4.2), then the fee record, its BURN lines,
the item changes, the whole-burn entry removals and one `fee_burn` audit event (event type 2,
oneof tag 6). Every refusal returns before any write; the caller rolls back. Migration 0023 proves
at commit that the Character root was advanced by the same physical transaction to the bound
revision, that every line matches the real item state and the plan order, and that a backpack
entry ends only as a whole-burn line. Fee-shape rows RL-01 22, RL-02 22, RL-06 22/64, RL-07 1
and the measured RL-07 payload 24,181 B / envelope 24,495 B are registered as new rows; the
one-item rows are unchanged.

## Interpretations (for the independent review)

- The fee record (`game_item_fee_burns`) is the item side of the one transaction, not a second
  receipt: CHARM-6's Character receipt remains the receipt. It is unique per cause occurrence so
  the same occurrence can never burn twice, and it carries the binding the packet's replay needs.
- Until CHARM-6 adds its unassign receipt, the database proves the Character change as "root
  written by this physical transaction at the bound committed revision"; the 0019/0020 chain
  guard then proves exactly one receipt of that revision.
- Gold-only is a strict subset of the decision plan (gold has the lowest worth, so it is always
  burned first and never yields change); a fee gold cannot pay is refused until 1b.
- Eligible inputs are any definition revision of the gold key; items carry no instance state
  other than quantity (hardening 3).
- `semantics.stack` in the content definitions is not rewritten: `COIN_STACK_MAXIMUM` = 100 (D176)
  is the typed maximum the planner and guards enforce, and the item transfer rules already give a
  stackable definition without a proven maximum 100.

## Closeout

- Validation: fmt, clippy (all targets, `-D warnings`), the package tests, and on PostgreSQL 17.6
  `item_fee_burn_postgres` plus the item, corpse, pickup, charm, authority and privilege targets.
  Six guard mutations of 0023 (plan order, same-transaction Character change, before-quantity
  evidence, removal ordinal, gold key, envelope size) each turn a case red.
- Follow-up: GOLD-FEE-1b carries the content write of `semantics.stack` = 100 for
  i3031/i3035/i3043, platinum and crystal inputs and the change MINT, and lands before CHARM-6
  (decision §4.2/§6 amended to this split). CHARM-6's migration replaces
  `game_item_fee_burn_consistency_guard` to require its CharmUnassign receipt (same occurrence,
  committed revision, bound TransactionId and fee), and CHARM-6 verifies the session-generation
  fence (decision §6).
- Size: about 2,300 hand-written lines, above the batch guidance. Recorded as an accepted
  exception: the migration, writer, audit event and their PostgreSQL cases are one atomic unit.
- Repair generation (independent review #1318 5907337686): a BURN line appended in a later
  transaction to an already-committed fee record proved an item change (duplication path). The
  item-change fee arm now also requires the line's fee record to be of the same physical
  transaction, and a deferred constraint trigger refuses any line whose fee record is not. The
  other 0023 arms (entry removal, envelope size) already bound the record to the same
  transaction. Two runtime-role PostgreSQL cases prove the refusal (red before the fix).
- Review: required independent exact-head review, triggered by the control plane on the frozen head.
