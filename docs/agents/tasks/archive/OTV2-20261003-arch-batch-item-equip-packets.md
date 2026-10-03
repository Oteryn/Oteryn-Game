# OTV2-20261003-arch-batch-item-equip-packets

```yaml
task_id: OTV2-20261003-arch-batch-item-equip-packets
title: "Architect batch: item view and move, equip, drop, speed, bags and exercise packets"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-batch-item-equip-packets
issue: 1622
pr: 1698
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-arch-batch-item-equip-packets.md
public_contracts: []
depends_on: []
blocks: [ITEM-VIEW-1a, ITEM-VIEW-1b, ITEM-MOVE-1, ITEM-CLIENT-1, ITEM-EQUIP-WIRE-1, ITEM-MOVE-2a, EQUIP-CONTENT-1, SPEED-1, EQUIP-RT-1, ITEM-MOVE-2b, BAGS-WIRE-1, BAGS-1, EXERCISE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **Order by playable value:** loot a corpse (ITEM-VIEW-1a, 1b, ITEM-MOVE-1, ITEM-CLIENT-1), then
  wear it with its effects (ITEM-EQUIP-WIRE-1, ITEM-MOVE-2a, EQUIP-CONTENT-1, SPEED-1, EQUIP-RT-1),
  then bags, drop and exercise.
- **Startable now:** ITEM-VIEW-1a, ITEM-SEM-2b-2 (#1672 rebased onto `main`, since carrier #1675
  replaced #1599) and SPEED-1.
- **Rulings:** ITEM-VIEW-1 splits into a wire slice and a server slice, and ITEM-MOVE-1 offers
  capability 4; ITEM-MOVE-1 needs no migration; 2a, 2b, BAGS-1 and EXERCISE-1 need one each;
  EQUIP-CONTENT-1 waits for ITEM-SEM-2b-2 and TIMED-CONTENT-1; 2a matches promoted vocations; the
  ARCH-SLOT-WIRING-1 call-site conditions apply as written.
- **Proposed leases:** migrations 0063 (2a), 0064 (BAGS-1), 0065 (2b), 0066 (EXERCISE-1);
  capabilities 12 `ITEM_EQUIP_DROP_V1`, 13 `PACED_MOVEMENT_V1`, 14 `CONTAINER_TREE_V1`; state
  domain 14 and command 21 for BAGS-WIRE-1. D212 already assigns capability 4, domains 9 and 11
  and command 9.
- **Not packeted (§1.8):** DEPOT, IMBUE, the rest of FORGE, EXERCISE-CONTENT-1, the parity and
  later BAGS slices, each with what it waits on. MAP-LOAD-1, ITEM-USE-WIRE-1 and GOLD-FEE-2 are
  the roots that block most of them.

- **Codex round 1 (2 × P1, 1 × P2), fixed in one push.**
  - 4175114690: the leases follow the expected merge order (BAGS-1 0064, 2b 0065). A migration
    packet merges only above every migration on `main`, and otherwise re-leases and refreezes
    (§0.1).
  - 4175114699: EQUIP-RT-1 owns the admission, reconnect, release and Premium-refresh paths,
    with a test per lifecycle trigger.
  - 4175114706: ITEM-CLIENT-2 (equipment), -3 (nested bags) and -4 (drop and Ground pickup) are
    packeted (§2.7a).

## Validation

- `python3 tools/agents/validate_governance.py`: pass
- `python3 -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass
