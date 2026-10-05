# MAP-OVERLAY-1b

```yaml
task_id: MAP-OVERLAY-1b
title: "MAP-OVERLAY-1b map-item materialization: MINT-then-TRANSFER pickup of eligible base-map entries"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: map-overlay
base_branch: main
branch: agent/map-overlay-1b-20261005
pr: PENDING
base_sha: 8f174761
owner: claude-code-session-01LZpLbhRJD359Hia74Vvg5J (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
packet: "docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md §2.6 (with §0, §1.5, §1.11, §1.12); ADR-0021 §4.4"
leases: "migration 0077; OneItemTransactionV1 operation tag 8 (control plane leases)"
owned_paths:
  - apps/game-server/migrations/0077_map_item_materialization.sql
  - apps/game-server/src/durability/map_item_mint.rs
  - apps/game-server/src/durability/map_item_mint_audit.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/item_mint_audit.rs
  - apps/game-server/src/map/overlay.rs
  - apps/game-server/src/map/overlay/pickup.rs
  - apps/game-server/tests/map_item_mint_postgres.rs
  - apps/game-server/tests/support/map_item_mint_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/MAP-OVERLAY-1b.md
depends_on:
  - "MAP-OVERLAY-1a ChannelOverlay on main"
public_contracts:
  - docs/contracts/game-events/v1/native_one_item_transaction.proto (additive operation tag 8)
external_repositories: []
```

## Outcome

- Migration 0077 adds `game_map_item_mint_reservations` and `game_map_item_mint_receipts`.
  - A receipt is unique per (World, Channel, base bundle digest, reset epoch, `placement_key`),
    so an entry is taken once per Channel and reset epoch, and another Channel can take it too.
  - A CHECK derives the receipt's tile from its `placement_key`.
  - It re-creates `game_item_mint_consistency_guard` (from 0075) and
    `game_item_ground_insertion_guard` (from 0049), each with a map-item MINT arm.
- `durability::map_item_mint`: a reserve/commit/reconcile MINT under the full current item fence.
  - It mints the entry into Ground at its origin tile in one transaction with the receipt and the
    `map_item_mint` audit event (OneItemTransactionV1 operation tag 8).
  - A retried MINT returns the existing item; a conflicting cause or identity is refused and
    writes nothing.
- `durability::map_item_mint_audit`: the event round-trips with its full cause, and the audit
  refuses a missing or extra cause field. The worst-case payload (4,514 B) and envelope
  (5,552 B) are recorded under DUR03-RL-07-* in the resource-limits registry.
- `map::overlay::pickup`:
  - eligibility from the bundle's tile record; each ineligible kind stays in place;
  - the reach check before every TRANSFER, a retried MINT included;
  - the origin hides at freeze and unhides only on a proven non-commit;
  - after a crash, the rebuild re-hides every origin with a receipt, even over the budget
    (counted and alarmed).
- Tests:
  - `apps/game-server/tests/map_item_mint_postgres.rs`, also run from
    `character_authority_postgres`, covers MINT-then-TRANSFER, refusals, every fence operator at
    freeze and at commit, and concurrent pickups on two roots;
  - the overlay pickup tests and the `pickup` unit tests.
- Out of scope: reset retirement and the reset record (1c); live wiring (MAP-CUTOVER-1).

## Review

Hard and persistence review on the final frozen head; the control plane requests it.
