# TIMED-ITEM-0B Runtime charges and duration

- Decision: `TIMEDITEM0B-RUNTIME-CHARGES-AND-DURATION-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  determinism and protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3), second architect lane
- Answers:
  - control-plane allocation D351 (#1622, owner 1b: TIMED-ITEM-0B and PROFICIENCY-1B as decisions
    first, an exception to D252);
  - TIMED-ITEM-0 §3 and §5 (D282, D285: everything that writes live values, with eight entry
    conditions);
  - EQUIP-0 R2 ("a `timed` item grants nothing until TIMED-ITEM-0");
  - EXERCISE-0 §5.1 (the one composed checkpoint writer TIMED-ITEM-0B must admit) and §5.3 (the
    expiry BURN sink);
  - ITEM-USE-0 §6 (continuous forms refused until this decision);
  - WORLD-INTERACTION-0 §11.4 (carried torches wait for the timed-item decision);
  - owner answer D360 (#1622, question 1 c with one addition: a lit torch burns in a hand or the
    Extra slot, on the ground and in houses, with the Ground-deadline model; put into any container
    it goes out).
- Builds on:
  - TIMED-ITEM-0 §4 (the timed-row table and its invariants, unchanged here) and §7 (the repair);
  - the round 1-12 runtime draft of #1471 (head `d708a63c`), as TIMED-ITEM-0 §5 directs;
  - GAME-ITEM-01 §4.2 and §4.4; EQUIP-0 §3; CONDITIONS-0 §3, §3.2, §3.3 and §6;
  - DUR-03 §7, §11.1, §15, §16.2, §24, §25, §28, §33 and §39.3;
  - ITEM-MOVE-WIRE-1 §3-§6 (equip, drop, pickup); GROUND-MOVE-1 (WORLD-INTERACTION-0 §7.2);
    HOUSE-CUSTODY-0 (house tiles); D3 (corpse decay as a resumable step); ITEM-USE-0 §4-§6; A13 §4.5 (checkpoints); OFFLINE-0
    (the logout marker); EXERCISE-0 §4-§5;
  - owner rule 5905825574.
- Amends, each pending on acceptance of TIMED-ITEM-0B and written by the named child in its own
  docs commit (this PR adds only this file, PROFICIENCY-1B and the task record):
  - TIMED-ITEM-0 implementation brief and §4 (TIMED-RT-1 creates the table with the added
    `deadline_at` column; TIMED-REPAIR-1 follows it);
  - EQUIP-0 §3.1 and §3.2 (TIMED-FX-1); CONDITIONS-0 §3, §3.2 and §3.3 (TIMED-FX-1);
  - DUR-03 §15, §33 and §39.3 (TIMED-RT-1); ITEM-MOVE-WIRE-1 §3 and §6 (TIMED-RT-1);
  - ITEM-USE-0 §6 (TIMED-RT-1); MARKET-0 §3.1 (TIMED-RT-1); WORLD-INTERACTION-0 §11.4 (TIMED-RT-1).
- Runtime, migration and production authority: NONE. Each child needs its own #1622 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| TIMED-RT-1 | hard (persistence), persistence and determinism review | the timed-row table with `deadline_at`, its guard and the write records (§4, TIMED-ITEM-0 §4); the per-item write lane (§5); checkpoints (§6); charge use (§7); expiry (§8); equip and use forms with their ceilings (§9); torches in slots and on Ground (§10.1-§10.5); the causes and DUR-03 shapes (§12) | TIMED-CONTENT-1; ITEM-MOVE-2a; ITEM-MOVE-2b (drop and pickup); EQUIP-RT-1 |
| TIMED-HOUSE-1 | impl, persistence review | lit torches on house tiles (§10.6), reusing §10's deadline shapes under the house scope | TIMED-RT-1; the house-lane item placement (HOUSE-CUSTODY-0) |
| TIMED-FX-1 | hard (combat), combat and determinism review | the EQUIP-0 active rule for timed items, `ITEM_REGENERATION` and the item mana shield (§11) | TIMED-RT-1; COND-1 |
| TIMED-WIRE-1 | impl, protocol review | capability `TIMED_ITEMS_V1`, the item fields and the Look text (§13) | TIMED-RT-1 |
| TIMED-PARITY-1 | impl | fixtures: ring and soft boots durations, protection charges per hit, a torch's burn time, against TibiaWiki | TIMED-FX-1 |

TIMED-REPAIR-1 (TIMED-ITEM-0 §7) now depends on TIMED-RT-1, which creates the table it writes.
EXERCISE-1 depends on TIMED-RT-1 for the composed checkpoint (§6.3) and the expiry sink (§8).

## 1. Question

How do the live charges and remaining time of a worn, held or lit item run, become durable,
run out and change form, so that no value is created or lost beyond one checkpoint, and so that
EQUIP-0 can finally let rings, amulets, soft boots and torches do something?

## 2. Facts

**PROVEN**

- TIMED-ITEM-0 §4 (merged): rows are lazy; an item without a row has its definition's full values;
  an absent row is revision 0; every write names its expected revision; `charges` is never 0;
  `state_revision` only grows; rows are never deleted while the item lives; a write is a DUR-03
  `STATE_MUTATION` or part of a `TRANSFORM` (`PRESERVE_INSTANCE`) under the item writer's fence and
  the holder's `character_root` lock.
- TIMED-ITEM-0 §5: the eight entry conditions this decision answers (the table in §15).
- Tibia manual (`CIPSOFT_OFFICIAL`, `combat.md`): soft boots run down only while worn; charged
  items show their charges.
- EXERCISE-0 §5: an exercise weapon's charges and the tries they pay for become durable together in
  one transaction; the last charge retires the weapon in that same transaction under the expiry sink
  this decision admits.
- ITEM-MOVE-WIRE-1 §3 has a `BLOCKED` move result; §6 admits the equip swap shape (2 items, 4
  location lines).
- CONDITIONS-0 §6.3: only food regeneration time is durable in V1.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`; not value evidence)

- Rings and soft boots have inactive and active forms (`transformEquipTo`, `transformDeEquipTo`);
  only the active form has a `duration`; the remaining time survives an unequip.
- `player.cpp` `blockHit`: an equipped charged item whose protection reduces a hit loses one charge.
- A lit torch decays to a burnt-out torch wherever it lies; decay runs only for items in the game
  world, so a logged-out character's items keep their time.

**OWNER** (D360): a lit torch burns in a hand or the Extra slot, on the ground and in houses; put
into a backpack or any container it goes out (it becomes the unlit item and its time stops).

**UNKNOWN** (TIMED-PARITY-1 checks against TibiaWiki; TibiaWiki was unreachable from this lane's
container, HTTP 402)

- Per-item durations and charges (TIMED-CONTENT-1 carries them, TibiaWiki first);
- whether a protection charge is spent per hit or per absorbed hit on Global.

## 3. Vocabulary

- **Timed item:** a live ItemInstance whose current definition is an admitted timed definition, or
  the inactive form of one (TIMED-ITEM-0 §4).
- **Live:** an item whose values are running in a runtime right now (§5.1). Every other timed item
  is **frozen**: its values are exactly its row (or its definition's full values without a row), and
  nothing changes them.
- **Live values:** the runtime's current charges and remaining time of a live item. They are never
  read by another runtime and never written except through the item's lane (§5).
- **Lane:** the ordered write queue of one live or deadline item (§5.2).
- **Deadline item:** a lit `continuous` item lying directly on a Ground tile or a house tile. Its
  time runs on a durable absolute deadline (`deadline_at`) on database time, not in a runtime's
  memory (§10).

## 4. State (TIMED-RT-1)

- **The table** is TIMED-ITEM-0 §4's `game_item_timed_states` with its invariants unchanged (lazy
  rows, absent row = revision 0, monotonic revision, rows never deleted while the item lives, the
  guard), plus one nullable column for D360:

  | Column | Meaning |
  |---|---|
  | `deadline_at` | NULL, or the database time (UTC ms) at which a deadline item expires (§10.3) |

  TIMED-RT-1 creates the table with its guard (the first child to need it; amends TIMED-ITEM-0's
  brief and §4), with a partial index on `deadline_at` where it is not NULL.
- **Guard arms for D360.** For a **live, located** item (not `RETIRED`): `deadline_at` is non-NULL
  exactly when the current definition is a lit `continuous` form and the item's immediate location
  is a Ground or house tile; while it is non-NULL, `remaining_ms` holds the budget at the moment
  the deadline was set. A lit `continuous` item never has a container as its immediate location.
  Both are checked at commit. A `RETIRED` item's row is inert as TIMED-ITEM-0 §4 says (a
  `WorldReset` of a dropped lit torch leaves its `deadline_at` as it was) and is outside both arms.
- **The expiry scheduler** reads the partial `deadline_at` index joined to live, located items
  only, so a retired item's inert deadline is never scheduled. Test: a `WorldReset` of a lit torch
  on the ground commits, writes no timed line, passes the guard, and is never expired.
- **Unlit forms.** TIMED-ITEM-0 §4's "inactive form" (a definition with `transform {trigger: equip}`
  into an admitted timed one) also covers a definition with `transform {trigger: use}` into an
  admitted `continuous` one (an unlit torch). Its row, when it has one, holds the lit form's values,
  so putting out and relighting keep the remaining time.
- **Write records.** A new immutable table `game_item_timed_state_writes`, one row per committed
  write of a timed row:

  | Column | Meaning |
  |---|---|
  | `item_instance_id`, `expected_revision` | primary key: the item and the revision the write expected (0 for "no row") |
  | `cause` | the `TimedItemCause` variant (§12) or `npc_repair` |
  | `transaction_id` | the DUR-03 TransactionId of the write |
  | `definition_before`, `definition_after` | definition keys (equal unless the write is part of a transform) |
  | `charges_before`, `remaining_ms_before` | the row before, or the definition's full values for an absent row |
  | `charges_after`, `remaining_ms_after` | the row after (NULL, NULL for spent; the before values for a BURN, whose row stays inert) |
  | `deadline_before`, `deadline_after` | `deadline_at` before and after |
  | `committed_at` | commit time |

  - The guard gains one arm: a commit that inserts or updates a timed row inserts exactly one write
    record with `expected_revision` equal to the row's previous revision (0 for an insert) and the
    after values equal to the new row. A BURN of a timed item inserts a record and leaves the row.
  - The primary key makes "at most one commit per (item, expected revision)" a database fact, so a
    replay or a stale write can never commit twice.
  - The record is the DUR-03 §24 receipt of the timed write. It is retained at least as long as the
    one-item audit of the same transaction (P90D, DUR-03 retention decision); after that a replay
    is answered from the row, whose revision already proves a later commit.
  - The repair (TIMED-ITEM-0 §7) writes its record under `npc_repair` in the same way.
  - Grants: REVOKE ALL from PUBLIC; `oteryn_game_runtime` SELECT and INSERT on the records and
    SELECT, INSERT and UPDATE on the rows (never DELETE); `oteryn_game_control` SELECT; guard
    functions with a fixed `search_path`; truncate rejected on both tables.
- **Content compatibility.** A content revision may raise but never lower an admitted timed
  definition's charges or duration, and may not change which forms pair. Lowering one needs its
  own decision (a stored row would break the guard's "never exceed" arm). Content validation
  refuses it.
- **No stacking.** An admitted timed definition is not stackable; content validation refuses
  otherwise (GAME-ITEM-01 §4.2: charges are per item, not stack quantity).

## 5. Ownership and the per-item lane (TIMED-RT-1)

### 5.1 Who owns live values

- **Live items.** An item is live only in one of these places, and only while its holder's actor
  is in the world on the hosting runtime (the ChannelRuntime, or the house scope that hosts it):
  1. an `on_equip` active form in its definition's `CharacterEquipment` slot (rings, soft boots);
  2. a charged item in its definition's `CharacterEquipment` slot that meets EQUIP-0 §3.2's other
     conditions (requirements met at equip, Premium held now), with charges above 0 (charged
     amulets and rings): its charges are live, spent only by §7.

  Live eligibility is decided from these placement and ownership facts alone, at equip, login,
  respawn and transfer; §11.1 then derives "active" from "live", never the reverse.
  3. an exercise weapon bound to a running EXERCISE-0 session (charges only);
  4. a `continuous` lit form in a `CharacterEquipment` slot (carried torches, §10).

  A deadline item (§10.3) is not live: its time runs on its durable deadline, and its lane is
  kept by the scope that owns its tile (the channel's Ground owner, DUR-03 §32, or the house scope).
- **One owner.** The hosting runtime is the only writer of a live item's row. While an item is
  live it is **reserved** (DUR-03 §7.1) against every other writer: a trade offer, a market
  listing, a repair, a burn or any move that is not an equip move of §9 first stops the item
  (§5.3) and runs only after its lane is empty, or is refused when it cannot wait (a trade offer
  of a live item answers its existing `NOT_TRADEABLE`).
- **Frozen items** are written only by an equip or use transform that makes them live (§9), by a
  pickup of a deadline item (§10.4), by the repair (TIMED-ITEM-0 §7), or never.

### 5.2 The lane (entry condition 1)

- Each live item has one lane in its hosting runtime. A lane issues **at most one timed write in
  flight**. The next write is issued only when the previous outcome is known:
  - **committed:** the lane's revision becomes the expected revision + 1;
  - **known not committed** (serialization or deadlock retry, DUR-03 §23.2): the same write is
    retried with the same TransactionId and the same expected revision;
  - **ambiguous** (DUR-03 §23.3): the lane looks up the write record by (item, expected revision)
    before anything else. Found with the same TransactionId: committed. Not found and the row is
    still at the expected revision: not committed, retried as above. Unknown after
    `TIMEDITEM0B-RL-02` (2,000 ms): the item stays inactive and reserved until reconciliation reads
    the record; nothing else is written for it meanwhile.
- **Every write names (item, expected revision)** and requires the row's current revision to equal
  it (or no row, for 0). The pair is the write's idempotency key (§4). A replay of a committed key
  returns the recorded result; a write whose key finds a different committed record writes nothing.
- **Checkpoint and expiry are serialized by the lane.** When the live values reach zero while a
  checkpoint is in flight, the expiry waits for it and is issued at the revision that checkpoint
  produced. An expiry is never keyed to a revision that an in-flight write is about to replace,
  which is the round 12 race (D285).
- **Unexpected revision.** A live item's row cannot move under its lane, because only the lane
  writes it (§5.1). If a write still finds another revision, its runtime treats it as lost
  authority:
  - if the runtime's fences are no longer current, it writes nothing more for any item of that
    actor, and the new owner starts from the rows (the §6.2 crash bound);
  - if its fences are still current (an integrity fault), it re-reads the row. If the write was an
    **expiry**, it is retried once at the row's current revision when the row is still of the same
    definition, so the live zero is preserved; otherwise the item is made inactive, frozen at the
    row's values, and an integrity alarm is raised. Never is a live zero discarded while the runtime
    still holds authority.
- **Durable exhaustion is final.** Once an expiry commits, no later write of the same lifetime can
  bring charges or time back: the row is spent, reset to the decay target's values with a higher
  revision, or the item is retired (§8). Test: a checkpoint in flight, live charges reach 0, crash
  after the checkpoint commits and before the expiry is issued, restart. The new owner loads the
  checkpointed value (one interval in the player's favour, §6.2). A second test: the expiry
  commits, restart, the item has no charge and is not active.

### 5.3 Stop

**Stopping** a live item means: the runtime stops its clock and charge use, takes the live values,
and puts a checkpoint (§6) with them on the lane. The item is inactive and frozen once that
checkpoint commits. A stop is skipped (no write) when the live values equal the row.

## 6. Checkpoints (TIMED-RT-1, entry condition 2)

### 6.1 When

A live item's checkpoint is put on its lane:
- at the A13 cadence, at most every 60 s (`TIMEDITEM0B-RL-01`), skipped when nothing changed;
- at a stop (§5.3), which precedes every move out of the slot or session (§9.1);
- at logout, before OFFLINE-0's `logout` marker: the logout waits until every lane of the actor
  is empty;
- before a channel transfer or house handoff: the handoff waits likewise;
- before a death settlement: the dying actor's runtime stops every live item and submits the
  settlement only after the lanes are empty.

**Charges are checkpointed, never written per spend.** Spending a charge changes only the live
value; the next checkpoint commits it.

### 6.2 Crash bound (R2)

A crash or fence loss between checkpoints returns at most one checkpoint interval of time and
charges to the player: the new owner starts from the rows. A live zero whose expiry had not yet
committed is in that interval (§5.2's first test). This is `PARITY_PENDING`, as A13's and
OFFLINE-0's checkpoint loss.

### 6.3 Shape

- **Plain checkpoint:** one one-item transaction with one `STATE_MUTATION` of the row (an insert at
  expected revision 0 for an item that had no row) and its write record, under
  `TimedItemCause::Checkpoint`. No location or value line and no `CharacterRevision` advance.
- **Composed checkpoint (EXERCISE-0 §5.1):** the same row write plus one build receipt (A13 §4.2,
  cause `training`) in one transaction under the actor's `character_root` lock. Lock order: the
  item writer's fence and session-generation fence, `character_root`, the build state, the timed
  row. The receipt advances the `CharacterRevision` exactly as a plain build checkpoint does. A
  revision mismatch on either writes nothing (EXERCISE-0 §5.1, never split). This is the only
  composed checkpoint admitted; any other composition needs its own decision.

## 7. Charge use (TIMED-RT-1)

- **Protection charges.** When an active charged item's `PROTECTION` (EQUIP-0 §3.1) contributes a
  non-zero reduction to an incoming hit of its element at the GAME-ABILITY-01 §10 stage, it loses
  one live charge for that hit. Several charged items protecting against the same hit each lose
  one. The spend is part of the hit's Effect Plan, ordered by EQUIP-0 §3.3's slot order, so it is
  deterministic. `PARITY_PENDING` against TibiaWiki (TIMED-PARITY-1).
- **Exercise charges:** EXERCISE-0 §4.2, one per 2 s tick, on the weapon's lane.
- **No other consumer.** A later charge consumer needs its own decision; it uses this lane and
  checkpoint.
- **The charge that reaches 0** makes the item inactive at once (it protects nothing more in the
  same hit's later stages) and puts its expiry on the lane (§8).

## 8. Expiry (TIMED-RT-1, entry condition 4)

- **When:** live remaining time reaches 0, or live charges reach 0.
- **No checkpoint first.** The expiry writes the final state directly, so 0 charges and 0 ms are
  never stored for a timed definition.
- **Decay target:** the definition's `transform {trigger: decay}` if present, else
  `temporal.decay_target`; content validation refuses both set to different targets. The same
  target applies to time and to charges.
- **One atomic one-item shape**, under `TimedItemCause::Expire {reason}`, `reason` `TimeExhausted`
  or `ChargesExhausted`, at the lane's expected revision:
  - **expiry transform** (a decay target exists): one `TRANSFORM` (`PRESERVE_INSTANCE`, 1 input /
    1 output) in place, and the row reset to the target's full values, or set spent when the target
    is not timed, with the revision + 1 (soft boots to worn soft boots; a lit torch to a burnt-out
    torch). A timed target in an equipment slot is live from the commit on, starting from its
    full values;
  - **expiry burn** (no decay target): one BURN of the item to `RETIRED`, one location line from its
    slot or container entry; the row stays as the inert row of a retired item (TIMED-ITEM-0 §4)
    and the write record carries the before values. `TimedItemCause::Expire` is a BURN sink
    (DUR-03 §15, the reserved `TimedItemCause`);
  - **composed expiry (EXERCISE-0 §5.3):** the expiry burn plus the last build receipt in one
    transaction, with §6.3's lock order.
- **Evidence:** one audit event (DUR-03 §27) with the item, the definition before and after, the
  location, the row before and after (charges, remaining time, revision), the reason and the
  TransactionId.

## 9. Equip and use forms (TIMED-RT-1, entry conditions 3 and 5)

### 9.1 No move carries a live value

- Every transaction that moves a live item out of its slot (an unequip, a swap, a death drop, an
  exercise weapon leaving the session) runs only after the item was stopped (§5.3) and its lane is
  empty. The move then writes no timed value line.
- If the move fails, the item is still in its slot: the runtime makes it live again from the row.
- A death drop therefore writes no timed line, and a dropped item lies frozen. A lit torch is put
  out before the death settlement (§10.4), so it never reaches the corpse lit.
- **Exceptions (D360, §10):** a drop of a lit item onto a Ground or house tile carries one timed
  line that sets its deadline, a pickup of a deadline item carries one that clears it, and a move
  of a lit item into a container carries the put-out `TRANSFORM`. Each comes after the stop, so
  none carries a live value: the values come from the row.

### 9.2 Equip forms

- **Equip.** When ITEM-MOVE-WIRE-1 §4 equips an item whose definition has `transform {trigger:
  equip}`, the same move transforms it (`PRESERVE_INSTANCE`) into the active form. The row is not
  written: it belongs to the item, and an absent row already means full values of the active
  form. The item is live after the commit, starting from the row.
- **Amends TIMED-ITEM-0 §4's creation rule.** Its list of first writes ("its first checkpoint,
  equip transform or expiry") loses "equip transform": an equip transform never writes the row, so
  a ring's row is created at expected revision 0 by its first checkpoint, `SetDeadline`,
  `ClearDeadline`, `PutOut`, expiry or repair. The other §4 invariants are unchanged.
- **Unequip.** After the stop checkpoint, a move with `transform {trigger: unequip}` transforms it
  back. `remaining_ms` carries across both forms (the row is the item's). An active form without an
  unequip transform stays the active form, frozen.
- **No timed line in a move.** Equip forms carry only their `TRANSFORM` line on the item the move
  already touches.
- **Swaps.** A swap in which **both** items need a timed transform (a ring for a ring) is refused
  `SWAP_TIMED_BOTH` and writes nothing. On the wire it answers the existing `BLOCKED`
  (ITEM-MOVE-WIRE-1 §3); the reason is logged, not sent. A swap in which one item needs one carries
  that one transform. This is a declared difference (Tibia swaps directly; R3).
- **Ceilings (DUR-03 §28),** registered by TIMED-RT-1 with max and max+1 tests:

  | Shape | Touched items | Location lines | Transform I/O | Timed row lines | Participants / work units |
  |---|---|---|---|---|---|
  | equip or unequip with a timed transform (`DUR03-RL-0x-TIMED-EQUIP`) | 1 | 2 | 1 / 1 | 0 | 1 / 4 |
  | swap with one timed transform (`DUR03-RL-0x-TIMED-SWAP`) | 2 | 4 | 1 / 1 | 0 | 2 / 7 |

  Payload and envelope stay within `DUR03-RL-07`; the child measures both shapes' encoded worst
  case and fails the build above it.

### 9.3 Use forms (lighting and putting out)

- A use whose `transform {trigger: use}` turns an item into a `continuous` form (lighting a torch or
  a lamp), or out of one (putting it out), is admitted for an item in a `CharacterEquipment` slot.
  It is one one-item `TRANSFORM` (`PRESERVE_INSTANCE`) under `ItemUseCause::Light` (amends
  ITEM-USE-0 §4.2 and §6), with the ITEM-USE-0 reservation and in-flight rules. No row is written:
  the row belongs to the item.
- Lighting an item in a container is refused, writing nothing, and answers the existing
  `REJECTED`: it would go out at once (D360). The hotkey form (ITEM-USE-0 field 5) searches the
  equipment slots first, so it lights an equipped torch.
- Putting out a live item first stops it (§5.3); the transform then runs.
- A transform never resets the time; only an expiry into a new form, a repair or a new item does.

## 10. Torches and continuous duration (TIMED-RT-1, TIMED-HOUSE-1; owner answer D360)

### 10.1 Admission

A `continuous` definition is admitted. It replaces TIMED-ITEM-0 §3's `NOT_ADMITTED` and ITEM-USE-0
§6's refusal. Content validation refuses a MINT source (loot, rewards, NPC offers) of a lit form:
items are minted unlit.

### 10.2 Where a lit item is, and how its time runs

| Where the lit item is | Its time |
|---|---|
| a `CharacterEquipment` slot of an in-world actor (a hand, the Extra slot) | live, in the hosting runtime (§5); it gives `LIGHT` (EQUIP-0 §3.2, WORLD-INTERACTION-0 §11.4) |
| a `CharacterEquipment` slot of a character not in the world | frozen (its row) |
| directly on a Ground tile (a channel or an instance) or a house tile | a deadline item: `deadline_at` on database time (§10.3) |
| any container (backpack, bag, corpse, depot, inbox, escrow) | never: a lit item put into a container goes out (§10.4) |

### 10.3 Deadline items

- **Setting the deadline.** A drop of a lit item from a slot onto a tile (ITEM-MOVE-WIRE-1 §5,
  ITEM-MOVE-2b) first stops it (§5.3), so the row holds the exact live budget. The drop transaction
  then carries one timed line under `TimedItemCause::SetDeadline {item, expected_revision}`:
  `deadline_at = database time + remaining_ms`, `remaining_ms` unchanged, revision + 1. Dropping and
  picking up never wins back time.
- **Moving on the ground.** A Ground-to-Ground move (GROUND-MOVE-1) keeps `deadline_at` and writes
  no timed line.
- **Expiry at the deadline.** The scope that owns the tile (the channel's Ground owner, or the
  house scope) keeps a lane per deadline item and expires it at `deadline_at` as a resumable step
  from durable state, the D3 corpse decay pattern, under `TimedItemCause::Expire {reason:
  Deadline}` (§8's shapes: a lit torch becomes a burnt-out torch in place). At most
  `TIMEDITEM0B-RL-06` (64) expiry steps run per scope per simulation tick; the rest wait for the next
  tick. A scope that loads late, or a house that activates, expires its overdue items first, from
  the index.
- **Database time (R5).** The deadline runs on database time, so scope downtime and an inactive
  house count against the item, as in Tibia, where items in houses decay. No write is needed to keep
  it running.
- A tile owner's moves and expiries of one deadline item run through its one lane (§5.2), so a
  pickup never races its expiry.
- **Fence and locks for tile items.** A deadline item has no holder, so its writes take no
  `character_root` lock. An expiry (or any tile-owner write of a deadline item) runs under the
  tile owner's fence instead: the channel's runtime-scope ownership generation (DUR-03 §32, the
  InstanceRuntime scope included) or the house scope's fence. Lock order (TIMED-PROF-0C, review
  finding 4174228270): that fence, the item's lifecycle row (`game_item_instances`), its location
  row, then its timed row, matching ITEM-MOVE-WIRE-1 §7.2 and DUR-03's Ground-move amendment, which
  lock item rows before Ground rows. A drop and a pickup are moves between a holder and a tile, so
  they take both: the holder's fences and `character_root`, then the tile owner's fence, then the
  item rows, the location rows and the timed row, the existing ITEM-MOVE-WIRE-1 order with the timed
  row last. Test: an expiry and a Ground move of the same item, run concurrently, never deadlock.
  A stale tile fence writes nothing; the new owner reloads the deadlines from the index.

### 10.4 Going out (D360)

- **Into a container.** A move of a lit item from a slot into any container entry (an unequip into
  the backpack, a move into a bag) first stops it (§5.3), then the move carries one `TRANSFORM`
  (`PRESERVE_INSTANCE`) to its unlit form (its `transform {trigger: use}` target), like an equip
  form (§9.2). The row keeps `remaining_ms`; nothing else is written.
- **A pickup** of a deadline item (Ground or a house tile into the main backpack) carries its
  `TRANSFORM` to the unlit form and one timed line under `TimedItemCause::ClearDeadline {item,
  expected_revision}`: `remaining_ms = deadline_at − database time`, `deadline_at` NULL, revision + 1.
  If the deadline has passed, the pickup writes nothing and the tile owner runs the expiry first;
  the pickup then sees the expired form.
- **Death.** Before a death settlement the dying actor's runtime puts out every lit item it wears:
  each is stopped and transformed to its unlit form by a one-item `TRANSFORM` under
  `TimedItemCause::PutOut {item, expected_revision}` on its lane. The settlement then moves unlit
  items into the corpse and stays unchanged.
- **Logout** keeps an equipped lit item lit and frozen (§10.2); it burns again at the next login.
- **Slot to slot** (a hand to the Extra slot) keeps it lit: stop, move, live again.
- **Ground to a slot** is not admitted by ITEM-MOVE-WIRE-1 today. When a later move decision admits
  it, it clears the deadline as a pickup does, without the transform, and the item becomes live.

### 10.5 Ceilings (DUR-03 §28)

Registered by TIMED-RT-1 with max and max+1 tests, on top of the base move shape's own rows:

| Shape | Extra transform I/O | Extra timed row lines | Extra work units |
|---|---|---|---|
| drop of a lit item (`DUR03-RL-0x-TIMED-DROP`) | 0 / 0 | 1 | 1 |
| pickup of a deadline item (`DUR03-RL-0x-TIMED-PICKUP`) | 1 / 1 | 1 | 2 |
| move of a lit item into a container (`DUR03-RL-0x-TIMED-CONTAIN`) | 1 / 1 | 0 | 1 |
| put out (`TimedItemCause::PutOut`, one-item) | 1 / 1 | 0 | 1 / 3 total |

Each timed line is fixed-size, at most 64 bytes; payload and envelope stay within `DUR03-RL-07`,
measured by the child.

### 10.6 Houses (TIMED-HOUSE-1)

House tiles use §10.3 and §10.4 unchanged under the house scope's fence. TIMED-HOUSE-1 starts when the
house lane admits player item placement on house tiles (HOUSE-CUSTODY-0); until then no item lies
on a house tile, so nothing waits on it.

## 11. When a timed item is active (TIMED-FX-1, entry condition 6)

### 11.1 The active rule

EQUIP-0 §3.2's "and it is not `timed`" is replaced by: a timed item is **active** when it meets
every other §3.2 condition, it is live (§5.1), its live charges (if it has charges) are above 0,
and its live remaining time (if it has a duration) is above 0. An inactive form, a frozen item and
an item whose expiry is pending are never active. Its abilities then apply as EQUIP-0 says, plus
two the active forms need.

### 11.2 `ITEM_REGENERATION`

- `REGENERATION {hp_per_tick, mana_per_tick, interval_ms}` (life ring, ring of healing) is an
  instance of the new CONDITIONS-0 family **`ITEM_REGENERATION`**, conflict key
  `item_regeneration:<equipment slot>`, so one instance per slot.
- It stacks with `FOOD_REGENERATION` and `RECOVERY`: each ticks on its own (Canary keeps the ring's
  regeneration as a separate condition; `PARITY_PENDING`).
- Provenance: source kind `item` and the ItemInstanceId (CONDITIONS-0 §3.2 gains source kind
  `item`).
- It is created when the item becomes active, has no duration of its own and ends when the item
  stops being active. In a protection zone its ticks regenerate nothing, like every regeneration.
- It is never durable and never carried by a transfer: the receiving runtime creates it again from
  the active set (EQUIP-0 §3.2), so no tick is dealt twice.

### 11.3 The item mana shield

- `MANA_SHIELD` (energy ring) is a CONDITIONS-0 `MANA_SHIELD` instance with source kind `item` and
  **no capacity**: while the item is active, damage that reaches the §3.3 stage goes to mana up to
  the mana available, and the rest to health.
- It has no `remaining` and no duration; it ends only when the item stops being active, never
  because mana reaches 0. This overrides CONDITIONS-0 §3.3's capacity and end rules for this source
  only.
- A spell mana shield applied while it exists replaces it under the `mana_shield` key's policy, and
  the item's instance is created again when the spell's ends, if the item is still active
  (`PARITY_PENDING`).

## 12. Causes and DUR-03 shapes (TIMED-RT-1)

- **`TimedItemCause`** (the reserved name, DUR-03 §15), closed:
  - `Checkpoint {item, expected_revision}`;
  - `Expire {item, expected_revision, reason: TimeExhausted | ChargesExhausted | Deadline}`;
  - `SetDeadline {item, expected_revision}` and `ClearDeadline {item, expected_revision}` (lines
    inside a drop or a pickup, §10.3-§10.4);
  - `PutOut {item, expected_revision}` (§10.4).

  No generic or caller-chosen reason. Equip forms carry the move's own cause; use forms carry
  `ItemUseCause::Light`. The repair keeps `FeeBurnCause::NpcRepair`.
- **Admitted one-item shapes** (DUR-03 §39.3 amendment), each with `DUR03-RL-01` 1,
  `DUR03-RL-07-EVENTS` 1, the existing payload and envelope ceilings and `DUR03-RL-08` 3,
  registered by TIMED-RT-1 with max and max+1 tests:

  | Shape | Location lines | Transform I/O | Participants / work units |
  |---|---|---|---|
  | checkpoint | 0 | 0 / 0 | 1 / 2 |
  | composed checkpoint | 0 | 0 / 0 | 1 / 2, plus the build receipt's own rows |
  | expiry transform (`DUR03-RL-04-TIMED-EXPIRY`) | 0 | 1 / 1 | 1 / 4 |
  | expiry burn | 1 | 0 / 0 | 1 / 4 |
  | composed expiry burn | 1 | 0 / 0 | 1 / 4, plus the build receipt's own rows |
  | use form (`ItemUseCause::Light`) | 0 | 1 / 1 | 1 / 3 |
  | put out (`PutOut`) | 0 | 1 / 1 | 1 / 3 |

  The deadline lines inside a drop and a pickup are §10.5's rows.

  They supersede the §39.1 exclusions of burn and transform for these shapes only. The equip form
  rows are §9.2's.

## 13. Wire (TIMED-WIRE-1, entry condition 7)

- **Capability `TIMED_ITEMS_V1`**, number **11** (control-plane lease D353).
  It gates fields only; no command and no state domain.
- **Fields.** Every item presentation that carries a count (inventory, container views, equipment,
  ground, trade and Look) gains optional `charges` (uint32, at most RL-01 65,535) and `remaining_s`
  (uint32, at most 604,800), sent only when the definition shows charges (`show_count`) or duration.
  The values are the row's, or the live values for a live item; a deadline item's `remaining_s` is
  `deadline_at` minus the current time, clamped to 0 for an item whose deadline has passed and whose
  expiry step is still queued (§10.3, RL-06).
- **Updates.** A live item's remaining time is sent at every form change and at most once per
  `TIMEDITEM0B-RL-03` (60 s); the client counts down between updates. A charge change is sent with
  the next presentation update of that item, at most once per second per item.
- **Look** adds "It has N charges left." or "It will expire in X minutes." from the same values.
- **Bound:** each field is a tag plus at most a 3-byte (charges) or 3-byte (seconds) varint, so at
  most 8 bytes per item. TIMED-WIRE-1 measures the largest affected message (a 30-entry container
  view, `ITEMV0-RL-01`) and re-registers its `max_payload_bytes` when it grows.
- **Without the capability** the client shows the item without these values; nothing else changes.

## 14. Rows

| Row | Value |
|---|---|
| `TIMEDITEM0B-RL-01` checkpoint interval of a live item | at most 60 s (A13 §4.5) |
| `TIMEDITEM0B-RL-02` ambiguous-write bound before the lane holds for reconciliation | 2,000 ms |
| `TIMEDITEM0B-RL-03` remaining-time updates to the client per live item | 1 per 60 s |
| `TIMEDITEM0B-RL-04` live timed items per actor | 11 (ten equipment slots plus one exercise weapon) |
| `TIMEDITEM0B-RL-05` timed writes in flight per lane | 1 |
| `TIMEDITEM0B-RL-06` deadline expiry steps per scope per simulation tick | 64 |

Deadline items per channel are bounded by the dropped-item bound `ITEMMOVE1-RL-02` (20,000).

TIMED-ITEM-0's `TIMEDITEM0-RL-01` (65,535 charges) and `RL-02` (7 days) are unchanged. Each row is
registered by its child with max and max+1 tests.

## 15. Entry conditions and tests

| TIMED-ITEM-0 §5 condition | Answered by | Tests (TIMED-RT-1 unless named) |
|---|---|---|
| 1. per-item serialization of checkpoint and expiry | §5.2 | a checkpoint in flight when charges reach 0: the expiry is issued at the checkpoint's new revision and commits; crash after the checkpoint, restart: the checkpointed value loads; after the expiry commits and a restart the item has no charge and is not active; a second write in flight on one lane is impossible (RL-05 max+1); an ambiguous write found by its record is not retried |
| 2. checkpoints at the A13 cadence, at logout and handoff, one per changed item, charges never written per spend | §6 | unchanged values write nothing; logout waits for every lane; spending a charge writes nothing until the next checkpoint; a first checkpoint inserts at expected revision 0 and a second insert at 0 writes nothing |
| 3. stop before leaving the slot | §9.1 | unequipping 59 s after a checkpoint stores the live value, so re-equipping never adds time; a failed unequip restarts the clock from the row; a death drop writes no timed line |
| 4. expiry as one atomic one-item shape, never storing 0 | §8 | a ring without a decay target is burned at 0 with its event and its row left inert; soft boots become worn soft boots with a spent row and revision + 1; a charge-only item with `transform {trigger: decay}` transforms and is burned only without a target; an expiry transform at 1 / 1 passes and 2 / 1 is refused; a replayed or stale `Expire` writes nothing |
| 5. equip forms with move and swap ceilings, `SWAP_TIMED_BOTH` as `BLOCKED` | §9.2 | unequip then re-equip keeps the remaining time; a fresh ring has no row until its first checkpoint; a ring-for-ring swap answers `BLOCKED` and writes nothing; a ring onto an empty or a non-timed slot works; both ceiling rows at max and max+1 |
| 6. the active rule, `ITEM_REGENERATION`, the item mana shield | §11 | TIMED-FX-1: a life ring's regeneration ticks beside food regeneration and stops at unequip; the energy ring's shield stays at 0 mana and ends at unequip or expiry; a frozen or pending-expiry item grants nothing |
| 7. the `TIMED_ITEMS_V1` wire | §13 | TIMED-WIRE-1: fields only with the capability; at most one time update per 60 s; the measured payload |
| 8. the TIMED-ITEM-0 §4 invariants unchanged | §4 | a write without a record is refused by the guard; the revision never repeats across a repair; a ring retired by `WorldReset` keeps an inert row and the reset writes no timed line |

Also: a composed exercise checkpoint commits the receipt and the row together, and a revision
mismatch on either writes neither (EXERCISE-1 repeats it); a lit torch in a hand burns and gives
light; lighting an equipped torch is admitted and writes no row;
putting out a live torch checkpoints first; a content revision lowering a ring's duration is
refused. D360: a torch dropped 30 s after a checkpoint gets `deadline_at` from the stopped budget, and
a drop and pickup cycle never adds time; a deadline item expires at its deadline into a burnt-out
torch, and one overdue at scope load expires first; a pickup after the deadline writes nothing and
sees the expired form; a lit torch moved into a bag becomes unlit with its budget kept; lighting a
torch in a bag answers `REJECTED`; a death puts out a lit torch before the settlement and the corpse
holds the unlit torch; the guard refuses a lit item in a container and a deadline on a non-tile
item; each §10.5 row at max and max+1; more than 64 overdue items expire over several ticks.

## 16. Rejected options

- **A write per charge or per second.** A13 checkpoints bound the write rate and the crash loss;
  TIMED-ITEM-0 §4 forbids writes per spend.
- **Expiry keyed only by the revision it saw, without a lane.** That is the round 12 race: a
  checkpoint commits in between, the expiry is stale, and the live zero is lost (D285).
- **Live values written inside moves.** Every move shape would need a timed line; stopping first
  keeps moves unchanged.
- **Container-tree clocks for torches.** Rounds 1-7 of #1471 needed a tree shape, a backfill and
  trade rules for lit items inside bags; D360 puts a lit item out in any container, so no tree
  ever holds one.
- **Freezing deadline items while their scope is down.** Every scope restart would need a write per
  deadline item; database time needs none (R5).
- **Time running for logged-out characters or in the depot.** Canary decays only in the game world,
  and players expect a ring to keep its time while offline.
- **A two-transform swap shape.** Refusing `SWAP_TIMED_BOTH` costs one extra move and adds no shape.

## 17. Architect rulings (owner rule 5905825574)

- **R1. Where a lit torch burns.** Owner answer D360: c) in slots, on the ground and in houses,
  with the Ground-deadline model; put into any container it goes out.
- **R2. Crash loss.** a) At most one checkpoint interval, in the player's favour (as A13 and
  OFFLINE-0); b) a write per change. **Ruled a)**, `PARITY_PENDING`.
- **R3. Ring-for-ring swap.** a) Refuse `SWAP_TIMED_BOTH` as `BLOCKED`; b) a two-transform shape.
  **Ruled a)**, a declared difference (TIMED-ITEM-0 §5 item 5 already fixed it).
- **R5. Deadline clock.** a) Database time, so downtime counts (recommended: no write per restart,
  as rounds 1-6 ruled); b) frozen while the scope is down. **Ruled a).**
- **R4. Integrity fault on a live expiry.** a) Retry the expiry once at the current revision while
  the runtime holds authority; b) drop the live zero. **Ruled a)** (D285).

## 18. Owner questions

None open. Question 1 (carried torches) was answered by D360 (§10). No new gold fee or sink is
added.

## 19. Decision test

- **Must decide now:** YES (D351). Without it every ring, charged amulet, soft boots and torch
  grants nothing (EQUIP-0 R2), EXERCISE-1 cannot start, and TIMED-REPAIR-1 has no table.
- **Blocked:** TIMED-RT-1, TIMED-FX-1, TIMED-WIRE-1, TIMED-PARITY-1, TIMED-HOUSE-1, TIMED-REPAIR-1,
  EXERCISE-1.
- **Minimum sufficient:** one write-record table and one deadline column beside the existing row
  table, one lane per live or deadline item, checkpoints at existing moments, one-item shapes and
  three bounded move lines, one closed cause, one capability with two fields. Putting lit items out
  in containers (D360) removes every container-tree shape.
- **Harder later:** the write records and their key bind every later timed writer; database-time
  deadlines (R5) would need a migration of every live deadline to change; torches going out in
  containers become player-visible behaviour; `TIMED_ITEMS_V1` fields join the wire compatibility
  surface.
- **Superseding evidence:** a TibiaWiki or official source on how protection charges are spent.
- **Deliberately not decided:** Ground-to-slot moves (a later move decision), invisibility from
  equipment, imbuement durations (IMBUE-FORGE-0), other charge consumers.

## 20. Before-freeze checklist

1. **Contract amendments:** listed in the header; each written by its child in its own docs commit.
2. **Serialization:** one lane per live or deadline item; every timed write is a one-item DUR-03
   transaction keyed by (item, expected revision) under the item writer's fence and the holder's
   `character_root` lock, or, for a deadline item, the tile owner's fence (§10.3); the composed
   checkpoint and expiry add the build receipt in the same transaction.
3. **Restart:** live items lose at most one checkpoint, in the player's favour; deadline items
   expire from their durable deadline; frozen items never change; durable exhaustion is final.
4. **Typed references:** ItemInstanceId, definition keys, TransactionId.
5. **Wire:** one capability, two optional fields, no command, no domain.
6. **Leases:** migration `0054` (D353) and capability 11 (D353) from the control
   plane.

## 21. Implementation packets

### TIMED-RT-1

```yaml
task_id: OTV2-YYYYMMDD-timed-rt-1
decision: TIMEDITEM0B-RUNTIME-CHARGES-AND-DURATION-V1 §4-§10, §12, §14-§15
worker: oteryn-hard-worker (persistence, session-generation fencing)
review: independent persistence and determinism review
depends_on: [TIMED-CONTENT-1, ITEM-MOVE-2a, ITEM-MOVE-2b, EQUIP-RT-1, this decision accepted]
migration_lease: 0054
owned_paths:
  - apps/game-server/migrations/0054_item_timed_states.sql
  - apps/game-server/src/durability/item_timed_state.rs
  - apps/game-server/src/durability/item_timed_state_audit.rs
  - apps/game-server/src/durability/mod.rs            # one mod line
  - apps/game-server/src/domain/timed_item.rs          # live values, the lane, stop
  - apps/game-server/src/domain/mod.rs                # one mod line
  - the ITEM-MOVE-2a/2b move modules                    # stop-before-move, equip and put-out transforms, drop/pickup deadline lines; named at allocation
  - the channel Ground owner's step scheduler           # deadline expiry steps (§10.3); named at allocation
  - apps/game-server/tests/item_timed_state_postgres.rs
  - apps/game-server/tests/durability_postgres.rs      # one mod line
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # §9.2, §12 and §14 rows
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # §15, §33, §39.3 paragraphs
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md  # brief pointer
  - docs/architecture/reviews/OTERYN_GAME_ITEM_MOVE_WIRE1_EQUIP_AND_DROP_DECISION_2026-09-30.md  # §3, §6 pointers
  - docs/architecture/reviews/OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md  # §4.2, §6
  - docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md  # §3.1: a live item is reserved (§5.1)
  - docs/architecture/reviews/OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION_2026-10-01.md  # §11.4 pointer
  - docs/agents/tasks/active/OTV2-YYYYMMDD-timed-rt-1.md
validation:
  - cargo fmt --all --check
  - cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test item_timed_state_postgres --quiet (PostgreSQL 17.6)
  - python tools/agents/validate_governance.py; git diff --check
acceptance: every §15 row's tests for conditions 1-5 and 8, plus the §15 extra tests, including D360's
```

### TIMED-FX-1

```yaml
task_id: OTV2-YYYYMMDD-timed-fx-1
decision: TIMEDITEM0B §11
worker: oteryn-hard-worker (combat)
review: combat and determinism review
depends_on: [TIMED-RT-1, COND-1, EQUIP-RT-1]
owned_paths:
  - the EQUIP-RT-1 active-set module and the COND-1 condition store (files named at allocation)
  - docs/architecture/reviews/OTERYN_GAME_EQUIP0_EQUIPMENT_EFFECTS_DECISION_2026-10-01.md  # §3.1, §3.2
  - docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md  # §3, §3.2, §3.3
  - docs/agents/tasks/active/OTV2-YYYYMMDD-timed-fx-1.md
validation: cargo fmt/clippy/test for oteryn-game-server; git diff --check
acceptance: §15 condition 6 tests
```

### TIMED-WIRE-1

```yaml
task_id: OTV2-YYYYMMDD-timed-wire-1
decision: TIMEDITEM0B §13
worker: oteryn-hard-worker (protocol-oteryn wire format)
review: protocol review
depends_on: [TIMED-RT-1]
capability_lease: 11 TIMED_ITEMS_V1
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json    # the capability row, re-measured max_payload_bytes
  - crates/protocol-oteryn/                            # the optional fields (files named at allocation)
  - the game-server item presentation encoder (named at allocation)
  - docs/agents/tasks/active/OTV2-YYYYMMDD-timed-wire-1.md
validation: cargo fmt/clippy/test for protocol-oteryn and oteryn-game-server; registry validator; git diff --check
acceptance: §15 condition 7 tests
```

TIMED-HOUSE-1 is an impl packet (persistence review) after TIMED-RT-1 and the house lane's item
placement: the house scope's deadline lane and expiry steps, reusing §10.3-§10.5; its owned paths
are named at allocation.

TIMED-PARITY-1 is an ordinary impl packet after TIMED-FX-1: fixtures only, under
`apps/game-server/tests/`, no production code.
