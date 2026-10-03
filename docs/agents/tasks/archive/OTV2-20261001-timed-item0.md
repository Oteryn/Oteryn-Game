# OTV2-20261001-timed-item0

```yaml
task_id: OTV2-20261001-timed-item0
title: "TIMED-ITEM-0 charges, duration, equip forms and repair"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-timed-item-0
pr: 1471
base_sha: aad17f99
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261001-timed-item0.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_EQUIP0_EQUIPMENT_EFFECTS_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_ITEM_MOVE_WIRE1_EQUIP_AND_DROP_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_OFFLINE0_STAMINA_AND_OFFLINE_TRAINING_DECISION_2026-10-01.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

TIMED-ITEM-0 decides items that run on charges or time, and the soft boots repair (owner answer 1a,
#162 5932648083).

- **Scope (narrowed by D282 and D285):** catalogue facts, the timed-row table shape and its
  invariants, and the NPC soft boots repair. All runtime persistence of charges and duration
  (clocks, checkpoints, expiry, equip forms, the active rule, conditions, wire) and continuous
  duration move to TIMED-ITEM-0B, with entry conditions in §5.
- **State:** `game_item_timed_states`, lazy rows (no row means full values), absent row =
  revision 0, monotonic revision, rows never deleted while the item lives.
- **Repair:** `FeeBurnCause::NpcRepair`; worn soft boots to soft boots for 10,000 gold.
- **Rulings:** R1 (runtime persistence to TIMED-ITEM-0B, D285), R3, R4; no open owner question.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: GAME-ITEM-01 §4.2 and §4.4; EQUIP-0 §3.2; DUR-03 §11.1, §15, §16.2; NPC-0 §5; owner
  answer 1a; Tibia manual.
- `OTS_HYPOTHESIS_ONLY`: Canary `items.xml`, `player.cpp` `blockHit`, decay in the game world,
  Aldo's repair.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. TIMED-1 needs persistence and combat review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, combat, protocol).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code; exercise weapons (EXERCISE-0); invisibility from equipment; imbuement durations.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.
- `git diff --cached --check`: clean.

## Closeout

- PR: #1471. Review: Codex round 1 (5382769280) on `e8212a64` answered in the next head; the exact frozen heads are in the #162 FREEZE_SHA entries. Merge commit/result: its squash merge.
- Amendments, each pending on acceptance of TIMED-ITEM-0: DUR-03 §15 and §39.3; ITEM-USE-0 §6;
  NPC-0 §6.2; OFFLINE-0 scope. The EQUIP-0, CONDITIONS-0, ITEM-MOVE-WIRE-1 and PLAYER-TRADE-0
  amendments of earlier heads were withdrawn with D285 and go to TIMED-ITEM-0B.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-timed-item-0
owner_action_required: null
blocker: null
next_action: null
```
