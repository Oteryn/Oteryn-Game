# TIMED-ITEM-0 Charges, duration, equip forms and repair

- Decision: `TIMEDITEM0-CHARGES-DURATION-AND-REPAIR-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  combat, protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: EQUIP-0's scope cut and ruling R2 ("a `timed` item grants nothing until TIMED-ITEM-0");
  the base-mechanics close-out plan (#162 5929069698, item 2: rings, soft boots, carried torches,
  duration and charges); owner answer **1a** (#162 5932648083: worn soft boots are repaired by an NPC
  for 10,000 gold, as in Tibia).
- Builds on: GAME-ITEM-01 §4.2 and §4.4 (charge and temporal capabilities); the item schema
  (`charges {count, show_count}`, `temporal {duration_ms, consumption_mode,
  stop_duration_while_unequipped, decay_target}`, `transform {trigger: use | equip | unequip |
  decay}`, `light`); EQUIP-0 §3 (active set, ability types); DUR-03 §11.1, §15, §16.2, §33 and §39.3
  (identity-preserving state mutation, burn sinks, `PRESERVE_INSTANCE`, equipment atomicity, the
  gold fee and NPC service amendments); ITEM-MOVE-WIRE-1 §4 and §6; ITEM-USE-0; NPC-0 §5-§6;
  CONDITIONS-0 (mana shield, regeneration); A13 §4.5 (checkpoints); the D3 corpse decay pattern
  (durable deadline, resumable steps); D3 database clock; owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| TIMED-CONTENT-1 | content lane | `charges`, `temporal` and `transform` facts on Item definitions (rings, amulets, soft boots, torches, lamps), from TibiaWiki first and Canary `items.xml` as the fallback (`charges`, `duration`, `decayTo`, `transformEquipTo`, `transformDeEquipTo`, `showcharges`, `showduration`, `stopduration`); the repair offer (§7) | ITEM-SEM-2b |
| TIMED-1 | hard (persistence), persistence and combat review | the timed state table, its guard and the backfill (§3), the row in every MINT and retirement path, the clocks (§4), charge use (§5), equip forms and toggles (§6), expiry, checkpoints, the active rule for EQUIP-0 (§8) | ITEM-MOVE-2a; EQUIP-RT-1; ITEM-USE-1 |
| TIMED-REPAIR-1 | impl, persistence review | the NPC repair service (§7) | TIMED-1; NPC-TRADE-1 (the gold fee path) |
| TIMED-WIRE-1 | impl, protocol review | charges and remaining time in item presentations and Look (§9) | TIMED-1 |

Later, each with its own decision: exercise weapons (EXERCISE-0, which uses this decision's charges;
R4), invisibility from equipment (with the invisibility family), item imbuement durations
(IMBUE-FORGE-0's own clock).

## 1. Question

How do items that run on charges or time work: when they count down, how they change form, what
happens when they run out, and how a worn pair of soft boots is repaired?

## 2. Facts

**PROVEN**

- GAME-ITEM-01 §4.2: charge state is a separate typed bounded value, not stack quantity; underflow
  and overflow fail closed. §4.4: a temporal item states its time mode; the model distinguishes a
  durable absolute deadline from an authoritative active-time budget.
- EQUIP-0 §3.2: `timed` is derived from `charges.count` or `temporal.duration_ms`; such items grant
  nothing until this decision.
- DUR-03 §11.1: a charge change keeps the ItemInstanceId; §16.2: a one-to-one transform may keep it
  (`PRESERVE_INSTANCE`); §15: a burn needs a named cause, and a new fee source needs an amendment
  of the gold fee paragraph (D178).
- NPC-0 §5: an NPC BUY of a charged item gives it the offer's charges; a SELL matches an item whose
  charges equal the offer's `count`.
- Tibia manual: soft boots run down only while worn; charged items show their charges (`combat.md`).
- Owner answer 1a: worn soft boots are repaired by an NPC for 10,000 gold.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`)

- `items.xml`: rings and soft boots have an inactive form and an active form
  (`transformEquipTo`, `transformDeEquipTo`); only the active form has a `duration` and decays; the
  remaining time survives an unequip. Soft boots' active form decays to worn soft boots; most rings
  decay to nothing. A lit torch has a `duration` and decays to a burnt-out torch wherever it lies.
- `player.cpp` `blockHit`: when an equipped item's protection reduces incoming damage, an item with
  charges loses one charge; at 0 it is removed or becomes its `decayTo` form.
- Decay runs only for items in the game world: items of a logged-out character keep their
  remaining time.
- `npc` Aldo (Venore): "repair" turns worn soft boots into soft boots for 10,000 gold.

## 3. State (TIMED-1)

Table `game_item_timed_states`, one row per ItemInstance whose current definition is `timed`:

| Column | Meaning |
|---|---|
| `item_instance_id` | primary key, references `game_item_instances` |
| `charges` | 1..65,535, or NULL for a definition without charges |
| `remaining_ms` | the active-time budget left, 0..`TIMEDITEM0-RL-02`, or NULL without `temporal` |
| `deadline_at` | NULL, or the database time at which a running **ground** item expires (§4) |
| `state_revision` | uint64, +1 per write |

- The row is created with the item: by **every** MINT that creates a timed item (loot, rewards, NPC
  BUY, conjure, any later source), with charges and duration from the definition, or the NPC offer's
  `count` for charges (NPC-0 §5; content validation refuses an offer whose `count` exceeds the
  definition's charges), and by a transform into a timed definition. A transform out of a timed
  definition, and every retirement of the item (`DECAY_RETIRE`, a burn, `Expire`), deletes it in the
  same transaction. Items minted before TIMED-1 lands get their row from a TIMED-1 backfill with the
  definition's values. Amended: DUR-03 §39.3.
- Every write is a DUR-03 `STATE_MUTATION` (§11.1) or part of a `TRANSFORM`, in a one-item
  transaction under the item writer's fence and the holder's `character_root` lock when a character
  holds the item, under a closed `TimedItemCause` (§10). The row moves with the item, because it is
  keyed by ItemInstanceId; no move writes it except where §4 starts or stops a clock.
- A deferred database guard checks, at commit, that a live timed item has exactly one row, that
  `charges` and `remaining_ms` are non-NULL exactly when the current definition has `charges` and
  `temporal`, and that they never exceed the definition's values.

## 4. Clocks (TIMED-1)

An item's time runs only while it is **live**, as in Canary:

| Item's definition (`consumption_mode`) | Live when | Time mode (GAME-ITEM-01 §4.4) |
|---|---|---|
| `on_equip` (rings, soft boots: the active form) | in a `CharacterEquipment` slot of a character whose actor is in the world | active-time budget |
| `continuous` (a lit torch, a lit lamp) | held by a character whose actor is in the world (equipment, backpack, containers in it) | active-time budget |
| `continuous` | on a `Ground` tile, in a container on Ground (a corpse included), or a `HouseInterior` item | durable absolute deadline |
| any | anywhere else (depot, inbox, mail, market escrow, a logged-out character's items) | frozen |

- **Held items** run in the channel runtime (or house scope) that hosts the actor. The runtime keeps
  the live remaining time and charges and commits them at the A13 checkpoint cadence (at most every
  60 s), at logout (at the actor end, before OFFLINE-0's `logout` marker and the terminal release),
  before a channel transfer or house handoff, and at expiry. A checkpoint is **its own item-writer
  transaction**, not part of the A13 build receipt: one transaction per character, under the item
  writer's fence and the character's `character_root` lock, updating that character's changed rows
  in ItemInstanceId order under `TimedItemCause::Checkpoint`. A crash loses at most one checkpoint
  of elapsed time and charges, in the player's favour (R2).
- **Deadline items** (Ground, containers on Ground, `HouseInterior`): a move into such a location
  sets `deadline_at = database time + remaining_ms` in the move transaction, including a death that
  drops a held item into a corpse; a move out sets `remaining_ms = max(0, deadline_at − database
  time)` and clears the deadline. The scope that hosts the location (the channel, or the house scope
  while active) expires items at their deadline as a resumable step from durable state, the D3
  corpse decay pattern; a scope that loads late, or a house that activates, expires overdue items at
  once. Scope downtime counts against the item (R3).
- **Frozen:** a move into a frozen location stops the clock in the move transaction (the runtime's
  live value is written); a move out starts it. DUR-03 §39.3 transfers that cross between live and
  frozen classes carry that one-row write. Amended: DUR-03 §39.3.
- **Expiry** at `remaining_ms = 0` (or the deadline): one transaction under
  `TimedItemCause::Expire {item, deadline or exhaustion revision}`, keyed by the item and its
  `state_revision`, so a replay finds the row already changed and writes nothing: a `TRANSFORM` (`PRESERVE_INSTANCE`) to the definition's `decay_target`,
  whose timed row is created fresh from its own definition or deleted; or, without a
  `decay_target`, a BURN that retires the item. The item stops being active (§8) at the moment the
  runtime reaches 0, before the commit.

## 5. Charges (TIMED-1)

- **Protection charges.** When an active item's `PROTECTION` (EQUIP-0 §3.1) contributes to reducing
  an incoming hit of its element at the GAME-ABILITY-01 §10 stage, an item with charges loses one
  charge per hit (Canary `blockHit`). Several charged items protecting against the same hit each
  lose one.
- The runtime keeps live charges and commits them like time (§4: checkpoints, logout, transfer). At
  0 the item stops being active at once and the runtime commits the expiry (§4) immediately.
- Other charge consumers (exercise weapons, R4) use the same column under their own decision.

## 6. Equip forms and toggles (TIMED-1)

- **Equip and unequip transforms.** When ITEM-MOVE-WIRE-1 §4 equips an item whose definition has a
  `transform {trigger: equip}`, the same move transaction transforms it (`PRESERVE_INSTANCE`) into
  the target and starts its clock; an unequip with `trigger: unequip` transforms it back and stops
  the clock. `remaining_ms` carries across both forms, so the inactive form keeps the time left
  (Canary). An unequip of an active form without an unequip transform keeps the active form, frozen
  if it is `on_equip`. Amended: DUR-03 §33, ITEM-MOVE-WIRE-1 §6.
- **Toggles.** A `transform {trigger: use}` between a lit and an unlit form (torch, lamp) is an
  ITEM-USE-0 use on a held item: one `TRANSFORM` (`PRESERVE_INSTANCE`) under
  `TimedItemCause::Toggle`, carrying `remaining_ms` across. Amended: ITEM-USE-0.
- A transform never resets the time; only a repair (§7) or a new item does.

## 7. Repair (TIMED-REPAIR-1; owner answer 1a)

- **Offer.** NPC content declares a repair offer `{npc, keyword "repair", from_item, to_item, price}`.
  v1 content has one: worn soft boots to soft boots for 10,000 gold, at Aldo in Venore (Canary,
  TibiaWiki). Any other repair offer is a new fee source and needs an owner answer (D178).
- **Dialogue.** As NPC-0 §4 travel: the keyword "repair" through `NPC_TALK_INTENT` opens a
  confirmation bound to the offer, the price and the content revision; "yes" within `NPC0-RL-06`
  confirms it; anything else or the timeout cancels. No new command.
- **Transaction.** One item-only transaction like NPC-0's travel (NPC-0 §5.2 and §6, DUR-03 §39.3
  NPC service amendment), with no `CharacterRevision` advance:
  - the gold fee plan with `F = price`, burned under the new `FeeBurnCause::NpcRepair {npc, offer,
    occurrence}`;
  - one `TRANSFORM` (`PRESERVE_INSTANCE`) of one live, non-equipped item of `from_item` in the
    player's backpack or its containers (the first in inventory order, as NPC-0 SELL finds its item),
    into `to_item`. The old timed row is deleted and a fresh one is created from `to_item`'s
    definition: full charges and full duration. Nothing carries over from the worn item.
  - Insufficient funds or no such item rejects the whole transaction and writes nothing.
- Amended: DUR-03 §39.3 (the `FeeBurnCause` variant), NPC-0 (the repair offer).

## 8. When a timed item is active (EQUIP-0)

EQUIP-0 §3.2's "and it is not `timed`" is replaced by: a timed item is active when it meets every
other §3.2 condition, its charges (if any) are above 0, its remaining time (if any) is above 0, and,
for an `on_equip` definition, it is equipped in its slot. Its abilities then apply as EQUIP-0
says, including two the active forms need:

- `REGENERATION {hp_per_tick, mana_per_tick, interval_ms}` (life ring, ring of healing): a
  CONDITIONS-0 `REGENERATION` instance whose provenance has source kind `item` and the
  ItemInstanceId, present while the item is active
  and removed when it stops being active; the protection-zone rule of CONDITIONS-0 still applies.
- `MANA_SHIELD` (energy ring): the CONDITIONS-0 mana shield instance with source kind `item`,
  present while the item is active.

Amended: EQUIP-0 §3.1 and §3.2; CONDITIONS-0 §3 and §3.2 (source kind `item`).

## 9. Wire (TIMED-WIRE-1)

- Under a new capability `TIMED_ITEMS_V1` (number reserved on #162 at allocation), every item
  presentation that carries a count (inventory, containers, equipment, Ground) gains optional
  `charges u16` and `remaining_s u32`, sent only when the definition has `show_count` or the
  `duration` display flag. A live item's remaining time is sent at every change of form and at most
  once per `TIMEDITEM0-RL-03` (60 s); the client counts down between updates.
- Look adds "It has N charges left." or "It will expire in X minutes." from the same values.
- Without the capability the client shows the item without these values; nothing else changes.

## 10. Causes (DUR-03)

Closed `TimedItemCause`: `Checkpoint`, `ClockStart`, `ClockStop`, `ChargeSpent`, `EquipForm
{direction}`, `Toggle`, `Expire {deadline}`. Each names its ItemInstanceId and an occurrence issued
by the runtime. `Expire` without a `decay_target` is a BURN sink. The repair burn is
`FeeBurnCause::NpcRepair` (§7). No generic or caller-chosen reason. Amended: DUR-03 §15 and §39.3.

## 11. Rows

| Row | Value |
|---|---|
| `TIMEDITEM0-RL-01` charges per item | 65,535 |
| `TIMEDITEM0-RL-02` active-time budget per item | 7 days (604,800,000 ms) |
| `TIMEDITEM0-RL-03` remaining-time updates to the client per item | 1 per 60 s |
| `TIMEDITEM0-RL-04` ground expiry steps per scope per simulation tick | 64; the rest wait for the next tick |

Each with max and max+1 tests. Content validation refuses a definition above RL-01 or RL-02.

## 12. Rejected options

- **A write per charge or per second.** A13 checkpoints bound both the write rate and the crash loss.
- **Time running for logged-out characters or in the depot.** Canary decays only in the game world;
  players expect a ring to keep its time while they are offline.
- **Charges as stack quantity.** GAME-ITEM-01 §4.2 forbids it.
- **A generic repair fee.** D178 admits fee sources one by one; the owner admitted this one.
- **Exercise weapons here.** They are online training with their own rules (dummies, house bonus,
  try rates); they get EXERCISE-0, which reuses §3 and §5.

## 13. Architect rulings (owner rule 5905825574)

- **R1. Where time runs.** a) Only in the game world, held or on Ground (recommended: Canary); b)
  always by wall clock. **Ruled a).**
- **R2. Crash loss.** a) Return at most one checkpoint of time and charges to the player
  (recommended: as A13 and OFFLINE-0); b) a write per change. **Ruled a)**, `PARITY_PENDING`.
- **R3. Deadline items' downtime.** a) The deadline runs on database time, so scope downtime
  and an inactive house count (recommended: no new write path; lit torches are low value, and Tibia
  decays items in houses too); b) freeze while the scope is down.
  **Ruled a)**, a declared difference.
- **R4. Exercise weapons.** a) A separate EXERCISE-0 that uses these charges (recommended: keeps this
  decision reviewable); b) here. **Ruled a).** OFFLINE-0's pointer is amended.

## 14. Owner questions

None open. Owner answer 1a settled the only fee source.

## 15. Decision test

- **Must decide now:** YES. Without it every ring, charged amulet, soft boots and torch grants
  nothing (EQUIP-0 R2).
- **Minimum sufficient:** one side table, runtime clocks committed at existing checkpoints, the D3
  ground-expiry pattern, closed causes, one repair offer.
- **Superseding evidence:** an official source on where items decay or how charges are spent.
- **Deliberately not decided:** exercise weapons (EXERCISE-0), invisibility from equipment, imbuement
  durations.

## 16. Before-freeze checklist

1. **Contract amendments:** EQUIP-0 §3.1 and §3.2; DUR-03 §15, §33 and §39.3; ITEM-MOVE-WIRE-1 §6;
   ITEM-USE-0; NPC-0 (repair offer); CONDITIONS-0 §3; OFFLINE-0 (exercise weapons pointer). Applied
   in this PR.
2. **Serialization:** every timed write is a one-item DUR-03 transaction under the item writer's
   fence and the holder's `character_root` lock; the runtime is the only live owner of a held item's
   clock and charges; a checkpoint is one item-writer transaction per character, separate from the
   A13 build receipt; expiry is keyed by (item, `state_revision`) and replays.
3. **Restart:** held items lose at most one checkpoint, in the player's favour; deadline items
   expire from their durable deadline; frozen items do not change; items minted before TIMED-1 get
   rows from its backfill.
4. **Typed references:** ItemInstanceId, definition keys, NPC offer ids, database times.
5. **Wire:** one capability with two optional fields; no new command.
6. **Split work:** expiry, repair and each clock start or stop are single-item transactions.
