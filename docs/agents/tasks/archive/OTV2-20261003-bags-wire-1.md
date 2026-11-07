# OTV2-20261003-bags-wire-1

```yaml
task_id: OTV2-20261003-bags-wire-1
title: BAGS-WIRE-1 Capability 14 CONTAINER_TREE_V1, domain 14 CONTAINER_VIEWS, command 21 CONTAINER_VIEW_INTENT
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/bags-wire-1-20261003
issue: 1622
packet: docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md §2.11
decision: BAGS-0 §5 and §10
migration_lease: none
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json        # capability 14, domain 14, command 21
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # own rows only
  - docs/contracts/protocol-oteryn/v1/container_tree_v1.proto   # new
  - docs/contracts/protocol-oteryn/v1/item_view_v1.proto   # the CONTAINER destination only
  - crates/protocol-oteryn/src/{lib,container_tree,container_tree_tests,item_view}.rs
  - apps/game-server/src/gameplay_transport/{container_view,container_view_tests}.rs   # new
  - apps/game-server/src/gameplay_transport/mod.rs         # one mod line
  - apps/game-server/src/gameplay_transport/connection.rs  # D469 1a: join-snapshot slot, USE OPEN arm, command 21 arm; D470 1a: FRESH literal
  - apps/game-server/src/gameplay_transport/item_view.rs   # D469 1a: View::Tree, bound, views_revision
  - apps/game-server/src/gameplay_transport/resume.rs      # D469 1a: domain 14 in the FND-02 fence
  - apps/game-server/src/gameplay_transport/capabilities.rs     # D470 1a: GATED row (14, [21], [14])
  - apps/game-server/src/gameplay_transport/item_view_tests.rs  # D470 1a: one struct literal field
  - docs/agents/tasks/archive/OTV2-20261003-bags-wire-1.md
```

## Owned-path widenings

- D469 1a and D470 1a (control plane): `item_view.rs`, `resume.rs` and `connection.rs` (own lines
  only: the join-snapshot slot, the `USE` open arm, the command 21 arm, the trait method and the
  `FRESH` literal), the `capabilities.rs` GATED row, and the one `views_revision` field of the
  `item_view_tests.rs` struct literal.
- D469 2a: the row `ITEMV0-RL-03-CONTAINER-TREE` = 637 with max and max+1 tests.
- D470 2a: `BAGS0-RL-04` is 10 view commands per second on a sliding window; an over-rate command is
  `REJECTED` with an empty payload.

## Outcome

- Protocol: capability 14 `CONTAINER_TREE_V1` (`offered: false` until BAGS-1, requires 4 and 12),
  domain 14 `CONTAINER_VIEWS` (snapshot and delta type 1, up to 16 views of up to 20 entries,
  10,384 bytes), command 21 `CONTAINER_VIEW_INTENT` (open, close, up; 15 bytes; result 4 bytes) and
  the command 9 `CONTAINER {handle}` destination (field 5, only with capability 14, within the
  29-byte capability 12 bound). Fail-closed codecs and max/max+1 tests in `container_tree_tests.rs`.
- Resource rows: `BAGS0-RL-03` (16 views), `BAGS0-RL-03-BYTES` (10,384), `BAGS0-RL-04` (10/s) and
  `ITEMV0-RL-03-CONTAINER-TREE` (637 = 301 + 16 x 21).
- Server: `container_view.rs` keeps one connection's open views. Inner entries take handles from the
  ITEM-VIEW-1b table as its fourth view under the 637 bound. Command 21 plans, asks the Channel
  owner (`observe_container`, default `None` = `STALE`) and sends the whole domain 14 view after
  the result. `USE` on a container handle opens it in a new view; anything else keeps the corpse
  path, so corpses stay domain 11 at depth 1. Domain 14 is an empty snapshot above the carried
  revision after every admission, reconnect, resume and transfer; `resume.rs` carries its revision
  in the FND-02 fence.
- Assumptions: an open of a container already shown moves it to the target view (one container in
  one view); `USE` maps `TOO_MANY_VIEWS` and `NOT_A_CONTAINER` to `NOTHING_TO_USE`; views that do not
  encode (a stale observation) are `STALE` and change nothing.

## Excluded

- Any durable tree move and the production `observe_container` (BAGS-1); offering capability 14.
- Closing views on walking out of reach (no reach trigger is in the packet; contents do not update
  before BAGS-1 either).

## Validation

- `cargo test --locked -p oteryn-protocol-oteryn`
- `cargo test --locked -p oteryn-game-server --quiet`
- `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` on both crates
- `python3 tools/agents/validate_governance.py` and its unit tests; `git diff --check`
