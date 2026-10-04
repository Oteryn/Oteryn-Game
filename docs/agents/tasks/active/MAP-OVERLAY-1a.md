# MAP-OVERLAY-1a

```yaml
task_id: MAP-OVERLAY-1a
title: "MAP-OVERLAY-1a: per-channel map overlay, expiry index, budget and Ground rebuild"
mode: IMPLEMENT
status: in_progress
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

## Next step

Author the overlay, tests and budget evidence; freeze.
