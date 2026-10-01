# ENCOUNTER-RT-0 Encounter runtime and boss levers

- Decision: `ENCOUNTERRT0-ENCOUNTER-RUNTIME-AND-BOSS-LEVERS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (determinism,
  combat, persistence and security) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the base-mechanics close-out plan (#162 5929069698). BOSS-RAID-0 §6.5 names this
  decision ("Its runtime is ENCOUNTER-RT-0"); WORLD-INTERACTION-0 §5 leaves boss levers out; the
  encounter format (§9, §12.7 runtime obligations, §13.7) defines the vocabulary but no runtime. By
  E3 every encounter-bound creature, every boss with mechanics among them, waits for it.
- Builds on: the encounter authoring format (D26-D34, D45, D46, CW2-1..4, SW-3..6); the WorldProject
  v2 encounter admission (E1-E4); BOSS-RAID-0 §6-§8 (boss rooms, admission, cooldowns, lifetime,
  contribution, reward chest); CREATURE-AI-0 §3, §4.6, §7, §8 (owner, randomness, budgets,
  summons); WORLD-INTERACTION-0 §5 (levers, overlay operations, reverts) and §9; HOUSE-RUNTIME-0 §4
  (SCOPE-HANDOFF-1); the instance baseline; SIM-DETERMINISM-01; DUR-03 §32, §39 and the ADR-0021
  `WorldReset` amendment; D3 (D130-D136); CHAT-0; owner rule 5905825574 (Tibia parity).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| ENC-RT-1 | hard (determinism), determinism review | the encounter instance, its state, the trigger queue, delayed rules and timers, the RNG purpose, faults (§3-§5), conditions and the internal actions (§6.1) | CREATURE-AI-1; the typed Encounter profile (E1) |
| ENC-COMBAT-1 | hard (combat), combat review | the two inline damage hooks and the encounter modifier stage (§4.2, §6.2); spawn, transform, remove, heal, damage, cast, `move_lock`, `attribute` through their owners (§6.2) | ENC-RT-1; COND-1; MONSTER-SUMMON-1 |
| ENC-WORLD-1 | impl, determinism review | `map_item`, teleport, `say`, `message`, `stepped_on`, `area_entered`/`area_left`, `item_used`, corpse provenance (§6.3) | ENC-RT-1; LEVER-1; FLOOR-1; CHAT-1; ITEM-USE-1 |
| ENC-OUTCOME-1 | hard (persistence), persistence and security review | outcome delivery to the reward, Bosstiary and quest consumers (§6.4); the `drop_item` MINT (§6.5) | ENC-RT-1; BOSS-REWARD-1; QUEST-STATE-1 |
| INSTANCE-GROUND-1 | hard (persistence), persistence review | Ground custody in an InstanceRuntime scope and the `InstanceRetire` cause (§8) | SCOPE-HANDOFF-1; GROUND-MOVE-1 |
| BOSS-LEVER-1 | impl, security review | the `BOSS_ENTRY` lever child (§7) | LEVER-1; BOSS-ROOM-1 |
| ENC-PARITY-1 | impl | fixtures against Canary and TibiaWiki: a transform boss (Urmahlullu), a prevent-death boss, a timer boss (King Zelos), a lever room for 5, the Soul War zone rule | every child above |

Tests: one fight replayed from its seed gives the same rule order, draws and outcomes; a trigger
loop over `ENCRT0-RL-01` faults the instance without an outcome; a crash loses the fight and
refunds nothing; an encounter-bound creature never exists without its live encounter instance.

Later, each with its own decision: scripted movement (D29), state shared by all parties beyond the
quest domain's `world_state` (D29), the Tibiadrome and World Changes (BOSS-RAID-0), Hazard.

## 1. Question

Who runs an encounter's rules, in what order and with which randomness; how do its actions reach
the owners of creatures, combat, the map, chat and items; how does a boss lever admit a group; and
what happens to items on the floor of a boss instance?

## 2. Facts

**PROVEN**

- Format §2.4: encounter state lives with one encounter instance and is discarded on reset; it is
  never character state. D26: `instance_per_party` by default, `channel_shared` explicit. Of the 83
  transcribed encounters, 73 are `instance_per_party` and 10 `channel_shared`.
- Format §9: rules run in listed order, a trigger fires its rules once per occurrence, actions run
  in order; `delay_ms` schedules a rule once per occurrence; draws are uniform and "a fight can be
  audited and replayed from its seed"; anchors must be bound before activation.
- Format §2.5, D27: character and account effects are not executed by the encounter; it emits named
  outcomes that the reward and quest domains consume under their own contracts.
- Format §12.7: runtime obligations: keep a player's identity through a rule delay and define
  instance membership for `in_anchor` (CW2-1); corpse provenance through decay (CW2-2); the item
  identity of a `stepped_on` (CW2-3); a free-tile query (CW2-4). §13.7: per-player state in a
  `channel_shared` instance.
- E3: "an encounter-bound creature is never activated without its encounter".
- Transcribed rule use (`tools/content-schema/encounter-authoring/samples`): triggers
  `creature_died` 164, `health_crossed` 33, `damage_taken` 28, `creature_spawned` 28,
  `timer_elapsed` 26, `heal_received` 23, `ability_cast` 13, `lethal_damage` 12; actions `spawn`
  351, `counter` 96, `remove` 68, `flag` 58, `say` 54, `emit_outcome` 51, `timer` 46, `map_item`
  41, `teleport` 27, `damage_modifier` 25, `drop_item` 22 (Ugly Monster's food), `heal` 21,
  `transform` 20.
- BOSS-RAID-0 §6: a boss room is an activity instance per admission (SCOPE-HANDOFF-1); admission by
  lever in one transaction with cooldowns; lifetime until reset or `time_limit_s`; exit to the
  origin channel; a crash loses the instance and refunds no cooldown. §7-§8: contribution and the
  reward chest. BOSS-RAID-0 does not say where the Ground items of an instance live.
- CREATURE-AI-0 §3: every creature has one scope owner; an encounter creature links to its
  encounter, "whose rules override this decision where they say so". §4.6: SIM purposes seeded by
  (actor, sequence, index). §7: 50 ms owner windows. §8.1: encounter summons stay with the
  encounter runtime.
- WORLD-INTERACTION-0 §5: lever children are overlay operations on anchors with `revert_after`,
  relocations, quest transitions and presentation, per channel; "Boss levers (room checks,
  `SCOPE_HANDOFF`) are not in this decision."
- DUR-03: Ground custody is fenced by the channel scope (§32); the `WorldReset` amendment admits a
  second retirement cause beside D136's `CorpseDecay` on the shared retirement tables.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`)

- `BossLever:onUse` (`data/libs/functions/boss_lever.lua:153-264`): the user must stand on one of
  the lever's player positions; a room with a player in its zone refuses ("There's already someone
  fighting"); each player on the positions is checked for level and cooldown, any failure refuses
  all; too few players refuses; then the zone's monsters are removed, the boss and waves are
  created, the players teleported, the cooldown set for all, and a timer of `timeToDefeat` kicks
  every player from the zone.
- `Lever:checkPositions` (`lever.lua:112-141`): the participant of a position is the tile's bottom
  creature.
- Creature events run synchronously inside the damage and death paths: `onPrepareDeath` before the
  death, `onHealthChange` with the final primary and secondary damage before the health change;
  `addEvent` runs later in the dispatcher.

## 3. The encounter instance (ENC-RT-1)

### 3.1 Identity and owner

- An **encounter instance** is `(WorldId, scope ref, encounter key and revision, instance
  sequence)`. The scope ref is the ChannelId with its scope ownership generation, or the
  `InstanceId`. The instance sequence is monotonic within the scope ownership generation.
- Its owner is the scope's runtime owner (`ChannelRuntime` or `InstanceRuntime`), the owner of its
  creatures (CREATURE-AI-0 §3). Only that owner runs its rules, inside its turns. It writes nothing
  durable of its own (§9).
- **`instance_per_party`:** exactly one encounter instance per boss-room InstanceRuntime, created
  with the instance (BOSS-ROOM-1) and ended with it.
- **`channel_shared`:** one encounter instance per channel scope ownership generation, created at
  channel activation; a reset (§3.3) replaces it with the next instance sequence.

### 3.2 Activation

An encounter instance is created only when its encounter is admitted for the active content
generation (E4) and every anchor is bound in that scope's map (format §9.5). Otherwise it is not
created, its encounter-bound creatures are not activated (E3), its lever refuses (§7), and the
owner raises an alarm. Nothing falls back to raw coordinates or to a creature without its rules.

### 3.3 Lifecycle

- **Start:** the format's `start` (first participant engaged, or an anchor entered) fires
  `encounter_started` once.
- **Reset** (no player in the arena anchor for `reset_after_ms`, or an explicit action): fire
  `encounter_reset`, then remove every creature linked to the instance (summons included), revert
  every overlay change it made, cancel its timers and delayed rules, discard its state. An
  `instance_per_party` reset ends the boss room (BOSS-RAID-0 §6.4). A `channel_shared` reset
  creates the next instance in initial state.
- **Fault** (§4.4): as a reset, with no outcome, and an alarm.
- **Membership** (CW2-1, §13.7): the instance's players are the characters currently in its scope;
  `in_anchor` and `players_in` read their current positions. A delayed rule keeps the
  `ExactActorRef` and CharacterId it captured; an actor that has left the scope, died or changed
  generation is absent, and an action on it does nothing (format §9.2). Per-player state a
  `channel_shared` rule keeps (SW-3) is keyed by CharacterId inside the instance and discarded with
  it.

## 4. Execution (ENC-RT-1)

### 4.1 Triggers and the queue

- The owner raises a **trigger occurrence** at the point the event happens: a death commit, a spawn,
  a health change, a cast, a step, a use, a timer due, a counter or phase change, a lifecycle
  change. Each occurrence gets the next occurrence sequence of the instance.
- Occurrences not handled inline (§4.2) go to the instance's FIFO queue. The owner drains the queue
  in the same owner turn, after the action list that raised them: for each occurrence, its rules
  in listed order, each rule's conditions then its actions in order (format §9.2). Occurrences
  raised while draining append to the queue.
- **Bound:** at most `ENCRT0-RL-01` occurrences drained per instance per owner turn. Reaching it is
  a content loop; the instance faults (§3.3). Content validation rejects a rule set whose static
  trigger graph has a cycle without a timer or a delay in it; the runtime bound is the backstop.

### 4.2 Inline hooks (Canary's synchronous events)

Two triggers run inline, inside the damage applier, before it continues:

- **`lethal_damage`**, after the final damage is known and before death applies. A
  `prevent_death` in a matching rule cancels the death; the hit still drains as format §7 states.
- **`damage_taken` and `heal_received`**, at the **encounter stage**: after every mitigation
  (armor, resistances, protections, EQUIP-0 and IMBUE-FORGE-0 sums) and before the health change,
  as Canary's `onHealthChange`. Their conditions read the health before the change; a
  `damage_modifier` with `this_hit`, `reflect_damage` and `convert_damage_to_heal` act on this hit.

Their other actions run inline too, in order; any occurrence they raise goes to the queue (§4.1).
Encounter modifiers that last (a `damage_modifier` with a duration, `shared_life`) are held by the
instance and applied at the same encounter stage.

### 4.3 Delayed rules and timers

- A rule with `delay_ms` or a timer becomes an entry on the owner's timer wheel, due at owner
  semantic time `now + delay`, rounded up to the next 50 ms window (CREATURE-AI-0 §7; Canary's
  dispatcher has the same granularity in practice). Due entries run in due time, then scheduling
  sequence. Values captured at the trigger (`death_position`, `role_position`, the triggering
  actor, the picked player) are stored in the entry.
- A repeating timer that is late fires once (`SKIP_TO_LATEST`), never a burst.
- At most `ENCRT0-RL-02` pending entries per instance; an action that would exceed it faults the
  instance.

### 4.4 Determinism and randomness

- Every draw of format §9.3 uses the SIM purpose `ENCOUNTER_DRAW`, seeded by (encounter instance,
  occurrence sequence, draw index within the occurrence); a delayed rule uses its own occurrence's
  seed. A retry never redraws (SIM-DETERMINISM-01 §12). Given the same occurrences, a fight
  replays identically.
- Instance work counts against the owner's budgets: a drained occurrence is one work unit per rule
  step, within `ENCRT0-RL-03` per instance per window. Above it, the rest of the queue waits for
  the next window in order (never dropped). Control, fencing and player input never wait for it.

## 5. Conditions (ENC-RT-1)

Every condition of format §5 is a read inside the owner turn. `killer_progress` and `world_state`
read the quest domain's published views (read-only, never written). `attacker_wears` reads the
equipment owner (EQUIP-0). A read that is unavailable (a stale view) evaluates false (fail closed).

## 6. Actions

### 6.1 Internal (ENC-RT-1)

`counter`, `flag`, `timer`, `set_phase` (raising `phase_entered`), `one_of` (an `ENCOUNTER_DRAW`),
and the rule-scoped subjects `triggering`, `spawned` and `picked`.

### 6.2 Creatures and combat (ENC-COMBAT-1)

- **`spawn`, `spawn_per_player`, `transform`, `remove`:** through CREATURE-AI-0's creature
  admission and removal, linked to the instance, counted by `CREATUREAI0-RL-04` and by
  `ENCRT0-RL-04` per instance. A position that needs a free tile (CW2-4) asks the Movement owner's
  placement query; no free tile means no creature, as Canary's failed `createMonster`, and the rule
  continues. Health follows the format (§9.4). Encounter summons keep their master link
  (CREATURE-AI-0 §8.1).
- **`heal`, `damage`:** through GAME-ABILITY-01 with origin `Encounter`; `damage` type `none` is a
  direct health change that nothing modifies and that credits no one (D31).
- **`damage_modifier`, `reflect_damage`, `convert_damage_to_heal`, `shared_life`:** at the encounter
  stage (§4.2).
- **`cast`:** an Ability or the encounter's own `encounter_ability`, through GAME-ABILITY-01.
- **`attribute`, `move_lock`:** overrides on the creature, held by the instance, read by
  CREATURE-AI-0 and the damage applier; they end with the creature or the instance.
- **`prevent_death`:** §4.2 only.

### 6.3 World, chat and items (ENC-WORLD-1)

- **`map_item`:** WORLD-INTERACTION-0 §5 overlay operations (`CREATE`, `TRANSFORM`, `REMOVE`) on the
  instance's anchors in its own scope, with `revert_after_ms` counted by `WORLDINT0-RL-09`. A
  teleporter's destination is an anchor of the same scope; the only destination outside it is the
  encounter's exit anchor, which runs the BOSS-RAID-0 §6.4 exit (SCOPE-HANDOFF-1). An
  `interaction` key hands the item's use or step to WORLD-INTERACTION-0.
- **`teleport`:** a server relocation through the Movement owner within the scope (as WORLD-
  INTERACTION-0's lever relocation), with FLOOR-1's landing rule.
- **`say`:** creature speech through CHAT-0 (`CHAT_V1`). **`message`:** a server text message to
  each player in the anchor.
- **Event triggers:** `area_entered`/`area_left` from the Movement owner's step commits;
  `stepped_on` from a step onto a tile holding the item, carrying the item's identity (CW2-3);
  `item_used` from ITEM-USE-1 when the use's target is a creature of the role (the item is spent by
  ITEM-USE-0's transaction first; the trigger fires on its commit).
- **Corpse provenance (CW2-2):** a corpse made by a creature of an instance carries (encounter
  instance, role) in runtime state through its decay transforms while the instance lives; it is
  not durable, so after a restart such a corpse matches no `corpse_of`.

### 6.4 Outcomes (ENC-OUTCOME-1)

- `emit_outcome` produces an `EncounterOutcomeV1`: the outcome key `(encounter instance, occurrence
  sequence, action index)`, the outcome name, the credited CharacterIds (format §6: damage
  contributors from BOSS-RAID-0 §7's accumulator, killer, players in an anchor, or the party
  PARTY-PVP-0 names), and the boss death key when the trigger is a death.
- The owner hands it, in the same turn, to each consumer the encounter's manifest binds to that
  name: BOSS-REWARD-1 and BOSSTIARY-1, the quest domain (QUEST-STATE-0), later others. Each
  consumer writes under its own contract and fence, idempotent by the outcome key.
- Content admission (E4) refuses an encounter whose outcome name has no bound consumer.
- An outcome that has not committed in its consumer when the owner crashes is lost, as the fight is
  (BOSS-RAID-0 ruling R3); it is never replayed from memory into a second write.

### 6.5 `drop_item` (ENC-OUTCOME-1; DUR-03 amendment)

- One DUR-03 MINT of one item into Ground custody at the drop position, under the closed cause
  `EncounterDropCause`, keyed by the outcome-style key `(encounter instance, occurrence sequence,
  action index)`, with the chance drawn by `ENCOUNTER_DRAW` first. It follows the D3 loot MINT
  shape, the tile and channel limits (a full tile drops nothing, as a full Canary tile refuses
  `createItem`) and the §32 fence of the scope.
- At most `ENCRT0-RL-05` drops per instance per 60 s; above it the drop is skipped and counted
  (a value-creation bound; Ugly Monster drops on hits).
- Amended: DUR-03 §39.3.

## 7. Boss levers (BOSS-LEVER-1; amends WORLD-INTERACTION-0 §5)

- A new lever child **`BOSS_ENTRY {encounter key}`**, at most one per firing, with the encounter's
  entry anchors (one point per participant position, in content order) and its arena anchor.
- On `USE`:
  1. The user must stand on one of the entry positions (Canary); otherwise the use does nothing.
  2. The participants are the players standing on the entry positions, one per position (Canary's
     bottom creature; a non-player there is ignored), at most `max_participants`
     (`BOSSRAID0-RL-07`), within the party size limits of the encounter's `entry` (format §3).
  3. A `channel_shared` boss encounter refuses while its arena anchor holds a player (`BUSY`,
     Canary's occupied zone). An `instance_per_party` encounter never does: each group gets its own
     room (D26).
  4. BOSS-RAID-0 §6.2 admission (one transaction: cooldowns, level, quest predicates, gates; the
     group refused as a whole with its typed reason, Canary's messages as text).
  5. On success the BOSS-ROOM-1 instance is created with its encounter instance (§3.1), the
     participants transition by SCOPE-HANDOFF-1 to the entry anchors inside, and the lever's own
     `TRANSFORM` with `revert_after` runs as any lever child.
- The lever's state is per channel (WORLD-INTERACTION-0 §5). Amended: WORLD-INTERACTION-0 §5.

## 8. Ground in an instance (INSTANCE-GROUND-1; DUR-03 amendment)

- Items reach the floor of a boss room (the boss's corpse with D121 loot for a non-reward boss,
  dropped and thrown items, `drop_item`). Ground custody therefore admits an **InstanceRuntime
  scope**: a Ground location row carries the scope ref (`InstanceId`) instead of a ChannelId, with
  the §32 fence of the instance's ownership, its tile limit (`ITEMMOVE1-RL-01`) and a per-instance
  counter at most `ENCRT0-RL-06`.
- **Retirement.** When the instance ends (reset, time limit, last player gone, or found dead after a
  crash), every live Ground root of it and its contents are retired by a new closed cause
  **`InstanceRetire`**, keyed by `(WorldId, InstanceId)`, on the shared retirement tables as the
  `WorldReset` amendment does: one-item steps, resumable from durable state, the instance record
  `RETIRING` until no live Ground item of it remains. A boot or the World job finds instances that
  ended or died with live Ground items and finishes them. An InstanceId is never reused.
- This is the instance form of Tibia's rule that what is left on a boss-room floor is gone when the
  room is cleared (the instance replaces the shared room, D26). Players are warned by the existing
  time-limit message.
- Amended: DUR-03 §32 and §39.3, BOSS-RAID-0 §6.4.

## 9. Persistence and restart

- Encounter state, timers, modifiers and corpse provenance are runtime-only. Durable effects are
  only those of their owners: creature deaths (DUR-03 A4), outcomes in their consumers, drops,
  cooldowns (BOSS-RAID-0), and Ground custody (§8).
- **Channel restart:** each `channel_shared` encounter starts a fresh instance; nothing resumes.
- **Instance crash:** the instance and its fight are lost; participants go to the exit anchor;
  cooldowns stay spent; Ground items retire (§8).

## 10. Rows (values fixed here, registered by each child)

| Row | Value | Note |
|---|---|---|
| `ENCRT0-RL-01` occurrences drained per instance per owner turn | 256 | above it, fault (§4.1) |
| `ENCRT0-RL-02` pending delayed rules and timers per instance | 1,024 | above it, fault |
| `ENCRT0-RL-03` rule-step work units per instance per 50 ms window | 2,048 | the rest waits |
| `ENCRT0-RL-04` live creatures linked to one instance | 512 | a spawn beyond it is skipped and counted |
| `ENCRT0-RL-05` `drop_item` MINTs per instance per 60 s | 60 | above it, skipped and counted |
| `ENCRT0-RL-06` live Ground roots per InstanceRuntime | 2,000 | a drop or MINT beyond it is `BLOCKED` |
| `ENCRT0-RL-07` live encounter instances per channel | 4,096 | content and admission bound |

Each with max and max+1 tests. `ENC-PARITY-1` measures a 15-player Ferumbras-sized fight against
`RL-03`; a measured p99 above it needs a new decision, never a silent raise.

## 11. Wire

None new. Creatures, effects, speech, overlay changes and Ground items reach clients through their
existing views (VIS-2, CHAT_V1, `WORLD_INTERACTION_V1`, `MAP_STATE_V1`).

## 12. Rejected options

- **Canary's shared boss arena.** D26 chose one instance per party.
- **Encounter scripts.** Format §2.2: data only.
- **Running every trigger inline.** Recursion would make order depend on call depth; the FIFO
  keeps Canary's order for one occurrence and bounds loops.
- **Durable encounter state.** Format §2.4; a crash loses the fight, as in Tibia.
- **Keeping instance Ground items after the instance ends.** Nothing could reach them; they would
  be value without an owner.

## 13. Architect rulings (owner rule 5905825574)

- **R1. Order.** a) Inline only for `lethal_damage` and the health-change triggers, FIFO for the
  rest (recommended: Canary's synchronous points, deterministic elsewhere); b) all inline.
  **Ruled a).**
- **R2. Instance floor.** a) Retire at instance end (recommended); b) move to the exit tile; c) mail
  to the owner. **Ruled a).** b) and c) have no Tibia basis.
- **R3. Lost outcomes on a crash.** a) Lost (recommended: BOSS-RAID-0 R3, no duplication); b)
  replayed. **Ruled a).**

## 14. Owner questions

None. D26 and D27 settled scope and outcomes; every other choice applies Canary and Tibia.

## 15. Decision test

- **Must decide now:** YES. Without it no encounter-bound creature can exist (E3): no boss with
  mechanics, no boss room, no boss loot on an instance floor.
- **Minimum sufficient:** one owner, one queue, two inline hooks, one lever child, one retirement
  cause; no new wire.
- **Superseding evidence:** Canary or TibiaWiki evidence that a mechanic depends on recursive event
  order; a measured fight over `ENCRT0-RL-03`.
- **Deliberately not decided:** scripted movement, cross-party state, the Tibiadrome, World
  Changes, Hazard.

## 16. Before-freeze checklist

1. **Contract amendments:** encounter format §10 (runtime pointer); BOSS-RAID-0 §6.4 and §6.5;
   WORLD-INTERACTION-0 §5; CREATURE-AI-0 §3; DUR-03 §32 and §39.3. Applied in this PR.
2. **Serialization:** everything in the scope owner's turn; inline hooks at two fixed points; one
   FIFO; due order by time then sequence.
3. **Restart:** no encounter state survives; durable effects belong to their owners; instance floor
   retirement is resumable.
4. **Typed references:** encounter keys and revisions, anchors, `ExactActorRef`, CharacterId, scope
   refs; no raw coordinates.
5. **Wire:** none new.
6. **Split work:** none; each action is one owner call in the turn.
