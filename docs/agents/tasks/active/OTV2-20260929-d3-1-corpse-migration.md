# OTV2-20260929-d3-1-corpse-migration

```yaml
task_id: OTV2-20260929-d3-1-corpse-migration
title: D3-1 corpse receipt migration, materialized_at trigger and corpse-cap enforcement
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/d3-1-corpse-migration
issue: 162
allocation: "#162 D3 slice; D3-1 row, docs/architecture/reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md §6"
base_sha: 4a828159fe854c66901f3d25ac0d7f39b4fbc63d
owner: "Oteryn: impl durability"
created_at: 2026-09-29
updated_at: 2026-09-29
owned_paths:
  - apps/game-server/migrations/0013_corpse_container_mint.sql
  - apps/game-server/src/durability/item_mint.rs
  - apps/game-server/tests/support/item_mint_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260929-d3-1-corpse-migration.md
public_contracts: [DUR-03 §39.4]
depends_on: [D3 decision (merged, #1198)]
blocks: [D3-2, D3-4, D3-6]
```

## Scope (D3-1 row only)

Migration `0013_corpse_container_mint.sql`, additive to merged 0010-0012, least-privilege grants:

- `game_item_mint_receipts` gains four nullable columns: `corpse_top_damage_character_id`,
  `materialized_at`, `destination_parent_item_instance_id`, `destination_ordinal`. The first is
  `NOT NULL` iff `loot_purpose_key = 'CORPSE_MATERIALIZATION'`; the corpse's own MINT stays Ground-only
  (`destination_parent_item_instance_id` never set for it).
- A deferred (`AFTER INSERT ... DEFERRABLE INITIALLY DEFERRED`) `SECURITY DEFINER` trigger writes
  `materialized_at` from `clock_timestamp()` immediately before commit, for a `CORPSE_MATERIALIZATION`
  receipt only. `game_item_mint_receipt_immutable` is narrowed (mirroring the 0011 audit-outbox
  publication mark) to admit exactly that one one-way `NULL -> value` transition; no other column, no
  second write.
- New table `game_item_corpse_container_entries` (`Container{parent=corpse}`, distinct from the
  character-scoped `game_item_container_entries` since a corpse loot entry has no character owner
  until picked up), with a deferred `SECURITY DEFINER` admission trigger
  (`game_item_corpse_container_entry_proven`) that locks the corpse's own receipt row before counting
  and enforces `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` = 16.
- `game_item_mint_consistency_guard` (0010/0012) gains a third branch admitting a loot entry's MINT
  into that Container family, gated on the parent carrying a live `CORPSE_MATERIALIZATION` receipt for
  the *same* death (the parent-live-receipt check).
- `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX`/`-PLACEMENT-DEPTH`/`-REACHABLE-ITEMS` (16/1/17) registered
  in `RESOURCE_LIMITS_REGISTRY.json` with max/max+1 boundary tests.

Rust (`durability/item_mint.rs`, additive only — `ItemMintRequest`/`freeze_item_mint`/
`commit_item_mint` are unchanged so `combat/death_reward.rs` keeps compiling untouched):

- `commit_corpse_mint`: a new method beside `commit_item_mint`, admitted only for a
  `CORPSE_MATERIALIZATION` cause, that writes the captured top-damage `CharacterId` into the receipt
  and, **before `fence_is_live` and before its own insert**, takes the per-scope
  `pg_advisory_xact_lock(hashtextextended('oteryn:corpse-cap:' || scope_key, 0))` and recounts live
  corpses for the scope, refusing `CapacityExceeded` at or above `COMBAT01-CORPSES-PER-SCOPE` = 64
  (already accepted by the VSL resource-rows decision; not re-registered here). `materialized_at` is
  never written from Rust — only the migration's trigger ever sets it.
- `freeze_item_mint` is reused unchanged for the corpse's own MINT (no new request shape needed there).

## Deferred (explicitly out of D3-1)

DUR-03 §39.4 itself defers widening `OneItemMintV1.destination` beyond Ground-only to child D3-6, and
`check_mint` today requires a Ground destination — so no real Rust freeze/commit caller exists yet for
a *loot* entry's MINT into a corpse container. The admission gate and its 16-entry ceiling are proven
directly against the schema (raw-SQL forged rows, mirroring the existing backpack-capacity test
pattern); D3-2 (composing caller) and D3-6 (proto/registry) wire a real caller against it. This is a
reported scope gap, not a silent omission.

## Validation

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`: pass (see handback for exact counts).
- `cargo test --quiet --lib`: pass (see handback).
- `cargo test --quiet --no-run` for `item_mint_postgres`/`character_authority_postgres`: compiles;
  no PG admin URL locally (`OTERYN_TEST_POSTGRES_ADMIN_URL` unset) so the PG cases themselves report
  PRE-ROUTING/NONCANONICAL locally and run in CI.
- `validate_governance.py`: pass. `git diff --check`: pass.

New PG tests in `tests/support/item_mint_postgres_cases.rs`: `corpse_mint_writes_top_damage_and_materialized_at_only_via_trigger`,
`commit_corpse_mint_rejects_a_non_corpse_cause`,
`concurrent_corpse_mints_at_capacity_produce_exactly_one_success` (N=3 concurrent corpse-MINT commits
at 63 live corpses; exactly one success, N-1 `CapacityExceeded`, never more than 64),
`corpse_container_entry_enforces_the_capacity_ceiling` (16th admitted, 17th rejected),
`corpse_container_entry_requires_a_live_parent_receipt_for_the_same_death`.

## Context checkpoint

```yaml
last_progress: branch pushed; local validation complete; no PR opened per instruction
next_action: "#162 allocates D3-2/D3-4/D3-6 against this migration; requires independent review before Merge Queue"
```
