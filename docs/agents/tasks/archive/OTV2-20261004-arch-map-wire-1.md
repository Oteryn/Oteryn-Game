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
- No code or registry change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
