# OTV2-20261003-arch-slot-wiring-1

```yaml
task_id: OTV2-20261003-arch-slot-wiring-1
title: "ARCH-SLOT-WIRING-1: the second of TIMED-RT-1b and ITEM-MOVE-2a wires the slot call sites"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-slot-wiring-1
issue: 1622
pr: 1696
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_TIMED_FORGE_PROF_PACKETS_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_ITEM_MOVE_WIRE1_EQUIP_AND_DROP_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20261003-arch-slot-wiring-1.md
public_contracts: []
external_repositories: []
```

## Outcome

This task carries #1692's round-3 P1 4174906452. The control plane routed it to this follow-up
under D317 and chose option (a).

- The finding: ITEM-MOVE-2a's packet did not depend on TIMED-RT-1b. So if ITEM-MOVE-2a merged
  first, nothing owned the slot wiring.
- The fix: whichever of the two merges second wires the `timed_item_host` slot call sites, as a
  merge condition with tests.
  - An equip or swap into a slot makes the item live.
  - An unequip or swap out of a slot stops the item, then waits until its lane is empty; a
    rejected move makes it live again from the row (round 1, #1696 P1s).
  - A drop from a slot to Ground is a move out of a slot: whichever of RT-1b and ITEM-MOVE-2b
    merges second wires it under the same rules (round 2, #1696 P1 4175041166; WIRE1 §5).
- The condition is written into both packets: the bundle's §1.1 and §2.3, and ITEM-MOVE-WIRE-1 §4.
  No ordering is added, so ITEM-MOVE-2a, FORGE-1b and TIMED-RT-1c are not blocked on RT-1b.

This changes no code, no contract, no wire and no resource value.

## Validation

- `python3 tools/agents/validate_governance.py`: pass
- `python3 -m unittest discover -s tools/agents/tests`: 54 tests, OK
- `git diff --check`: pass
