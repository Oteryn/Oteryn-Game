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

- **Scope (narrowed at round 7):** charges, `on_equip` duration, equip forms, expiry and the repair;
  `continuous` duration (lit torches, Ground and house deadlines, tree clocks) is `NOT_ADMITTED`
  until TIMED-ITEM-0B.
- **State:** `game_item_timed_states` (charges, active-time budget), keyed by ItemInstanceId,
  written only by one-item DUR-03 shapes under the closed `TimedItemCause`; backfill by one-item
  rows before the guard is enabled.
- **Clocks:** time runs only while equipped in the game world; the runtime commits at A13
  checkpoints and checkpoints a live item before it leaves its slot, so moves carry no timed value.
- **Charges:** one per hit an active item's protection reduces; at 0 the item expires (transform, or
  a one-item BURN aggregate when there is no decay target).
- **Forms:** equip/unequip transform with `PRESERVE_INSTANCE`, keeping the time; a ring-for-ring
  swap is refused `SWAP_TIMED_BOTH`.
- **Repair:** `FeeBurnCause::NpcRepair`; worn soft boots to soft boots for 10,000 gold.
- **Active rule:** EQUIP-0 grants a timed item's abilities while it has charges and time;
  `REGENERATION` and `MANA_SHIELD` added.
- **Wire:** `TIMED_ITEMS_V1` adds charges and remaining time to item presentations and Look.
- **Rulings:** R1-R5 (exercise weapons to EXERCISE-0; continuous duration to TIMED-ITEM-0B); no open owner question.

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
- Amendments, each pending on acceptance of TIMED-ITEM-0: EQUIP-0 §3.2; PLAYER-TRADE-0 §4; DUR-03 §15, §33, §39.3;
  ITEM-MOVE-WIRE-1 §6; ITEM-USE-0 §6; NPC-0 §6.2; CONDITIONS-0 §3; OFFLINE-0 scope.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-timed-item-0
owner_action_required: null
blocker: null
next_action: null
```
