# OTV2-20261005-arch-item-sem-bed-packet-1

```yaml
task_id: OTV2-20261005-arch-item-sem-bed-packet-1
title: "ITEM-SEM-BED-PACKET-1: bed-part semantics group 19, artifact v7 and resource profile V4 packet"
mode: ARCHITECTURE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-item-sem-bed-20261005
issue: 162
pr: 1847
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-item-sem-bed-packet-1.md
depends_on: []
blocks: [OTV2-20261005-item-sem-bed-1-bed-group, OTV2-20261004-bed-content-1]
```

## Outcome

- Answers the control plane request (#162, 2026-10-05) for ITEM-SEM-BED-1 and the BED-CONTENT-1
  input on its 39 held bed types.
- Group 19 `bed` holds `part`, `partner_direction`, `occupied_male` and `occupied_female`. The
  free look is the placed Item, and halves pair by direction. Occupied targets carry the same
  part and direction (set rule, enforced at compile and at load). Amends BED-0 §3 and §8.
- Artifact v7 (`OTRPA07\0`) and resource profile V4. The cross-Item target rows are V3 = 13
  (correction for v6) and V4 = 15.
- Lowering follows the engine's parse and apply steps from Canary `04b83b51` `items.xml`. A
  missing, zero or non-bed target means "no change", as `BedItem::updateAppearance` does.
  `bedpartof` is not read.
- Holds: the 34 `bedpartof` holds go away, and the 7 non-bed targets lower as "no change". 370
  of 377 bed types lower; every placed and house bed type is covered.
- BED-CONTENT-1 now depends on ITEM-SEM-2b and ITEM-SEM-BED-1.
- Codex round on b163ca47 is fixed: §1.5 cites the apply step (`bed.cpp`) for non-bed targets
  such as 743 → 727, and BED-CONTENT-1 lists and tests them (#1847 review 4186980432).
- Owner acceptance: not required (V2 and V3 precedents). An independent contract review on the
  frozen head is required.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
