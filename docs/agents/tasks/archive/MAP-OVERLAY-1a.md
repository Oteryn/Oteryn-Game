# MAP-OVERLAY-1a

```yaml
task_id: MAP-OVERLAY-1a
title: "MAP-OVERLAY-1a: per-channel map overlay, expiry index, budget and Ground rebuild"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: agent/map-overlay-1a-20261004
pr: PENDING
base_sha: 673f092e
owner: oteryn-hard-worker
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md §2.5 with §0, §1.5, §1.11; ADR-0021 §4.4, §4.8"
review: persistence review (ADR-0021 §4.8)
leases: none (no migration, no wire change, no operation tag)
owned_paths:
  - apps/game-server/src/map/overlay.rs
  - apps/game-server/src/map/overlay/**
  - apps/game-server/src/map/mod.rs            # the `mod overlay` line only
  - apps/game-server/tests/map_overlay_*.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json   # MAP01-CHANNEL-OVERLAY-BYTES only
  - docs/agents/evidence/MAP-OVERLAY-1a-*.md
  - docs/agents/tasks/archive/MAP-OVERLAY-1a.md
depends_on:
  - "MAP-LOAD-1 merged (#1783)"
public_contracts: []
external_repositories: []
```

## Outcome

- `map::overlay::ChannelOverlay`: one Channel's overlay over the World's `Arc<WorldBase>`.
  - Per tile, a `u64` hidden bitmask over top-level ordinals 0..63, and added items in insertion
    order: volatile items with their attributes, or durable `GroundItemInstance` records.
  - Nothing is ever merged, so each base `placement_key` keeps exactly one entry.
- The expiry index removes a decayed volatile item at its deadline second: never before the
  decay, and within 1 s of it.
- The `MAP01-CHANNEL-OVERLAY-BYTES` budget (default 64 MiB):
  - A volatile entry or a freeze-time hide over the budget is refused atomically.
  - Durable Ground items and rebuild re-hides are admitted, counted and alarmed.
  - The measured value is recorded in `docs/agents/evidence/MAP-OVERLAY-1a-overlay-budget.md`
    and in the registry row.
- `ChannelOverlay::rebuild` restores every durable Ground item. It fails closed on:
  - a `map_revision` that is not `sha256:<bundle digest>`;
  - another World or Channel;
  - an undecodable or unmapped position;
  - a duplicate.
- Tests are in `apps/game-server/tests/map_overlay_channel.rs`. There are seven acceptance tests,
  plus one ignored release-mode budget measurement.
- Out of scope:
  - map item MINT and pickup (1b);
  - the reset record (1c);
  - live wiring into the channel loop (MAP-CUTOVER-1).

## Review

The persistence review runs on the final frozen head; the control plane requests it.
