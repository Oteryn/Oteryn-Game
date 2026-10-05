# OTV2-20261005-arch-map-track-1

```yaml
task_id: OTV2-20261005-arch-map-track-1
title: "ARCH-MAP-TRACK-PACKETS-V1 D730 amendment: MAPW-A1 rejected, map cutover waits for ITEM-MOVE-1"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-map-track-a1-20261005
issue: 162
pr: 1823
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_WIRE_PACKETS_2026-10-04.md
  - docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-map-track-1.md
public_contracts:
  - docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md
depends_on: [OTV2-20261005-arch-map-track-0]
blocks: [MAP-CUTOVER-1b, MAP-CLIENT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- CP D730, owner answer 1b to `ARCH-MAP-TRACK-PACKETS-V1` §1.5: Amendment MAPW-A1 is rejected.
  Capability 18 keeps requiring 4 and 6; the MAP-WIRE-1 contract §3 block says so and §4 is
  unchanged.
- MAP-CUTOVER-1b (§2.4) and MAP-CLIENT-1 (§2.6) depend on ITEM-MOVE-1
  (`OTV2-20261003-item-move-1`, offers capability 4). The `display_only` fallback for
  item-handle entries and the protocol-oteryn requires change are removed from both packets.
- ARCH-MAP-WIRE §2.3 pointer note updated to the new dependency.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
