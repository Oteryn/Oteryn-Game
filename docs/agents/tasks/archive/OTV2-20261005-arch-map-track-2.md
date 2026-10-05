# OTV2-20261005-arch-map-track-2

```yaml
task_id: OTV2-20261005-arch-map-track-2
title: "ARCH-MAP-TRACK-PACKETS-V1 §1.6: item definition reference space and MAP-ITEM-REF-1 (D738, owner 1b)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-map-item-ref-1-20261005
issue: 162
pr: 1828
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_WIRE_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_ITEM_MOVE_WIRE0_ITEM_VIEW_AND_MOVE_DECISION_2026-09-30.md
  - docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-map-track-2.md
public_contracts:
  - docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md
depends_on: [OTV2-20261005-arch-map-track-1]
blocks: [OTV2-20261005-map-item-ref-1, MAP-CUTOVER-1b, MAP-CLIENT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Owner answer 1b to `ARCH-MAP-TRACK-PACKETS-V1` §1.5 question 2, after CP D738. ITEM-MOVE-1
  keeps capability 4 `offered: false`. The new hard packet MAP-ITEM-REF-1 (§2.7) adds the
  production item definition reference, the capability-4 reads, the corpses in domain 1 and the
  offer of 4.
- §1.6 ruling on the reference value:
  - `item_definition_ref` is 1 plus the Item compact id in the session's content generation.
  - The lookup is typed by `TypedDefinitionRef` key and revision.
  - The value is immutable per generation and never stored durably.
  - The Terrain reference is 1 plus the palette id.
- Owning amendments: ITEM-MOVE-WIRE-0 §4.5 and MAP-WIRE-1 contract §3.
- MAP-CUTOVER-1b (§2.4) and MAP-CLIENT-1 (§2.6) depend on MAP-ITEM-REF-1. MAP-CUTOVER-1b checks
  the palette ids against the index at boot.
- The ARCH-MAP-WIRE §2.3 pointer note is updated to match.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
