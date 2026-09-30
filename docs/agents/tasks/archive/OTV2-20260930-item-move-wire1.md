# OTV2-20260930-item-move-wire1

```yaml
task_id: OTV2-20260930-item-move-wire1
title: "ITEM-MOVE-WIRE-1 equip and drop"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-item-move-wire1
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: 1852a69d
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ITEM_MOVE_WIRE1_EQUIP_AND_DROP_DECISION_2026-09-30.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
  - docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md
  - docs/agents/tasks/archive/OTV2-20260930-item-move-wire1.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ITEM-MOVE-WIRE-1 lets a player equip, unequip, drop and pick up whole items (architect programme
plan, #162 5910870596, M1). It extends ITEM-MOVE-WIRE-0 (PR #1344) and integrates after it.

- **Wire.** A new capability `ITEM_EQUIP_DROP_V1` (number at allocation) adds the `EQUIPMENT
  {slot}` and `GROUND {position}` destinations to command 9 and nine slots to domain 9.
- **Equip.** Content slot and hands through `equipment.rs`; hands conflicts refused; containers not
  equipped; a swap moves only the occupant; requirements checked in the transaction.
- **Ground.** Drops within 15 tiles with line of sight; pick-up next to the item; per-tile and
  per-channel limits; D191 `WorldReset` retires dropped items; actors rank before items in D87.
- **Persistence.** The §39 supersessions, the per-child persistence deltas and fixed resource rows.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: ITEM-MOVE-WIRE-0; DUR-03; B3; ADR-0021 D191; migrations `0010`, `0011`, `0023`;
  `equipment.rs`; the MOVE-RL-11 decision; content item semantics.
- `DERIVED`: the Tibia manual; Canary `04b83b51` (`OTS_HYPOTHESIS_ONLY`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. ITEM-MOVE-2a and 2b need persistence review; ITEM-EQUIP-WIRE-1
protocol review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (protocol and persistence).
- [ ] Protected Merge Queue integration, after #1344.

## Excluded scope

- Code, migrations and content; partial counts, reordering, bags, weight, Ground to Ground, corpse
  or Ground into a slot, map items, depot, trade.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only) of the first draft: 7 material findings (duplicate
  cleanup sink, unfenced container slot, swap rule, split identity, §39 aggregates and rows,
  migration contradictions, protocol) and 6 others, all fixed.
- Second pass: 5 material findings (swap room, split overflow, row values, entry deletion and
  guard deltas, capability ordering) and 5 others; fixed by cutting partial counts and slot
  pick-up to later decisions, a new capability, fixed row values, a complete delta list, the D87
  order amendment and the lock order.
- Control-plane rule (#162 5912405163): the DUR-03, composition and MOVE-RL-11 edits read
  "pending on acceptance of ITEM-MOVE-WIRE-1".
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-item-move-wire1
owner_action_required: null
blocker: null
next_action: null
```
