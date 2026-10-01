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
  ITEM-MOVE-WIRE-1 §3-§6; BAGS-0 §4-§6; WORLD-INTERACTION-0 §7.2; RANGED-0 §6.1.2 (the Ground ordinal
  and the top Ground item; PR #1439, accepted first); the composition decision rules 2 and 4; owner
  rule 5905825574 (Tibia parity).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| STACK-WIRE-1 | impl, protocol review | capability `ITEM_STACK_COUNT_V1` and the optional `count` of command 9 (§3); the client count dialog | ITEM-EQUIP-WIRE-1; BAGS-WIRE-1 |
| STACK-1 | hard (persistence), persistence review | the partial and merge shapes of §4-§5 for every destination of command 9, the DUR-03 amendment, rows (§6) | ITEM-MOVE-2a; ITEM-MOVE-2b; BAGS-1; GROUND-MOVE-1; RANGED-1's Ground ordinal |
| STACK-PARITY-1 | impl | fixtures against Canary: 100 onto 70 in a full backpack, 30 of 100 to the ground onto a matching top stack, a partial move onto an occupied slot | STACK-1 |

Tests: a move without `count` behaves exactly as today; `count` 0 or above the quantity is
`REJECTED`; each shape of §4 commits atomically or not at all; a receiver taken by another
transaction is never merged into twice.

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
- Onto an occupied inventory slot with a different item, the slot's item is exchanged into the
  source's container first, then the moved amount goes to the slot.
- A tile's destination item is its top item (`Tile::queryDestination`, RANGED-0 §2).

## 3. Wire (STACK-WIRE-1)

- **Capability `ITEM_STACK_COUNT_V1`**, which requires `ITEM_EQUIP_DROP_V1` and `CONTAINER_TREE_V1`;
  its number is reserved on #162 at allocation.
- Command 9 gains an optional `count` (1 to the source's quantity) under the capability. Absent means
  the whole item, so every existing shape is unchanged. `count` 0, above the quantity, or not equal
  to 1 on a non-stackable item is `REJECTED`. The frozen intent binds the count.
- New result `PARTIAL {moved}` (within the 4-byte result): fewer units moved than asked because the
  destination took fewer (Canary's `m`); `MOVED` when all moved.
- The client asks for the amount the Tibia way (a count dialog when dragging a stack with the split
  modifier, default the whole stack).

## 4. Shapes (STACK-1)

Source stack S (quantity q), asked amount `c` (the whole q when absent), destination D.

### 4.1 Receivers

- **Main backpack and containers:** B3 §4.4 / BAGS-0 §6: the first compatible stack with room in the
  container's display order.
- **Ground:** the tile's top Ground item when it is an unreserved compatible stack with room
  (RANGED-0 §6.1.2), rechecked at commit under the tile row `FOR SHARE`.
- **An equipment slot:** the slot's item when it is a compatible stack with room.
- Compatible: B3 §4.4. A receiver is reserved under DUR-03 §7.1 for the transaction.

### 4.2 Amounts

- `room` = the receiver's free units (stack maximum minus quantity; 0 without a receiver);
  `new` = the units D can take as one new item (the stack maximum when D has a free entry, a free
  slot, or a tile under its limits; else 0).
- `m = min(c, room + new)`; `m = 0` is refused with the destination's existing result (`NO_ROOM`,
  `BLOCKED`, `SLOT_MISMATCH`). `n = min(room, m)` merges into the receiver; `m − n` becomes a new
  item. This replaces B3 §4.4's refusal "when no free entry" by Canary's partial fit (its
  `UNKNOWN` parity row). Amended: B3 §4.4.

### 4.3 The source

- `m = q`: S leaves its location (a whole move, a full merge retires it under §11.5, a top-up keeps
  it with the remainder per B3 §4.4).
- `m < q`: S keeps its identity with `q − m`; the moved units go by §13 into the receiver and/or by a
  §12 split into a planned identity for the new item.

### 4.4 Occupied slot with a different item

As Canary: the slot's item T moves to a new entry of S's container (or the main backpack when S is
not in a container; refused `NO_ROOM` without a free entry), then the moved amount goes to the slot
as a new item (a split, or S itself when `m = q`). ITEM-MOVE-WIRE-1 §4's whole-item swap stays the
case `m = q`. A two-handed claim or requirement failure refuses as today.

### 4.5 Where the shapes apply

Every source and destination command 9 already admits, with their rules unchanged (reach, line of
sight, protection zones, D133 corpse windows, capacity and weight once B3-3 lands, house tiles,
`ITEMMOVE1-RL-01` and `-RL-02` for a new Ground item only): backpack and container entries, slots,
Ground (drop, pickup, Ground to Ground), depot boxes and the Inbox where BAGS-0 admits them. A merge
on the Ground consumes no tile or channel limit. Amended: ITEM-MOVE-WIRE-0 §5, ITEM-MOVE-WIRE-1 §3,
§4 and §5, BAGS-0 §6, WORLD-INTERACTION-0 §7.2.

## 5. Persistence (STACK-1; DUR-03 amendment)

- One DUR-03 transaction per command under its CommandRef (composition rules 2-4), with at most three
  touched items: S, the receiver B, the new item N (or the displaced slot item T, §4.4).
- Lines: a §12 split (S to N, planned identity), a §13 quantity transfer (S or N's units into B), a
  TRANSFER (whole S, or T), and S's retirement at zero.
- Lock order: rule 4 as extended (ITEM-MOVE-WIRE-1 §7.2, BAGS-0 §4.2): `character_root`, item rows in
  ItemInstanceId order (S, B, T), the container-slot row, then Ground tile rows in tile key order
  (`FOR SHARE` for the top check, a real lock for a new Ground item) and the channel counter.
- A receiver changed, taken or no longer the top at commit refuses the commit under the same
  TransactionId (§23): nothing moves.
- Amended: DUR-03 §39.1 and §39.3.

## 6. Rows (values fixed here, registered by STACK-1)

| Row | Value |
|---|---|
| `DUR03-RL-01-STACK` touched items | 3 |
| `DUR03-RL-02-STACK` location lines; quantity changes | 3 location (new N, whole S or T, removal of S at zero); 3 quantity changes (S, B, N) |
| `DUR03-RL-06-STACK` participants / effect work units | 3 / 9 (each item: participant, its line, a retirement or insertion) |
| `DUR03-RL-07-STACK` envelope and payload | the one-item caps; STACK-1 proves the three-item worst case within them |

## 7. Rejected options

- **Keep whole-item moves.** Tibia players count coins and split potions every session.
- **Merge into any compatible Ground stack on the tile.** Canary merges only into the top item.
- **Refuse when the whole amount does not fit.** Canary moves what fits; B3 left it `UNKNOWN`.
- **Choose a receiver by ItemInstanceId.** DUR-03 §13 forbids it.

## 8. Architect rulings (owner rule 5905825574)

- **R1. Partial fit.** a) Canary: move what fits, report `PARTIAL` (recommended); b) refuse.
  **Ruled a).**
- **R2. Occupied slot.** a) Canary's exchange into the source's container (recommended); b) refuse a
  partial move onto a different item. **Ruled a).**
- **R3. Wire.** a) An optional count behind a capability (recommended: older clients unchanged);
  b) a new command. **Ruled a).**

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

1. **Contract amendments:** DUR-03 §39.1, §39.3; B3 §4.4; ITEM-MOVE-WIRE-0 §5; ITEM-MOVE-WIRE-1 §3, §4,
   §5; BAGS-0 §6; WORLD-INTERACTION-0 §7.2. Applied in this PR.
2. **Serialization:** rule 4's lock order; receivers reserved; the Ground top rechecked under the tile
   row; a losing transaction commits nothing.
3. **Restart:** every shape is one durable transaction with its receipt; a replay returns the first
   outcome.
4. **Typed references:** items by ItemInstanceId, receivers chosen by display order or Ground ordinal,
   never by identity.
5. **Wire:** an optional field behind `ITEM_STACK_COUNT_V1`; absent means today's behaviour.
6. **Split work:** none; one transaction per move.
