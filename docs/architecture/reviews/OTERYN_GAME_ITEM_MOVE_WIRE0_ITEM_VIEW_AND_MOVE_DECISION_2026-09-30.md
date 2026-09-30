# ITEM-MOVE-WIRE-0 Item views and item move

- Decision: `ITEM-MOVE-WIRE0-ITEM-VIEW-AND-MOVE-V1`
- Status: **CANDIDATE**. Owner answers 1a and 2a given (D212, §9). Acceptance needs exact-head
  validation, independent review (protocol and persistence) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the quest blocker map (#162 5909219797, I1) as corrected by the architect ruling
  5909483942: moving an item has no command
- Builds on: FND-02 §13.3 and §15, USE-WIRE-V1 (command type 2), the MOVE-RL-11 visibility
  decision (D84-D87), the B3 decision (D80-D83), DUR-03 §39.3 (the B3 and corpse amendments),
  D133/D134, migrations `0011` and `0014`, ADR-0021 §4.4
- Amends: USE-WIRE-V1 (reserved field 2 becomes the item target, §4.3); the MOVE-RL-11 visibility
  decision §4.2 (one added wire field, the item handle, §4.1). ITEM-VIEW-1 updates the
  `world_object_v1.proto` comment and adds a back-pointer to the MOVE-RL-11 decision.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| ITEM-VIEW-1 | impl, protocol review | domains 9 and 11 (§4.2), the item handle field and the session handle table (§4.1), the USE item target (§4.3), codecs, limits, client views | VIS-2 and the `WORLD_SPATIAL_ENTITIES` capability (MOVE-RL-11 §5), which carry corpses in domain 1 |
| ITEM-MOVE-1 | hard, persistence review | command type 9 (§5): handle resolution, reach, and the existing corpse-entry TRANSFER through `item_transfer.rs` | ITEM-VIEW-1; the combat loot MINT wired into the runtime (combat lane child, MOVE-RL-11 child E) |

Every write that changes a view emits its delta after the durable commit: the loot TRANSFER, the
reward chest MINT into the backpack (`chest_use.rs`), the corpse and loot MINT, and corpse decay
(`item_decay_retire.rs`), which also closes an open corpse.

## 1. Question

How does a player see their backpack and a corpse's contents, and how do they loot into the
backpack?

## 2. Facts

**PROVEN**

- The registry has command type 2 `USE_INTENT` (USE-WIRE-V1, owner acceptance 5864914163). Its
  only target is a world object by placement and expected revision. `world_object_v1.proto`
  reserves fields 2, 3 and 4, in the order item-in-container, creature and use-with.
- D85 (MOVE-RL-11 §4.2) puts corpses and ground items into domain 1 `WORLD_SPATIAL_VISIBILITY`
  (identity, position, item definition, quantity), under D87's 256-entity ceiling, in the D84/D86
  area. The identity is a channel-wide 16-byte `EntityIdentity` that keys the interest index and
  breaks ties in D87's canonical order (`movement/interest.rs`). VIS-1 (merged, #1153) builds the
  interest set; VIS-2 carries entities on the wire with the `WORLD_SPATIAL_ENTITIES` capability.
- No domain shows the character's backpack or a corpse's contents.
- Admitted DUR-03 TRANSFER shapes: Ground to the main backpack slot or its direct entries
  (`0011`, B3 D80-D83), and a corpse container entry to the same destinations (`0014`). The
  writer (`item_transfer.rs`) takes the FND-02 CommandRef as its cause, judges D133 on the
  database clock and enforces D134. It does not check reach, and the caller picks the destination
  (container slot or main backpack entry). No production code calls it.
- The corpse and loot MINT has no production caller yet (`combat.rs`).
- FND-02 §13.3: a reserved CommandId survives a reconnect of the same GameSession and may run after
  it. FND-02 §15: a domain revision never wraps or is reused in its scope. Resume state carries the
  spatial and overlay revisions today (`resume.rs`).
- Protocol numbers in use or reserved: command types 1-8, state domains 1-8, capabilities 1-3
  (#162 5907282001, 5909366267).

**DERIVED**

- In Tibia, a corpse is opened with USE and its contents appear in a container window; items are
  dragged into the backpack (official manual, controls and combat notes).
- OTClient (`opentibiabr/otclient` at `eb262531`, `OTS_HYPOTHESIS_ONLY`, client-side reference
  only) sends one move packet for every move: source position, item id, stack position,
  destination position and count (`src/client/protocolgamesend.cpp:350-363`,
  `src/client/game.cpp:808-818`). A container slot is the virtual position
  `{0xFFFF, container_id | 0x40, slot}` and an inventory slot `{0xFFFF, slot, 0}`
  (`src/client/container.h:37`, `src/client/thing.cpp:86`). The native client does not use this
  addressing; it confirms the shape of the Tibia behaviour this decision narrows.

## 3. Scope of the first slice (owner answer 2a)

- **In:** see the own main backpack; open a corpse within reach; loot a corpse entry into the main
  backpack.
- **Later, with their prerequisites:** picking up a Ground item (nothing produces durable Ground
  items before a drop shape exists) and a map-authored item (MAP-LOAD-1, MAP-OVERLAY-1 and
  MAP-WIRE-1). Both become new sources of command 9 without a new command.
- **Out, refused as `NOT_SUPPORTED`:** dropping, moving within the backpack, equipping, nested
  bags, splitting a stack.

## 4. Views and handles (ITEM-VIEW-1)

| Kind | Id | Name | Content |
|---|---|---|---|
| capability | 4 | `ITEM_VIEW_MOVE_V1` | gates domains 9 and 11, the USE item target and command type 9; requires `WORLD_SPATIAL_ENTITIES` |
| state domain | 9 | `CHARACTER_INVENTORY` | the main backpack slot and its direct entries in display order: handle, item definition, count, sub-type |
| state domain | 11 | `OPEN_CONTAINER` | the one open corpse: its handle and its entries (handle, item definition, count, sub-type) |

State domain 10 (`GROUND_ITEMS` in D212) is released; ground items stay in domain 1 (D85). The
next free numbers are command type 10, state domain 10 and capability 5.

### 4.1 Item handles

- Every item the client sees carries a handle, a `uint64` assigned by the server: items in domains
  9 and 11, and corpses and ground items in domain 1 as one added wire field of the D85 item entry.
  D85's channel-wide `EntityIdentity` stays the internal identity for the interest index and for
  D87's canonical order; the handle does not replace it.
- Handles are monotonic per `GameSessionId` and never reused, so a command reserved before a
  reconnect cannot hit a different item after it. A handle whose item is gone or no longer
  visible to this session resolves to `STALE`.
- **Bounded table.** A handle stays live while its item is in at least one of this session's views;
  when it leaves every view the entry is dropped, and an item that returns gets a new handle. Live
  handles per session are limited by `ITEMV0-RL-03`.
- **Continuity.** The handle counter and the high-water revisions of domains 9 and 11 are part of
  the session's resume state next to the spatial and overlay revisions (`resume.rs`), and move with
  it on reconnect and channel transfer.
- A handle never exposes an ItemInstanceId, a placement key or a database row. Map-authored base
  items get their handle with MAP-WIRE-1, which keeps ADR-0021's rules (`placement_key` as the wire
  placement identity, `content_generation` matched to the active bundle).

### 4.2 Revisions

- Domains 9 and 11 are owned by the channel runtime. Their revision streams are monotonic per
  `GameSessionId` and never reused (FND-02 §15). Every admission, reconnect and channel transfer
  sends a new snapshot at a revision above any the session has seen. A delta is emitted only after
  the durable commit that changed the view.
- Commands carry no view revision: the handle check replaces a whole-domain check, and the FND-02
  envelope rules are unchanged.

### 4.3 Opening a corpse (USE-WIRE-V1 amendment)

- USE reserved field 2 becomes `item = 2`, `ItemTargetV1 {handle}`, for any handle-bearing item.
  In this slice only a corpse is accepted; any other item is `NOTHING_TO_USE`. Fields 3 and 4 stay
  reserved.
- Opening a corpse is a non-durable view action: it opens domain 11 for that corpse and writes
  nothing. It is not a GAME-INTERACTION transition and has no occurrence of its own.
- Reach: Chebyshev distance at most 1, same floor. Anyone in reach may open a corpse; D133 and
  D134 are enforced when an item is taken (§5), on the database clock.
- USE dispositions: `COMMITTED` (opened), `TOO_FAR`, `STALE_STATE` (handle gone),
  `NOTHING_TO_USE` (not a corpse). No new disposition.
- **Closing.** At most one container is open. The server closes it when the character leaves
  reach, changes floor or is teleported, dies, logs out or transfers channel, when the corpse
  decays, or when another corpse is opened. After a reconnect the snapshot includes the corpse only
  if it is still in reach; otherwise domain 11 is empty. A client may hide the window locally at
  any time.

### 4.4 Limits

Measured and registered by ITEM-VIEW-1: `ITEMV0-RL-01` entries in domain 9 (at least 21: the slot
and 20 entries); `ITEMV0-RL-02` entries in domain 11 (the corpse container capacity);
`ITEMV0-RL-03` live handles per session; snapshot and delta bytes within the FND-02 limits. Items
in domain 1 stay under D87's 256-entity ceiling.

## 5. Move (ITEM-MOVE-1)

| Kind | Id | Name | Content |
|---|---|---|---|
| command type | 9 | `ITEM_MOVE_INTENT` | `{source: handle, destination: MAIN_BACKPACK}` |

- **Source.** In this slice, a handle of an entry of the open corpse (domain 11). Any other source
  is `NOT_SUPPORTED`.
- **Reach.** The runtime checks that the corpse is within Chebyshev distance 1 on the same floor
  before the TRANSFER; the writer does not check reach.
- **Destination.** Only `MAIN_BACKPACK`. The runtime picks the writer's destination with the B3
  rule (D80, §4.1 there): the equipment container slot only if it is empty and the item is an empty
  container with a complete container-slot equip pattern; otherwise the main backpack, where the
  writer merges, tops up or adds a new direct entry (D83).
- **Whole item only.** The whole stack moves.
- **Value.** One admitted DUR-03 TRANSFER (`0014` shape) through the existing writer, with the
  command's CommandRef as its cause. No new DUR-03 shape.
- **Results** (at most 4 bytes), mapping every writer refusal (`ItemTransferRefusal`):

| Result | When |
|---|---|
| `MOVED` | committed |
| `STALE` | handle gone or not visible; `SourceNotOnGround` |
| `TOO_FAR` | out of reach |
| `NO_BACKPACK` | `NoMainBackpack` |
| `NO_ROOM` | `MainBackpackFull` |
| `NOT_OWNER` | `CorpseExclusiveWindow` (D133) |
| `NOT_PICKUPABLE` | `CorpseNotPickupable` (D134), or an item that cannot be picked up |
| `NOT_SUPPORTED` | a source or destination outside §3; `ContainerNotEmpty` |
| `REJECTED` | `DefinitionMismatch`, `UnknownStackClass`, `UnsupportedStackMaximum`, `QuantityAboveStackMaximum`, `UnsupportedContainerCapacity`, and any authority or availability failure |

`NotContainerSlotEquippable` and `ContainerSlotOccupied` cannot occur, because the runtime picks
the container slot only when both conditions hold.

## 6. Rejected options

- **A separate ground-item domain.** D85 already puts corpses and ground items in domain 1 under
  one ceiling.
- **The handle as D85's identity.** The identity is channel-wide and orders the interest set; a
  per-session handle would make the order depend on the observer.
- **Addressing items by position and stack index, as Tibia, Canary and Crystal do.** Stack
  indices shift with every change and need their own staleness rule; handles do it once. The
  native client does not speak the Tibia protocol.
- **Handles and revisions reset per connection.** A command reserved before a reconnect could hit
  another item.
- **Gating corpse opening by D133 on the runtime clock.** It would disagree with the writer's
  database clock; the writer refuses the take instead.

## 7. Decision test

- **Must decide now:** YES. Looting is the first item action of a playable game, and the writer
  exists without a command.
- **Minimum sufficient:** one command, two domains, one USE target, one handle field, one
  capability, no new DUR-03 shape.
- **Superseding evidence:** a DUR-03 shape for dropping, reordering or equipping (then command 9
  gains those destinations).
- **Deliberately not decided:** ground and map-item pick-up (prerequisites in §3), drop, reorder,
  equip, split, nested bags, depot, player trade, and the client drag-and-drop UI.

## 8. Before-freeze checklist

1. **Contract amendments:** USE-WIRE-V1 (field 2) and D85 (one handle field) are amended here.
2. **Serialization:** each move is one existing DUR-03 TRANSFER with its fence.
3. **Restart:** views are runtime-local with GameSession-monotonic revisions and handles carried
   in resume state; items are durable.
4. **Typed references:** handles are per GameSession, bounded and never reused; the server
   resolves them.
5. **Wire:** §4 and §5, capability-gated.
6. **Split work:** one move per command.

## 9. Owner questions and answers

**Q1. Accept this wire contract (views, handles, pick-up command, corpse opening)?**
a) Yes, capability-gated (recommended); b) no, another addressing model first.

**Q2. What should the first slice let a player do with items?** a) Pick up and loot into the
backpack only (recommended); b) also drop items on the ground now.

**Owner answers (2026-09-30, given directly in the architect session; #162 5909624320; D212):**
"1a 2 a", after the architect's comparison with Canary, Crystal and Tibia Global.

### 9.1 Architect adjustments after the answers

The self-reviews found three places where the draft the owner accepted conflicted with rules that
already bind it. The architect corrected them without a new owner question: each follows an
earlier owner decision or a foundation rule, and none widens what the owner accepted.

1. **Ground items stay in domain 1**, as the owner's D85 already decided. The proposed domain 10
   `GROUND_ITEMS` is released; `OPEN_CONTAINER` keeps domain 11.
2. **Handles last for the whole game session**, not one connection, so a command sent just before
   a reconnect cannot hit another item (FND-02 §13.3 and §15).
3. **The first slice is corpse looting.** Ground pick-up waits until something can put player items
   on the ground and the map is loaded (§3); it needs no new command then. This is the narrow end of
   answer 2a, not a change of it.
