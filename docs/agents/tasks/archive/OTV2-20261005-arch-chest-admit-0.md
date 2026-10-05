# OTV2-20261005-arch-chest-admit-0

```yaml
task_id: OTV2-20261005-arch-chest-admit-0
title: "ARCH-CHEST-APPEARANCE-ADMIT-V1: admit chest appearances 28827 and 28828, ADR-0021 §4.5 palette-key amendment"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-chest-admit-20261005
issue: 162
pr: 1834
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_CHEST_APPEARANCE_ADMIT_2026-10-05.md
  - docs/architecture/ADR-0021-world-map-runtime-loading.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-chest-admit-0.md
public_contracts: []
depends_on: []
blocks: [OTV2-20261005-chest-appearance-admit-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Answers the control plane's readiness request for CHEST-APPEARANCE-ADMIT-1, which had no packet
  body.
- Rules that appearances 28827 and 28828 are admitted as appearance-only Items under A12 §4.1,
  with the #1795 cascade.
- Amends ADR-0021 §4.5: a palette id with no `ots/item_server_id` binding takes its A12 Item key
  `oteryn:item.tibia.i<id>` when that Item record exists. This sits after the binding and before
  the Terrain, WorldObject and donor fallbacks, and duplicate keys still fail.
- The packet owns every file the #1795 cascade moved, including the TibiaWiki navigation facts
  and the world-object qualification pins (#1834 review 4183308722).
- Packet CHEST-APPEARANCE-ADMIT-1 (impl worker), after #1830 and #1805 merge, with one writer on
  `content/world/pins/`.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
