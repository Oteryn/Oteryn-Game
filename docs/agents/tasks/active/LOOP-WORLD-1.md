# LOOP-WORLD-1

```yaml
task_id: LOOP-WORLD-1
title: "LOOP-WORLD-1: teleports, keyed/level/quest doors, levers and tile damage on the bundle node (mechanics plan row 65)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/loop-world-1-20261014
issue: 1622
governing_issue: 1622
pr: null
base_sha: 75ec7985
head_sha: null
final_head_sha: null
owner: claude-code-worker (CP session_01XYC239givrMonrYoBwwECV)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/LOOP-WORLD-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Waiting on CP/owner answers before any code. The entry gate is met (#2018, ENCOUNTER-RT-0
ACCEPTED #2108, BUNDLE-COMBAT-ENABLE-1 #2025, CHAT-1b-2b #2093). The earlier branch
`claude/loop-world-1-worldint-wire-1-20261013` is gone: its slice merged as WORLDINT-WIRE-1 (#2094),
so this task starts fresh from main@75ec7985.

The only child of the appendix graph (WORLD_INTERACTION0 + ENCOUNTER_RT0) whose listed
prerequisites are all merged is **KEY-1** (DOOR-1 #1966, CHEST-1 #1217). It is not a small slice
and is held on the questions below.

## Architecture and source of truth

- PROVEN: WORLD_INTERACTION0 §4.3-4.4 and §12 (`WORLDINT0-RL-10`, `DUR03-RL-07-KEY`) define KEY-1:
  an immutable instance `key_number` set at a RewardClaim MINT, carried by TRANSFER, matched on
  USE-WITH against a key door's binding.
- PROVEN: `game_item_instances` (0010) has no attribute column; the only typed per-item state is the
  side table pattern of 0054 (`game_item_timed_states`). `OneItemStateV1` /
  `OneItemRewardClaimMintV1` (game-events contract) carry no key number. KEY-1 therefore needs a
  migration (0086 or later; #2110 holds 0085), a game-events evidence change and resource-limit rows.
- PROVEN: the map drops a placement's action id at runtime (`map/facts.rs` `Placement { bound }`);
  a locked door is a solid, non-door placement. `connection.rs` rejects every `use_with` and
  `use_with_map_item` before any handler; the `map_item` arm is "not offered before WORLDINT-USE-1".
- PROVEN: content already models keys per number as definitions (`oteryn:item.tibia.i21392`,
  "silver key (Key:0010)", materializable, not stackable); 27 authored reward-claim variants carry
  `reward.key_binding` (`canary:door-key/NNNN`), `readiness: waiting_implementation`, skipped by
  `content/world_reward_claims.rs` (`LeftOut::Variant`).
- CONFLICT: the GAME-ITEM-01 §4 and DUR-03 §39.1 key amendments say "pending on acceptance of
  WORLD-INTERACTION-0"; that decision (ACCEPTED D1019) says its amendments "stay pending on their
  own documents".

## Children: status and single blocker

| Child | Status | Single blocker |
|---|---|---|
| WORLDINT-WIRE-1 | merged #2094 | none |
| DOOR-1 | merged #1966 (normal doors) | none |
| FLOOR-1 | merged #1967 (teleports also #2018) | none |
| KEY-1 | prerequisites merged; held | owner questions Q1-Q3 below; its door half also needs USE-WITH on a map item (WORLDINT-USE-1) |
| WORLDINT-USE-1 | blocked | WO-3 |
| LEVER-1 | blocked | QUEST-TRIGGER-1 |
| PUSH-1 | blocked | VSL-MOVE-01 movement owner (implementation not started) |
| GROUND-MOVE-1 | blocked | ITEM-MOVE-2b |
| WORLDINT-ADMIT-1 | blocked | FIELD-WIRE-1 |
| FIELD-2 | blocked | FIELD-1 |
| HAZARD-1 | blocked | LEVER-1 |
| CLOCK-1 residue | in flight | open PR #2114 |
| WORLDINT-CONTENT-1 | blocked | WO-3 |
| ENC-RT-1 | blocked | the typed Encounter profile (E1) not found on main |
| ENC-COMBAT-1 | blocked | ENC-RT-1 |
| ENC-WORLD-1 | blocked | LEVER-1 |
| ENC-OUTCOME-1 | blocked | BOSS-REWARD-1 |
| INSTANCE-GROUND-1 | blocked | GROUND-MOVE-1 |
| BOSS-LEVER-1 | blocked | LEVER-1 |
| ENC-PARITY-1 | blocked | every child above |

Level and quest doors (QUEST-GATE-1) are not a child of this graph; DOOR-1's QUEST-GATE-1 gate
is unmerged, so they stay sealed.

## Owner questions (sent to CP)

- Q1: Key number at instance (decision §4.4: side table, migration 0086+, game-events change,
  `DUR03-RL-07-KEY` and `WORLDINT0-RL-10` rows) or at definition (content already has per-number
  key definitions such as i21392; no migration, number is immutable via the definition)?
- Q2: Are the GAME-ITEM-01 §4 and DUR-03 §39.1 key amendments in force (CONFLICT above)?
- Q3: Allocate the KEY-1 door half (USE-WITH a key on a locked door, runtime key binding from the
  placement action id) now under capability 15, or keep it behind WORLDINT-USE-1 as the
  WORLDINT-WIRE-1 gate says? Without it KEY-1 mints keys that open nothing.

## Validation

- `python tools/agents/validate_governance.py`
