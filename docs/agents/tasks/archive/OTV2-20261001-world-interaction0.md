# OTV2-20261001-world-interaction0

```yaml
task_id: OTV2-20261001-world-interaction0
title: "WORLD-INTERACTION-0 doors, levers, fields and the World clock"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-world-interaction-0
pr: "1432"
base_sha: b73fe23c
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_018aTt5eRKVUGPJJYwwoMcqw (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261001-world-interaction0.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/architecture/reviews/OTERYN_GAME_RUNE_USE0_USING_RUNES_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

WORLD-INTERACTION-0 decides the first slice of `GAME-INTERACTION-01` at Tibia parity, applying
owner answers 6a (world-object state per channel, reset at server save; quest progress durable per
character) and 7a (one Tibia clock per World from a World epoch, 2.5 real minutes per game hour).

- **Interaction model:** same floor, Chebyshev 1; own cooldown keys `world_action` 200 ms,
  `world_ex_action` 1,000 ms, `push` 1,000 ms; 512 acts per channel per tick; the PZ block bars
  every entry into a PZ, landings included (ruling R1).
- **Doors:** normal, key (locked, closed, open; immutable instance `key_number` minted by a
  RewardClaim, R2), quest and level (QUEST-GATE-0), Premium and vocation gates; door tiles refuse
  items (R3).
- **Levers and plates:** triggers with a closed child set of at most 16 (overlay operations,
  relocation, quest transition, presentation, environment damage); no script engine.
- **Floors and tools:** walk-on floor changes, ladders and grates, rope (8 ordered candidates),
  shovel, pick, machete, climbing at height 3, teleports (D216), a one-hop landing rule.
- **Pushing and moving:** creature push; Ground to Ground TRANSFER of durable items and overlay
  moves of movable base items (15 / 2 tiles).
- **Fields:** player-affecting fields, Magic Wall and Wild Growth, creature-made fields, traps;
  gated by an FND-04 admission rule (WORLDINT-ADMIT-1).
- **Clock and light:** `tibian_minute = floor((t - epoch) / 2,500 ms) mod 1,440`; Canary's light
  curve as a pure function; domain `WORLD_CLOCK` and a VIS-2 `light` field under `WORLD_LIGHT_V1`.
- **Owner questions:** none; R1-R5 are architect rulings under owner rule 5905825574.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: owner answers 6a and 7a (#162, 2026-10-01); ADR-0021 §4.4-§4.7; the relocation and
  world-object owners proposal (D37, D38, §7); WO-0 and D216; QUEST-GATE-0; QUEST-STATE-0;
  RUNE-USE-0 §11; MAP-WIRE-1; ITEM-MOVE-WIRE-1; PARTY-PVP-0; PREMIUM-ACTIVATION; GAME-ITEM-01 §4;
  SIM-DETERMINISM-01 §15.
- `DERIVED`: the Tibia manual (`controls.md`, `combat.md`, `world.md`, `interface.md`); TibiaWiki
  (Time, Key, Magic Wall Rune, Wild Growth Rune, Gate of Expertise, Jungle Grass, Loose Stone Pile,
  Rope Spot, Light; fetched 2026-10-01); Canary `04b83b51` (`OTS_HYPOTHESIS_ONLY`, file and line
  cited).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. KEY-1 and GROUND-MOVE-1 need persistence review; WORLDINT-ADMIT-1
protocol and security review with the FND-04 owner; WORLDINT-WIRE-1 and CLOCK-1 protocol review;
FIELD-2, PUSH-1 and HAZARD-1 combat review; WORLDINT-USE-1, DOOR-1 security review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (protocol, security, persistence, combat, determinism).
- [ ] Protected Merge Queue integration.
- [ ] Capability, command, domain and field numbers reserved on #162 at child allocation.

## Excluded scope

- Code, migrations and content; carried torches and timed items, the Trap item, digging and harvest
  yields, writing, beds, boss levers and rooms, Destroy Field, items lost in water.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.
- `git diff --cached --check`: clean.

## Closeout

- PR: #1432. Merge commit/result: its squash merge.
- Amendments, each pending on acceptance of WORLD-INTERACTION-0: the GAME-INTERACTION-01 horizon
  section, the scope matrix, RUNE-USE-0 §11, QUEST-GATE-0 §3.1, PARTY-PVP-0 §7.2, DUR-03 §39.1,
  GAME-ITEM-01 §4. The FND-04 admission amendment belongs to WORLDINT-ADMIT-1.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-world-interaction-0
owner_action_required: null
blocker: null
next_action: null
```
