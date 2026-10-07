# OTV2-20261005-gold-fee-act-1

```yaml
task_id: OTV2-20261005-gold-fee-act-1
title: "GOLD-FEE-ACT-1 type-2 audit activation fence and tuple selection (migration 0079)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: bank
base_branch: main
branch: claude/gold-fee-act-1-20261005
pr: 1850
base_sha: 0886ab6c
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: oteryn-hard-worker
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
packet: "docs/architecture/reviews/OTERYN_GAME_GOLD_FEE_ACT1_TYPE2_ACTIVATION_DECISION_2026-10-04.md §2.1 (accepted #1746)"
leases: migration 0079
owned_paths:
  - apps/game-server/migrations/0079_type2_audit_activation.sql
  - apps/game-server/src/durability/item_mint_audit.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/charm_state.rs
  - apps/game-server/src/durability/character_revision_sequencer.rs
  - apps/game-server/src/durability/item_mint.rs
  - apps/game-server/src/durability/item_transfer.rs
  - apps/game-server/src/durability/reward_claim_mint.rs
  - apps/game-server/src/durability/item_decay_retire.rs
  - apps/game-server/src/durability/item_timed_state.rs
  - apps/game-server/src/durability/item_fee_burn.rs
  - apps/game-server/src/durability/map_item_mint.rs            # D781
  - apps/game-server/src/durability/map_item_mint_audit.rs      # D781
  - apps/game-server/src/durability/item_transfer_audit.rs      # D781
  - apps/game-server/src/durability/reward_claim_mint_audit.rs  # D781
  - apps/game-server/src/durability/item_decay_retire_audit.rs  # D781
  - apps/game-server/src/durability/item_timed_state_audit.rs   # D781
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/tests/support/gold_fee_bank_postgres_cases.rs
  - apps/game-server/tests/support/item_fee_burn_postgres_cases.rs
  - apps/game-server/tests/support/type2_audit_activation_postgres_cases.rs  # D786
  - apps/game-server/tests/character_authority_postgres.rs                   # D786, include only
  - docs/agents/tasks/archive/OTV2-20261005-gold-fee-act-1.md
depends_on:
  - "GOLD-FEE-2 merged (#1781, migration 0072)"
public_contracts: []
external_repositories: []
```

## Outcome

- **Migration 0079.**
  - Adds the type-2 activation row. It is insert-only, a singleton (`CHECK id = 1`), and the runtime role can only SELECT it.
  - Adds the OTA01 outbox guard: an outbox row must carry the activated tuple, with an exact grandfather exception for rows written before activation.
  - Adds the reservation markers. Each is set at INSERT under the fence and cannot be changed afterwards.
  - Existing reservations are backfilled as pre-activation.
- **Fence.**
  - `begin_type2_transaction` takes `TYPE2_AUDIT_ACTIVATION_FENCE` shared, reads the activation row, and returns a `Type2Transaction`.
  - Activation takes the same fence exclusive.
  - `SelectedType2Tuple` can be built only inside `item_mint_audit`.
- **Writers.**
  - Every `game_item_audit_outbox` insert binds `tuple.schema_revision()` and `tuple.retention_profile_id()` from its `Type2Transaction`, and every type-2 encoder takes the tuple as a parameter.
  - This covers item_mint, item_transfer, reward_claim_mint, item_decay_retire, item_timed_state, item_fee_burn and, under D781, map_item_mint.
  - The compile-time stamp is removed.
- **No protocol or contract change.**

## `burn_fee_in_transaction` callers on `main`

- `apps/game-server/src/durability/item_fee_burn.rs`: definition. It now takes `&mut Type2Transaction`.
- `apps/game-server/src/durability/mod.rs`: re-export and reference only. Unchanged.
- `apps/game-server/tests/support/gold_fee_bank_postgres_cases.rs`: converted to `begin_type2_transaction`.
- `apps/game-server/tests/support/item_fee_burn_postgres_cases.rs`: converted to `begin_type2_transaction`.
- `apps/game-server/src/durability/character_revision_sequencer.rs`: source-text assertions only. Unchanged.
- `apps/game-server/src/durability/charm_state.rs`: no caller. Unchanged.

## Decisions

- **D781 (a)+(c).** map_item_mint's commit pass now runs in `begin_type2_transaction` and takes `tx.tuple()`. Its other passes stay semantic because they insert no outbox row. The audit encoders for map_item_mint, item_transfer, reward_claim_mint, item_decay_retire and item_timed_state take the tuple. The coverage test `every_type2_insert_takes_its_tuple_from_a_type2_transaction` stays strict, with no allowlist.
- **D786 (a).** The PostgreSQL cases live in `tests/support/type2_audit_activation_postgres_cases.rs` and are pulled into `character_authority_postgres` by a `#[path]` include, so they run in CI.
- **Owner-approved (a).** The encoder-literal assertion is scoped to `item_mint_audit`'s constants. The bank ledger envelope (`bank_audit.rs`, `ECONOMY_LEDGER_RETENTION_V1`) is a separate family and is unchanged.

## Validation

All runs used local PostgreSQL; the DB cases ran rather than being skipped.

- `cargo test --locked -p oteryn-game-server <filter>` passes for each filter: type2_audit_activation, item_mint, item_transfer, reward_claim_mint, item_decay_retire, item_timed_state, item_fee_burn, charm_state, character_revision_sequencer, map_item_mint and gold_fee_bank.
- `cargo test --locked -p oteryn-game-server --lib`: pass.
- `cargo test --locked -p oteryn-game-server --test character_authority_postgres`: pass (1283 cases). Two gold_fee_bank guard cases that run after activation now use the activated tuple `(2, V2)`: the item-less event and the coin-only fee. Each case still checks the same guard.
- `cargo check --locked --workspace --all-targets`: pass.
- `cargo fmt --all -- --check`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `python tools/agents/validate_governance.py` -> pass
- `python -m unittest discover -s tools/agents/tests` -> pass
- `python3 tools/repository/validate_repository_policy.py`: pass.

## Notes for review

- Merge condition: 0079 merges only after 0078 (#1807) has merged or been released.
- A fence wait that runs out of time can surface as `Unavailable(Database)` (lock_timeout 55P03) instead of `RootPassDeadlineExceeded`, because `begin_semantic_transaction` sets the session timeouts from the remaining deadline.
