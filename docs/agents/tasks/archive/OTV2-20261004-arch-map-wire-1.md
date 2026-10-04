# OTV2-20261004-arch-map-wire-1

```yaml
task_id: OTV2-20261004-arch-map-wire-1
title: "ARCH-MAP-WIRE-1: MAP-WIRE-1 world map view contract candidate; MAP-WIRE-2 and MAP-CLIENT-1 packets"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-map-wire-20261004
issue: 162
pr: 1793
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_WIRE_PACKETS_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-map-wire-1.md
public_contracts: []
depends_on: []
blocks: [MAP-WIRE-2, MAP-SPRITE-1, MAP-CLIENT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- MAP-WIRE-1 contract candidate: a server-streamed 18x14 viewport of composed tile stacks
  (base minus hidden, plus added and Ground items), cut at 10 entries per tile, bound to
  `(content_generation, bundle_digest, reset_epoch)`, within the FND-02 limits. It is a candidate
  for owner acceptance and allocates nothing; capability 18, domain 17 and `MAPW-RL-01..04` are
  proposals for the control plane to lease.
- Wire target (#1792 §1.8): the 32-byte bundle digest plus the 8-byte bundle placement key in
  `WorldObjectTargetV1.placement`, resolved to the canonical `PlacementKey` through the
  RewardClaim binding. No durable change.
- Packets: MAP-WIRE-2 (hard, protocol review), MAP-SPRITE-1 (impl, the 15.30 sprite pipeline)
  and MAP-CLIENT-1 (impl, with the joint ground-speed switch of the ADR-0021 MAP-LOAD-1
  amendment), after owner acceptance and MAP-OVERLAY-1a.
- #1793 Codex round 1:
  - P1 4179313297: capability 18 requires 6 and 4 (handles and `ItemTargetV1`).
  - P1 4179313303: the map-view handle budget is `MAPW-RL-04` = 1,024, given nearest-first,
    with a `display_only` origin beyond it. The session bound is `ITEMV0-RL-03-MAP-VIEW` = 1,325,
    or 1,661 with capability 14.
  - P2 4179313306: `ground_speed` is bounded to `0..=1000`, with tests at 1000, absent and 1001.
- Owner decisions on #1793: Q1a MAP-WIRE-1 accepted, effective once Codex review of the fixed
  head is clean; Q2a digest plus placement key; Q3a 10 entries plus `more`; Q4a server-streamed
  viewport; Q5b real 15.30 appearance sprites, with the client assets distributed.
- Q5b adds `MapItemV1.appearance_id` (from the palette key, 0..=65,535) to the candidate and
  splits the sprite pipeline into MAP-SPRITE-1, parallel to MAP-WIRE-2. MAP-CLIENT-1 depends on
  both.
- #1793 Codex round 2 (CP D615):
  - P1 4179361854: `MapItemV1` carries a family-tagged definition reference, a oneof of
    `item_definition_ref` (1) and `terrain_definition_ref` (8, the Terrain palette compact id).
  - P1 4179361856: assets are checked against the `sha256` in the 15.30 manifest, not against
    the hash token in the file name.
  - P2 4179361859: a Ground or overlay item takes its appearance from its item content
    definition key; a donor key uses its `source_item_id`.
  - P2 4179361866: the depth pattern uses `z = -floor`, with a floor -7 example and test.
- #1793 Codex round 3 (CP D618):
  - P1 4179407519: a base entry eligible for pickup (ADR-0021 §4.4) carries an `item_handle`
    bound to `(bundle_digest, placement_key, reset_epoch)`; command 9 from it is the §4.4 MINT
    then TRANSFER.
  - P1 4179407524: `base_ordinal` entries carry `object_revision` (9), the overlay revision; a
    `USE` sends it as `expected_revision`, and each transition resends the tile.
  - P1 4179407527: the header origin is not part of the binding; a delta's origin is the view's
    origin or one step from it on the same floor, else it fails closed.
  - P2 4179407529: an origin-only move delta (no tile, no cleared entry) is valid.
  - P2 4179407531: sprites draw in bounded batches of 81,920 quads, with `MAX_ENTRY_CELLS`
    measured and at least 16.
- #1793 Codex round 4 (CP D622):
  - P1 4179449650: the command-9 pickup of a base entry moves to MAP-PICKUP-1 (packets §2.4),
    after MAP-WIRE-2, MAP-OVERLAY-1b and ITEM-MOVE-1; until then it is
    `ITEM_MOVE_OUTCOME_NOT_SUPPORTED` with no write. MAP-WIRE-2 still sends the handle.
  - P1 4179449652: the client target is selected by the entry's origin: `base_ordinal` gives the
    40-byte `WorldObjectTargetV1` with `expected_revision`, `item_handle` gives `ItemTargetV1`,
    `display_only` gives no command; with one acceptance case per origin.
  - P1 4179449653: header field 5 `first_visible_floor`, computed by the server with the OTClient
    roof rule over the full composed stack and the bundle Terrain kinds; presentation only. With
    the audit §9 fixtures, the indoor, roof, open, doorway and underground vectors.
  - P1 4179449654: MAP-CLIENT-1 draws each tile in render phases (ground, border, on-bottom,
    common, then the tile's actors, then on-top), tiles back to front, with overlap tests.
  - Adversarial pass: the 10-entry cut keeps the bottom entry and the 9 topmost; every floor in
    view is sent and a `first_visible_floor`-only delta is valid; `more` tiles are drawn and
    targeted only by received entries; tiles drop their handles with them; actors join tiles by
    position; the header stays at most 95 bytes inside the 128-byte overhead.
- No code or registry change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
