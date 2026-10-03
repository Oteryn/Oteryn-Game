# OTV2-20261003-timed-rt-1b

```yaml
task_id: OTV2-20261003-timed-rt-1b
title: "TIMED-RT-1b: timed-item expiry, its audit event and the hosting runtime's lanes"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/timed-rt-1b
pr: "exact PR in the #1622 FREEZE_SHA entry"
base_sha: c3cab005
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-015sWxwCsfiHgQNMeQvbYbUc (hard worker)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
migration_lease: "0058"
packet: docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_TIMED_FORGE_PROF_PACKETS_2026-10-03.md §2.3
owned_paths:
  - apps/game-server/migrations/0058_item_timed_expiry.sql
  - apps/game-server/src/durability/item_timed_state.rs
  - apps/game-server/src/durability/item_timed_state_audit.rs
  - apps/game-server/src/domain/timed_item.rs
  - apps/game-server/src/domain/timed_item_host.rs
  - apps/game-server/src/domain/mod.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/tests/support/item_timed_state_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/agents/tasks/archive/OTV2-20261003-timed-rt-1b.md
touched_outside_owned_paths:
  - apps/game-server/src/durability/item_mint_audit.rs   # oneof tag 7 for the expiry payload
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json   # one sentence on tag 7
depends_on: ["#1681 merged"]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

CP answers on #1622: Q1 (a), a narrow resolver over `ReferenceItemSemantics.temporal`/`charges`
with test fixtures and no content edits. Q2 unanswered; built on assumption (a).

- Migration 0058: admits `Expire {TimeExhausted | ChargesExhausted}` (causes 2, 3). The record
  carries the before full values, lit flag and event id; the guard pins the before and after
  definition facts. The expiry transform changes the item's definition in place and resets the
  row to the target's full values (spent when the target is not timed) at revision + 1. The
  expiry burn retires a direct container entry, deletes its entry and leaves the row. One audit
  outbox event is required in the same transaction. The item guard admits an in-place definition
  change only with its expiry record; the entry-removal proof admits the burn. A checkpoint
  storing 0 ms is refused (P2 4174680696), in `validate_checkpoint` and in the guard. Causes 4-7
  stay refused.
- `durability::item_timed_state::commit_timed_expiry` under the holder's gameplay fence, with
  replay, conflict, `NotHeld` and `NotBurnable` (slot container, item with contents).
- `durability::item_timed_state_audit`: the `OneItemTimedExpiryV1` payload (tag 7) and the DUR-03
  §12 shape rows: checkpoint, composed checkpoint, expiry transform, expiry burn and composed
  expiry burn (#1689 P1 4174803489). Per the CP correction, the plain checkpoint row is proven at
  max and max+1 on `commit_timed_checkpoint` (the 0058 guard admits one timed write per physical
  transaction); the composed checkpoint row is registered as a shape only, and its writer-backed
  tests go to the PR that adds the composed writer.
- `domain::timed_item_host`: the resolver and the actor's host. Items in their slot at login,
  respawn or arrival become live, the exercise binding API, cadence, write dispatch once per
  outcome, the ambiguous hold, and stop-all with the all-empty check. RL-01, -02, -04 and -05 are
  proven at the host boundary.

Not wired: no login, respawn, arrival, logout, transfer or death path on `main` loads a
Character's items into an actor yet. The host is the API those paths call. The composed
checkpoint and composed expiry burn writers are ITEM-MOVE-2a's and EXERCISE-1's.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --quiet`: pass
- `cargo test --locked -p oteryn-game-server --test item_timed_state_postgres` against a local
  PostgreSQL 17.11 (the 17.6 version assertion relaxed locally only; CI runs 17.6): every timed
  case passes. The proficiency case that requires exactly 17.6 is the only failure.
- `python3 tools/agents/validate_governance.py`: pass
- `git diff --check`: pass
