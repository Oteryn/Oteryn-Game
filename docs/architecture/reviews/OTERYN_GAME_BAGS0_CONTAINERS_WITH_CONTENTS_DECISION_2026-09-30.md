# BAGS-0 Containers with contents

- Decision: `BAGS0-CONTAINERS-WITH-CONTENTS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  persistence and performance; security for the trade child) and protected integration. It
  extends ITEM-MOVE-WIRE-0, ITEM-MOVE-WIRE-1 and ITEM-USE-0 and integrates after them.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner direction of 2026-09-30 (full Tibia Global parity), the B3 deferral "bags
  inside the backpack (RL-05 > 0)", and MAIL-0's need for filled parcels (MAIL-0 §7, open PR)
- Builds on: B3 (D80-D83, §4.1, §4.2, §4.5), DUR-03 §5.2, §10, §13, §28, §29, §34 and §39,
  ITEM-MOVE-WIRE-0 (capability 4, domains 9 and 11, handles, command 9), ITEM-MOVE-WIRE-1
  (`ITEM_EQUIP_DROP_V1`, `BLOCKED`, Ground), ITEM-USE-0 (fields 2 and 5), DEPOT-0, MARKET-0 §3.1
  and §5, PLAYER-TRADE-0, HOUSE-CUSTODY-0 §3.1, HOUSE-OWN-0 §7 (tree count and hash), D3/A10
  (corpse depth 1), the composition decision rules 1, 2 and 4, owner rule 5905825574
- Amends, each pending on acceptance of BAGS-0 (#162 5912405163), in this PR: DUR-03 §10 (a
  paragraph at its end); B3 §4.5; ITEM-MOVE-WIRE-0 §4.4; ITEM-MOVE-WIRE-1 §4; ITEM-USE-0 §3;
  DEPOT-0 §3; PLAYER-TRADE-0 §4 (offers, wire handles, swap shape and rows); HOUSE-CUSTODY-0 §3.1. MAIL-0 is not edited (open PR, §9).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| BAGS-WIRE-1 | impl, protocol review | capability `CONTAINER_TREE_V1`, the container views domain, the view command, inner handles, the `CONTAINER` destination (§5) | ITEM-VIEW-1; ITEM-EQUIP-WIRE-1 |
| BAGS-1 | hard, persistence and performance review | entries keyed by parent item, the tree guards, the tree move, moves into a nested container, the container slot with contents, the rows (§3, §4, §6) | ITEM-MOVE-2a; BAGS-WIRE-1 |
| BAGS-USE-1 | impl, combat review | nested handles for use and the breadth-first hotkey search (§8) | ITEM-USE-1; BAGS-1 |
| BAGS-GROUND-1 | hard, persistence review | dropping and picking up trees; `WorldReset` retirement of a tree (§9) | ITEM-MOVE-2b; BAGS-1 |
| BAGS-DEPOT-1 | hard, persistence review | trees in depot boxes and in the Inbox, counted per item; views anchored to the depot (§9) | DEPOT-1; INBOX-1; BAGS-1 |
| BAGS-TRADE-1 | hard, persistence and security review | a container tree as a trade offer, with its content binding (§9) | TRADE-1; BAGS-1 |

Later, each with its own decision or amendment: parcels with nested bags (the MAIL-0 amendment of
`MAIL0-RL-03`, 2026-10-01, MAIL-0 §7), containers with contents in houses (HOUSE-RUNTIME-0), containers dropped on death
(DEATH-3), bags inside monster corpses (loot content), weight (B3-3), the quiver, Manage
Containers and auto-loot, container sorting, containers larger than 20 entries.

## 1. Question

How does a player put a bag with contents into the backpack, open it, and move, store, trade or
drop it whole, with every item still in exactly one location?

## 2. Facts

**PROVEN**

- B3: `GAMEITEM01-PLACEMENT-DEPTH` 1, `GAMEITEM01-CONTAINER-ENTRIES-MAX` 20,
  `GAMEITEM01-REACHABLE-ITEMS` 21 (30 after ITEM-MOVE-WIRE-1). A moved container must be empty, so
  `DUR03-RL-05` (container expansion) is 0. Migration `0011` keys each entry to the character's
  container slot by a composite foreign key, which makes depth 2 unrepresentable
  (`0011_item_transfer_backpack.sql:54-72`; `item_transfer.rs:100`).
- DUR-03 §10: a contained item's location is its parent; moving a container changes only the
  root's location; capacity, type and nesting validate before commit; no orphan or cycle. §28:
  every expansion needs a hard ceiling. §29: advisory locks are never the sole authority.
- The registry: `DUR03-RL-05` 0, `DUR03-RL-06-EFFECT-WORK-UNITS` 6, `DUR03-RL-08` 3 retry work
  units, each pass within `DFR-DB-PASS-MS` (2,000 ms).
- Refusals of contents today: B3 §4.2, ITEM-MOVE-WIRE-1 §4 (containers in slots), DEPOT-0 §3,
  MARKET-0 §3.1 and §5, PLAYER-TRADE-0 §4, HOUSE-CUSTODY-0 §3.1, D3 (corpse depth 1). MAIL-0 §7
  (open) allows 10 children without contents and waits for this decision.
- HOUSE-OWN-0 §7 records a fenced set as a count and a SHA-256 over the sorted ItemInstanceIds.
- ITEM-MOVE-WIRE-0: handles are per GameSession, monotonic, live while in a view, `STALE` after.
  Domain 11 holds one open container. ITEM-USE-0 §3: the hotkey form searches only main backpack
  direct entries (`PARITY_PENDING`).
- The composition decision: rule 1 (item-only transactions do not advance `CharacterRevision`),
  rule 4 (lock `character_root`, then items in ItemInstanceId order).

**CIPSOFT_OFFICIAL** (the Tibia manual, `docs/reference/tibia-manual/`)

- The Container Slot holds bags and backpacks (`interface.md:62`). "Open in new window" exists
  only on containers (`controls.md:13`).
- Nested containers exist, and sorting them has an unspecified depth limit (`controls.md:78`,
  `:106`).
- A trade may offer a whole container; its contents are shown to the partner; at most 100 items
  per trade (`controls_trading.md:25-27`). A container that still has contents is not a Market
  ware (`controls_trading.md:36`).
- Bags and backpacks with contents drop on death (`characters.md:202`, `:208`).
- Capacity is weight; picking up over capacity fails (`interface.md:79`).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`, read-only)

- Limits per top container: 5,000 items, 500 containers, traversal depth 200, all configurable
  (`config.lua.dist:93-97`; checked in `container.cpp:594-610`).
- A container cannot enter itself or a descendant (`container.cpp:573-576`).
- A container's weight includes its contents (`container.cpp:236-239`, `:291-292`); a container
  counts one plus its contents for depot limits (`container.cpp:277-289`).
- The hotkey search is breadth-first: the player's slots, then containers level by level
  (`game.cpp:3013-3060`, called at `:1261`).
- A move "anywhere" into a player whose main container is full falls through to nested
  containers, breadth-first (`player.cpp:4887-4995`).
- The protocol carries a 4-bit container id, so 16 windows (`game.cpp:1122`, `:1209`).
- A traded container holds at most 99 items besides itself (`game.cpp:5921`).

## 3. Trees and bounds (BAGS-1)

- A **tree** is a root item and every item below it. A root lives in any location family: the
  container slot, a main backpack entry, a depot box, the Inbox, Ground, later a house tile.
- **Depth.** An item's depth is the number of parent edges from it up to its tree's root: the root
  has depth 0 and a direct entry of the root depth 1. A subtree's **height** is the depth of its
  deepest item below its own root, counted the same way (0 for an item without contents). Every
  item in a character's main backpack tree has depth at most `BAGS0-RL-01` = **8**, which replaces
  `GAMEITEM01-PLACEMENT-DEPTH` 1. Any other tree has the same bound.
- **Size.** A tree holds at most `BAGS0-RL-02` = **500** items, root included. Both bounds hold
  after every commit; a move that would break one is refused.
- **Each container's own capacity** stays its definition's entry count, at most
  `GAMEITEM01-CONTAINER-ENTRIES-MAX` (20). A bag of 8 holds 8 entries.
- **Why these numbers** (architect ruling R1, §13): a main backpack of 20 full backpacks, the
  common Tibia supply layout, is 421 items at depth 2; 500 covers it with room, matches Canary's
  500 containers per tree, and caps one tree move (§4) at about 500 rows. Depth 8 covers bag
  chains well beyond ordinary play and caps every ancestor walk at 8 reads. Global publishes no
  limit (`PARITY_PENDING`).
- **Reachable items** per character: the main backpack tree and the nine slots, 509
  (`GAMEITEM01-REACHABLE-ITEMS`, was 30).
- **No cycles.** An item never enters itself or a descendant; checked by walking the destination's
  ancestors (at most 8) and enforced by the database.

## 4. Storage, locks and the tree move (BAGS-1)

### 4.1 Entries keyed by parent item

- Every container entry is `Container {parent ItemInstanceId, ordinal}` (DUR-03 §5.2), with the
  B3 ordinal rule (highest plus one, never renumbered, newest first). A descendant stores **no
  owner**: the owning Character, depot, Inbox or Ground scope is the root's location, found by
  walking at most 8 parents.
- BAGS-1 re-keys the `0011` entries to their parent item: the composite foreign key to the
  container slot and the stored `character_id` go. The parent must be a live item whose definition
  is a container. Entries join the item-level exclusivity guard (HOUSE-CUSTODY-0 §3.3).
- So moving a container, the main backpack included, writes only its own location, and the
  children keep theirs (DUR-03 §10).

### 4.2 Locks

- **One lock order: rule 4 as already amended, unchanged.** BAGS-0 adds no lock order and no
  composition amendment. A write anywhere in a tree takes the locks in the order rule 4 already
  fixes for its families (composition decision §4, the DEPOT-0, Market and ITEM-MOVE-WIRE-1
  amendments; ITEM-MOVE-WIRE-1 §7.2): `character_root` first; then every item row in one
  ItemInstanceId order; then the container-slot row; then the Ground tile row and the per-channel
  counter (Ground), or the Inbox counters by CharacterId (MARKET-0 §5). The Ground tile row is
  never taken before an item row. The writer finds the root by the parent walk before locking and
  checks the walk again after the item locks; a changed walk retries within `DUR03-RL-08`.
- **Row locks inside the item step.** The item step takes, in the same single ItemInstanceId
  order: `FOR UPDATE` on each moved root and on each container whose entries change (a placement
  or removal), and `FOR SHARE` on every other item of a moved tree. So any concurrent placement,
  removal or state change inside the tree conflicts with the move in the database, not only in the
  runtime (DUR-03 §29), and competing pickups, drops and moves of the same tree and tile take their
  locks in the same order.

### 4.3 The tree move

- **Shape.** One TRANSFER of the root (one removal and one placement line). The descendants are
  locked and checked, not moved. Class TRANSFER only; no value lines.
- **Checks before commit:** the destination container's free entry, the cycle walk, the depth
  bound counting the insertion edge (the moved root lands at the destination container's depth
  plus 1, so the destination container's depth + 1 + the moved subtree's height must be at most 8;
  a root placed as a tree's own root, such as into the container slot or on Ground, needs its
  height at most 8), the destination tree's size plus the moved tree's size at most 500 (only when the move changes trees; the destination tree is read under the
  locks, and moved plus destination items are at most 500 in all, so this stays inside the 502), and each family's own capacity
  (depot and Inbox count every item, §9).
- **Tree binding.** The receipt records the moved tree's item count and a SHA-256 over its sorted
  ItemInstanceIds (the HOUSE-OWN-0 §7 idiom), read under the locks of §4.2. A trade binds more
  (§9). A replay compares both.
- **Cost.** 3 work units for the root (participant, removal, placement) and 1 per descendant read
  and locked: at most 502. Container levels expanded: at most 8 (`DUR03-RL-05-TREE`). Moving an
  item without contents stays the existing one-item shape, plus the ancestor walk of the
  destination.
- **Performance bound.** BAGS-1 measures the commit of a 500-item, depth-8 tree move
  (`BAGS0-RL-06`); its p99 must stay at or below 100 ms on the reference database, inside the
  2,000 ms pass. If it does not, the tree size ceiling is lowered by a new decision; the pass bound
  is never raised for it.
- **Revision.** Every tree move between locations of the acting character is item-only (rule 1):
  no `CharacterRevision` advance.

## 5. Wire (BAGS-WIRE-1)

- **Capability `CONTAINER_TREE_V1`**, which requires capability 4 and `ITEM_EQUIP_DROP_V1`; its
  number is reserved on #162 at allocation. Without it, nothing below is sent, a container with
  contents never appears in a view (such a client sees it as an entry only), and command 9 keeps
  its earlier meanings.
- **Domain `CONTAINER_VIEWS`** (number reserved on #162 at allocation): up to `BAGS0-RL-03` = 16
  open views (Canary's 4-bit id). Each view has a view id (0-15), the container's handle, its
  parent's handle when the parent is visible (for "up"), its capacity, and its entries (handle,
  item definition, count, sub-type), at most 20. Domain 9 is unchanged: the slots and the main
  backpack's direct entries.
- **Command `CONTAINER_VIEW_INTENT`** (type number reserved on #162 at allocation), non-durable, no
  occurrence, as a corpse open:
  - `OPEN {handle, replace_view?}`: open the container in a new view, or in place of an open view;
  - `CLOSE {view}`; `UP {view}`: replace the view by its parent container.
  - USE field 2 on a container handle under the capability is `OPEN` in a new view.
  - Results: `OPENED`, `CLOSED`, `STALE`, `TOO_FAR`, `NOT_A_CONTAINER`, `TOO_MANY_VIEWS`, at most 4
    bytes. Its rate is `BAGS0-RL-04`, measured and registered by BAGS-WIRE-1.
- **Reach.** A container in the character's own trees is always in reach. A container in a depot
  or Inbox tree opens only while that depot view is open (BAGS-DEPOT-1). A traded tree opens in
  the trade view (BAGS-TRADE-1), read-only, by a partner-facing handle (§9, trade handles). A
  container on the Ground opens only while it is within reach of the character's existing
  item-on-Ground reach (the ITEM-MOVE-WIRE-1 §5 pickup distance, same floor and channel), as a
  view anchored to the Ground root; it is read-only as to contents until picked up except that an
  entry may be moved out into the character's own trees under the same reach. The view goes `STALE`
  when the character leaves reach, the tree is picked up, moved or retired, or the channel changes.
  Looting a Ground container is otherwise unchanged; in-place editing on the Ground is a declared
  difference deferred to BAGS-GROUND-1 (architect ruling R8, §13).
  Corpses keep domain 11 and depth 1.
- **Handles.** Inner entries get handles when a view shows them (ITEM-MOVE-WIRE-0 §4.1: live while
  in a view, never reused). Live handles per session `ITEMV0-RL-03` is re-measured with 16 views of
  20 entries (at least 382).
- **Command 9 under the capability.** The source may be the handle of any entry of an open view.
  New destination `CONTAINER {handle}`: into the named container, which must be visible (a domain
  9 entry, a view, or an entry of a view). Whole items only.
- **Results** reuse the existing set: `NO_ROOM` (the container has no free entry, or the tree
  would pass 500), `BLOCKED` (depth over 8, or into itself or a descendant), `STALE`,
  `NOT_SUPPORTED` (a destination this decision does not admit yet).
- **Invalidation.** A view shows one container by its item, not its position, so moving an
  ancestor inside the character's own trees keeps it open. A view closes when its container, or an
  ancestor, leaves the character's own trees (dropped, stored while the depot is closed, traded,
  posted), when its anchor view closes, when the container retires, and on death, logout,
  reconnect and channel transfer. Every handle that leaves every view becomes `STALE`. After a
  reconnect `CONTAINER_VIEWS` is empty; the client may reopen (R3).

## 6. Moves in the character's own trees (BAGS-1)

- **Into a nested container** (`CONTAINER {handle}`): a whole item or tree from any entry of the
  own trees, from an open corpse entry (the `0014` source, D133 and D134 unchanged), or from a
  depot, Inbox or Ground source when its child admits it.
- **Merge.** A stackable item moved into a container merges or tops up into a compatible stack of
  that container, first in display order, by the D83 shapes; else it takes a new entry. Merging
  (Amendment, pending on acceptance of STACK-0; `reviews/OTERYN_GAME_STACK0_PARTIAL_COUNTS_AND_STACK_MERGES_DECISION_2026-10-01.md`: with an optional `count`, STACK-0 §4 moves part of a stack.)
  never searches other containers.
- **`MAIN_BACKPACK`** keeps B3: only direct entries; a full main backpack is `NO_ROOM`. There is no
  fall-through into nested bags (R2).
- **Container slot.** A container with contents may move into the empty container slot; it becomes
  the main backpack, its tree bounded as §3. The main backpack may leave the slot as a tree to a
  destination whose child admits it (Ground, a depot box); the character then has no main backpack.
- **Unequip into a nested bag.** ITEM-MOVE-WIRE-1's unequip may target `CONTAINER {handle}`.
- Containers stay refused in the nine other slots (the quiver waits).

## 7. Capacity and weight

- Each container checks its own free entries; the tree bound is §3.
- Weight stays unchecked until B3-3 (D81). When B3-3 lands, a container's weight is its own plus
  its contents' (Canary evidence), a character carries the weight of its slots and main backpack
  tree, and only a move into the character from outside checks it. Moves inside the character's
  trees never change carried weight.

## 8. Using nested items and hotkeys (BAGS-USE-1)

- **Field 2** (ITEM-USE-0, RUNE-USE-0) may name any entry of an open view in the character's own
  trees.
- **Field 5, the hotkey form.** The first matching, unreserved stack, searched breadth-first: the
  equipment slots (after ITEM-MOVE-2a), then the main backpack's direct entries in display order,
  then each container met, level by level, in the same order. Closed bags are searched. This is
  Canary's order (`PARITY_PENDING` against Global).
- The search reads the runtime projection only, at most 509 items (`BAGS0-RL-07`); the found stack
  is then reserved and burnt by the existing shapes, whose owner lock (§4.2) covers its depth.
- Runes use the same form: RUNE-USE-0's "nested bags for hotkeys" is answered here.

## 9. Other locations: which refusals lift

Each item names the decision, its refusal today, the result and the child that builds it.

- **B3 §4.2 and §4.5** (only empty containers; depth 1): lifted to the trees of §3. BAGS-1.
- **ITEM-MOVE-WIRE-1 §4 and §5** (containers in slots; dropping the main backpack): the container
  slot with contents is lifted (§6); the other nine slots stay refused. BAGS-1.
- **ITEM-MOVE-WIRE-1 §5 and ADR-0021 D191** (Ground items without contents): lifted. A tree is
  dropped and picked up whole. **Ground counter (architect ruling R6, §13).** Only a tree's root has
  a Ground location row, but `ITEMMOVE1-RL-02` (20,000 per channel) and the ITEM-MOVE-WIRE-1 §5
  counter count every item reachable from a Ground root: the counter equals the number of live
  Ground roots plus all their live descendants, and the rebuild invariant is stated on that basis
  (a rebuild walks each Ground root's tree). A drop adds the tree's item count, a pickup subtracts
  it, a move of an entry out of a Ground tree into the character's own trees (§5 Reach) subtracts
  the extracted subtree's item count (1 for an item without contents), and a per-item `WorldReset`
  retirement subtracts one, each in the same transaction and under the same tile and counter row
  lock as the move or retirement (§4.2 order; for an extraction, the tile of the Ground root found
  by the parent walk). In general every commit that changes the number of items reachable from
  Ground roots adjusts the counter by exactly that change, in that commit. A drop that would
  exceed the ceiling is refused whole (`GROUND_FULL`); a tree drop or pickup never adjusts the
  counter by its root alone. `WorldReset` keeps DUR-03 §39.3 step 3 unchanged in shape: each item
  of a Ground tree is retired by its own one-item `DECAY_RETIRE` transaction (one participant,
  three work units, one `OneItemTransactionV1` event, cause `WorldReset {world_id, reset_epoch, item_instance_id}`), in post-order: every descendant
  before its parent, the root last. This generalizes the D3 order (entries first, then the root) to
  depth 8; a container is never retired while it has a live entry (DUR-03 §10). At most 500 such
  transactions per tree (`DUR03-RL-01-TREE-RETIRE`); a crash resumes by the per-item retirement
  uniqueness. There is no aggregate tree retirement (architect ruling R4, §13). BAGS-GROUND-1.
- **DEPOT-0 §3** (only items without contents): lifted. A tree sits in a box, every item counts
  against the depot limit (the manual and Canary count contents), and a bag in a box opens
  anchored to the depot view. BAGS-DEPOT-1.
- **MARKET-0 §5, the Inbox** (whole items without contents): lifted for deliveries that are trees
  (parcels, house disposition); every item counts; a tree leaves whole. BAGS-DEPOT-1, which also
  writes the MARKET-0 §5 pointer (MAIL-0's open PR amends that section now).
- **MARKET-0 §3.1, wares** (containers with contents): **kept**, as Global refuses them
  (`controls_trading.md:36`). No amendment.
- **PLAYER-TRADE-0 §4** (an item with contents): lifted. One tree per side of at most 100 items
  (the manual; Canary 99 plus the container). The main backpack itself stays refused
  (`PARITY_PENDING`). BAGS-TRADE-1.
- **HOUSE-CUSTODY-0 §3.1** (only items without contents): explicit follow-up. HOUSE-RUNTIME-0
  admits house trees with the §4 shape; contents already count against `HOUSEOWN0-RL-14`, and a
  disposition step moves whole trees.
- **D3 and A10** (corpse depth 1): kept for corpses. Looting into a nested bag is lifted (§6).
  Bags in corpses wait for DEATH-3 (death drops) and loot content (monster loot bags).
- **MAIL-0 §7** (10 children without contents): lifted by the MAIL-0 amendment of `MAIL0-RL-03`
  (2026-10-01): a parcel tree within §3; MAIL-PARCEL-1 uses the §4 tree move.

- **Trade restrictions.** Every item of an offered tree, the root and each descendant, must pass
  the PLAYER-TRADE-0 `NOT_TRADEABLE` and binding restrictions (an untradeable or bound item makes
  the whole offer refused `NOT_TRADEABLE`, naming no item). The check runs at offer time and again
  under the swap row locks of §4.2 before `TRANSFERRING` commits; a descendant that has become
  restricted cancels the trade. A restricted item is never carried to the counterparty by moving
  its ancestor (architect ruling R7, §13).
- **Trade binding.** A traded tree binds, per item, the ItemInstanceId, definition key and
  revision, quantity, a state digest, and its topology: the immediate parent ItemInstanceId and
  the entry ordinal (none for the root), hashed in ItemInstanceId order. So a reparent or reorder
  inside the offered tree changes the binding like any other change. Any change cancels the
  trade before `TRANSFERRING` (PLAYER-TRADE-0 §4); the swap checks the binding under the §4.2
  locks. The receiver needs one free entry for the root.
- **Trade handles** (BAGS-WIRE-1 with BAGS-TRADE-1; the PLAYER-TRADE-0 §3 wire amendment). Under
  `CONTAINER_TREE_V1`, the `PLAYER_TRADE` domain gives each offered item that is a container, and
  each container nested in it, a handle local to the recipient's GameSession (ITEM-MOVE-WIRE-0
  §4.1 allocation: monotonic, never reused), issued when the partner's offer becomes visible to
  that session. The offerer keeps its own handles. `OPEN` accepts a trade handle only
  read-only: its view and the handles of its entries admit `OPEN`, `CLOSE` and `UP` (not above
  the offered root) and are refused as a command 9 source or destination, a USE field 2 target
  and every other command (`NOT_SUPPORTED`). Every trade handle of a trade, and every view opened
  by one, becomes `STALE` on any change to either offer (which also cancels the trade) and when
  the trade ends (`COMPLETED`, `CANCELLED` or any other close).
- **Two-tree trade shape** (architect ruling R5, §13). One swap transaction moves both roots, each
  by the §4.3 root shape, and locks and checks both trees under §4.2 (PLAYER-TRADE-0 §5 order,
  every item of both trees in one ItemInstanceId order). Its bounds override the PLAYER-TRADE-0
  two-item rows for a swap in which at least one offer is a tree: at most 200 touched items (two
  trees of `BAGS0-RL-05`), 2 participants, 4 location lines (one removal and one placement per
  root), and work units the sum of both sides by the §4.3 formula, 3 per root plus 1 per
  descendant, so at most 2 × (3 + 99) = 204. Each side's destination main-backpack tree is also
  validated under the swap locks (no cached subtree counts, §11): both main-backpack roots are
  locked and every other item of both destination trees is read to prove the post-swap 500-item
  limit and the §4.3 depth check (insertion edge counted), at most 800 further reads (2 × 400,
  roots included), one work unit each, so at most 204 + 800 = 1004 work units and
  200 + 800 = 1000 items locked or read in all. A destination that is not a main-backpack tree
  adds its own family's bounded check (depot and Inbox counters, §4.3). The receipt records both
  tree bindings (count and SHA-256 per side, §9 trade binding); the trade event is the PLAYER-TRADE-0 two-line event plus,
  per side, one count and one 32 B binding digest, within the measured `DUR03-RL-07-TRADE` bound
  (above it the shape returns for a new decision). A swap of two items without contents keeps the
  PLAYER-TRADE-0 `2 / 6` shape.
- **Depot and Inbox counts.** A tree move into a box or the Inbox counts every item of the tree;
  BAGS-DEPOT-1 widens the `DEPOT0-RL-01` guard and the Inbox counter by the tree count read in
  §4.3.

## 10. Rows (registered by each child before implementation)

| Row | Value |
|---|---|
| `BAGS0-RL-01` placement depth below the root | 8 (replaces `GAMEITEM01-PLACEMENT-DEPTH` 1) |
| `BAGS0-RL-02` items per tree, root included | 500 |
| `BAGS0-RL-03` open container views per session | 16 |
| `BAGS0-RL-04` view commands per session per second | measured and registered by BAGS-WIRE-1 |
| `BAGS0-RL-05` items in one traded tree | 100 |
| `BAGS0-RL-06` tree move commit p99 at 500 items, depth 8 | at most 100 ms, measured by BAGS-1 |
| `BAGS0-RL-06-TRADE` two-tree swap commit p99 with two 100-item trees and two full 500-item destination backpacks (1004 work units) | at most 100 ms, measured by BAGS-TRADE-1; above it the shape returns for a new decision |
| `BAGS0-RL-07` items read by one hotkey search | 509 (runtime only) |
| `GAMEITEM01-REACHABLE-ITEMS` | 509 per character |
| `DUR03-RL-01-TREE-MOVE` touched items | 500 (1 moved, up to 499 locked and checked) |
| `DUR03-RL-02-TREE-MOVE` location lines | 2 |
| `DUR03-RL-05-TREE` container levels expanded | 8 |
| `DUR03-RL-06-TREE-MOVE` participants / work units | 1 / 502 |
| `DUR03-RL-07-TREE-MOVE` envelope and payload | one-item caps; adds a count and a 32 B hash |
| `DUR03-RL-01-TREE-TRADE` touched items in one two-tree trade swap | 200 offered (2 roots moved, up to 198 locked and checked), plus up to 800 destination-tree validation reads |
| `DUR03-RL-02-TREE-TRADE` location lines | 4 |
| `DUR03-RL-06-TREE-TRADE` participants / work units | 2 / sum of both sides at 3 per root plus 1 per descendant (at most 204), plus 1 per destination-tree validation read (at most 800): at most 1004 |
| `DUR03-RL-07-TREE-TRADE` envelope and payload | the `DUR03-RL-07-TRADE` event plus, per side, a count and a 32 B binding digest; within the measured `DUR03-RL-07-TRADE` bound (BAGS-TRADE-1) |
| `DUR03-RL-01-TREE-RETIRE` one-item `DECAY_RETIRE` transactions per Ground tree at `WorldReset` | at most 500, post-order; each the one-item shape: 1 participant / 3 work units, one `OneItemTransactionV1` (BAGS-GROUND-1) |
| `ITEMV0-RL-03` live handles per session | re-measured, at least 382 |
| `CONTAINER_VIEWS` snapshot bytes | within FND-02 limits, measured; else `BAGS0-RL-03` falls |

`DUR03-RL-08` (3) and `GAMEITEM01-CONTAINER-ENTRIES-MAX` (20) are unchanged.

## 11. Rejected options

- **Unbounded nesting, as Canary's depth 200.** DUR-03 §28 forbids an unbounded expansion, and
  every ancestor walk and tree read must have a ceiling.
- **Storing the owning Character on every descendant.** Moving a tree would rewrite every child's
  row, 500 location lines instead of 2.
- **Moving every descendant as its own TRANSFER line.** DUR-03 §10 says only the root moves; it
  would multiply audit and receipts by the tree size.
- **Checking a tree only in the runtime.** DUR-03 §29 needs the database to reject a concurrent
  change; §4.2's row locks do.
- **Cached subtree counts on each container.** Every placement would update up to 8 ancestors;
  a bounded read of at most 500 rows is cheaper and cannot drift.
- **One open view in domain 11 for bags too.** Global and Canary open many windows at once, and the
  corpse and depot close rules differ from the own trees.
- **Nested bags as a Market ware.** Global refuses them.

## 12. Owner-rule applications

**Global parity kept:** bags inside bags; opening several windows; moving a bag with its contents;
putting the main backpack on or taking it off with its contents; looting into a chosen bag;
stacking into the destination container; hotkeys that find items in closed bags; whole containers
in trades with contents shown; bags in the depot counted per item; no bags with contents on the
Market.

**Declared differences:**
- Depth 8 and 500 items per tree (Global unpublished, `PARITY_PENDING`, R1).
- A full main backpack does not fall through into nested bags (R2).
- Container windows close on reconnect (R3).
- Bags in houses, bags inside parcels and bags in monster corpses wait for their follow-ups.

## 13. Architect rulings (owner rule 5905825574)

**R1. Depth and tree size.** Global publishes no nesting limit; DUR-03 §28 needs one. a) 8 and 500
(recommended: covers a backpack of full backpacks, matches Canary's container count, bounds a move
to about 500 rows); b) 4 and 200: cheaper, but refuses real supply layouts; c) Canary's 200 and
5,000: a tree move of thousands of rows in one transaction. **Ruled a).**

**R2. A full main backpack.** Canary falls through into nested bags breadth-first; the Global
behaviour is not sourced. a) Keep B3: `NO_ROOM`, the player chooses a bag (recommended: no silent
deep placement, one bounded check); b) Canary's fall-through. **Ruled a)**, `PARITY_PENDING`.

**R3. Views after reconnect.** a) Close all; the client may reopen (recommended: the view set is
runtime state and FND-02 resume stays unchanged); b) carry views in resume state. **Ruled a).**

**R4. Retiring a Ground tree at `WorldReset`.** a) Keep DUR-03 §39.3: one bounded one-item
`DECAY_RETIRE` per item, in post-order (recommended: the accepted participant, work-unit, event and
audit shapes are reused; no new aggregate schema to qualify); b) one aggregate transaction of up to
500 items, which would need its own participants, work units, payload bound and event schema.
**Ruled a).**

**R5. A trade swap of two trees.** The manual allows a whole container per side; PLAYER-TRADE-0
registers only a two-item swap. a) One swap transaction for both trees with its own override
rows: 200 touched items, 2 participants, 4 location lines, 204 work units, both tree bindings in
the receipt and event (recommended: one atomic swap as Global, bounded by `BAGS0-RL-05` per side,
the accepted §4.3 root shape reused per side); b) two separate tree moves, which breaks the
all-or-nothing trade. Round 4 adds the destination main-backpack validation (2 root locks, at
most 800 tree reads, 1004 work units in all) to these rows and to the `BAGS0-RL-06` trade
qualification; a durable per-root count was rejected (§11, cached subtree counts). **Ruled a).**

**R6. Ground counter with trees.** a) Count every item reachable from Ground roots, with atomic
tree-count adjustments on drop, pickup, extraction of an entry from a Ground tree and per-item
retirement (recommended: keeps the 20,000
ceiling meaningful and the counter/rebuild invariant exact); b) count roots only, which lets 500-item
trees bypass the ceiling. **Ruled a).**

**R7. Trade restrictions on descendants.** a) Every item of an offered tree passes the trade and
binding restrictions, rechecked under the swap locks (recommended: fail closed); b) root only,
which lets a restricted item cross inside a bag. **Ruled a).**

**R8. Opening a container on the Ground.** a) Reach-bound read-only view anchored to the Ground root,
closed by leaving reach or any tree change, with in-place editing a declared difference deferred to
BAGS-GROUND-1 (recommended: smallest definition that keeps the parity path reachable); b) defer
entirely. **Ruled a).**

## 14. Owner questions

None. Every choice here is a Global-parity application or a bound under DUR-03 §28, which owner
rule 5905825574 gives to the architect.

## 15. Decision test

- **Must decide now:** YES. Owner direction (full parity); MAIL-0 parcels, depot, trade and loot
  all refuse bags until this exists.
- **Minimum sufficient:** one bound pair, entries keyed by parent, one tree move shape, one
  capability with one domain, one command and one destination; results, handles, D83 merges and
  the owner locks are reused.
- **Superseding evidence:** official Global nesting or tree limits; the Global full-backpack
  behaviour; a measured tree move over `BAGS0-RL-06`.
- **Deliberately not decided:** houses, parcels with bags, death drops, loot bags, weight, the
  quiver, auto-loot, sorting, containers larger than 20, Stash.

## 16. Before-freeze checklist

1. **Contract amendments:** DUR-03 §10; B3 §4.5; ITEM-MOVE-WIRE-0 §4.4; ITEM-MOVE-WIRE-1 §4;
   ITEM-USE-0 §3; DEPOT-0 §3; PLAYER-TRADE-0 §4 (offers, trade handles, the two-tree swap shape
   and its rows); HOUSE-CUSTODY-0 §3.1; each pending on
   acceptance of BAGS-0. Capability, domain and command numbers are reserved at allocation.
2. **Serialization:** rule 4's existing order (ITEM-MOVE-WIRE-1 §7.2): `character_root`, then all
   item rows in one ItemInstanceId order (moved roots and changed containers `FOR UPDATE`, the rest
   of a moved tree `FOR SHARE`), then the container-slot row, then the Ground tile row and counter;
   `WorldReset` retires a tree per item in post-order (§9); replay by CommandRef.
3. **Restart:** items and entries are durable; views, handles and hotkey searches are runtime.
4. **Typed references:** handles on the wire; `Container {parent, ordinal}` in storage; no owner
   stored on descendants.
5. **Wire:** §5, capability `CONTAINER_TREE_V1`.
6. **Split work:** one root per move (two per trade swap, at most 200 offered items plus 800 destination validation reads); at most 500 items
   read and locked; at most 8 levels.
