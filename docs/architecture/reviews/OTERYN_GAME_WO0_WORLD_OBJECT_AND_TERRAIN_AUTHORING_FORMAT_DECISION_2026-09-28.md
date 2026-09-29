# WO-0 WorldObject and Terrain authoring format decision

- Decision: `WO0-WORLD-OBJECT-TERRAIN-FORMAT-V1`
- Status: **CANDIDATE with owner decisions D93-D94 (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- **Amended** by A12 (`OTERYN_GAME_A12_ITEM_IDENTITY_TIBIA_ID_DECISION_2026-09-29.md` §4.6): D93 family keys use the Tibia numbering,
  `oteryn:world-object.tibia.i<id>` and `oteryn:terrain.tibia.i<id>`.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Routed as architecture package item A7 (#162 comments 5876505672, 5876526764)
- Owner decisions posted: #162 comment 5876870559
- Admission baseline: `main@9961e4d4afc90d06a92ad7af59cfdbe6857e99b3`
- Identity minting, schema, runtime, registry and production authority: **NONE**. WO-1 (tooling)
  and WO-2 (population, identity minting) each need their own allocation; WO-2 needs independent
  identity review.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

The Item converter routes 25,846 source ids away from Item (ground, borders, walls, doors,
furniture, corpses). `content/world/terrain/` and `content/world/objects/` are
`READY_UNPOPULATED`, and no format or identity rule exists for either family. What identity do
these ids get, which facts belong to each family, and how do they relate to the runtime
`LocalObject` overlay?

## 2. Owner decisions

| # | Decision | Owner choice (2026-09-28) |
|---|---|---|
| D93 | Family-scoped keys with the same frozen CW2-B1 sequence number: `oteryn:terrain.registry.iNNNNNNNN` and `oteryn:world-object.registry.iNNNNNNNN`. No new allocator, no renumbering; the Item registry entry stays as a non-materializable pointer. | "Ten sam numer, własna rodzina" |
| D94 | The first catalog covers every recognised routed id (about 21.4k): ground, borders, walls, world objects and corpses; not the 4,351 empty appearance slots and 105 ids without a client appearance. | "Wszystkie rozpoznane" |

## 3. Facts

**PROVEN** (main `9961e4d4afc90d06a92ad7af59cfdbe6857e99b3`)

- CW2-B1 (`apps/game-server/src/content/cw2_b1_import.rs`): 38,157 Item keys
  `oteryn:item.registry.iNNNNNNNN` over every Crystal `items.xml` id, allocated by ascending
  source id, frozen by the source generation and allocator epoch (task #504: "future reimports
  cannot renumber or remap it"); the mapping is explicit in `imports/crystalserver/bindings/items.json`.
  Routed ids, ground included, are today Item records with `materializable: false`.
- Converter routing (`tools/content-schema/item-authoring/samples/population-crystal-ff7ede5.json`):
  WorldObject `immovable_unclassified` 8,450, `appearance_placeholder_slot` 4,351, `corpse` 3,344,
  `primarytype_world_object` 981, `no_client_appearance` 105, `corpse_decoration` 14; Terrain
  `primarytype_world_object` 5,138, `ground_or_border` 3,443; Fluid 20.
- Field ownership (`tools/content-schema/item-authoring/crystal-field-dispositions.json`; Item
  formal schema §4): 33 `WORLD_OBJECT` fields (blocking, projectile and sight blocking, avoid,
  floor change, hang, hook, rotate, height and elevation, fluid source, liquid pool, bed parts and
  sleepers, corpse flags, level door, walk stack, replaceable, ground speed and bank flags) and 3
  `TERRAIN` fields (automap colour, automap flag, field). `type` values door, bed, ladder,
  teleport, depot, mailbox, trash holder, carpet, reward chest, magic field and dummy belong to
  WorldObject, Terrain or Interaction.
- Runtime: `DefinitionFamily` has `Terrain` and `LocalObject` (`reference_playable.rs`);
  `ProjectReferenceRecord::LocalObject { identity, client_projection, states }` carries per-state
  collision; placements carry `local_object_initial_state`, attributes and `revert_after_ms`
  (owners proposal §4, §9). Terrain uses the generic record. No placements exist yet
  (`content/world/worlds/world.json`).
- Source policy: Reference is Global at 2026-09-27; tibia.com, then TibiaWiki; Canary and Crystal
  are `OTS_HYPOTHESIS_ONLY`. Map and item metadata may be recorded as reference data;
  original binaries are not redistributed (`LICENSE-ASSETS.md`).

## 4. Decision

### 4.1 Identity (D93)

- **Rule.** A routed id keeps its CW2-B1 sequence number `NNNNNNNN`. Its definition key is
  `oteryn:terrain.registry.iNNNNNNNN` for a Terrain route and
  `oteryn:world-object.registry.iNNNNNNNN` for a WorldObject route. The mapping is a pure function
  of the frozen Item allocation and the route; it mints no new number and cannot collide.
- **Item pointer.** The Item record `oteryn:item.registry.iNNNNNNNN` stays, non-materializable,
  with `routed_to` as a typed, versioned definition reference `{family, key, revision}` (Item
  schema §2) to the Terrain or WorldObject definition. It is never deleted or reused (frozen
  allocator). The Item formal schema and `ProjectReferenceRecord::Item` do not admit `routed_to`
  today; WO-1 amends both (§5).
- **Route is identity-relevant.** The route is fixed by the converter rule at the frozen source
  generation. A later route change is a new decision and never moves an existing key between
  families.
- **Exclusions (D94).** Empty appearance slots and ids without a client appearance get no family
  key; the 20 Fluid ids stay with Item fluid.

### 4.2 Terrain record

`content/world/terrain/` holds one record per Terrain key:

| Field | Source | Notes |
|---|---|---|
| `identity` | D93 key and revision | |
| `kind` | route: `ground`, `border`, `wall` or `field` | from the converter rule |
| `walkable` | inverse of the unpass flag | KNOWN or UNKNOWN |
| `ground_speed` | the bank waypoints value | ground only |
| `blocks_projectile`, `blocks_sight` | blockprojectile, unsight | |
| `floor_change` | floorchange | |
| `automap` | automap colour and flag | |
| `field` | the field type (fire, energy, poison), as a typed reference | effect behaviour belongs to Ability or Interaction |
| `client_projection` | the appearance reference | facts only; no asset bytes |
| `provenance` | source and revision per field | |

### 4.3 WorldObject record

`content/world/objects/` holds one record per WorldObject key:

| Field | Source | Notes |
|---|---|---|
| `identity` | D93 key and revision | |
| `kind` | route and `type`: `object`, `door`, `ladder`, `bed`, `container_fixture` (depot, mailbox, trash holder, reward chest), `teleport`, `corpse`, `decoration` | typed, closed set |
| `collision` | unpass, avoid, blockprojectile, unsight | |
| `movable` | unmove | always false for this family unless proven |
| `placement` | hang, hook direction, rotate and rotate target, height and elevation, walk stack | |
| `floor_change` | floorchange | ladders and holes |
| `bed` | bed parts, sleepers, transforms, partner direction | the sleep behaviour belongs to Interaction |
| `fluid_source` | fluidsource, liquid pool | the fluid type is a typed Item fluid reference |
| `corpse` | corpse and player-corpse flags | decay target, duration and container capacity stay Item-owned facts (`ITEM_TYPED`, `/item/temporal/*`, `/item/container/capacity`) on the routed Item record, read through its `routed_to` link; no second authority. The loot and death link belong to Combat and DUR-03; a corpse definition holds no loot |
| `door` | level door, house-guest use | the open/close behaviour is the `LocalObject` state machine |
| `client_projection` | the appearance reference | facts only |
| `provenance` | source and revision per field | |

Fields keep the `{"state": "KNOWN", "value": …}` / `{"state": "UNKNOWN"}` wrapper already used by
Item definitions; an unknown fact is never defaulted.

### 4.4 Relation to the runtime `LocalObject` overlay

- A **WorldObject** record is static catalog content: what an object is.
- A **LocalObject** record (existing runtime family) is the stateful placement behaviour: its
  states, per-state collision, transitions and attributes (owners proposal §4, §9).
- **Presentation link.** Each LocalObject state gains an optional `presentation` field, a typed,
  revision-bound reference `{family: WorldObject, key, revision}` to the WorldObject definition
  that presents it (for example a closed and an open door are two WorldObject keys). Today
  `LocalObjectStateDefinition` and `ProjectReferenceRecord::LocalObject` carry no such field; the
  WO-3 child adds it to both, validated fail-closed (the referenced definition must exist at the
  bound revision). Absent, the state keeps today's behaviour; state keys, transitions and the
  owners proposal's runtime binding are otherwise unchanged.
- A placement references a WorldObject or Terrain key directly when it has no state machine, and a
  LocalObject when it has one. The owners proposal's runtime binding is unchanged.
- Anything a player can pick up is an Item (owners proposal §4 value boundary). A WorldObject is
  never materialized as an Item by this decision.

### 4.5 Sources

Facts come from the pinned `appearances.dat` flags and `items.xml` at the frozen source
generation, recorded as reference data with provenance (tibia.com and TibiaWiki decide where they
state a value; Canary and Crystal stay hypotheses). No original asset bytes are committed.

## 5. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| WO-1 | Schema, validator and census for `content/world/terrain/` and `content/world/objects/`, reusing the converter routing; the Item formal schema and `ProjectReferenceRecord::Item` amendment admitting the typed `routed_to` reference; no identity | this decision |
| WO-2 | Population: family keys by the D93 rule, facts with provenance, Item `routed_to` pointers; independent identity review | WO-1; the item re-pin; B1b for donor ids |
| WO-3 | The optional typed `presentation` reference on LocalObject states (`LocalObjectStateDefinition`, `ProjectReferenceRecord::LocalObject`), validated fail-closed | WO-2 |
| Later | Runtime `WorldObject` family in code, map placements, corpse runtime link | their owners |

## 6. Rejected options

- **Keep Item keys for world objects.** The owner chose family-scoped keys (D93).
- **A new allocator.** It would add a second frozen sequence for ids that already have one.
- **Catalog on demand.** The full world map references all of them (D94).
- **Merge WorldObject into LocalObject.** LocalObject is the stateful runtime overlay; the static
  catalog is shared by many placements and states.

## 7. Decision test

- **Must decide now:** YES. WO-1 and WO-2 and the world map population wait on it.
- **Minimum sufficient:** a pure key mapping, two typed records built from the existing field
  dispositions, and the existing LocalObject overlay unchanged.
- **Superseding evidence:** a source generation change; Global evidence that contradicts a route.
- **Deliberately not decided:** runtime code, map placements, corpse loot link, field effects,
  bed and door behaviour (Interaction), sprites.

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#162 5876505672 (WO-0), routed as A7 in 5876526764"
owner_decisions: [D93, D94]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # WO-1 may be allocated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (identity mapping, field ownership, overlay relation)"
implementation_lanes: [WO-1, WO-2, WO-3]
required_revalidation:
  - "WO-1: every routed id maps to exactly one family key by the D93 rule; excluded routes get none; the key is reproducible from the frozen allocation; unknown facts stay UNKNOWN"
  - "WO-1: an Item record with a typed routed_to {family, key, revision} validates; a bare key or an unknown family is rejected"
  - "WO-2: no key collides with an existing key; every routed Item record points to its family key; no Item key is removed or renumbered; corpse decay and capacity facts stay on the Item record only"
  - "WO-3: a LocalObject state with a presentation reference to an existing WorldObject revision validates; a missing or wrong-family reference is rejected; a state without one behaves as today"
remaining_unknowns:
  - Global evidence for contested routes
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates WO-1."
```
