# ITEM-MOVE-WIRE-1 Equip and drop

- Decision: `ITEM-MOVE-WIRE1-EQUIP-AND-DROP-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol and
  persistence) and protected integration. It integrates after ITEM-MOVE-WIRE-0 (PR #1344), which it
  extends.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the architect programme plan (#162 5910870596, M1): a player cannot wear a weapon or
  armor, or drop an item
- Builds on: ITEM-MOVE-WIRE-0 (command type 9, capability 4, domains 9 and 11, item handles),
  GAME-ITEM-01 §6, DUR-03 §5.2, §28, §29, §32, §33 and §39, the B3 decision (D80-D83, §4.5 and
  §4.6), ADR-0021 (D191, `WorldReset`), the composition decision §3 and §3.1, the MOVE-RL-11
  decision (D84-D87), EQUIP-a (`domain/equipment.rs`), ITEM-SEM-2, PROD-ENTITLEMENTS-01 §6 and
  D76, owner rule 5905825574 (Global parity)
- Amends: ITEM-MOVE-WIRE-0 §3 and §5; DUR-03 §39.1 and §39.3 (paragraph after §33); the composition
  decision §3.1 (paragraph at its end); the MOVE-RL-11 decision §4.3 (D87 order, paragraph there)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| ITEM-EQUIP-WIRE-1 | impl, protocol review | capability `ITEM_EQUIP_DROP_V1`, the new destinations and results, the nine slots in domain 9 (§3) | ITEM-VIEW-1 |
| ITEM-MOVE-2a | hard, persistence review | equip, unequip and swap (§4), with the deltas and rows of §6 | ITEM-MOVE-1; ITEM-EQUIP-WIRE-1; ITEM-SEM-2b (slot, hands, requirements in content) |
| ITEM-MOVE-2b | hard, persistence review | drop to Ground and pick-up of Ground items (§5), with the deltas and rows of §6 | ITEM-MOVE-2a; VIS-2; MAP-OVERLAY-1 (Ground rebuild and `WorldReset` retirement) |

Later, each with its own decision: partial counts and stack splits, reordering a container, the
container slot and bags (B3 RL-05 > 0), weight and capacity (B3-3), Ground to Ground, corpse or
Ground straight into a slot, map items (MAP-WIRE-1), the depot (DEPOT-0), player trade
(PLAYER-TRADE-0).

## 1. Question

How does a player equip and unequip items, and drop and pick up items on the ground?

## 2. Facts

**PROVEN**

- ITEM-MOVE-WIRE-0 (candidate) gives command type 9 `{source handle, MAIN_BACKPACK}` under
  capability 4, domain 9 (main backpack slot and direct entries), domain 11 (the open corpse) and
  per-session handles. Every other source and destination is `NOT_SUPPORTED`. ITEM-VIEW-1 owns
  the domains and the wire; ITEM-MOVE-1 the persistence.
- DUR-03: §32 the Ground scope fence; §33 equip legality belongs to GAME-ITEM, and old locations,
  the new occupancy and any displacement commit all or none; §39.1 admits one-item TRANSFER into
  `CharacterInventory` only, and B3 §4.6 added the merge and top-up shapes and the backpack
  entries. Registry today: `DUR03-RL-01` 2, `DUR03-RL-02` 2, `DUR03-RL-06-PARTICIPANTS` 2,
  `DUR03-RL-06-EFFECT-WORK-UNITS` 6; the fee shape has its own rows (`DUR03-RL-01-FEE-BURN` and
  others).
- Migrations: Ground rows are inserted only by a fresh MINT, and Ground removal evidence is keyed
  by the item (`0011`); `corpse_ref` and the native placement context are NOT NULL (`0010`);
  backpack entries cannot be deleted except by a fee whole-burn line (`0011`, `0023`); the
  TRANSFER guard requires Ground removal evidence and a backpack-entry receiver (`0011`); a receipt
  has one source and one receiver (`0011`). The container slot's entries are keyed to it, and B3
  admits only an empty container into it.
- ADR-0021 D191: player items on the ground survive a crash and are retired at the planned world
  reset by the `WorldReset` `DECAY_RETIRE` cause; MAP-OVERLAY-1 builds it.
- `domain/equipment.rs`: each category has exactly one slot (the quiver is a left-hand item);
  `check_equip` refuses a hands conflict and assumes one occupant.
- Content: every Item's `equipment` semantics are `UNKNOWN`; ITEM-SEM-2b is planned to lower
  slot, hands, requirements and weight from the pinned TibiaWiki snapshot.
- MOVE-RL-11 D87: at most 256 entities per snapshot, nearest first by floor, distance, then
  identity.
- No Game migration holds Premium; it is an Account entitlement (D69, D76, PROD-ENTITLEMENTS-01).

**CIPSOFT_OFFICIAL**

- The Tibia manual (`interface.md` §3.4.1): an item goes only into a slot of its type, with the
  two-handed exclusions. Items can be thrown on the floor and picked up (`world.md`).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- A hands conflict is refused (`player.cpp:4650-4718`). Moving onto an occupied slot swaps only
  that occupant, which goes to the source's place (`player.cpp:4772-4776`,
  `game.cpp:2489-2521`).
- Moving an item off the ground needs the player next to it (`game.cpp:2123-2147`). Throw range
  is 15 tiles for a pickupable item with line of sight (`item.hpp:327-329`,
  `game.cpp:2222-2230`). A tile accepts items in a protection zone and on some non-walkable tiles
  (`tile.cpp:925-957`).

## 3. Wire (ITEM-EQUIP-WIRE-1, amends ITEM-MOVE-WIRE-0 §5)

- **Capability `ITEM_EQUIP_DROP_V1`**, which requires capability 4; its number is reserved on #162
  at allocation. Without it, command 9 keeps ITEM-MOVE-WIRE-0's meaning exactly, and domain 9
  shows no equipment.
- Command type 9 gains, only under the new capability, two destinations in its oneof:
  `EQUIPMENT {slot}` and `GROUND {WorldTilePosition}`. `slot` is its own proto enum
  `EquipmentSlotV1` with `UNSPECIFIED = 0` and the nine non-container slots (head, necklace,
  armor, right hand, left hand, legs, feet, ring, ammo). The container slot is not a destination.
- Every move is of a whole item. There is no count field.
- Domain 9 gains the nine slots: each empty, or with a handle, definition, count and sub-type.
- New results: `SLOT_MISMATCH` (wrong slot, a hands conflict, a container, or unknown
  equipment semantics), `REQUIREMENT_NOT_MET`, `BLOCKED` (the tile does not accept the item, no
  line of sight, or a limit is reached). The result stays at most 4 bytes.
- Durable slot rows use the GAME-ITEM-01 §6.1 semantic slot keys, never the proto enum values.

## 4. Equip and unequip (ITEM-MOVE-2a)

- **Moves.** A whole main backpack entry to a slot; a slot's item to `MAIN_BACKPACK`.
- **Legality.** `check_equip` with the item's content slot and hands. Refused as `SLOT_MISMATCH`:
  unknown `equipment` semantics, a hands conflict (refused, not resolved, as in Canary), and any
  item with `container` semantics (a quiver or a bag), whose entries would need a second
  container location.
- **Requirements.** Level and vocation are checked inside the transaction under the
  `character_root` lock. Premium is read from PROD-ENTITLEMENTS-01 §6 evidence at commit and fails
  closed when stale or unavailable. An item stays equipped when the level drops or Premium ends; a
  Premium-only item's benefits stop at use, as D76 says for other Premium benefits.
- **Swap.** If the target slot is occupied, its occupant takes a new main backpack entry, which
  replaces the source entry that the moving item leaves (entries are immutable). The number of
  entries is unchanged, so a swap always has room. Two items are touched. Same-definition stacks
  (ammunition) swap too; merging them waits for the partial-count decision (`PARITY_PENDING`).
- **Unequip.** Into the main backpack by the B3 rule: a D83 merge or top-up into a compatible stack
  (same definition key and revision, equal state), else a new entry. Only a new entry needs room;
  without one it is `NO_ROOM`. With no main backpack it is `NO_BACKPACK`.
- **Revision.** Item-only: rule 1 of the composition decision covers every slot (§7.2), so
  equipping never advances `CharacterRevision`. Stats from equipment are derived at runtime;
  ATTACK-0 reads the equipped weapon and shield.

**Amendment (pending on acceptance of BAGS-0;
`reviews/OTERYN_GAME_BAGS0_CONTAINERS_WITH_CONTENTS_DECISION_2026-09-30.md` §6, §9).**
A container with contents may enter the empty container slot and may leave it as a tree to a
destination whose BAGS-0 child admits it (Ground, a depot box). Unequip may target a nested
container. The nine other slots still refuse containers. Dropping and picking up a tree is built by
BAGS-GROUND-1. The §5 Ground counter (`ITEMMOVE1-RL-02`) then counts every item reachable from a
Ground root, adjusted atomically by the tree's item count on drop and pickup, by the extracted
subtree's item count when an entry is moved out of a Ground tree into the character's own trees,
and by one per item retirement; every commit that changes that number adjusts the counter by
exactly the change, under the tile and counter row lock.

## 5. Drop and pick up (ITEM-MOVE-2b)

- **Drop.** A whole main backpack entry or a slot's item to `GROUND {position}`: within 15 tiles
  on each axis, same floor, visible to the session, with line of sight, and a tile that accepts
  items (content; `PARITY_PENDING` against Canary, which also allows protection zones). House tiles
  are `BLOCKED` until the house ownership child of HOUSE-CUSTODY-0 §3.5 decides them.
- **Pick up.** A live, pickupable Ground item that is not a corpse (a dropped item, or a map item
  left on Ground by the ADR-0021 amendment) to `MAIN_BACKPACK`, standing on it or next to it
  (Chebyshev distance 1, same floor), by the existing `0011` shape with the deltas of §6. Into a
  slot it waits for a later decision. The client walks; the server never walks the player.
- **Corpses.** D133 and D134 bind every move whose source is a corpse entry, on the database clock
  with reach. A corpse is never a move source.
- **Limits.** `ITEMMOVE1-RL-01` loose items per tile: 10. `ITEMMOVE1-RL-02` dropped items per
  channel: 20,000, with an alarm at 80%. A drop over a limit is `BLOCKED`. The per-tile limit takes
  a real row lock on the tile.
- **Counter upkeep.** The per-channel count of dropped items goes down in the same transaction as
  a pick-up of a dropped item and as each `WorldReset` retirement of one, so it always equals the
  live dropped rows of the channel. To avoid one hot row, the count may be kept in a fixed number of shard rows per channel, summed for the limit check.
- **Visibility.** D87's canonical order is amended by the owner's D222 (#162 5911768800) so that
  actors (players and creatures) rank before items; within each group the order is unchanged. Dropped items can then never push an
  actor out of a snapshot (§7.3).
- **Reset.** Dropped items follow D191: they survive a crash and are retired at the planned world
  reset by `WorldReset`. No new sink.

## 6. Persistence and DUR-03

### 6.1 Supersessions (as B3 §4.6)

For the shapes of §4 and §5 only, this decision supersedes:

- §39.1: "multiple touched items" is unsupported. The swap (two items) is admitted.
- §39.1 and §39.3: TRANSFER goes only to `CharacterInventory` and the B3 destinations. The nine
  non-container `CharacterEquipment` slots, and `Ground` from a backpack entry or a slot, are
  admitted.

Every other §39 obligation is unchanged: fences, cause (the command's CommandRef), evidence,
idempotency, current authority, one event per transaction with complete TransactionEventRef
membership.

### 6.2 Deltas each child must make

- **2a:**
  - slot rows for the nine slots, keyed by semantic slot key, joining the item-level
    single-location guard;
  - a database-enforced uniqueness per reserved hand resource for a two-handed claim (§29);
  - deletion of a backpack entry by a TRANSFER out of it (today only a fee whole-burn line may
    delete one);
  - the TRANSFER guard widened to the source kinds backpack entry and slot, and the receiver kinds
    slot and backpack entry, without Ground removal evidence for them;
  - a swap receipt with two sources and two receivers.
- **2b:**
  - Ground insertion by TRANSFER, not only by MINT;
  - Ground removal evidence keyed by (item, transaction), so a re-dropped item can be picked up
    again;
  - nullable `corpse_ref` and native placement context for a dropped item;
  - the §32 scope fence on every Ground write;
  - the per-tile row lock and the per-channel counter.

### 6.3 Resource rows (values fixed here, registered by each child before implementation)

| Row | Value |
|---|---|
| `DUR03-RL-01-EQUIP-SWAP` touched items | 2 |
| `DUR03-RL-02-EQUIP-SWAP` location lines | 4 (two removals, two placements) |
| `DUR03-RL-06-EQUIP-SWAP` participants / effect work units | 2 / 6 (each item: 1 + removal + placement) |
| `DUR03-RL-07` envelope and payload | unchanged; the child proves the swap's worst case within them |
| Equip, unequip, drop, pick-up (one item) | the existing one-item rows: 1 item, 2 location lines, 3 work units |
| `GAMEITEM01-REACHABLE-ITEMS` | 30 per character (the main backpack, 20 entries and 9 slots) |
| `ITEMV0-RL-01` entries in domain 9 | 30 |
| `ITEMMOVE1-RL-01` | 10 loose items per tile |
| `ITEMMOVE1-RL-02` | 20,000 dropped items per channel, alarm at 16,000 |

## 7. Other amendments

### 7.1 ITEM-MOVE-WIRE-0

Its §3 "out" list and §5 destinations are extended by §3 to §5 here, under the new capability.
ITEM-MOVE-WIRE-0's replay rule (the committed result looked up by CommandRef before the handle is
resolved, the intent bound to the ItemInstanceId) and its handle reissue on every reconnect
snapshot also cover the `EQUIPMENT` and `GROUND` destinations and the slot handles of domain 9. A
back-pointer is added to it when this PR integrates after #1344.

### 7.2 Composition decision §3.1

Rule 1 covers every `CharacterEquipment` slot. The Character-related effects are the character's
own item locations; Ground and corpse entries are endpoints outside the Character, bound by rule 2
and by DUR-03 §32 for their scope. Rule 4's lock order is extended after `character_root`: the
items in ItemInstanceId order, the container-slot row that the deferred `0011` placement check
takes, then the Ground tile row and the per-channel counter.

### 7.3 MOVE-RL-11 §4.3 (D87, amended by D222)

The owner decided this as D222 (#162 5911768800, verbatim "54 a"), which amends D87. The canonical
order becomes: actors (players and creatures) before items, then floor distance, Chebyshev distance
and entity identity as before. The 256 ceiling and the degrade and resync
dispositions are unchanged.

## 8. Rejected options

- **Resolving a hands conflict by unequipping the other hand.** Canary and Global refuse it.
- **Partial counts in this decision.** Their split and overflow shapes need new receipts and
  reservation columns; whole items make equipping and dropping playable first.
- **A new ground cleanup sink.** D191 already retires dropped items at the planned world reset.
- **Unequipping or dropping the main backpack.** Its entries are keyed to the container slot; it
  needs the bags decision.
- **Reusing capability 4 for the new destinations.** Its servers and clients would not know them.
- **Guessing slots from Canary data.** Content is the authority.
- **Server-driven walking to an item.** No owner exists for it (ATTACK-0 has the same cut).

## 9. Decision test

- **Must decide now:** YES. Without equipping, combat has fists only; without dropping, a full
  backpack has no way out.
- **Minimum sufficient:** two destinations under one capability, whole items only, one two-item
  swap shape, no new sink.
- **Superseding evidence:** official throw and drop rules.
- **Deliberately not decided:** partial counts, reordering, bags and the container slot, weight and
  capacity, Ground to Ground, corpse or Ground into a slot, map items, depot, trade, the
  drag-and-drop UI.

## 10. Before-freeze checklist

1. **Contract amendments:** ITEM-MOVE-WIRE-0 §3 and §5; DUR-03 §39.1 and §39.3 (§6.1, paragraph
   after §33); composition §3.1 (§7.2); MOVE-RL-11 §4.3 (§7.3). Each is written pending on
   acceptance of ITEM-MOVE-WIRE-1 (#162 5912405163). The capability number is reserved
   at allocation.
2. **Serialization:** one DUR-03 transaction per move, CommandRef as cause, rule 2 fence, §32 for
   Ground, the lock order of §7.2.
3. **Restart:** items are durable; dropped items follow D191.
4. **Typed references:** handles; `EquipmentSlotV1` on the wire, semantic slot keys in storage;
   `WorldTilePosition`.
5. **Wire:** §3, capability `ITEM_EQUIP_DROP_V1`.
6. **Split work:** at most two items per move.
