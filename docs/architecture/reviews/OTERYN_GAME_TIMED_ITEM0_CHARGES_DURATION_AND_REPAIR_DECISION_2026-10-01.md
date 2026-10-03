# TIMED-ITEM-0 Charges, duration, equip forms and repair

- Decision: `TIMEDITEM0-CHARGES-DURATION-AND-REPAIR-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  combat, protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: EQUIP-0's scope cut and ruling R2 ("a `timed` item grants nothing until TIMED-ITEM-0");
  the base-mechanics close-out plan (#162 5929069698, item 2: rings, soft boots, carried torches,
  duration and charges; carried torches are deferred to TIMED-ITEM-0B, §3); owner answer **1a** (#162 5932648083: worn soft boots are repaired by an NPC
  for 10,000 gold, as in Tibia).
- Builds on: GAME-ITEM-01 §4.2 and §4.4 (charge and temporal capabilities); the item schema
  (`charges {count, show_count}`, `temporal {duration_ms, consumption_mode,
  stop_duration_while_unequipped, decay_target}`, `transform {trigger: use | equip | unequip |
  decay}`, `light`); EQUIP-0 §3 (active set, ability types); DUR-03 §11.1, §15, §16.2, §33 and §39.3
  (identity-preserving state mutation, burn sinks, `PRESERVE_INSTANCE`, equipment atomicity, the
  gold fee and NPC service amendments); ITEM-MOVE-WIRE-1 §4 and §6; ITEM-USE-0; NPC-0 §5-§6;
  CONDITIONS-0 (mana shield, regeneration); A13 §4.5 (checkpoints); owner rule 5905825574; the control
  plane's round-7 scope cut (continuous duration to TIMED-ITEM-0B).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| TIMED-CONTENT-1 | content lane | `charges`, `temporal` and `transform` facts on Item definitions (rings, amulets, soft boots, torches, lamps), from TibiaWiki first and Canary `items.xml` as the fallback (`charges`, `duration`, `decayTo`, `transformEquipTo`, `transformDeEquipTo`, `showcharges`, `showduration`, `stopduration`); the repair offer (§7) | ITEM-SEM-2b |
| TIMED-1 | hard (persistence), persistence and combat review | the timed state table and its guard (§4, lazy rows), clocks, checkpoints, charge use and expiry (§5), equip forms (§6), the active rule for EQUIP-0 (§8) | ITEM-MOVE-2a; EQUIP-RT-1; ITEM-USE-1 |
| TIMED-REPAIR-1 | impl, persistence review | the NPC repair service (§7) | TIMED-1; NPC-TRADE-1 (the gold fee path) |
| TIMED-WIRE-1 | impl, protocol review | charges and remaining time in item presentations and Look (§9) | TIMED-1 |

Later, each with its own decision: `continuous` duration (TIMED-ITEM-0B: lit torches and lamps,
Ground and house deadlines, container-tree clocks), exercise weapons (EXERCISE-0, which uses this decision's charges;
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

## 3. Scope after narrowing (control plane, round 7)

This decision admits **charges**, **`on_equip` duration** (rings, soft boots), equip forms, expiry
and the soft boots repair (owner answer 1a). **`continuous` duration** (lit torches and lamps, Ground
and house deadlines, container-tree clocks, their backfill and trade rules) moves to a later
decision, **TIMED-ITEM-0B**, after close-out. Until then:

- a definition whose `temporal.consumption_mode` is `continuous` is **`NOT_ADMITTED`**: it has no
  timed row, grants nothing (EQUIP-0 R2 stays in force for it), and a use that would transform an
  item into a `continuous` form (lighting a torch) is refused `NOT_ADMITTED`, writing nothing;
- content validation reports every such definition; nothing about them is decided here.

An `on_equip` item's time runs only in an equipment slot, and charges are spent only by an equipped
item. Neither ever runs on Ground, in a container or in a house, so this decision needs no
deadline, no tree shape and no location clock.

## 4. State (TIMED-1)

Table `game_item_timed_states`, one row per ItemInstance whose current definition is an admitted
`timed` definition (charges, or `on_equip` temporal), or is the **inactive form** of such a pair: a
definition with a `transform {trigger: equip}` into one, such as an unworn ring or unworn soft boots:

| Column | Meaning |
|---|---|
| `item_instance_id` | primary key, references `game_item_instances` |
| `charges` | 1..65,535, or NULL for a definition without charges or a spent row (0 is never stored: reaching 0 is an expiry, §5) |
| `remaining_ms` | the active-time budget left, 0..`TIMEDITEM0-RL-02`, or NULL without `temporal` |
| `state_revision` | uint64, +1 per write |

- **Creation (one rule): rows are lazy.** No MINT, migration or foreign shape ever writes a row.
  An item of an admitted timed definition **without a row has the full charges and duration of its
  definition** (for an inactive form, of its paired active form). Its first own write creates the
  row at expected revision 0: its first checkpoint while equipped (§5), its first equip transform
  (§6), its expiry (§5) or its repair (§7). There is no write per charge spent. So the MINT paths (loot, rewards, NPC BUY, any later source) and the existing
  population need no row and no backfill. Content validation refuses an NPC offer whose `count`
  differs from the definition's charges, since a fresh item always has full charges (NPC-0 §5).
- **Paired forms keep the row.** The row belongs to the item, not to the form. An unequip transform
  into the inactive form keeps `remaining_ms`; the next equip continues from it.
- **Only this decision's shapes transform such an item** (equip forms, expiry, repair); content
  validation refuses any other transform rule from or into an admitted timed definition. **Once created, a row
  is never deleted while the item lives**, so `state_revision` only ever grows for an instance:
  expiry and repair reset its values and add 1 to the revision (§5, §7). A row of an item whose
  current definition is not timed (worn soft boots after expiry) is **spent**: both values NULL.
  **Every other retirement path** (`DECAY_RETIRE`, `WorldReset`, any other burn) **leaves the row
  untouched**: the guard checks only live items, so a `RETIRED` item's row is inert, never read and
  never written again, and those aggregates gain no timed line.
- **Absent row = revision 0.** Every write names the revision it expects. Expecting 0 means "no row
  yet": the write is an insert with the precondition that no row exists, creating revision 1. If a
  row already exists, the write finds no match and writes nothing, exactly like a revision mismatch;
  a replay of the same occurrence then returns the result its first commit recorded (keyed by the
  item and the expected revision). A first checkpoint, a first equip, an expiry
  and a repair before any earlier write all use expected revision 0.
- **Writes.** Every write is a DUR-03 `STATE_MUTATION` (§11.1) or part of a `TRANSFORM`
  (`PRESERVE_INSTANCE`), in a one-item transaction (or as the one timed line of an equip move, §6),
  under the item writer's fence and the holder's `character_root` lock, under a closed
  `TimedItemCause` (§10). The row moves with the item; no move writes it except §6.
- **Guard.** A deferred database guard checks, at commit, that a live item has at most one row, that
  the row is spent unless the item's definition (for an inactive form, its paired active form) is
  an admitted timed one, that `charges` and `remaining_ms` are non-NULL exactly when that definition has `charges` and
  `temporal` (both NULL, spent, when the current definition is not timed), that they never exceed
  its values, and that `state_revision` only increases. TIMED-1 needs no migration of existing items.

## 5. Clocks and charges (TIMED-1)

- **Live** means: an `on_equip` active form in its `CharacterEquipment` slot of a character whose
  actor is in the world. Everything else is frozen (R1).
- **The runtime owns live values.** The channel runtime (or house scope) that hosts the actor keeps
  the live remaining time and charges of the actor's equipped timed items and commits them as a
  **checkpoint**: its own one-item item-writer transaction per changed item, under
  `TimedItemCause::Checkpoint {item, state_revision}`, requiring the row to still have that revision,
  so a replay or a superseded checkpoint writes nothing. Checkpoints run at the A13 cadence (at most
  every 60 s), at logout before OFFLINE-0's `logout` marker, and before a channel transfer or house
  handoff. Charges are runtime-owned like time: spending one changes only the live value, and the
  next checkpoint commits it; there is no synchronous per-spend write. A crash loses at most one checkpoint of time and charges, in the player's
  favour (R2).
- **Stop before leaving the slot.** Before any transaction moves a live item out of its slot (an
  unequip, a swap, a death drop), the runtime stops its clock and
  commits its checkpoint with the exact live value; the move then carries no timed value write. If
  the move fails, the item is still equipped and its clock restarts from the row. A death drop
  therefore writes no timed line: the dying actor's runtime commits those checkpoints before it
  submits the death settlement, and a dropped active form lies frozen.
- **Trade.** PLAYER-TRADE-0 refuses as `NOT_TRADEABLE` an equipped item whose clock is running, once
  equipped items can be offered; an unequipped timed item is frozen and trades with its row.
  Amended: PLAYER-TRADE-0 §4.
- **Protection charges.** When an active item's `PROTECTION` (EQUIP-0 §3.1) contributes to reducing
  an incoming hit of its element at the GAME-ABILITY-01 §10 stage, an item with charges loses one
  charge per hit (Canary `blockHit`). Several charged items protecting against the same hit each lose
  one. Other charge consumers (exercise weapons, R4) use the same column under their own decision.
- **Decay target.** An item's decay target is its definition's `transform {trigger: decay}` if
  present, else `temporal.decay_target`; content validation rejects a definition with both set to
  different targets. The same target applies whether the item runs out of time or of charges.
- **Expiry** at `remaining_ms = 0` or 0 charges: the item stops being active at once, and the
  runtime commits, **with no checkpoint first**, one atomic one-item transaction that writes the
  final state directly (so the row never stores 0 charges) under `TimedItemCause::Expire {item, reason}`, `reason`
  `TimeExhausted {state_revision}` or `ChargesExhausted {state_revision}`, keyed by (item, reason)
  and requiring the row's current `state_revision` to equal the reason's, so a replay or a stale
  reason writes nothing. It is either:
  - a `TRANSFORM` (`PRESERVE_INSTANCE`) to the decay target in place, in which the row's values are
    reset from the target's definition (frozen unless that form is equipped and
    live), or set spent when the target is not timed, with the revision + 1; or
  - only when there is no decay target, a **BURN** that retires the item: it moves from its
    equipment slot to `RETIRED` with no location, and its row stays as the inert row of a retired
    item. Its evidence is one audit
    event with the item, definition, location, charges, remaining time and `state_revision` before,
    `RETIRED` after, and the cause. This one-item shape is admitted by the DUR-03 §39.3 amendment
    (§10), which supersedes the §39.1 burn exclusion for it only.

## 6. Equip forms (TIMED-1)

- **Equip and unequip transforms.** When ITEM-MOVE-WIRE-1 §4 equips an item whose definition has a
  `transform {trigger: equip}`, the same move transaction transforms it (`PRESERVE_INSTANCE`) into the
  active form, creating its row if the inactive form had none, and the runtime starts the clock from
  the row after the commit. An unequip with `trigger: unequip` transforms it back after the stop
  checkpoint (§5). `remaining_ms` carries across both forms (Canary). An unequip of an active form
  without an unequip transform keeps the active form, frozen.
- **Equip moves carry at most one timed transition.** A move that equips or unequips one item with
  such a transform carries that item's `TRANSFORM` and, for a first equip, its row creation, on the
  item the move already touches (one touched item; the existing move rows plus one transform
  input/output and one timed line). A **swap** in which **both** items need a timed transition (a
  ring for a ring) is refused `SWAP_TIMED_BOTH`, writing nothing; the client unequips first. A swap
  in which one item needs one carries that one transition on its already-touched item, within the
  swap's 2 touched items. Amended: DUR-03 §33, ITEM-MOVE-WIRE-1 §6.
- **Wire result.** `SWAP_TIMED_BOTH` adds no result code: the move answers the existing `BLOCKED`
  (ITEM-MOVE-WIRE-1 §3), and the server's reason is logged, not sent. Amended: ITEM-MOVE-WIRE-1 §3.
- **Ceilings (DUR-03 §28), registered by ITEM-MOVE-2a with TIMED-1, with max and max+1 tests:**

  | Shape | Touched items | Location lines | Transform I/O | Timed row lines | Participants / work units |
  |---|---|---|---|---|---|
  | equip or unequip with a timed transform | 1 | 2 | 1 / 1 | 1 | 1 / 5 |
  | swap with one timed transform | 2 | 4 | 1 / 1 | 1 | 2 / 8 |

  Each timed row line is fixed-size, at most 64 bytes, inside the existing `DUR03-RL-07` envelope
  and payload ceilings; the child measures the encoded worst case of both shapes and fails the build
  above them.
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
  - one `TRANSFORM` (`PRESERVE_INSTANCE`) of one live item of `from_item` that is a **direct entry
    of the equipped main backpack** (the first in B3 order, as NPC-0 SELL finds its item; never an
    equipped item or one inside a nested bag, so no ancestor path is locked and
    `DUR03-RL-05` stays 0), into `to_item`. Worn soft boots in a bag must be moved to the main
    backpack first; otherwise the offer answers that the item is missing. The row (spent, since worn soft boots are not timed) is reset to
    `to_item`'s full charges and duration (for an inactive form, its paired active form's) with
    the revision + 1; a worn item without a row gets one at expected revision 0. The revision never
    repeats, so an expiry key `(item, revision)` from the earlier lifetime can never match again.
    Nothing carries over from the worn item.
  - The gold fee plan is admitted with at most **19** coin inputs here (not 20), so the 19 inputs,
    at most 2 change stacks and the repaired item stay within 22 touched items. Repair-specific rows,
    registered by TIMED-REPAIR-1 with max and max+1 tests: `DUR03-RL-04-NPC-REPAIR` transform I/O
    1 / 1; `DUR03-RL-05` 0; `DUR03-RL-06-NPC-REPAIR` 22 participants / 68 work units (the fee plan's
    64 plus the item's transform, its timed-row reset and its participant check); payload and
    envelope within `DUR03-RL-07`, measured at 19 inputs. 10,000 gold fits in one crystal coin, so this only refuses a
    player who pays from 20 or more small stacks.
  - Insufficient funds, more than 19 coin inputs, or no such item rejects the whole transaction and
    writes nothing.
- Amended: DUR-03 §39.3 (the `FeeBurnCause` variant), NPC-0 (the repair offer).

## 8. When a timed item is active (EQUIP-0)

EQUIP-0 §3.2's "and it is not `timed`" is replaced by: a timed item is active when it meets every
other §3.2 condition, its charges (if any) are above 0, its remaining time (if any) is above 0, and,
for an `on_equip` definition, it is equipped in its slot. A `continuous` definition is never active
until TIMED-ITEM-0B (§3). Its abilities then apply as EQUIP-0
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

## 10. Causes and the DUR-03 shapes

Closed `TimedItemCause`: `Checkpoint {item, state_revision}`, `EquipForm
{direction}`, `Expire {reason: TimeExhausted
{state_revision} | ChargesExhausted {state_revision}}`. Each names its ItemInstanceId and an
occurrence issued by the runtime. No generic or caller-chosen reason. The repair burn is
`FeeBurnCause::NpcRepair` (§7).

Admitted one-item shapes (DUR-03 §39.3 amendment), each with `DUR03-RL-01` 1, `DUR03-RL-07-EVENTS`
1, the existing payload and envelope ceilings and `DUR03-RL-08` 3, and these shape rows, registered
by TIMED-1 with max and max+1 tests:

| Shape | `DUR03-RL-02` location lines | `DUR03-RL-04` transform I/O | `DUR03-RL-06` participants / work units |
|---|---|---|---|
| row write | 0 | 0 / 0 | 1 / 2 |
| expiry transform | 0 | 1 / 1 (`DUR03-RL-04-TIMED-EXPIRY`) | 1 / 4 |
| expiry burn | 1 | 0 / 0 | 1 / 4 |

Lines per shape:

| Shape | Cause | Lines |
|---|---|---|
| row write | `Checkpoint` (creating the row at expected revision 0 on its first write) | one `STATE_MUTATION` of the row; no location or value line |
| expiry transform | `Expire` with a decay target | one `TRANSFORM` (`PRESERVE_INSTANCE`, 1 input / 1 output) and the row reset (or set spent), revision + 1 |
| expiry burn | `Expire` without a decay target | one BURN to `RETIRED` (1 location line) and the row left inert; `Expire` is a BURN sink (DUR-03 §15) |
| equip form | `EquipForm` | the transform and row line inside the equip move or swap (§6) |

They supersede the §39.1 exclusions of burn and transform for these shapes only. Amended: DUR-03 §15,
§33 and §39.3.

## 11. Rows

| Row | Value |
|---|---|
| `TIMEDITEM0-RL-01` charges per item | 65,535 |
| `TIMEDITEM0-RL-02` active-time budget per item | 7 days (604,800,000 ms) |
| `TIMEDITEM0-RL-03` remaining-time updates to the client per item | 1 per 60 s |

Each with max and max+1 tests. Also tested: unequip then re-equip keeps the remaining time; a
replayed or stale `Expire` writes nothing; a ring without a decay target is burned at 0 with its
event; unequipping 59 s after the last checkpoint stores the live value, so re-equipping never adds
time; a ring-for-ring swap is refused `SWAP_TIMED_BOTH` and a ring onto an empty or a non-timed slot
works; a death drop writes no timed line and the dropped ring keeps its checkpointed time; lighting a
torch is refused `NOT_ADMITTED`; repairing worn soft boots into unworn ones writes the row with full values
and revision + 1 (or creates it at expected revision 0 when none existed); worn soft boots inside a bag are not found by the repair,
and found once moved to the main backpack; a first checkpoint inserts at expected revision 0 and a second insert attempt writes nothing; soft boots repaired after expiry keep their row with a higher revision, so the old expiry key never matches; a ring retired by `WorldReset` keeps an inert row and
the reset writes no timed line; an expiry transform at 1 / 1 passes and 2 / 1 is refused; a newly minted unworn ring has no row until its first equip, which creates it at expected revision 0; spending a charge writes nothing until the next checkpoint; the last charge's expiry commits without a prior checkpoint and never stores 0 charges; a
ring-for-ring swap answers `BLOCKED`; a repair paid from 19 coin stacks commits and from 20 is refused; a life
ring's regeneration ticks alongside food regeneration; the energy ring's shield stays while mana is
0 and ends at unequip; a charge-only item at 0 charges with a `transform {trigger: decay}`
transforms, and is burned only without any decay target; a freshly minted ring has no row and
full values, and its first checkpoint creates the row. Content validation refuses a definition above RL-01 or RL-02.

## 12. Rejected options

- **A write per charge or per second.** A13 checkpoints bound both the write rate and the crash loss.
- **Time running for logged-out characters or in the depot.** Canary decays only in the game world;
  players expect a ring to keep its time while they are offline.
- **Charges as stack quantity.** GAME-ITEM-01 §4.2 forbids it.
- **Live values written inside moves.** A move would need a timed value line per item (trees,
  swaps, deaths); a checkpoint before leaving the slot keeps every move free of timed values.
- **Continuous duration here.** Ground deadlines and container trees need their own shapes; seven
  review rounds showed they are a separate decision (TIMED-ITEM-0B).
- **A two-transition swap shape.** Refusing `SWAP_TIMED_BOTH` costs the player one extra move and
  adds no shape.
- **A generic repair fee.** D178 admits fee sources one by one; the owner admitted this one.
- **Exercise weapons here.** They get EXERCISE-0, which reuses §4 and §5.

## 13. Architect rulings (owner rule 5905825574)

- **R1. Where time runs.** a) Only while equipped in the game world (recommended: Canary for
  `on_equip`); b) always by wall clock. **Ruled a).**
- **R2. Crash loss.** a) Return at most one checkpoint of time and charges to the player
  (recommended: as A13 and OFFLINE-0); b) a write per change. **Ruled a)**, `PARITY_PENDING`.
- **R3. Continuous duration.** a) `NOT_ADMITTED` until TIMED-ITEM-0B (control-plane scope cut,
  owner informed); b) here. **Ruled a).**
- **R4. Exercise weapons.** a) A separate EXERCISE-0 that uses these charges (recommended); b) here.
  **Ruled a).** OFFLINE-0's pointer is amended.
- **R5. Ring-for-ring swap.** a) Refuse `SWAP_TIMED_BOTH` (recommended: no new shape); b) a bounded
  two-item timed swap shape. **Ruled a)**, a declared difference (Tibia swaps directly).

## 14. Owner questions

None open. Owner answer 1a settled the only fee source.

## 15. Decision test

- **Must decide now:** YES. Without it every ring, charged amulet and soft boots grants nothing
  (EQUIP-0 R2).
- **Blocked:** TIMED-1, TIMED-REPAIR-1, TIMED-WIRE-1, EXERCISE-0.
- **Minimum sufficient:** one side table, runtime clocks committed at existing checkpoints and before
  leaving a slot, one-item shapes, closed causes, one repair offer.
- **Harder later:** the per-item timed table and its guard couple this decision's transform
  shapes to it, and "no row means full values" binds every later writer; TIMED-ITEM-0B must add location clocks to this table without breaking the
  frozen-by-default rule; `SWAP_TIMED_BOTH` becomes client-visible behaviour; `TIMED_ITEMS_V1`
  fields join the wire compatibility surface.
- **Superseding evidence:** an official source on where items decay or how charges are spent.
- **Deliberately not decided:** `continuous` duration (TIMED-ITEM-0B), exercise weapons
  (EXERCISE-0), invisibility from equipment, imbuement durations.

## 16. Before-freeze checklist

1. **Contract amendments:** EQUIP-0 §3.1 and §3.2; DUR-03 §15, §33 and §39.3; ITEM-MOVE-WIRE-1 §6;
   ITEM-USE-0 (continuous forms not admitted); PLAYER-TRADE-0 §4; NPC-0 (repair offer); CONDITIONS-0
   §3; OFFLINE-0 (exercise weapons pointer). Applied in this PR.
2. **Serialization:** every timed write is a one-item DUR-03 transaction or the one timed line of an
   equip move, under the item writer's fence and the holder's `character_root` lock; the runtime is
   the only live owner of an equipped item's clock and charges, and checkpoints before the item
   leaves its slot; an absent row is revision 0; rows are never deleted while the item lives, so
   `state_revision` is monotonic and expiry keyed by (item, `state_revision`) never repeats.
3. **Restart:** equipped items lose at most one checkpoint, in the player's favour; frozen items do
   not change; an item without a row has its definition's full values, so no backfill exists.
4. **Typed references:** ItemInstanceId, definition keys, NPC offer ids.
5. **Wire:** one capability with two optional fields; no new command.
6. **Split work:** expiry, repair and each checkpoint are single-item transactions.
