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

- Narrowed at Codex round 5 (control-plane scope cut): state shape, reserved causes, closed operation
  list and fail-closed gate only.
- Table `game_character_proficiency_modifications`: PK (character, item key, slot), UNIQUE
  (character, item key, level); slot 1 at level 3, slot 2 at Mastery; `MODIFIED_LEVEL`.
- Shaping revisions retained while referenced; incompatible definition revisions clear the row in the
  migration receipt.
- Reserved `perk_modification` and `ProficiencyCause`; no operation command, refusal or wire exists
  and catalysts have no use until PROFICIENCY-1B.
- PROFICIENCY-1B entry conditions: CommandId-derived occurrence, revision binding with a persisted
  terminal refusal, receipt CHECKs, offers (rank frozen while pending), shaping-revision migration,
  draws, composed shapes, measured wire bounds, values.
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
