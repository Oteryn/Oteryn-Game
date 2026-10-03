# MOVE-RL-11 visibility decision

- Decision: `MOVE-RL-11-VISIBILITY-V1`
- Status: **CANDIDATE with owner decisions D84-D87 (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Profile: `Oteryn Reference` (Global Tibia at 2026-09-27, D33)
- Answers: the flagged dependency in the VSL §19 Combat rows decision (§4.4): with `MOVE-RL-11` = 1,
  a client sees only its own player, which blocks client observation of creatures, corpses and
  other players (Combat child E, the GAME-AI slice D53, spell targeting)
- Re-decides: `MOVE-RL-11` (#139 `FIRST-CONTROLLED-STEP-LIMITS-V1`, owner acceptance 5853424970)
  and the companion rows `MOVE-RL-08`, `-09`, `-10` from the wave-2 limits packet
- Owner decisions posted: #162 comment 5875312123
- Admission baseline: `main@8e2e474a3d82a6bfbc21be409f444cfd64d42961`
- Runtime, registry, protocol-registry and production authority: **NONE**. Registration and the
  protocol schema belong to the implementing allocations.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

`WORLD_SPATIAL_VISIBILITY` carries exactly one actor, the observer's own
(`world_spatial_v1.proto`, 64 B payload; `MOVE-RL-11` = 1). VSL-MOVE-01 §15 leaves the view
rectangle, floor rules and delta packing undecided, and §17 invents no numbers. What does a client
see, how far, and how many entities at most?

## 2. Owner decisions

Owner direction: "zrób jak global, ale tu chcę zaznaczyć, że chciałbym mieć możliwość testowania
innych rozmiarów".

| # | Decision |
|---|---|
| D84 | Area as in Global: 18 × 14 tiles around the observer (the 15 × 11 view plus a movement margin) in the Reference profile. The size is a server setting within a bounded range, so other sizes can be tested. Every session in one Channel uses the same value. |
| D85 | Visible objects as in Global: creatures, other players, corpses and ground items. |
| D86 | Floors as in Global: above ground (floor ≤ 7) the observer sees floors 7 down to 0; underground it sees its floor ± 2. |
| D87 | Entity ceiling: 256 per snapshot or delta, nearest first beyond it, own actor always included. Global states no cap; this value is the architect's under the "as in Global" direction. Amended by D222 (#162 5911768800), pending on acceptance of ITEM-MOVE-WIRE-1: actors rank before items. |

## 3. Facts

**PROVEN** (main `8e2e474a3d82a6bfbc21be409f444cfd64d42961`)

- Registry: `MOVE-RL-11` = 1 ("Semantic entities in one WORLD_SPATIAL_VISIBILITY snapshot or
  delta", `CAPACITY_EXCEEDED`, client-visible), enforced structurally by the schema's single
  non-repeated `ActorPositionV1`; `MOVE-RL-02` = 1 and `MOVE-RL-03` = 1 are the only other
  registered Movement rows.
- The wave-2 packet (`OTERYN_GAME_WAVE2_RESOURCE_LIMITS_DECISION_PACKET_2026-08-24.md`) defines
  `MOVE-RL-08` (enter/leave/update entities per delta), `-09` (query candidates), `-10` (query
  results) and `-11` (snapshot entities) with the dispositions "reject/degrade/resync", "bounded
  snapshot/resync/degradation" and "never unbounded allocation". `MOVE-RL-12` to `-15` inherit the
  FND-02 envelope rows.
- Protocol registry: domain 1 `WORLD_SPATIAL_VISIBILITY`, delta and snapshot type 1, 64 B each;
  domain 2 `WORLD_OBJECT_OVERLAY` exists for object state (USE-WIRE-V1).
- FND-02: `ORDINARY-REPEATED-ENTRIES` 4,096; `STATE-DELTA-PAYLOAD-BYTES` 262,144;
  `SNAPSHOT-CHUNK-BYTES` 524,288.
- Scope counts: up to 64 creatures per scope (D57), 64 corpses per scope (D78,
  `COMBAT01-CORPSES-PER-SCOPE`); no row bounds players per Channel.
- The first production content profile admits one distinct floor
  (`OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md`).
- The native client UI addendum leaves the field-of-view policy undecided and asks for server
  relevance and fairness evidence.

**DERIVED from Global protocol behaviour (OTS evidence, not in the tibia.com manual):** the 15 × 11
view, the 18 × 14 map area and the floor rule. The manual states only a character-centred view
(§interface 3.1).

## 4. Decision

### 4.1 Interest area (D84, D86)

- The observer's interest area is a rectangle of `width × height` tiles that includes the
  observer's tile, placed for every accepted size by one formula:
  `west = floor((width − 1) / 2)`, `east = width − 1 − west`,
  `north = floor((height − 1) / 2)`, `south = height − 1 − north`.
  An even size therefore extends one tile further east and south, as in Global: the Reference
  18 × 14 gives 8 west, 9 east, 6 north and 7 south; 15 × 11 gives 7, 7, 5 and 5.
- Floors: above ground (observer floor ≤ 7) floors 7 down to 0; underground floors observer ± 2,
  clamped to the map's floors. Other floors are offset as Global offsets them (one tile diagonally
  per floor of distance).
- Settings: `width` 18 and `height` 14 in the Reference profile. A server may set other values for
  testing within `width` 15..36 and `height` 11..28. The value is fixed per Channel for its
  lifetime and applies to every session in it, so no player sees farther than another. A Reference
  production Channel uses 18 × 14.
- Visibility changes only from committed server state (VSL-MOVE-01 §13). Line of sight,
  invisibility and spectators stay out of scope.

### 4.2 Visible entities (D85)

- Creatures and other players: identity (runtime actor id and generation), kind, position,
  direction, appearance reference and health percentage.
- Corpses and ground items: identity, position and item definition reference; a stack shows its
  quantity.
- All visible entities travel in the one new `WORLD_SPATIAL_VISIBILITY` revision (§4.5), so one
  ceiling applies. They are not carried in `WORLD_OBJECT_OVERLAY`, whose registered
  `WOBJ-RL-03` (486 entries) serves object state and stays unchanged.
- The observer's own actor is always included.
- **Amendment (pending on acceptance of NPC-BEHAVIOUR-0; `reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md` §3.2).** NPCs are visible actors of entity kind 5 `Npc`,
  added to capability 6's schema before it is offered; they count in the D87 ceiling.

### 4.3 Ceiling and degradation (D87)

- At most 256 entities per snapshot or delta, including the own actor.
- **Canonical order:** floor distance, then Chebyshev distance on the observer's plane, then
  entity identity (byte order). The interest index enumerates candidates in this order, ring by
  ring outwards from the observer, so any cutoff keeps the nearest.
- If more entities are in the area, the snapshot carries the first 256 in canonical order. The rest enter as others leave. This is the
  packet's "degrade" disposition; it never allocates beyond the ceiling and never fails the
  session.
- A delta carries at most 256 enter, leave or update entries; a larger change is sent as a new
  snapshot instead (the packet's "resync" disposition).

**Amendment (ITEM-MOVE-WIRE-1, 2026-09-30; owner decision D222, #162 5911768800), pending on
acceptance of ITEM-MOVE-WIRE-1.** The canonical order ranks actors (players and
creatures) before items; within each group it is floor distance, Chebyshev distance, then entity
identity, as above. So dropped items can never push an actor out of a snapshot. The ceiling and the
degrade and resync dispositions are unchanged
(`OTERYN_GAME_ITEM_MOVE_WIRE1_EQUIP_AND_DROP_DECISION_2026-09-30.md` §7.3).
**Amendment (pending on acceptance of NPC-BEHAVIOUR-0; `reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md` §3.2).** NPCs rank with players and creatures as actors.
VIS-3 does not offer capability 6 before NPC-VIS-1 adds kind 5 to `world_spatial_v1.proto`
`EntityKind`.
**Amendment (pending on acceptance of RANGED-0; `reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md` §6.1.2).** Items of one
tile are ordered by their `ground_ordinal`, top last, so the client's top item is the server's.
Selection and emission are separate orders. **Selection** (which entities survive the 256 cutoff):
actors first as above; items by floor distance, Chebyshev distance, then the canonical tile key
`(floor, y, x)` in native floor coordinates (`OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1` §11) so
tiles at equal distance have a total order, then within one tile from the top down (descending
`ground_ordinal`, then entity identity for legacy rows at ordinal 0). A cutoff inside a tile
therefore drops its bottom items, never its top item. **Emission** (the order the client stacks a
tile's selected items): ascending `ground_ordinal`, top last. `WorldSpatialEntityV1` carries no
ordinal, so a delta may add an item to a tile the client already shows only when it lands above
every shown item of that tile (a new top); any other change of a shown tile's selected items (a
cutoff moving inside the tile, a lower item entering) is sent as a new snapshot (the "resync"
disposition), which emits every tile in order.

### 4.4 Resource rows

Registered by the implementing allocation under the registry single-writer lease, with max and
max+1 tests.

| Row | Value | Note |
|---|---|---|
| `MOVE-RL-11` | **256** entities per snapshot or delta | re-decided from 1; beyond it, nearest first (§4.3) |
| `MOVE-RL-08` | 256 enter/leave/update entries per delta | beyond it, a new snapshot |
| `MOVE-RL-09` | 1,024 candidates per visibility query | enumerated in the §4.3 canonical order; the query stops at 256 results or 1,024 examined candidates, whichever comes first, so the selection is the same on every replay |
| `MOVE-RL-10` | 256 results per query | equals `MOVE-RL-11` |
| `MOVE-VIEW-WIDTH` | default 18, range 15..36 tiles | new, configurable (D84) |
| `MOVE-VIEW-HEIGHT` | default 14, range 11..28 tiles | new, configurable (D84) |
| `MOVE-VIEW-FLOORS` | 8 above ground, 5 underground | new (D86) |
| Payload bytes | at most 128 B per entity entry, so ≤ 33,792 B per snapshot or delta payload (256 × 128 B + 1,024 B header) | within `FND02-STATE-DELTA-PAYLOAD-BYTES` 262,144; the protocol registry's 64 B for domain 1 changes with the new schema revision |

### 4.5 Protocol

- The new schema revision (a new snapshot and delta type of domain 1) carries a repeated entity
  entry (§4.2) under the protocol registry lease and the protocol lane, and fails closed on
  unknown fields as today.
- It is gated by a new optional capability, `WORLD_SPATIAL_ENTITIES`, under the FND-02 capability
  model (FND-02 §4 and §9): an older same-major client cannot read the new payload, so the server
  sends the new types only to a session whose negotiation selected the capability. Other sessions
  keep receiving `world_spatial_v1` with their own actor only.
- VIS-2 adds cross-version fixtures: an old client against a new server receives only v1; a new
  client against an old server falls back to v1; a selected capability the server does not know
  fails closed.

## 5. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| VIS-1 | Server interest set: area, floors, ceiling and ordering, settings; rows registered | this decision |
| VIS-2 | The new `WORLD_SPATIAL` schema revision, the `WORLD_SPATIAL_ENTITIES` capability, codec, client decode and cross-version fixtures; protocol registry entries | VIS-1; protocol lane |
| E (Combat) and AI | Creature appear, move, disappear and corpse observation through VIS-2 | VIS-2 |

## 6. Rejected options

- **Keep one entity.** Nobody would see creatures, corpses or other players.
- **A per-client view size.** A larger view would be an unfair advantage; the size is per Channel.
- **Fail the snapshot when the ceiling is reached.** A crowd would disconnect players; nearest
  first keeps play going.

## 7. Decision test

- **Must decide now:** YES. Combat E, the AI slice's client observation and spell targeting wait
  on it.
- **Minimum sufficient:** one rectangle, the Global floor rule, one ceiling with a deterministic
  order, and a configurable size for tests as the owner asked.
- **Superseding evidence:** measured dense-scene cost; players-per-Channel limits; multi-floor
  content; the native client field-of-view decision.
- **Deliberately not decided:** line of sight, invisibility, spectators, delta packing details,
  the client viewport rendering.

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "VSL §19 Combat rows decision §4.4 (MOVE-RL-11 flagged dependency)"
owner_decisions: [D84, D85, D86, D87]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md
resource_values_changed: true   # registered by VIS-1
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # VIS-1 may be allocated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (visibility bounds, fairness, protocol revision)"
implementation_lanes: [VIS-1, VIS-2]
required_revalidation:
  - "VIS-1: the offset formula at 18 × 14, 15 × 11 and 36 × 28; the floor rule above and below ground; 256 entities accepted and the 257th farthest left out by the §4.3 order; own actor always present; settings outside 15..36 × 11..28 rejected at startup; one value per Channel"
  - "VIS-1: a delta with more than 256 changes becomes a snapshot; with more than 1,024 candidates the selection is the canonical nearest and identical on replay, whatever the index insertion order"
  - "VIS-2: the new schema round-trips 256 maximum-size entries within 33,792 B; unknown fields fail closed; only sessions that selected WORLD_SPATIAL_ENTITIES receive it; old and new peers interoperate through v1"
remaining_unknowns:
  - players per Channel
  - dense-scene cost evidence
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates VIS-1."
```
