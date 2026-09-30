# HOUSE-CUSTODY-0 House item custody

- Decision: `HOUSE-CUSTODY0-HOUSE-ITEM-CUSTODY-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence)
  and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the follow-up named by ADR-0021 (house tiles, owner answer a) and the owner's direct
  request of 2026-09-30 to decide it now and have a worker implement it
- Builds on: EXP-HOUSES-01 (accepted) §4.1, §4.3, §5, §14, §19 and §20; DUR-03 §5 and §39.3;
  ADR-0021 §4.4 and §4.7
- Amends: DUR-03 §5.2 (the `HouseInterior` family, in this PR); ADR-0021 §4.4 (house tiles, in
  this PR)
- Runtime, migration and production authority: NONE. HOUSE-CUSTODY-1 needs its own #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **HOUSE-CUSTODY-1** (durability lane, `oteryn-hard-worker`, persistence review). Owned paths:
  - the next free migration;
  - `apps/game-server/src/durability/**` (house custody module);
  - its `*_postgres` tests.
- It builds:
  - the `HouseInterior` location table, exclusive with every other location family (§3.1);
  - the `HousingReclaimProvenance` table, written in the same transaction as every placement
    (§3.2);
  - a runtime scope kind `House {world, house}` in the scope-assignment model, with the same
    generation fence as a channel scope (§3.3);
  - the one-item TRANSFER shapes in and out of `HouseInterior` (§3.4). They stay closed to
    players until a house has an owner (§3.5).
- It also:
  - makes `WorldReset` retirement skip `HouseInterior` (§3.6);
  - extends `verify_character_integrity` if it reads item locations;
  - grants and revokes as `0017` does.
- **Tests:**
  - an item has exactly one location across all families;
  - a placement without provenance fails;
  - a stale house generation writes nothing;
  - a reset leaves `HouseInterior` untouched;
  - the player writer refuses while the house has no owner;
  - replay and conflict.
- Binding: §3 of this document; EXP-HOUSES-01 §14 and §19; DUR-03 §5.2 (as amended).

## 1. Question

Where does a durable item live when it is in a house, and what happens to it at a world reset?
ADR-0021 retires every Ground item at a planned reset, including items on house tiles, until this
decision exists (owner answer, house tiles a).

## 2. Facts

**PROVEN**

- EXP-HOUSES-01 (accepted):
  - §4.1: a house belongs to one World and not to a Channel. Its durable item and container state
    exists once across all Channels.
  - §5.1 and §5.4: one authoritative interior runtime per active house, fenced by generation.
    Instance primitives may be reused, but `HouseId` stays the identity.
  - §14: every item entering housing gets `HousingReclaimProvenance` in the same transaction,
    naming the placing Character or a typed domain subject. Without one, the placement fails
    closed.
  - §19: one authoritative location, no copy per Channel, and no destruction of forgotten items.
- DUR-03 §5.2: house custody must be "a separately typed/versioned family with named owner and
  explicit WorldId/scope semantics". §5.3 forbids generic custody.
- ADR-0001 §7: house ownership is World-level state.
- The House content family (#1285, #1298) holds house keys, tiles and doors. No ownership,
  rent or ACL runtime exists yet.

**UNKNOWN**

- The house runtime, ACL and ownership lifecycle. They belong to later EXP-HOUSES-01 children.
- Storage budgets per house (EXP-HOUSES-01 §19). They belong to the economy decisions.

## 3. Decision

### 3.1 Location family

- A new DUR-03 location family:

  ```text
  HouseInterior {
    world_id: WorldId,
    house_ref: {family: House, key, revision},
    spatial_position: native WorldTilePosition,
    stack_ordinal: u8
  }
  ```

- It is World-scoped, with no `ChannelId`. The position must be one of the house's tiles in the
  active content revision.
- Nested items use the existing `Container` family under a `HouseInterior` root.
- An item is in exactly one location family at a time, enforced across all location tables.
- The owner is the Game housing domain (EXP-HOUSES-01 §22.1).

### 3.2 Reclaim provenance

- Every transaction that makes an item's location `HouseInterior`, or a container under it, also
  writes `HousingReclaimProvenance {world, house_ref, item, reclaim_subject, placement
  transaction, revision}` (EXP-HOUSES-01 §14.2).
- In the first slice the subject is the placing `CharacterId`. A typed-domain subject needs its
  own later decision.
- A placement that cannot name a subject fails closed. The provenance is not a location and
  grants no authority.

### 3.3 Writer and fence

- House item writes are fenced by a runtime scope of kind `House {world, house}`. It uses the
  existing scope-assignment writer and generation model, next to the `(world, channel)` kind.
- One live generation per house matches the one interior runtime (EXP-HOUSES-01 §5.1). A stale
  generation writes nothing.
- Character-side locks (`character_root`) apply as for other TRANSFERs.

### 3.4 Transfer shapes

- One-item TRANSFER from `CharacterInventory`, `CharacterEquipment` or Ground to `HouseInterior`,
  with provenance.
- One-item TRANSFER from `HouseInterior` to `CharacterInventory`; the provenance is retired in
  the same transaction.
- One-item TRANSFER between tiles of the same house; the provenance is kept.
- Stack merge, split, container moves and cross-house moves follow the existing DUR-03 shapes when
  admitted. They are not admitted by this decision.

### 3.5 Closed until ownership

- The player-facing writers are admitted only for a house that has an owner and an ACL grant to
  the acting Character, under EXP-HOUSES-01 §16. Neither exists yet.
- Until then the family, provenance, fence and shapes exist and are tested with fixtures, but
  every player command refuses them. House tiles keep ADR-0021 rule a (Ground, retired at reset).

### 3.6 World reset

- `WorldReset` retires only Ground roots and their container entries (DUR-03 §39.3). It never
  touches `HouseInterior` or containers under it.
- An owned house's tiles are served by its interior runtime, not by the channel overlay. Items
  dropped there become `HouseInterior`, not Ground.
- A new map revision that removes a house tile holding items needs the EXP-HOUSES-01 §14.7
  content fence and evacuation before activation. The reset refuses to activate while such an
  item exists.

## 4. Delivery

| Child | Scope | Depends on |
|---|---|---|
| HOUSE-CUSTODY-1 | Migration, guards, provenance, house scope kind, transfer shapes (closed to players), reset exemption, tests | this decision; ADR-0021 accepted |
| House ownership, ACL, interior runtime | EXP-HOUSES-01 children; they open the §3.5 gate | HOUSE-CUSTODY-1 |

## 5. Rejected options

- **House items as Ground with an exemption flag.** Ground is channel-scoped, while house items
  are World-scoped (EXP-HOUSES-01 §4.1). A flag would create copies per channel.
- **A generic "domain custody" row.** DUR-03 §5.3 forbids it.
- **Waiting with the family until ownership exists.** The owner asked for it now, and the reset
  exemption needs the family to exist.

## 6. Decision test

- **Must decide now:** YES, by owner request. It also fixes the ADR-0021 reset boundary before
  any house can be owned.
- **Minimum sufficient:** one location family, one provenance record, one scope kind and three
  transfer shapes, closed to players.
- **Superseding evidence:** an EXP-HOUSES-01 runtime decision that needs a different fence, or a
  storage budget that needs capacity rows.
- **Deliberately not decided:** ownership, ACL, rent, auctions, storage budgets and the
  interior runtime.

## 7. Before-freeze checklist

1. **Contract amendments:** DUR-03 §5.2 and ADR-0021 §4.4 are amended in this PR.
2. **Serialization:** one live house-scope generation, plus `character_root` locks.
3. **Restart:** the durable location and provenance are enough. The interior runtime rebuilds
   from them.
4. **Typed references:** `house_ref` is `{family, key, revision}`, and the provenance names its
   transaction.
5. **Wire:** none. House views belong to MAP-WIRE-1 and the house runtime.
6. **Split work:** every shape is one transaction, with the provenance written in the same
   transaction.
