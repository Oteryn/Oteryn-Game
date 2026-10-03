# TIMED-ITEM-0 Timed-item state and the soft boots repair

- Decision: `TIMEDITEM0-CHARGES-DURATION-AND-REPAIR-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence)
  and protected integration.
- Scope: **narrowed twice** by the control plane: continuous duration to TIMED-ITEM-0B (D282), then
  all runtime persistence of charges and duration to TIMED-ITEM-0B (**D285**, #1622). This decision
  keeps the catalogue facts, the timed-row table shape with its invariants, and the NPC soft boots
  repair (owner answer 1a).
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
  CONDITIONS-0 (mana shield, regeneration); A13 §4.5 (checkpoints); owner rule 5905825574; D282 and D285.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| TIMED-CONTENT-1 | content lane | `charges`, `temporal` and `transform` facts on Item definitions (rings, amulets, soft boots, torches, lamps), from TibiaWiki first and Canary `items.xml` as the fallback (`charges`, `duration`, `decayTo`, `transformEquipTo`, `transformDeEquipTo`, `showcharges`, `showduration`, `stopduration`); the repair offer (§7) | ITEM-SEM-2b |
| TIMED-REPAIR-1 | impl, persistence review | the timed-row table and its guard (§4), and the NPC repair service (§7), the table's first writer | NPC-TRADE-1 (the gold fee path) |

Until TIMED-ITEM-0B, timed items still grant nothing (EQUIP-0 R2). Later, each with its own decision:
**TIMED-ITEM-0B** (runtime persistence of charges and duration, clocks, checkpoints, expiry, equip
forms, the active rule for EQUIP-0, conditions, wire; continuous duration; §5), exercise weapons
(EXERCISE-0, R4), invisibility from equipment, item imbuement durations (IMBUE-FORGE-0's own clock).

**Amendment (pending on acceptance of TIMED-ITEM-0B §4 and §9.2).** TIMED-RT-1a creates the §4
table in migration 0054, with TIMED-ITEM-0B's `deadline_at` column and write records; TIMED-REPAIR-1
depends on it and writes its record under `npc_repair`. An equip transform never writes the row: its
first write is a checkpoint, `SetDeadline`, `ClearDeadline`, `PutOut`, an expiry or the repair.

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

## 3. Scope (D282, D285)

- **Here:** catalogue facts (TIMED-CONTENT-1), the timed-row table shape and its invariants (§4),
  and the NPC soft boots repair with its rows (§7).
- **TIMED-ITEM-0B:** everything that writes live values: clocks, checkpoint cadence, charge use,
  checkpoint/expiry sequencing, the stale-write and replay rules for live-value writes, equip forms,
  when a timed item is active (EQUIP-0), its conditions, the wire, and continuous duration.
- A definition whose `temporal.consumption_mode` is `continuous` is `NOT_ADMITTED`: a use that would
  transform an item into a `continuous` form (lighting a torch) is refused, writing nothing, and
  answers ITEM-USE-0's existing `REJECTED` disposition (no new wire value). Amended: ITEM-USE-0.

## 4. The timed-row table (TIMED-REPAIR-1)

Table `game_item_timed_states`, one row per ItemInstance whose current definition is an admitted
`timed` definition (charges, or `on_equip` temporal), or is the **inactive form** of such a pair: a
definition with a `transform {trigger: equip}` into one, such as an unworn ring or unworn soft boots:

| Column | Meaning |
|---|---|
| `item_instance_id` | primary key, references `game_item_instances` |
| `charges` | 1..65,535, or NULL for a definition without charges or a spent row (0 is never stored; reaching 0 is TIMED-ITEM-0B's expiry) |
| `remaining_ms` | the active-time budget left, 0..`TIMEDITEM0-RL-02`, or NULL without `temporal` |
| `state_revision` | uint64, +1 per write |

- **Creation (one rule): rows are lazy.** No MINT, migration or foreign shape ever writes a row.
  An item of an admitted timed definition **without a row has the full charges and duration of its
  definition** (for an inactive form, of its paired active form). Its first own write creates the
  row at expected revision 0: here the repair (§7); in TIMED-ITEM-0B its first checkpoint, equip
  transform or expiry. No write per charge spent ever exists. So the MINT paths (loot, rewards, NPC BUY, any later source) and the existing
  population need no row and no backfill. Content validation refuses an NPC offer whose `count`
  differs from the definition's charges, since a fresh item always has full charges (NPC-0 §5).
- **Paired forms keep the row.** The row belongs to the item, not to the form. An unequip transform
  into the inactive form (TIMED-ITEM-0B) keeps `remaining_ms`.
- **Only timed shapes transform such an item** (the repair here; equip forms and expiry in
  TIMED-ITEM-0B); content validation refuses any other transform rule from or into an admitted
  timed definition. **Once created, a row
  is never deleted while the item lives**, so `state_revision` only ever grows for an instance:
  the repair (§7) and TIMED-ITEM-0B's expiry reset its values and add 1 to the revision. A row of an item whose
  current definition is not timed (worn soft boots after expiry) is **spent**: both values NULL.
  **Every other retirement path** (`DECAY_RETIRE`, `WorldReset`, any other burn) **leaves the row
  untouched**: the guard checks only live items, so a `RETIRED` item's row is inert, never read and
  never written again, and those aggregates gain no timed line.
- **Absent row = revision 0.** Every write names the revision it expects. Expecting 0 means "no row
  yet": the write is an insert with the precondition that no row exists, creating revision 1. If a
  row already exists, the write finds no match and writes nothing, exactly like a revision mismatch;
  a replay of the same occurrence then returns the result its first commit recorded (keyed by the
  item and the expected revision). Every first write of an item, the repair
  here and TIMED-ITEM-0B's writes alike, uses expected revision 0.
- **Writes.** Every write is a DUR-03 `STATE_MUTATION` (§11.1) or part of a `TRANSFORM`
  (`PRESERVE_INSTANCE`), in a one-item transaction (here: the repair's single timed-row write), under the item writer's
  fence and the holder's `character_root` lock. The row moves with the item; no move writes it.
- **Guard.** A deferred database guard checks, at commit, that a live item has at most one row, that
  the row is spent unless the item's definition (for an inactive form, its paired active form) is
  an admitted timed one, that `charges` and `remaining_ms` are non-NULL exactly when that definition has `charges` and
  `temporal` (both NULL, spent, when the current definition is not timed), that they never exceed
  its values, and that `state_revision` only increases. The table needs no migration of existing items.

## 5. Moved to TIMED-ITEM-0B, with entry conditions

TIMED-ITEM-0B takes the runtime design from this PR's earlier heads (rounds 1-12, #1471) as its
starting draft and is accepted only when it states and tests:

1. **Per-item serialization of checkpoint and expiry**, or a retry of the expiry against the new
   revision that preserves the live zero, with a test that an exhausted item never regains a
   durable charge after a restart (D285; round 12).
2. Checkpoints at the A13 cadence, committed at logout and handoff, one per changed item, with
   charges checkpointed and never written per spend.
3. The stop-before-leaving-the-slot rule, so no move carries a timed value write.
4. Expiry as one atomic one-item shape (transform or BURN sink) with its DUR-03 rows, never storing
   0 charges.
5. Equip forms with their move and swap ceilings, and `SWAP_TIMED_BOTH` answered as `BLOCKED`.
6. The active rule for EQUIP-0 and the `ITEM_REGENERATION` and item mana shield conditions.
7. The `TIMED_ITEMS_V1` wire.
8. The §4 invariants unchanged: absent row = revision 0, monotonic revision, rows never deleted
   while the item lives.

## 6. Rows and tests

| Row | Value |
|---|---|
| `TIMEDITEM0-RL-01` charges per item | 65,535 |
| `TIMEDITEM0-RL-02` active-time budget per item | 7 days (604,800,000 ms) |

Content validation refuses a definition above RL-01 or RL-02, and an NPC BUY or SELL offer of an
admitted timed definition whose `count` differs from the definition's charges. Tests: the guard
(at most one row per live item, spent rows only for non-timed definitions, monotonic revision);
repairing worn soft boots writes a full-value row with revision + 1, or inserts it at expected
revision 0; a second insert at revision 0 writes nothing; soft boots in a bag are not found and are
found once moved to the main backpack; a repair paid from 19 coin stacks commits and from 20 is
refused; the repair's audit event carries the item id, the transform before and after and the timed row before and after (including "absent → revision 1"); `DUR03-RL-04-NPC-REPAIR` and `DUR03-RL-06-NPC-REPAIR` at max and max+1; lighting a torch is
refused and answers `REJECTED`.

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
    repeats.
    Nothing carries over from the worn item.
  - The gold fee plan is admitted with at most **19** coin inputs here (not 20), so the 19 inputs,
    at most 2 change stacks and the repaired item stay within 22 touched items. Repair-specific rows,
    registered by TIMED-REPAIR-1 with max and max+1 tests: `DUR03-RL-04-NPC-REPAIR` transform I/O
    1 / 1; `DUR03-RL-05` 0; `DUR03-RL-06-NPC-REPAIR` 22 participants / 68 work units (the fee plan's
    64 plus the item's transform, its timed-row reset and its participant check); payload and
    envelope within `DUR03-RL-07`, measured at 19 inputs including the repair's own audit lines.
  - **Evidence.** The transaction's one audit event (DUR-03 §27) carries the fee evidence and also
    the repaired item's ItemInstanceId, the `TRANSFORM` line (definition before and after) and the
    timed row before and after (charges, remaining time and revision, or "absent → revision 1"). 10,000 gold fits in one crystal coin, so this only refuses a
    player who pays from 20 or more small stacks.
  - **Bank part (TIMED-ITEM-0A; owner answer 2, #1622 5968564302; pending on acceptance of
    BANK-FEE-0 and on GOLD-FEE-2).** The bank `FEE_DEBIT` writer, its ledger entry, schema and audit
    belong to GOLD-FEE-2 (BANK-FEE-0 §4). TIMED-REPAIR-1 admits the bank path only after GOLD-FEE-2
    has merged. Until then the repair stays coins only and rejects a short payer. The repair pays as
    Tibia NPCs do: carried coins first, then the bank, under
    BANK-FEE-0 §3 unchanged. If the eligible coins are worth `T >= F`, the coin plan above runs and
    the bank is not touched. If `T < F` and the payer is not junior, every eligible coin is burned
    whole with no change, and `F - T` is debited from the payer's (Account, World) balance as one
    `FEE_DEBIT` value line under `FeeBurnCause::NpcRepair` (BANK-FEE-0 §4). A junior payer
    (BANK-0 §4.4) keeps coins only.
    - The bound still holds. The main backpack has at most 20 direct entries, and the worn soft
      boots take one. So at most 19 coin inputs exist, the bank path mints no change, and the
      touched items stay within 20. The repair rows above are unchanged. `DUR03-RL-03-FEE` is 1
      when the bank is used, and 0 otherwise. The value line counts in the measured payload of
      `DUR03-RL-07` at 19 inputs.
    - Lock order: the item writer's fence and the `character_root` lock, then the backpack with its
      coin entries and the repaired item, then the bank balance row (BANK-0 §4.1).
    - The fee record's root-advance guard composes with the repair's item-only branch as it does
      with NPC-TRADE-1 and NPC-TRAVEL-1 (BANK-FEE-0 §4.1). The bank part is an outcome, not part of
      the request binding.
    - The transaction's one audit event carries the value line, and no separate BANK-0 bank event
      is emitted (BANK-FEE-0 §4.3).
  - Rejection: the whole transaction is rejected and nothing is written when:
    - there is no such item;
    - there are more than 19 coin inputs on the coin path;
    - funds are insufficient, meaning `T` plus the bank balance is below `F`, or a junior payer's
      coins are below `F`.
- Amended: DUR-03 §39.3 (the `FeeBurnCause` variant and, by TIMED-ITEM-0A, the bank part), NPC-0
  (the repair offer).

## 8. Rejected options

- **Iterating the runtime persistence here.** Twelve review rounds showed it is its own decision
  (D285); the table shape and the repair do not depend on it.
- **A generic repair fee.** D178 admits fee sources one by one; the owner admitted this one.
- **Exercise weapons here.** They get EXERCISE-0.

## 9. Architect rulings (owner rule 5905825574)

- **R1. Runtime persistence.** a) TIMED-ITEM-0B, with §5's entry conditions (control plane D285);
  b) here. **Ruled a).**
- **R3. Continuous duration.** a) `NOT_ADMITTED` until TIMED-ITEM-0B (D282); b) here. **Ruled a).**
- **R4. Exercise weapons.** a) A separate EXERCISE-0 (recommended); b) here. **Ruled a).** OFFLINE-0's
  pointer is amended.

## 10. Owner questions

None open. Owner answer 1a settled the only fee source. Owner answer 2 (#1622 5968564302) admitted
the bank part of the repair (TIMED-ITEM-0A, §7). It adds no fee source and no sink to D178.

## 11. Decision test

- **Must decide now:** YES. The soft boots repair (owner answer 1a) and the catalogue facts need the
  table shape; TIMED-ITEM-0B needs fixed invariants to build on.
- **Blocked:** TIMED-REPAIR-1, TIMED-CONTENT-1, TIMED-ITEM-0B. TIMED-REPAIR-1's bank path also
  depends on GOLD-FEE-2 (§7).
- **Minimum sufficient:** one table with its guard and three invariants, one repair offer with its
  rows.
- **Harder later:** "no row means full values", "absent row = revision 0" and the monotonic
  revision bind every later timed writer, TIMED-ITEM-0B included.
- **Superseding evidence:** an official source on repair prices or offers.
- **Deliberately not decided:** all runtime persistence (TIMED-ITEM-0B), continuous duration,
  exercise weapons, invisibility from equipment, imbuement durations.

## 12. Before-freeze checklist

1. **Contract amendments:** DUR-03 §15 and §39.3 (`NpcRepair`, its rows, the reserved
   `TimedItemCause`); ITEM-USE-0 (continuous forms not admitted); NPC-0 (the repair offer);
   OFFLINE-0 (exercise weapons pointer). Applied in this PR.
2. **Serialization:** the repair is one item-only transaction under the item writer's fence and the
   holder's `character_root` lock.
3. **Restart:** nothing runs on a clock in this decision.
4. **Typed references:** ItemInstanceId, definition keys, NPC offer ids.
5. **Wire:** none.
6. **Split work:** TIMED-ITEM-0B.
