# OTV2-20261003-item-move-1

```yaml
task_id: OTV2-20261003-item-move-1
title: "ITEM-MOVE-1: command 9 item move from the open corpse (server side)"
mode: IMPLEMENTATION
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-move-1-20261005
pr: null
base_sha: 0ec917e6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
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
`item_view.rs` (a doc line) are added under option A of the QUESTION sent to the CP on 2026-10-05.

## Outcome

Pending.
