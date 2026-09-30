# HOUSE-CUSTODY-0 House item custody

- Decision: `HOUSE-CUSTODY0-HOUSE-ITEM-CUSTODY-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence)
  and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the house contract named by D196 (ADR-0021 house tiles, owner answer a) and the owner's direct
  request of 2026-09-30 to decide it now and have a worker implement it
- Builds on: EXP-HOUSES-01 (accepted) §4.1, §5, §14, §15, §16 and §19; DUR-03 §5 and §39.3;
  ADR-0021 §4.4 and §4.7
- Amends: DUR-03 §5.2 (the `HouseInterior` family) and §39.3 (reset preflight); ADR-0021 §4.4,
  the house-tiles owner-answer row and §7 (all in this PR)
- Runtime, migration and production authority: NONE. HOUSE-CUSTODY-1 needs its own #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **HOUSE-CUSTODY-1** (durability lane, `oteryn-hard-worker`, persistence review). Owned paths:
  - the next free migration, `0025` at the time of writing (main has `0021`; `0022`-`0024` are
    reserved for CHAR-NAME-1, LCFA-1 and GOLD-FEE-1a); the control plane confirms it at
    allocation;
  - its `*_postgres` tests.
- It builds one migration, storage only:
  - the `HouseInterior` location table (§3.1): revision-free `HouseId`, an ordinal unique per
    `(world, house, position)`;
  - the `HousingReclaimProvenance` table (§3.2): primary key `item_instance_id`, 1:1 with a live
    `HouseInterior` row (deferred), FK to `character_roots` with `ON DELETE RESTRICT`;
  - one deferred item-level exclusivity guard on every location table (§3.3);
  - **no runtime grants** on the house tables (§3.5).
- It does not build a scope kind, a runtime writer, transfer shapes or reset code. Those belong to
  the house interior runtime child and to MAP-OVERLAY-1 (§4).
- **Tests** (`_postgres`):
  - a live item has exactly one row across all location tables, and a retired item has none;
  - a `HouseInterior` row without its provenance, or a provenance without its row, fails at
    commit;
  - a second provenance for the same item fails;
  - two items with the same `(world, house, position, ordinal)` fail;
  - the runtime role cannot read or write the house tables;
  - deleting a `character_roots` row referenced by a provenance fails.
- Binding: §3 of this document; EXP-HOUSES-01 §14 and §15; DUR-03 §5.2 (as amended).

## 1. Question

Where does a durable item live when it is in a house, and what happens to it at a world reset?
ADR-0021 retires every Ground item at a planned reset, including items on house tiles, until this
decision exists (D196).

## 2. Facts

**PROVEN**

- EXP-HOUSES-01 (accepted):
  - §4.1: a house belongs to one World and not to a Channel. Its durable item and container state
    exists once across all Channels.
  - §5.1 and §5.2: one authoritative interior runtime per active house, fenced by generation.
    Instance primitives may be reused, but `HouseId` stays the stable identity.
  - §5.3: entry into a house is an explicit handoff from the origin Channel.
  - §14.1 to §14.3: every item entering housing gets `HousingReclaimProvenance` in the same
    transaction. The Character subject comes only from the placer's own inventory, equipment or
    container custody. Without a subject, the placement fails closed. Old provenance is retired
    only when the next transfer commits.
  - §15: a `CharacterId` referenced by a provenance is part of the deletion and World-transfer
    guard.
  - §19: one authoritative location, no copy per Channel, and no destruction of forgotten items.
- DUR-03 §5.2: house custody must be "a separately typed/versioned family with named owner and
  explicit WorldId/scope semantics". §5.3 forbids generic custody.
- ADR-0001 §7: house ownership is World-level state.
- Each existing DUR-03 shape guard counts a fixed list of location tables (`0011`, `0013`,
  `0014`). No guard sees a new table.
- `game_item_container_entries` requires a `character_id` and a `container_slots` parent.

**DERIVED / CANDIDATE**

- The House content family (#1285, #1298) has a CANDIDATE schema and an unpopulated
  `content/houses/`. Its source data lists 2,534 wall tiles shared by two houses.

**UNKNOWN**

- The house runtime, ACL and ownership lifecycle. They belong to later EXP-HOUSES-01 children.
- Storage budgets per house (EXP-HOUSES-01 §19). They belong to the economy decisions.

## 3. Decision

### 3.1 Location family

- A new DUR-03 location family:

  ```text
  HouseInterior {
    house_id: HouseId {world_id, house_key},
    spatial_position: native WorldTilePosition,
    stack_ordinal: NUMERIC(20)
  }
  ```

- `HouseId` is revision-free: the World and the House content key. A content revision bump does
  not change it.
- It is World-scoped, with no `ChannelId`. The owner is the Game housing domain
  (EXP-HOUSES-01 §22.1).
- **Tile of the house.** The position must be a tile assigned to exactly this house, and to no
  other house, in the active bundle. Shared wall tiles are never house tiles. The check runs at
  every write and at every rebuild. A rebuild fails closed when a row's house or tile is missing
  from the active bundle.
- **Ordinal.** As in `0011`: taken as the highest plus one under the house lock, unique per
  `(world, house, position)`, never renumbered.
- **No contents.** Only items without contents may enter in this slice. Containers under a house
  root need a later decision.
- **Exclusivity.** A live item is in exactly one location family (§3.3).

### 3.2 Reclaim provenance

- Every transaction that makes an item's location `HouseInterior` also writes
  `HousingReclaimProvenance {house_id, item_instance_id, reclaim_subject, placement_transaction_id,
  provenance_revision}` (EXP-HOUSES-01 §14.2).
- In the first slice the subject is the placing `CharacterId`, taken from the placer's own
  `CharacterInventory` or `CharacterEquipment` source (§14.3). A typed-domain subject needs its
  own later decision. A placement without a subject fails closed.
- At most one live provenance exists per item. It is deleted in the same transaction in which the
  item leaves `HouseInterior`. The transfer receipt is the audit record. A same-house tile move
  keeps it and bumps the revision.
- The referenced `CharacterId` is guarded under EXP-HOUSES-01 §15: character deletion or World
  transfer cannot proceed while a provenance names the Character, until that workflow settles it.
- The provenance is not a location and grants no authority.

### 3.3 Exclusivity guard

- One deferred item-level guard covers every location table that exists at its migration:
  `game_item_ground_locations`, `game_item_container_slots`, `game_item_container_entries`,
  `game_item_corpse_container_entries` and the new `HouseInterior` table.
- A live item has exactly one row across them. A retired item has none.
- Any later location family joins this guard in its own migration.

### 3.4 Writer, fence and transfer shapes (non-binding direction)

**Pointer (pending on acceptance of HOUSE-RUNTIME-0; `reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md` §3, §6.1).** The house scope and the three shapes below become binding there;
SCOPE-HANDOFF-1 builds the scope kind migration and HOUSE-RUNTIME-1 the shapes.

This section is **non-binding direction** for the EXP-HOUSES-01 house interior runtime decision,
which fixes the fence and scope schema. HOUSE-CUSTODY-1 builds none of it.

- **Fence.** One live generation per `HouseId`, matching the one interior runtime
  (EXP-HOUSES-01 §5.1). The suggested route reuses the `Instance` runtime scope bound to `HouseId`
  (§5.2). A new migration of that child alters the tables from `0003` and `0006`:
  - a scope kind column and a house column, with `channel_id` nullable;
  - a tagged `scope_key` CHECK (`\x02` for a house);
  - `IS DISTINCT FROM` in the immutability guard;
  - house-scoped grants;
  - the writer accepting the `Instance` scope;
  - ADR-0021 reset step 2 filtering by kind.
  A stale generation writes nothing. Character-side `character_root` locks apply as for other
  TRANSFERs.
- **Shapes**, each one item and one transaction with its provenance:
  - from `CharacterInventory` or `CharacterEquipment` to `HouseInterior`;
  - from `HouseInterior` to `CharacterInventory`;
  - between tiles of the same house.
- Ground is not a source: it has no Character subject (§14.3), and the fence would span a channel
  runtime and a house runtime, against the explicit entry handoff (§5.3).
- Stack merge, split, containers and cross-house moves are not admitted by this decision.

### 3.5 Closed until ownership

- Player writers are admitted only for a house that has an owner and an ACL storage grant to the
  acting Character (EXP-HOUSES-01 §16.3, §16.4).
- Until the child that opens them, the runtime role has no grant on the house tables. The closure
  is enforced by the database, not only by Rust.
- House tiles keep ADR-0021 rule a (Ground, retired at the reset).
- **Gate for the ownership child.** Before any house can become owned, that child decides what
  happens to live Ground items on the house's tiles in each channel. Map-authored items on house
  tiles are never pickupable (ADR-0021 §4.4), so a reset cannot respawn them into an owned house.

### 3.6 World reset

- `WorldReset` retires only Ground roots and their container entries (DUR-03 §39.3). It never
  touches `HouseInterior`.
- **Target check.** Every live `HouseInterior` row's `(house_key, position)` must be a tile of the
  same house in the target bundle. A removed house, a re-keyed house, a removed tile and a tile
  moved to another house all fail it.
- **Preflight.** The check runs before reset step 1. A failure aborts the reset cleanly: no
  record, no closed admission.
- **Recheck.** Step 4 repeats the check in its own transaction, under a lock that blocks
  `HouseInterior` inserts. A failure there leaves the record RETIRING. Only an EXP-HOUSES-01
  §14.7 evacuation clears it.
- Until the house runtime exists the table is empty, so the check is trivially true. MAP-OVERLAY-1
  builds the preflight, the recheck and a test that the reset touches only Ground.

## 4. Delivery

| Child | Scope | Depends on |
|---|---|---|
| HOUSE-CUSTODY-1 | One migration: `HouseInterior`, provenance, exclusivity guard, no runtime grants, `_postgres` tests | this decision |
| MAP-OVERLAY-1 | Reset preflight, step-4 recheck, "reset touches only Ground" test (§3.6) | ADR-0021 accepted |
| House interior runtime | Fence and scope schema (decided there; §3.4 is direction), a new migration altering the `0003`/`0006` tables, writer, transfer shapes | HOUSE-CUSTODY-1; EXP-HOUSES-01 runtime decision |
| House interior runtime (pointer, pending on acceptance of HOUSE-RUNTIME-0; `reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md`) | SCOPE-HANDOFF-1, HOUSE-RUNTIME-1, HOUSE-VIEW-1, HOUSE-ITEM-WIRE-1 (the client item path, after its own wire decision) | HOUSE-CUSTODY-1 |
| House ownership and ACL | Opens §3.5, decides the Ground-on-house-tiles gate | house interior runtime |

> **Pointer (pending on acceptance of HOUSE-OWN-0, #162 5912405163).** When HOUSE-OWN-0
> (`OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md`) is accepted, it decides the
> ownership and ACL row: its HOUSE-1 child keeps this order (it depends on the house interior
> runtime child), answers the Ground-on-house-tiles gate (its §8: a house-tile guard on Ground
> rows and a settlement check), and opens §3.5 only through SECURITY DEFINER disposition
> functions.

## 5. Rejected options

- **House items as Ground with an exemption flag.** Ground is channel-scoped, while house items
  are World-scoped (EXP-HOUSES-01 §4.1). A flag would create copies per channel.
- **A generic "domain custody" row.** DUR-03 §5.3 forbids it.
- **`house_ref` with a content revision.** A revision bump would orphan every stored item
  (EXP-HOUSES-01 §5.2).
- **Building the scope kind and shapes now.** Nothing can reach them before the interior runtime
  and ownership exist (playable-first). They are given as direction here (§3.4) and decided and built with their first caller.
- **Docs only, no migration.** The owner asked for the worker now. The storage slice is small,
  and it makes the exclusivity guard cover house rows from the start.

## 6. Decision test

- **Must decide now:** YES, by owner request. It also fixes the ADR-0021 reset boundary before
  any house can be owned.
- **Minimum sufficient:** one table, one provenance table and one guard, with no runtime access.
- **Superseding evidence:** an EXP-HOUSES-01 runtime decision that needs a different fence, or a
  storage budget that needs capacity rows.
- **Deliberately not decided:** ownership, ACL, rent, auctions, storage budgets, containers in
  houses, the Ground-on-house-tiles question and the interior runtime.

## 7. Before-freeze checklist

1. **Contract amendments:** DUR-03 §5.2 and §39.3, and ADR-0021 §4.4, the owner-answer row and
   §7, are amended in this PR.
2. **Serialization:** the exclusivity guard now. One live house generation plus `character_root`
   locks later, as non-binding direction (§3.4).
3. **Restart:** the durable location and provenance are enough. A rebuild fails closed on a
   missing house or tile.
4. **Typed references:** a revision-free `HouseId`. The provenance names its transaction.
5. **Wire:** none. House views belong to MAP-WIRE-1 and the house runtime.
6. **Split work:** every shape is one transaction, with the provenance written in the same
   transaction.
