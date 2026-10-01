# DEPOT-0 Character depot

- Decision: `DEPOT0-CHARACTER-DEPOT-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol and
  persistence) and protected integration. It extends ITEM-MOVE-WIRE-0 (PR #1344) and ITEM-MOVE-WIRE-1
  (PR #1354) and integrates after them.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction to start the depot (2026-09-30) and the owner answer **2b** (one
  depot per Character, world-wide, reachable from any town), given in the architect session on
  2026-09-30, verbatim record on #162 5912593702
- Builds on: DUR-03 §5.2, §5.3, §28, §39.1 and §39.3; HOUSE-CUSTODY-0 (custody family pattern and
  the item-level exclusivity guard); ITEM-MOVE-WIRE-0 (command 9, domain 11, handles, the USE item
  target, the non-durable view open); ITEM-MOVE-WIRE-1 (backpack entry deletion by TRANSFER);
  USE-WIRE-V1; WO-0 (`container_fixture`); ADR-0021 (base map lockers); B3 (D80-D83); the
  composition decision §3; PROD-ENTITLEMENTS-01 §9; owner rule 5905825574
- Amends, each pending on acceptance of DEPOT-0 (#162 5912405163): DUR-03 §5.2 and §39.1/§39.3
  (the `CharacterDepot` family and its shapes, a paragraph after the HOUSE-CUSTODY-0 amendment in
  §5.2); the composition decision (a paragraph before its §5). USE-WIRE-V1 and ITEM-MOVE-WIRE-0 §5
  are not edited here: DEPOT-WIRE-1 makes those edits after #1344 and #1354 integrate.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| DEPOT-WIRE-1 | impl, protocol review | capability `DEPOT_V1`, the locker USE, the box target, the paginated depot views in domain 11 and the `DEPOT {box}` destination (§4) | ITEM-VIEW-1; MAP-LOAD-1; MAP-WIRE-1 |
| DEPOT-1 | hard, persistence review | the `CharacterDepot` table and every persistence delta of §6, the capacity guard, the two TRANSFER shapes (§5) | HOUSE-CUSTODY-1 (the exclusivity guard, not yet built); ITEM-MOVE-2a (backpack entry deletion by TRANSFER) |
| DEPOT-CONTENT-1 | content lane | one admitted definition binding for the base map's depot lockers, as `container_fixture` world objects; the per-town depot id is ignored under 2b (§4.1) | WO-1; MAP-LOAD-1 |

Later, each with its own decision: moving items between boxes, the Stash, the Inbox (parcels,
Market deliveries, items recovered from house loss, Store deliveries: gap register §32), mail, bags
in boxes, partial counts, depot search.

## 1. Question

Where does a character keep items outside its backpack, and how does it reach them?

## 2. Facts

**PROVEN**

- DUR-03 §5.2: a custody family is introduced only as a typed family with a named owner and World
  and scope semantics. HOUSE-CUSTODY-0 (docs, #1315) admitted `HouseInterior` and one item-level
  exclusivity guard that later families join; its migration (HOUSE-CUSTODY-1) is not built yet.
- Migrations: backpack entries can be deleted only by a fee whole-burn line (`0023`) until
  ITEM-MOVE-2a widens it; placement is proven by `game_item_placement_proven` (`0011`, `0012`);
  reservations and receipts allow destination kinds 1-2 and shapes 1-4 with a NOT NULL
  `channel_id` (`0011`); the TRANSFER consistency guard requires Ground or corpse removal evidence
  and counts a fixed list of location tables (`0011`, `0014`); corpse entry removal captures
  evidence through a SECURITY DEFINER function (`0014`). The audit payload is `OneItemTransferV1`.
- ITEM-MOVE-WIRE-0 (candidate): domain 11 shows one open container (sized to a corpse,
  `ITEMV0-RL-02`); handles name items only; USE field 2 opens a corpse as a non-durable view.
- USE-WIRE-V1 field 1 runs the unique bound transition of a world object; zero candidates fail
  closed. WO-0 kinds are closed; depots are `container_fixture`. The base map carries depot lockers
  with legacy per-town depot ids (ADR-0021).
- PROD-ENTITLEMENTS-01 §9: the Premium consumer contract is not accepted, and no Game table holds
  Premium; charm state reads a source that answers Free until it is (`charm_state.rs`, `0020`).

**CIPSOFT_OFFICIAL** (the Tibia manual, `world.md`, depots)

- Depots are protection zones. The personal locker holds the Depot Chest, the Stash and the Inbox.
- The Depot Chest is private per character, holds 17 depot boxes, and takes 2,000 items on a free
  account and 15,000 with Premium; it is reachable from any city's depot. The manual leaves open
  whether stacks count once and whether the limit warns or blocks.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- The locker holds the Market, Inbox, Stash and chest (`player.cpp:2038-2053`); the chest holds the
  depot boxes (`utils_definitions.hpp:559-578`; 20 by default, free 2,000, Premium 8,000:
  `configmanager.cpp:55-63`). Boxes are keyed by index and shared by every town's locker
  (`player.cpp:1997-2032`).
- Every item counts once whatever its stack size; a container counts one plus its contents
  (`container.cpp:281-286`, `depotchest.cpp:29-43`). The chest view is paginated (32 per page,
  `depotchest.cpp:17-18`).

**Owner decision**

- **2b** (#162 5912593702, verbatim: "Jeden wspólny w całym świecie, jak w obecnej Tibii, gdzie
  depozyt i stash są dostępne z każdego miasta"): one depot per Character, world-wide, reachable
  from any town. This matches Global.

## 3. Storage (DEPOT-1)

- **Family.** A new DUR-03 location family (DUR-03 §5.3, a typed position):

  ```text
  CharacterDepot {
    character_id: CharacterId,
    box: 1..17,
    ordinal: NUMERIC(20)
  }
  ```

- **Scope.** Character + World, with no channel and no runtime scope: every depot locker on every
  channel of the World opens the same depot. The owner is the Game item domain on behalf of the
  Character.
- **Boxes.** The 17 boxes (the manual's number; Canary configures 20) are fixed compartments, not
  item instances: nothing to move, lose or duplicate.
- **Ordinal.** The highest plus one under the Character's lock, unique per (character, box), never
  renumbered; the newest entry is shown first.
- **No contents.** Only items without contents enter a box in this slice.
- **Capacity.** Each depot entry counts as one item whatever its stack quantity (Canary evidence).
  - The database enforces the hard ceiling `DEPOT0-RL-01` = 15,000 with a deferred guard that
    locks `character_root` and counts the character's depot rows.
  - The runtime applies the account limit, 2,000 free or 15,000 Premium, from a Premium source that
    answers Free until the PROD-ENTITLEMENTS-01 consumer contract is accepted (the charm state
    precedent). A move into the depot over that limit is `NO_ROOM`.
  - Nothing is removed when Premium ends; taking items out is always allowed.
- **Lifetime.** Never touched by death, `WorldReset` or channel changes. `character_id` references
  the Character root with RESTRICT, so a future deletion workflow must empty the depot first.

**Amendment (pending on acceptance of BAGS-0;
`reviews/OTERYN_GAME_BAGS0_CONTAINERS_WITH_CONTENTS_DECISION_2026-09-30.md` §9).**
A container with contents may enter a box as a tree (BAGS-DEPOT-1). Every item of the tree counts
against `DEPOT0-RL-01` and the account limit, and a bag in a box opens in a view anchored to the
depot view.

## 4. Wire and access (DEPOT-WIRE-1)

### 4.1 Opening the depot

- The base map's depot lockers are `container_fixture` world objects bound by DEPOT-CONTENT-1; the
  per-town depot id is ignored under 2b.
- **USE-WIRE-V1 amendment.** USE field 1 on a world object bound as a depot locker is a
  non-durable view open, like ITEM-MOVE-WIRE-0's corpse open: `expected_revision` is still checked,
  no overlay revision changes, and no GAME-INTERACTION occurrence is created. It requires standing
  next to the locker (Chebyshev distance 1, same floor).
- It opens the depot in domain 11 as the box list: 17 boxes with their entry counts.
- **Box target.** New USE field 6 `DepotBoxTargetV1 {box, page}` (field 5 is ITEM-USE-0's) opens one page of a box in domain
  11. A page holds at most `DEPOT0-RL-02` (32) entries; the server reads at most one page per open.
  Fields 3 and 4 stay reserved.
- The depot views are new oneof variants of domain 11, gated by `DEPOT_V1`; the corpse view keeps
  its meaning.
- At most one container view is open. It closes as ITEM-MOVE-WIRE-0 §4.3 says (leaving reach,
  floor change, teleport, death, logout, channel transfer), and when the character leaves the
  locker; its handles then become `STALE`.

### 4.2 Capability and destination

- **Capability 5 `DEPOT_V1`** (reserved by the control plane, #162 5912405163), which requires
  capability 4. Without it, a locker is `NOTHING_TO_USE` and domain 11 never shows a depot.
- Command 9 gains the destination `DEPOT {box}`, and its source may be the handle of an entry on the
  open box page. Whole items only. The depot view must be open when the command runs.
- Results: `NO_ROOM` (the depot is full for the account), `NOT_SUPPORTED` (an item with contents, a
  corpse or Ground source), and the existing results.

## 5. Moves (DEPOT-1)

- **Shapes**, each one item and one transaction:
  - a main backpack direct entry into a box (`DEPOT {box}`);
  - a depot entry out to the main backpack: into the container slot when it is empty and the
    item is an empty container with a container-slot pattern (so a character without a backpack
    can take a spare one from its depot); otherwise into a new direct entry. There is no merge or
    top-up out of the depot in this slice, so every move is one item and a depot row is only ever
    removed by that item's TRANSFER. No free entry is `NO_ROOM`.
- **Merges later.** Merging into an existing stack on the way out is a SPLIT_MERGE_QUANTITY shape
  (two items, a depot row retired by merge) and waits for its own amendment.
- **Checks when the command runs:** the open depot view, the character's current position next to
  the locker it opened (a command reserved before a reconnect is checked again, FND-02 §13.3), and
  the capacity of §3.
- **Sources refused:** a corpse entry or a Ground item cannot go into the depot (`NOT_SUPPORTED`),
  so D133 and D134 cannot be bypassed.
- **Fence.** The composition rule 2 fence of the acting Character with its `character_root` lock;
  no runtime scope owns the depot, so rule 2's §32 binding does not apply. One lease per Character
  and the root lock serialize two sessions.
- **Revision.** Item-only: rule 1 covers depot locations (§7.2), so no `CharacterRevision`
  advance.

## 6. Persistence deltas (DEPOT-1)

Each keeps the previous function body and adds one clause:

- **Depot table:** foreign keys (item, world) to item instances and `character_id` to the Character
  roots (RESTRICT); the character's World equals the item's World; `box` SMALLINT CHECK 1..17;
  UNIQUE (character, box, ordinal); rows immutable (insert and delete only).
- **Exclusivity:** the table joins the HOUSE-CUSTODY-1 item-level guard.
- **Insertion proof:** a depot clause in `game_item_placement_proven`, proven in the same
  transaction.
- **Removal proof:** a depot row is deleted only by a TRANSFER, with evidence captured through a
  SECURITY DEFINER function in the `0014` idiom; the backpack entry removal clause comes from
  ITEM-MOVE-2a.
- **Reservations and receipts:** a depot source kind and destination kind with a
  `destination_box`, two new shape numbers, and a nullable `channel_id` for depot shapes.
- **Consistency guard:** the backpack-entry and depot source kinds without Ground or corpse removal
  evidence; the depot table added to the counted location tables; a depot destination refused for
  Ground and corpse sources.
- **Audit:** additive depot location messages in `OneItemTransferV1`.
- **Rows:** the `DUR03-RL-02` and `DUR03-RL-06` allocation texts gain the depot source; each shape
  is one item, two location lines, three work units.

**Amendment (pending on acceptance of MARKET-0 (#1367); `OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md` §5, §7).** The Inbox is decided: `CharacterInbox`
is the third custody family, with two out-shapes (Inbox to backpack, Inbox to a depot box) and the
Inbox view on the locker. A sell offer takes depot entries into Market escrow and a buy-offer fill
takes them to the buyer's Inbox, with at most one split of the last entry (its row stays, its item's
quantity falls). Rule 4's order stays `character_root`, items, container-slot row, with the Market's
book and offer rows between the root and the items.

## 7. Other amendments

### 7.1 DUR-03

A paragraph after the HOUSE-CUSTODY-0 amendment in §5.2 admits `CharacterDepot` as the second
custody family and, for its two shapes only, supersedes the §39.1 and §39.3 source and destination
limits. Every other §39 obligation is unchanged.

### 7.2 Composition decision

Rule 1 also covers `CharacterDepot` locations of the acting character: an item-only transaction
between its backpack and its depot does not advance `CharacterRevision`. No runtime scope owns the
depot, so rule 2's §32 binding does not apply. Rule 4's lock order: `character_root`, then the
items in ItemInstanceId order, then the container-slot row; the capacity guard's `character_root`
lock is already held.

## 8. Rows (registered by DEPOT-1 and DEPOT-WIRE-1)

| Row | Value |
|---|---|
| `DEPOT0-RL-01` depot entries per character | 15,000 hard; the account limit 2,000 free, 15,000 Premium |
| `DEPOT0-RL-02` entries per box page | 32 |
| `DEPOT0-RL-03` pages read per open | 1 |
| One-item shape rows | 1 item, 2 location lines, 3 work units |

## 9. Rejected options

- **One depot per town (older Tibia).** The owner chose 2b, which is Global today.
- **The chest and boxes as item instances.** They could be moved, lost or duplicated.
- **Channel-scoped depots.** The scope matrix and Global share the depot across channels.
- **Counting units for capacity.** Canary counts each item once, whatever its stack.
- **A new `depot_locker` world object kind.** WO-0's kinds are closed; lockers are already
  `container_fixture` objects on the base map.
- **Showing a whole box at once.** A box can hold 15,000 entries; pages bound the view.

## 10. Decision test

- **Must decide now:** YES. The owner asked for the depot now, and a 20-entry backpack cannot hold
  a character's belongings.
- **Minimum sufficient:** one location family with 17 fixed boxes, two one-item shapes, one
  destination, one USE field, one capability.
- **Superseding evidence:** official capacity semantics.
- **Deliberately not decided:** box-to-box moves, Stash, Inbox, mail, bags in boxes, partial counts,
  depot search, Market access.

## 11. Before-freeze checklist

1. **Contract amendments:** DUR-03 §5.2 with §39.1 and §39.3 for the depot shapes; the composition
   decision rule 1, 2 and 4 texts; both written "pending on acceptance of DEPOT-0". USE-WIRE-V1
   (locker open, field 6) and ITEM-MOVE-WIRE-0 §5 (the depot destination) are edited by
   DEPOT-WIRE-1 after #1344 and #1354.
2. **Serialization:** one transaction per move, Character fence and root lock.
3. **Restart:** depot items are durable and untouched by resets.
4. **Typed references:** CharacterId, box 1..17, page, handles.
5. **Wire:** §4, capability `DEPOT_V1`.
6. **Split work:** one item per move; one page per view.
