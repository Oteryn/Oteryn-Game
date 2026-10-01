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

Table `game_item_timed_states`, one row per ItemInstance whose current definition is `timed`, or is
the **inactive form** of a timed pair: a definition with a `transform {trigger: equip}` or
`transform {trigger: use}` into a timed definition, such as an unworn ring, unworn soft boots or an
unlit torch:

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
  definition other than its own inactive form, and every retirement of the item (`DECAY_RETIRE`, a
  burn, `Expire`), deletes it in the same transaction.
- **A new row starts in its current clock class (§4).** The transaction that creates or resets a row
  (a MINT, a transform into a timed definition, an `Expire` transform into another timed stage, a
  repair) sets it from the item's location in that same transaction: in a deadline location
  (Ground, a container on Ground such as a corpse, `HouseInterior`) a `continuous` item gets
  `deadline_at = database time + remaining_ms`; a held item's clock is started by the hosting runtime
  from the committed `remaining_ms`; anywhere else the row is frozen. So loot minted into a corpse,
  and a ground item that expires into another timed stage, always have a deadline.
- **Paired forms keep the row.** The row belongs to the item, not to the form. A transform into the
  inactive form (an unequip, or a use that puts out a torch) keeps the row frozen with its
  `remaining_ms`; the next transform into the active form (an equip, or a use that lights it)
  restarts the clock from that value. An inactive form minted new has no row, which means the full
  duration of its active form; the first transform into the active form creates the row. Items minted before TIMED-1 lands get their row from a TIMED-1 backfill with the
  definition's values. Amended: DUR-03 §39.3.
- **Backfill, sequenced through one-item transactions.** TIMED-1 ships with timed behaviour off and
  the deferred guard not yet enabled, but with the RL-05 admission check (§4) **on** from the start,
  so no move, MINT or transform can add a `continuous` timed item to a tree at or above 32 while the
  backfill runs and the preflight's result cannot be invalidated afterwards. Its migration then runs, resumably, only
  admitted one-item DUR-03 transactions, each under the item writer's fence and the holder's
  `character_root` lock:
  1. **Rows.** One `STATE_MUTATION` per live timed item without a row, under
     `TimedItemCause::BackfillRow`, creating the row from the definition in the item's current clock
     class (above). A replay finds the row and writes nothing.
  2. **Over-cap trees.** BAGS-0 lets a pre-TIMED-1 tree hold more than `TIMEDITEM0-RL-05`
     `continuous` timed items. For each such tree, every one beyond the first 32 in ItemInstanceId
     order is put out by its own one-item `TRANSFORM` (`PRESERVE_INSTANCE`) into its inactive form,
     keeping its full `remaining_ms` frozen, under `TimedItemCause::BackfillCap {item}`; each takes
     the BAGS-0 tree lock `FOR SHARE` to recount and writes nothing once the tree has 32 or fewer.
     Nothing is lost; the player can light it again when the tree has room.
  3. **Enable.** A preflight confirms that every live timed item has its row and every tree meets
     RL-05. Only then are the deferred guard and timed behaviour enabled.

  A `continuous` item with no inactive form cannot be put out: if step 3's preflight finds a tree
  still above 32, nothing is enabled and the trees are reported; TIMED-1 is not enabled until an
  owner answer settles those items. After the backfill every tree meets RL-05, so no tree move ever
  needs more than 32 descendant lines. No multi-item migration shape is admitted.
- Every write is a DUR-03 `STATE_MUTATION` (§11.1) or part of a `TRANSFORM`, in a one-item
  transaction (or, for a container tree's move, the bounded `TimedTreeMove` of §4) under the item writer's fence and the holder's `character_root` lock when a character
  holds the item, under a closed `TimedItemCause` (§10). The row moves with the item, because it is
  keyed by ItemInstanceId; no move writes it except where §4 starts or stops a clock.
- A deferred database guard checks, at commit, that a live timed item has exactly one row and an
  inactive form at most one, that `charges` and `remaining_ms` are non-NULL exactly when the timed
  definition (the current one, or for an inactive form its active form) has `charges` and
  `temporal`, and that they never exceed that definition's values.

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
  before a channel transfer or house handoff, and at expiry. A checkpoint is **its own one-item
  item-writer transaction per changed item**, not part of the A13 build receipt: under the item
  writer's fence and the holder's `character_root` lock, it updates that item's row under
  `TimedItemCause::Checkpoint {item, state_revision}`. A character's item checkpoints are
  independent of each other; each is keyed by (item, the `state_revision` it read) and requires the
  row to still have that revision, so a replay or a superseded checkpoint writes nothing. A crash loses at most one checkpoint
  of elapsed time and charges, in the player's favour (R2).
- **Deadline items** (Ground, containers on Ground, `HouseInterior`): a move into such a location
  sets `deadline_at = database time + remaining_ms` in the move transaction, where for an item
  that was held `remaining_ms` is the runtime's **current** live budget, not the persisted value,
  which can lag by up to one checkpoint. The hosting runtime reads its live budget (and charges) for
  the root and every affected descendant at the move, passes them into the move transaction, and
  that transaction writes them to the rows and derives `deadline_at` from them atomically; the
  runtime drops its live clock for those items only when the commit succeeds. Picking an item up and
  dropping it again therefore never wins back time. This holds for a death drop too, a death that
  drops a held item into a corpse included; a move out sets `remaining_ms = max(0, deadline_at − database
  time)` and clears the deadline. The scope that hosts the location (the channel, or the house scope
  while active) expires items at their deadline as a resumable step from durable state, the D3
  corpse decay pattern; a scope that loads late, or a house that activates, expires overdue items at
  once. Scope downtime counts against the item (R3).
- **Frozen:** a move into a frozen location stops the clock in the move transaction (the runtime's
  live value is written); a move out starts it.
- **Container trees (BAGS-0).** A tree move, a death drop and a pickup change the clock class of
  every `continuous` timed item in the tree, not only of the root. The move transaction locks those
  descendants' timed rows `FOR UPDATE` in ItemInstanceId order, after the BAGS-0 tree locks, and
  writes each one (deadline set or cleared, clock stopped or started) in the same commit, as
  `TimedItemCause::TreeClock {root, move occurrence}` lines. This is the bounded multi-item shape
  `TimedTreeMove` that this decision adds to DUR-03 §39.3: the root's own TRANSFER plus at most
  `TIMEDITEM0-RL-05` (32) `STATE_MUTATION` lines of descendant timed rows, no location line and no
  value line for a descendant, at most 33 touched items, and one audit event that carries the
  root's lines and one before/after line per descendant. Its ceilings (DUR-03 §28), registered by
  TIMED-1 before implementation with max and max+1 tests: 33 touched items, 1 location line (the
  root's), 0 value lines, 0 transform I/O, container expansion 8 (`DUR03-RL-05-TREE`), 33
  participants / 99 work units, 1 audit event, and the existing `DUR03-RL-08` retry and
  reconciliation work (3; a retry repeats the whole transaction under the same TransactionId). Each
  descendant line is fixed-size, at most 64 bytes (ItemInstanceId, before and after `deadline_at`,
  `remaining_ms`, `charges`, `state_revision`), so 32 lines take at most 2,048 bytes next to the
  root's one-item evidence, within `DUR03-RL-07-PAYLOAD-BYTES` (7,936) and
  `DUR03-RL-07-ENVELOPE-BYTES` (9,216); TIMED-1 measures the encoded size at 32 lines and fails the
  build above those ceilings.
- **The tree bound.** A tree holds at most `TIMEDITEM0-RL-05` (32) `continuous` timed items. Every
  transaction that would add one to a tree checks the tree's count under the BAGS-0 tree locks and
  refuses `NO_ROOM`, writing nothing: a move or a MINT into the tree, a `Toggle` into a lit form, and
  any transform into a `continuous` form.
- A single-item transfer that crosses between live, deadline and frozen classes carries its one-row
  write in the same transaction. Amended: DUR-03 §39.3.
- **Player trade.** A trade is live to live, so it would carry a persisted value up to one
  checkpoint old. PLAYER-TRADE-0 therefore refuses as `NOT_TRADEABLE` an offer of an item whose
  clock is running (a lit `continuous` item, an equipped active `on_equip` item), or of a tree that
  contains one; the player puts it out or unequips it first, which writes the live value. The trade
  aggregate gains no timed line. Amended: PLAYER-TRADE-0 §4.
- **Decay target.** An item's decay target is its definition's `transform {trigger: decay}` if
  present, else `temporal.decay_target`; content validation rejects a definition with both set to
  different targets. The same target applies whether the item runs out of time or of charges.
- **Expiry** at `remaining_ms = 0`, at the deadline, or at 0 charges: one transaction under
  `TimedItemCause::Expire {item, reason}`, where `reason` is `Deadline {deadline_at}` (a deadline
  item), `TimeExhausted {state_revision}` (a held item's budget reached 0) or `ChargesExhausted
  {state_revision}`. It is keyed by (item, reason); the transaction requires the row's current
  `deadline_at` or `state_revision` to equal the reason's, so a replay, or a reason made stale by a
  later write, finds no match and writes nothing: a `TRANSFORM`
  (`PRESERVE_INSTANCE`) to the decay target, whose timed row is created fresh from its own
  definition or deleted; or, only when the definition has no decay target, a BURN that retires
  the item. The item stops being active (§8) at the moment the
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
  the clock. `remaining_ms` carries across both forms in the same row (§3), so the inactive form keeps the time left
  (Canary). An unequip of an active form without an unequip transform keeps the active form, frozen
  if it is `on_equip`. Amended: DUR-03 §33, ITEM-MOVE-WIRE-1 §6.
- **Toggles.** A `transform {trigger: use}` between a lit and an unlit form (torch, lamp) is an
  ITEM-USE-0 use on a held item: one `TRANSFORM` (`PRESERVE_INSTANCE`) under
  `TimedItemCause::Toggle`, carrying `remaining_ms` across. Lighting an item inside a tree that
  already holds 32 `continuous` timed items is refused `NO_ROOM` (§4). Amended: ITEM-USE-0.
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
    into `to_item`. The old timed row is deleted and a fresh one is created with full charges and
    full duration from `to_item`'s definition, or, when `to_item` is an inactive form (unworn soft
    boots), from its paired active form's definition, as the §3 guard reads it. Nothing carries over
    from the worn item.
  - The gold fee plan is admitted with at most **19** coin inputs here (not 20), so the 19 inputs,
    at most 2 change stacks and the repaired item stay within the fee shape's 22 touched items and
    64 work units (DUR-03 §39.3). 10,000 gold fits in one crystal coin, so this only refuses a
    player who pays from 20 or more small stacks.
  - Insufficient funds, more than 19 coin inputs, or no such item rejects the whole transaction and
    writes nothing.
- Amended: DUR-03 §39.3 (the `FeeBurnCause` variant), NPC-0 (the repair offer).

## 8. When a timed item is active (EQUIP-0)

EQUIP-0 §3.2's "and it is not `timed`" is replaced by: a timed item is active when it meets every
other §3.2 condition, its charges (if any) are above 0, its remaining time (if any) is above 0, and,
for an `on_equip` definition, it is equipped in its slot. Its abilities then apply as EQUIP-0
says, including two the active forms need:

- `REGENERATION {hp_per_tick, mana_per_tick, interval_ms}` (life ring, ring of healing): an
  instance of the new CONDITIONS-0 family **`ITEM_REGENERATION`**, conflict key
  `item_regeneration:<equipment slot>`, so there is one instance per slot. It stacks with
  `FOOD_REGENERATION` and `RECOVERY`: each ticks on its own, as in Tibia, where a life ring adds to
  food regeneration (Canary keeps the ring's regeneration as a separate condition id). Its
  provenance has source kind `item` and the ItemInstanceId. It is created when the item becomes
  active, has no duration of its own and ends when the item stops being active. In a protection zone
  its ticks are skipped, like every regeneration (CONDITIONS-0).
- `MANA_SHIELD` (energy ring): a CONDITIONS-0 `MANA_SHIELD` instance with source kind `item` and
  **no capacity**: while the item is active, damage that reaches the §3.3 stage goes to mana up to
  the mana available, and the rest to health. It has no `remaining` and no duration of its own; it
  ends only when the item stops being active (unequip, expiry, charges at 0), never because a
  capacity or mana reaches 0. This overrides CONDITIONS-0 §3.3's capacity and end rules for this
  source only. A spell mana shield applied while the ring's instance exists replaces it under the
  `mana_shield` key's policy, and the ring's instance is created again when the spell's ends, if the
  ring is still active (`PARITY_PENDING`).

Amended: EQUIP-0 §3.1 and §3.2; CONDITIONS-0 §3 (the `ITEM_REGENERATION` family), §3.2 (source kind
`item`) and §3.3 (the item-sourced mana shield).

## 9. Wire (TIMED-WIRE-1)

- Under a new capability `TIMED_ITEMS_V1` (number reserved on #162 at allocation), every item
  presentation that carries a count (inventory, containers, equipment, Ground) gains optional
  `charges u16` and `remaining_s u32`, sent only when the definition has `show_count` or the
  `duration` display flag. A live item's remaining time is sent at every change of form and at most
  once per `TIMEDITEM0-RL-03` (60 s); the client counts down between updates.
- Look adds "It has N charges left." or "It will expire in X minutes." from the same values.
- Without the capability the client shows the item without these values; nothing else changes.

## 10. Causes (DUR-03)

Closed `TimedItemCause`: `Checkpoint {item, state_revision}`, `ClockStart`, `ClockStop`,
`ChargeSpent`, `EquipForm {direction}`, `Toggle`, `TreeClock {root, move occurrence}`,
`BackfillRow` and `BackfillCap {item}` (the TIMED-1 migration only, one-item transactions, §3), `Expire {reason: Deadline {deadline_at} | TimeExhausted {state_revision} |
ChargesExhausted {state_revision}}`. Each names its ItemInstanceId and an occurrence issued by the
runtime. `Expire` of a definition with no decay target is a BURN sink. The repair burn is
`FeeBurnCause::NpcRepair` (§7). No generic or caller-chosen reason. Amended: DUR-03 §15 and §39.3.

## 11. Rows

| Row | Value |
|---|---|
| `TIMEDITEM0-RL-01` charges per item | 65,535 |
| `TIMEDITEM0-RL-02` active-time budget per item | 7 days (604,800,000 ms) |
| `TIMEDITEM0-RL-03` remaining-time updates to the client per item | 1 per 60 s |
| `TIMEDITEM0-RL-04` ground expiry steps per scope per simulation tick | 64; the rest wait for the next tick |
| `TIMEDITEM0-RL-05` `continuous` timed items per container tree | 32 |

Each with max and max+1 tests. Also tested: unequip then re-equip keeps the remaining time (the
timer does not reset); putting out a torch and lighting it again keeps its remaining time; a replayed
or stale `Expire` writes nothing; dropping a held lit torch 59 s after its last checkpoint sets its
deadline from the live budget, and a pick-up/drop cycle never adds time; a pre-TIMED-1 tree with 33
lit torches is backfilled with 32 lit and 1 put out, all rows present, and the guard is enabled only
after, and a bag move that would put a 33rd lit item into a tree during the backfill is refused;
a trade offer of a lit torch, or of a bag holding one, is refused; repairing worn soft boots into
unworn ones gives the row the active form's full duration; loot minted lit into a corpse and a ground item that expires into another timed stage get a
deadline in that transaction; a repair paid from 19 coin
stacks commits and from 20 is refused; a life ring's regeneration ticks alongside food regeneration; the
energy ring's shield stays while mana is 0 and ends at unequip; a lit torch inside a dropped bag gets its Ground deadline in the drop
commit; a charge-only item at 0 charges with a `transform {trigger: decay}` transforms, and is
burned only without any decay target. Content validation refuses a definition above RL-01 or RL-02.

## 12. Rejected options

- **A write per charge or per second.** A13 checkpoints bound both the write rate and the crash loss.
- **Time running for logged-out characters or in the depot.** Canary decays only in the game world;
  players expect a ring to keep its time while they are offline.
- **Charges as stack quantity.** GAME-ITEM-01 §4.2 forbids it.
- **Clocks derived from the tree root.** A descendant would store its budget against its root's
  clock and never be written by a root move. But moving a subtree into another tree changes the
  root of every item in it, so their anchors would need rewriting anyway, and a held class also
  changes at login and logout without any move. The bounded `TimedTreeMove` shape is simpler and
  keeps every clock explicit.
- **One checkpoint per character.** A multi-item write with no DUR-03 shape; per-item checkpoints
  are independent and replay-safe.
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
- **Harder later:** the per-item timed table and its guard couple every MINT, transform and
  retirement path to it; the `TimedTreeMove` shape and RL-05 bind container trees, so raising RL-05
  needs new measured rows; the database-time deadline (R3) means switching to frozen-while-down later
  would need a migration of every live deadline; `TIMED_ITEMS_V1` fields join the wire compatibility
  surface.
- **Superseding evidence:** an official source on where items decay or how charges are spent.
- **Deliberately not decided:** exercise weapons (EXERCISE-0), invisibility from equipment, imbuement
  durations.

## 16. Before-freeze checklist

1. **Contract amendments:** EQUIP-0 §3.1 and §3.2; DUR-03 §15, §33 and §39.3; ITEM-MOVE-WIRE-1 §6;
   PLAYER-TRADE-0 §4;
   ITEM-USE-0; NPC-0 (repair offer); CONDITIONS-0 §3; OFFLINE-0 (exercise weapons pointer). Applied
   in this PR.
2. **Serialization:** every timed write is a one-item DUR-03 transaction (or a bounded `TimedTreeMove`) under the item writer's
   fence and the holder's `character_root` lock; the runtime is the only live owner of a held item's
   clock and charges; a checkpoint is one one-item item-writer transaction per changed item,
   separate from the A13 build receipt; a tree move is the bounded `TimedTreeMove` shape; expiry is keyed by (item, `state_revision`) and replays.
3. **Restart:** held items lose at most one checkpoint, in the player's favour; deadline items
   expire from their durable deadline; frozen items do not change; items minted before TIMED-1 get
   rows from its backfill, which brings every tree within RL-05 (§3).
4. **Typed references:** ItemInstanceId, definition keys, NPC offer ids, database times.
5. **Wire:** one capability with two optional fields; no new command.
6. **Split work:** expiry, repair and each clock start or stop are single-item transactions.
