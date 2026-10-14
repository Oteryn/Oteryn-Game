# LOOP-WORLD-1

```yaml
task_id: LOOP-WORLD-1
title: "LOOP-WORLD-1: teleports, keyed/level/quest doors, levers and tile damage on the bundle node (mechanics plan row 65)"
mode: IMPLEMENT
status: implementing
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
  - apps/game-server/migrations/0086_item_keys.sql
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
  - apps/game-server/src/durability/
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/world_reward_claims.rs
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/tests/content_reference_playable.rs
  - apps/game-server/tests/world_reward_claims_binding.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

KEY-1, mint half (CP Q3). A RewardClaim MINT can carry an immutable instance `key_number`
(1..65,535, `WORLDINT0-RL-10`) on one non-stackable item: migration 0086 `game_item_keys` (written
only with the reward-claim MINT of its one item in the same transaction; immutable, no delete or
truncate), `OneItemStateV1.key_number = 6` (0 = not a key, so every existing event encodes
byte-identically), the MINT intent binding (`0x4b` + u16, absent for non-keys), the audit gates
(loot and map MINT never carry a number; TRANSFER carries it unchanged) and the TRANSFER source
read. The binder admits the 21 authored key variants whose only pending lowering is
`KEY_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED` and whose binding is `canary:door-key/<n>`.

Held: open the PR only after #2110 (migration 0085) merges; the resource-limit rows
`WORLDINT0-RL-10` and `DUR03-RL-07-KEY` wait for the #2113 registry throttle.

Earlier: waiting on CP/owner answers before any code. The entry gate is met (#2018, ENCOUNTER-RT-0
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
| KEY-1 mint half | this task | PR held until #2110 (migration 0085) merges |
| KEY-1 door half | blocked | WORLDINT-USE-1 (USE-WITH on a map item) |
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

## CP decisions

- Q1: instance-level number per ACCEPTED §4.4, no deviation; migration 0086 built locally.
- Q2: architect ruling: WI0 §4.4/§7.2 are ACCEPTED (D1019); the GAME-ITEM-01 §4 and DUR-03 §39.1
  amendment blocks are not yet in force as those contracts' text. KEY-1 implements WI0 §4.4/§7.2;
  the amendments stay untouched. Open to CP: must a docs-only flip of those two markers land first?
- Q3: KEY-1 here is the mint half only; the door half waits for WO-3 / WORLDINT-USE-1.

## Residue

- Other producers (item_use, decay, npc_trade, timed, fee_burn) emit `key_number` 0. Keys do not
  decay or transform in the reference evidence; NPC sale of a key is unverified.
- Left out: `oteryn:reward-claim.quest.key.id3002` (binding `crystalserver:door-key/3002`, a
  namespace the binder does not admit) and the 4 key variants that also wait for container,
  instance or timed lowering.
- The world_reward_claims reader is not wired into runtime boot yet (tests only).

## Owner questions (answered by CP)

- Q1: Key number at instance (decision §4.4: side table, migration 0086+, game-events change,
  `DUR03-RL-07-KEY` and `WORLDINT0-RL-10` rows) or at definition (content already has per-number
  key definitions such as i21392; no migration, number is immutable via the definition)?
- Q2: Are the GAME-ITEM-01 §4 and DUR-03 §39.1 key amendments in force (CONFLICT above)?
- Q3: Allocate the KEY-1 door half (USE-WITH a key on a locked door, runtime key binding from the
  placement action id) now under capability 15, or keep it behind WORLDINT-USE-1 as the
  WORLDINT-WIRE-1 gate says? Without it KEY-1 mints keys that open nothing.

## Validation

- `python tools/agents/validate_governance.py`
