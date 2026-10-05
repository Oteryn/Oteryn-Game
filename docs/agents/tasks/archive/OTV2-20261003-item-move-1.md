# OTV2-20261003-item-move-1

```yaml
task_id: OTV2-20261003-item-move-1
title: "ITEM-MOVE-1: command 9 item move from the open corpse (server side)"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-move-1-20261005
issue: 1622
pr: 1827
base_sha: 0ec917e6
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: oteryn-hard-worker (CP #1622)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
migration_lease: none
owned_paths:
  - apps/game-server/src/gameplay_transport/item_move.rs
  - apps/game-server/src/gameplay_transport/item_move_tests.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/combat/pickup.rs
  - apps/game-server/src/durability/item_transfer.rs
  - apps/game-server/src/gameplay_transport/item_view.rs
  - apps/game-server/tests/corpse_transfer_postgres.rs
  - apps/game-server/tests/support/corpse_transfer_postgres_cases.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20261003-item-move-1.md
public_contracts: []
depends_on: [ITEM-VIEW-1b, ITEM-TRANSFER-1]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Packet: `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md` §2.6
(ITEM-MOVE-WIRE-0 §5). `durability/item_transfer.rs` (a read-only receipt lookup by CommandRef) and
`item_view.rs` (the open-corpse seams and a doc line) are added under CP D731 (option A).

## Outcome

- Command 9 (`ItemMoveIntentV1`) moves an entry of the open corpse into the main backpack
  through the corpse TRANSFER (`item_move.rs`). Replay first: the committed TRANSFER receipt of
  the CommandRef answers `MOVED` before the handle is resolved (new read-only
  `read_item_transfer_receipt` on `game_item_transfer_receipts`; no migration).
- Every TRANSFER outcome maps to WIRE-0 §5 (`outcome_of`). After `MOVED`, the domain 11 delta and
  then the domain 9 delta follow. An unknown outcome or an unreadable committed view ends the
  connection before the CommandId is sequenced.
- After `MOVED` the backpack view is updated before the corpse view, so an entry taken whole
  keeps its handle from the corpse into the backpack (review finding on c442bd84).
- Production `committed_item_move` reads the receipt table. Production `take_corpse_entry` fails
  closed until a corpse source exists. Production corpse visibility stays with D525 and
  KILL-REWARD-COMP-1 (CP D731).
- CP D738: capability 4 stays `offered: false`. The flip to `offered: true` belongs to the PR that
  adds the production `TypedDefinitionRef` -> `item_definition_ref` mapping (MAP-CUTOVER line).
  Selection is tested through a fixture offered set.

## Excluded

- kill_reward, `capabilities.rs`, production corpse source and domain 9 definition mapping.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server`: pass (Postgres cases run in the CI PG lane)
- `cargo test --locked -p oteryn-protocol-oteryn`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
