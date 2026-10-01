# OTV2-20261001-proficiency1

```yaml
task_id: OTV2-20261001-proficiency1
title: "PROFICIENCY-1 perk modification, the Lunar Ascension Orb and catalysts"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-proficiency-1
pr: 1475
base_sha: aad17f99
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY1_PERK_MODIFICATION_AND_CATALYSTS_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261001-proficiency1.md
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Up to 2 modification rows per track (slot 1 at level 3, slot 2 at Mastery), receipt cause
  `perk_modification`.
- Operations MODIFY, RANK_UP, ORB_RANK, RESHAPE_OFFER, RESHAPE_CHOOSE, CLEAR: one Character
  transaction each; draws under `proficiency_shaping`; durable paid reshape offer; replay by
  occurrence; the full revision set bound at reservation (`REVISION_CHANGED`).
- Value shapes not admitted: `ProficiencyCause` is reserved; the value operations refuse
  `NOT_ADMITTED` until a later DUR-03 amendment admits each composed shape with its evidence.
- Migration lines carry the modification rows; no catalyst is admitted.
- Fail-closed admission gate until costs, pools, odds and effects are evidenced; a gold cost needs an
  owner answer (D178).
- No owner question now.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Closeout

- PR: #1475. Review: Codex 5382980334 on `55fdf751` answered in the next head; frozen heads in the #162 FREEZE_SHA entries. Merge commit/result: its squash merge.
- Amendments, pending on acceptance: PROFICIENCY-0 §4.5; DUR-03 §15; IMBUE-FORGE-0 §9.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-proficiency-1
owner_action_required: null
blocker: null
next_action: null
```
