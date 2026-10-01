# STACK-0 Partial counts and stack merges

- Decision: `STACK0-PARTIAL-COUNTS-AND-STACK-MERGES-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence and
  protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's confirmation (#162 5930208391, "zgadza się") of the gap list: moving part of a
  stack does not exist; dropping a stack on the ground, moving it across the ground and putting it
  on an equipped stack never merge. ITEM-MOVE-WIRE-1 §3 ("every move is of a whole item; there is no
  count field") and §4 ("merging them waits for the partial-count decision") name this decision.
- Builds on: DUR-03 §7.1, §11-§13, §23, §29, §32, §39; B3 §4.4 (D82, D83); ITEM-MOVE-WIRE-0 §5;
  ITEM-MOVE-WIRE-1 §3-§6; BAGS-0 §4-§6; D3 §4.4-§4.7 (D133, D134, D136); PLAYER-TRADE-0 §4;
  WORLD-INTERACTION-0 §7.2; RANGED-0 §6.1.2 (the Ground ordinal and the top Ground item; PR #1439,
  accepted first); the composition decision rules 1-4; owner rule 5905825574 (Tibia parity).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| STACK-WIRE-1 | impl, protocol review | capability `ITEM_STACK_COUNT_V1` and the optional `count` of command 9 (§3); the client count dialog | ITEM-EQUIP-WIRE-1; BAGS-WIRE-1 |
| STACK-1 | hard (persistence), persistence review | the partial and merge shapes of §4-§5 for every destination of command 9, the DUR-03 amendment, rows (§6) | ITEM-MOVE-2a; ITEM-MOVE-2b; BAGS-1; GROUND-MOVE-1; RANGED-1's Ground ordinal |
| STACK-PARITY-1 | impl | fixtures against Canary: 100 onto 70 in a full backpack, 30 of 100 to the ground onto a matching top stack, the four slot cases of §4.4 | STACK-1 |

Tests: a session without the capability never receives `PARTIAL` and keeps its refusals (§3);
`count` 0, above the quantity or above 100 is `REJECTED`; each shape of §4 commits atomically or
not at all; a receiver taken by another transaction is never merged into twice; a partial corpse
loot outside the D133 window is refused by the database.

Later, each with its own decision: reordering inside a container ("move up"), partial items in
player trade and the Market (whole items there), auto-stacking across containers (Tibia does not).

## 1. Question

How does a player move part of a stack, and when does a moved stack join a stack already at its
destination?

## 2. Facts

**PROVEN**

- ITEM-MOVE-WIRE-0 §5 and ITEM-MOVE-WIRE-1 §3: command 9 moves a whole item; "there is no count
  field". ITEM-MOVE-WIRE-1 §4: same-definition stacks in a slot swap; "merging them waits for the
  partial-count decision (`PARITY_PENDING`)". ITEM-MOVE-WIRE-1 §5 and WORLD-INTERACTION-0 §7.2: a drop
  or a Ground move always makes or moves one Ground item.
- B3 §4.4 (D83): on pickup into the main backpack the receiver is the compatible stack with room
  first in display order; compatible means the same definition key and revision, both stackable, equal
  state apart from quantity; a full merge, a top-up with a new entry for the remainder, or a refusal
  "when no free entry" ("partial pickup is `UNKNOWN` parity"). BAGS-0 §6 uses the same rule inside
  any container. D82: stack maximum 100.
- DUR-03 §12: a split of `x < q` into a planned identity; §13: a quantity transfer to a compatible
  receiver keeps the receiver's identity, "UUID/client list ordering never selects
  survivor/receiver"; §7.1 reservations; §23 retry.
- RANGED-0 §6.1.2 (candidate, PR #1439): a database-assigned `ground_ordinal` on every Ground
  location row; the top Ground item is the live root with the highest ordinal.
- OTClient (`OTS_HYPOTHESIS_ONLY`, ITEM-MOVE-WIRE-0 §2) sends a count with every move.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`, `game.cpp`
`Game::internalMoveItem`)

- The moved amount is `m = min(count, the destination's maximum)`; a destination that can take
  nothing refuses.
- A stackable moved onto an equal destination item merges `n = min(room, m)` into it; the rest
  `m − n` becomes a new item at the destination; the source keeps what was not moved.
- Onto an inventory slot (`Player::queryMaxCount`, `player.cpp:4773-4776`): a slot item that is
  non-stackable or of another id is exchanged into the source cylinder (`NEEDEXCHANGE`); a same-id
  stack takes `min(room, count)` and refuses with `NOTENOUGHROOM` when it has no room.
- A container skips the source as its own receiver and does not autostack within the same container
  (`container.cpp` `queryDestination`); a move onto the same top item is ignored
  (`game.cpp`, `item == toItem`).
- A tile's destination item is its top item (`Tile::queryDestination`, RANGED-0 §2).

## 3. Wire (STACK-WIRE-1)

- **Capability `ITEM_STACK_COUNT_V1`**, which requires `ITEM_EQUIP_DROP_V1` and `CONTAINER_TREE_V1`;
  its number is reserved on #162 at allocation.
- Command 9 gains `optional uint32 count` (explicit presence, so absent is distinguishable from 0)
  under the capability. Absent means the whole item. A present `count` of 0, above the source's
  quantity, above the source definition's stack maximum (at most 100, D82), or not 1 on a
  non-stackable item is `REJECTED`. A
  session without the capability that sends the field is `REJECTED`. The frozen intent binds the
  count.
- New result `PARTIAL` (capability sessions only) beside a new `uint32 moved` (1 to 99, one varint
  byte, so the result stays within its 4-byte budget): fewer units moved than asked because the
  destination took fewer (Canary's `m`, §4.2). `MOVED` when every asked unit moved.
- The client asks for the amount the Tibia way (a count dialog when dragging a stack with the split
  modifier, default the whole stack).
- **Sessions without the capability.** Every move is of the whole item. The merges of §4.1 apply
  to them too (they are world rules and end as `MOVED`, a result those clients know), but a whole
  move that cannot move every unit is refused with the destination's existing result, as today
  (B3 §4.4's last row, ITEM-MOVE-WIRE-1 §4-§5); `PARTIAL` is never sent to them.

## 4. Shapes (STACK-1)

Source stack S (quantity q), asked amount `c` (q when absent), destination D.

### 4.1 Receivers

- **Main backpack and containers:** B3 §4.4 / BAGS-0 §6: the first compatible stack with room in the
  container's display order, never S itself. When the destination container is S's own container,
  there is no receiver at all (Canary does not autostack within one container, §2, §4.3): the move
  only splits into a new entry.
- **Ground:** the tile's top Ground item when it is a compatible stack with room (RANGED-0 §6.1.2),
  rechecked at commit under the tile row lock (§5.2).
- **An equipment slot:** the slot's item, by the four cases of §4.4.
- **Compatible:** B3 §4.4. A corpse or a corpse entry is never a receiver; a corpse is never a
  destination (D134 unchanged).
- **Reserved receivers.** A candidate already reserved by another transaction (DUR-03 §7.1: a
  pickup, a throw, a decay retirement, a trade in `TRANSFERRING`) has no room for this move: a
  container search goes on to the next compatible stack in display order, and the Ground or a slot
  has no receiver. The chosen receiver is reserved for this transaction.

### 4.2 Amounts

- `max` = the stack maximum of the moved definition, resolved from its definition revision
  (at most 100, D82; for example 30 where a definition sets 30), the same value the durability
  guard enforces.
- `room` = the receiver's free units (`max` minus its quantity; 0 without a receiver).
- `new` = the units D can take as one new item: `max` when one more item passes every check the
  destination applies to a whole-item move, else 0. Those checks are a free entry (minus entries
  reserved by a PLAYER-TRADE-0 swap), `BAGS0-RL-01` depth and `BAGS0-RL-02` tree size,
  `GAMEITEM01-REACHABLE-ITEMS`, the DEPOT-0 item counts, the Inbox counters, and for a Ground tile
  or a Ground-rooted tree `ITEMMOVE1-RL-01` and `ITEMMOVE1-RL-02`. Every count is the **net count
  after the move**: a whole S (`m = q`) that leaves the same tile, container or tree it lands in
  adds nothing, so a lower Ground S moved whole onto its own full tile still passes.
- `m = min(c, room + new)`. `m = 0` is refused with the destination's existing result (`NO_ROOM`,
  `BLOCKED`, `SLOT_MISMATCH`). `n = min(room, m)` merges into the receiver; `m − n` becomes a new
  item. With the capability this replaces B3 §4.4's refusal "when no free entry" by Canary's
  partial fit (its `UNKNOWN` parity row). Amended: B3 §4.4.

### 4.3 The source

- `m = q`: S leaves its location (a whole move; a full merge retires it under §11.5; a top-up moves
  S with the remainder as the new item, B3 §4.4).
- `m < q`: S keeps its identity and location with `q − m`; the moved units go by §13 into the
  receiver and/or by a §12 split into a planned identity N for the new item.
- **S and its own location.** S is never its own receiver. Inside S's own container, `c < q` splits
  into a new entry of that container, and `c = q` is a reorder (deferred, `NOT_SUPPORTED`). The
  move is a no-op only when destination resolution returns S itself (S is the tile's top Ground
  item, or the item in the target slot): nothing happens (Canary ignores `item == toItem`), no
  transaction, result `MOVED`. A lower Ground item moved onto its own tile whose top is a different
  compatible stack B merges into B by §4.2; onto a top that is not a receiver, it becomes the new
  top as a Ground-to-Ground move on one tile.

### 4.4 Equipment slots (as Canary `Player::queryMaxCount`)

| The slot holds | Result |
|---|---|
| nothing | the moved amount takes the slot as a new item (a split, or S when `m = q`) |
| a compatible stack with room | merge `n = min(room, m)`; a new item is impossible, so `new = 0` |
| a compatible stack that is full, or the same definition with unequal state | refused `NO_ROOM` (Canary `NOTENOUGHROOM`) |
| a different definition, or a non-stackable item | **exchange**: the slot's item T goes to S's parent, then the moved amount takes the slot |

- This replaces ITEM-MOVE-WIRE-1 §4's swap of same-definition stacks (Canary merges or refuses).
  Amended: ITEM-MOVE-WIRE-1 §4.
- **T's place** (Canary's source cylinder): a new entry of S's container when S is a container
  entry (the main backpack included); the main backpack when S is in another slot. When `m = q`
  and S leaves a main backpack entry, T takes that entry as ITEM-MOVE-WIRE-1 §4 does; otherwise T
  needs a destination that passes every `new` check of §4.2 for T, and that container must admit
  the move (not the Inbox, a corpse or a container offered in a trade). Without one: `NO_ROOM`.
- A two-handed claim or a requirement failure refuses as today. Ground to a slot stays
  `NOT_SUPPORTED` (ITEM-MOVE-WIRE-1 §5).

### 4.5 Where the shapes apply

Every source and destination command 9 already admits, with their rules unchanged at the runtime
and the database (reach, line of sight, protection zones, capacity and weight once B3-3 lands,
house tiles, `ITEMMOVE1-RL-01` and `-RL-02` for a new Ground item only): backpack and container
entries, slots, Ground (drop, pickup, Ground to Ground), depot boxes and the Inbox as a source where
BAGS-0 admits them. A merge on the Ground consumes no tile limit. Amended: ITEM-MOVE-WIRE-0 §5,
ITEM-MOVE-WIRE-1 §3, §4 and §5, BAGS-0 §5 and §6, WORLD-INTERACTION-0 §7.2.

- **Corpse loot.** A partial loot of a corpse entry is a §12 split or a §13 transfer, not a
  TRANSFER. STACK-1 extends the D133 window trigger and the `CorpseNotPickupable` trigger (D3 §4.4,
  §4.5) to every split and quantity line whose source is a corpse entry, so they bind it as they
  bind a TRANSFER. The entry keeps its identity, its parent and its D136 decay: a later decay
  retires the remainder by its own per-entry step. An entry reserved for decay retirement is not a
  move source (`STALE`). Amended: D3 §4.5.
- **Trade.** A split or merge that touches an offered item, or an entry of an offered container,
  cancels a trade in `OFFERED` or `READY` and is refused while it is `TRANSFERRING`, as any move of
  them (PLAYER-TRADE-0 §4).

## 5. Persistence (STACK-1; DUR-03 amendment)

### 5.1 Transaction

- One DUR-03 transaction per command under its CommandRef (composition rules 1-4: no
  `CharacterRevision` advance), with at most three touched items: S, the receiver B, and the new
  item N or the displaced slot item T.
- Lines: a §12 split (S to N, planned identity), a §13 quantity transfer (S's or N's units into B),
  a TRANSFER (whole S, or T), and S's retirement at zero.
- A receiver changed, taken, no longer compatible or no longer the top at commit refuses the commit
  under the same TransactionId (§23): nothing moves, result `STALE`.
- **Counters** change by exactly the change in the items they count, in the same transaction: the
  Ground counter −1 when a Ground-counted S retires by a full merge or leaves the Ground family, +1
  for each N or T that lands on the Ground or in a Ground-rooted tree; the Inbox and depot
  counters the same way.

### 5.2 Locks

Rule 4 as extended (ITEM-MOVE-WIRE-1 §7.2, BAGS-0 §4.2): `character_root`; item rows in
one ascending ItemInstanceId order `FOR UPDATE` over S, B, T **and every container whose entries
change** (the destination parent, S's container when it receives T or a split entry), because a
container is an item and BAGS-0 §4.2 locks it in that same global order; then the container-slot
row (the main backpack `0011` placement check, ITEM-MOVE-WIRE-1 §7.2, BAGS-0 §4.2); then Ground tile
rows in tile key order,
then the counters. **Each tile row's lock mode is fixed by the frozen plan and never upgraded:** a
real lock when the plan inserts on the tile or removes or retires a Ground root from it (the source
tile when S leaves or retires), `FOR SHARE` only when the tile is touched solely by a merge into
its top item. The real lock also covers the top check.

### 5.3 Database deltas (STACK-1)

- a STACK receipt keyed by the CommandRef carrying the frozen plan with role-specific fields, each
  bound to its identity and before/after state: `source` S (location, quantity before and after),
  optional `receiver` B (quantity before and after), optional `new_item` N (planned identity,
  destination, quantity), optional `displaced` T (from the slot, to its frozen destination entry),
  `count`, `moved` and the shape. A partial move onto a slot holding a different item fills S, N
  and T together; a CommandRef retry reconstructs the plan only from this receipt (the `0011`
  receipt admits one source and one receiver);
- planned-N reservation columns (DUR-03 §11.3) and receiver reservations (§7.1);
- the TRANSFER guard admits a Ground source that shrinks without Ground removal evidence (a
  partial pickup or a Ground-to-Ground split);
- quantity changes of Ground and depot stacks by §13, and Ground insertion by split from every
  admitted source family (backpack and container entries, slots, Ground, depot, Inbox);
- container-entry quantity changes and split insertion in nested, depot and Inbox trees;
- the D133 and `CorpseNotPickupable` triggers on split and quantity lines (§4.5);
- the counter upkeep of §5.1.

Amended: DUR-03 §39.1 and §39.3.

## 6. Rows (values fixed here, registered by STACK-1)

Worst cases: a partial move onto a slot with a different item (S quantity; T removal and
placement; N placement and quantity) and a merge with a split remainder (S, B and N quantities;
N placement). A whole-item exchange (`m = q`) is the existing `EQUIP-SWAP` shape and rows.

| Row | Value |
|---|---|
| `DUR03-RL-01-STACK` touched items | 3 |
| `DUR03-RL-02-STACK` location lines; quantity changes | 3; 3 |
| `DUR03-RL-06-STACK` participants / effect work units | 3 / 8 (each item: one participant plus each of its location lines and quantity changes) |
| `DUR03-RL-07-STACK` envelope and payload | the one-item caps; STACK-1 measures the three-item worst case against them, and if it does not fit, STACK-1 stops and returns for a new decision (no row is raised by the child) |

Tests with max and max+1: a fourth touched item, `count` = q + 1 and 101, a receiver at `max − 1` and `max` (30 and 100 definitions),
a tree at 500 and an entry count at its limit.

## 7. Rejected options

- **Keep whole-item moves.** Tibia players count coins and split potions every session.
- **Merge into any compatible Ground stack on the tile.** Canary merges only into the top item.
- **Refuse when the whole amount does not fit.** Canary moves what fits; B3 left it `UNKNOWN`.
- **Choose a receiver by ItemInstanceId.** DUR-03 §13 forbids it.

## 8. Architect rulings (owner rule 5905825574)

- **R1. Partial fit.** a) Canary: move what fits, report `PARTIAL` (recommended); b) refuse.
  **Ruled a).**
- **R2. Occupied slot.** a) Canary's four cases, with the exchange into the source's parent
  (recommended); b) refuse a partial move onto a different item. **Ruled a).**
- **R3. Wire.** a) An optional count behind a capability (recommended: older clients get no new field or result);
  b) a new command. **Ruled a).**
- **R4. Old clients.** a) Merges for everyone, partial fits only with the capability (recommended:
  world rules stay one, results stay closed); b) every new behaviour only with the capability.
  **Ruled a).**

## 9. Owner questions

None. The owner confirmed the scope; every choice is a Tibia-parity application.

## 10. Decision test

- **Must decide now:** YES. Without it players cannot split or count stacks, and stacks never join on
  the ground or in a slot.
- **Minimum sufficient:** one optional field, one receiver rule per destination family, one
  three-item shape.
- **Superseding evidence:** an official source on partial fits or ground stacking.
- **Deliberately not decided:** reordering inside a container, partial items in trade and the Market.

## 11. Before-freeze checklist

1. **Contract amendments:** DUR-03 §39.1, §39.3; B3 §4.4; ITEM-MOVE-WIRE-0 §5; ITEM-MOVE-WIRE-1 §3,
   §4, §5, §6.1, §6.3, §10; BAGS-0 §5, §6; D3 §4.5; WORLD-INTERACTION-0 §7.2. Applied in this PR.
2. **Serialization:** rule 4's lock order; lock modes fixed by the plan, never upgraded; receivers
   reserved; the Ground top rechecked under the tile row; a losing transaction commits nothing.
3. **Restart:** every shape is one durable transaction with its receipt; a replay returns the first
   outcome.
4. **Typed references:** items by ItemInstanceId, receivers chosen by display order or Ground ordinal,
   never by identity.
5. **Wire:** an optional field behind `ITEM_STACK_COUNT_V1`; absent means today's behaviour.
6. **Split work:** none; one transaction per move.
