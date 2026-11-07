# Oteryn World Spatial / Coordinate Profile v1

- Contract ID: `oteryn-world-spatial-v1`
- Semantic revision: `1`
- Coordination ID: `OTERYN-GAME-ATLAS-V1-SPATIAL`
- Canonical owner: `Oteryn-Game` (current source lineage: `Oteryn/Oteryn-Game`)
- Status: **PROPOSED until this exact content is merged to protected `main`; ACCEPTED thereafter unless explicitly superseded**
- Runtime implementation status: `NOT_IMPLEMENTED`
- Production status: `NOT_ENABLED`

## 1. Purpose

This profile freezes the minimum canonical spatial semantics required by the accepted native World/Content model and by `docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md`.

It defines coordinate axes, numeric domains, world bounds, floor identity and order, bounds inclusion, deterministic same-position presentation ordering, and presentation anchor/displacement meaning. It deliberately does **not** freeze a serializer, chunk size, floor packing, asset encoding, pathfinding algorithm or legacy OTBM/Tibia coordinate mapping.

This profile is semantic authority. A physical schema may encode it differently but must preserve these meanings exactly.

## 2. Decision timing

**Must decide now:** `YES`.

**Blocked downstream work:**

- first executable Game -> Atlas physical-profile contract;
- `DYN-ATLAS-001 — Semantic Thais Z7 Proof` preserving Game-owned floor/order semantics rather than inventing them;
- future native World Project / World Bundle spatial schema implementation;
- Atlas consumer validation for coordinates, floor switching and same-position ordering.

**What becomes harder later if left implicit:**

- legacy `z`/stack conventions can leak into the canonical model by accident;
- Studio, Game exporter and Atlas can disagree about up/down, bounds inclusion or render order;
- serializer/chunk experiments can accidentally become semantic authority;
- migrations can require coordinate reinterpretation instead of representation-only changes.

**Supersession evidence:** a native vertical slice, Studio authoring evidence, renderer evidence, world-scale measurements or migration evidence demonstrating that a semantic rule here is insufficient or creates material correctness/operability cost.

**Deliberately not decided:** serializer/IDL, compression, chunk dimensions, floor packing, concrete world extents for a particular world, pixel resolution, GPU texture layout, collision/pathfinding representation, legacy source mapping, or public asset redistribution.

## 3. Accepted constraints

### PROVEN

- ADR-0005 makes the canonical Oteryn World/Content Model independent from OTBM and requires deterministic ordering, bounded chunks and explicit conversion provenance.
- `OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1` requires an explicit versioned coordinate profile before producer/consumer implementation may claim compatibility.
- The 2026-08-16 Game -> Atlas physical-profile readiness evidence identified exactly the missing axes/orientation, coordinate domain/bounds, floor identity/order, same-position ordering and anchor/displacement authority.
- The existing `client-domain::Position` is a non-authoritative client projection and is not promoted by this contract.

### Design rule

The profile chooses native Oteryn semantics directly. Legacy formats may be mapped into these semantics only by explicit importer profiles with provenance.

## 3A. Decision analysis

### Problem

Game, Studio, the Game-owned Atlas exporter and Atlas consumers need one stable semantic interpretation of position/floor/order before any physical profile or browser proof can be called compatible. The current accepted architecture deliberately does not supply that precision.

### Realistic options

**Option A — explicit native Oteryn integer tile/floor profile with versioned legacy conversion adapters.**  
Define native axes, finite bounds, floor ordering, explicit presentation order and anchor/displacement meaning now; require every legacy source to map through a pinned conversion profile.

**Option B — adopt Tibia/OTBM `x/y/z/stackpos` conventions as the canonical profile.**  
This is operationally convenient for migration but violates ADR-0005 by making a legacy representation the native semantic authority and would couple future Studio/Game/Atlas evolution to source-format behavior.

**Option C — keep axes/floors/order opaque and let each serializer/consumer define them.**  
This preserves short-term flexibility but fails the accepted Game -> Atlas contract: two valid physical encodings could disagree semantically while each claimed compatibility, and DYN-ATLAS would be forced to infer missing truth.

### Trade-offs

Option A adds one deliberate native mapping step during migration and requires explicit conversion evidence, but it keeps serializer/chunk experiments representation-only and gives every consumer one fail-closed semantic contract. Option B minimizes importer translation at the cost of permanent legacy coupling. Option C minimizes decisions now but creates cross-product ambiguity and makes later migration substantially more expensive.

### Risks and mitigations

- **Risk:** chosen integer domains are wider than current content requires. **Mitigation:** concrete world extents and resource ceilings remain per-world/implementation data; type domain is not a production allocation limit.
- **Risk:** +Y south / larger floor means higher could later conflict with a renderer or imported source. **Mitigation:** renderer transforms and legacy mappings are adapters; native semantic orientation stays stable, and supersession requires named evidence rather than silent reinterpretation.
- **Risk:** presentation ordering becomes mistaken for gameplay interaction authority. **Mitigation:** the contract explicitly separates presentation order from collision, targeting, movement and mutation semantics.
- **Risk:** sub-tile displacement precision is frozen prematurely. **Mitigation:** only direction/meaning are canonical here; precision is bound by a versioned appearance/asset `units_per_tile` profile.

### Recommendation

Select **Option A**. It is the smallest decision that closes the named evidence gap while preserving the architecture rule that legacy formats and physical encodings remain adapters rather than canonical World authority.

### Future impact

Native World/Studio/runtime types can implement this profile directly. Legacy imports must carry mapping provenance. Atlas can reject unknown/incompatible coordinate profiles without parsing legacy formats. Serializer, chunking and asset-pixel changes can evolve without renumbering world positions or redefining stack order.

## 4. Horizontal tile coordinate system

Canonical static world positions use a discrete tile grid.

```text
WorldTilePosition {
  x: i32,
  y: i32,
  floor: i16
}
```

Semantic axis orientation is:

- increasing `x` = east / right in the canonical north-up map projection;
- decreasing `x` = west / left;
- increasing `y` = south / down;
- decreasing `y` = north / up.

The origin has no geographic or Tibia-specific meaning. A world/project chooses its authored coordinate envelope; `(0, 0)` is merely a valid integer location when included by that envelope.

Floating-point world tile coordinates are not canonical tile identity.

## 5. Coordinate numeric domain and world bounds

`x` and `y` use the signed 32-bit integer semantic domain. Implementations must use checked arithmetic when translating, expanding bounds, indexing or calculating chunk-local coordinates.

Every canonical world revision MUST declare one finite horizontal world envelope. Bounds are represented semantically with a wider integer envelope so half-open maxima can represent the full `i32` position domain without overflow:

```text
WorldBounds2D {
  min_x: i64,
  min_y: i64,
  max_x_exclusive: i64,
  max_y_exclusive: i64
}
```

Validity requires:

```text
i32::MIN <= min_x
max_x_exclusive <= i32::MAX + 1
i32::MIN <= min_y
max_y_exclusive <= i32::MAX + 1
min_x < max_x_exclusive
min_y < max_y_exclusive
min_x <= i64(x) < max_x_exclusive
min_y <= i64(y) < max_y_exclusive
```

Bounds are therefore **half-open** on the positive edge. This rule applies to rectangular selections, chunk coverage, Atlas viewport/bounding-box selection and compiler validation unless a more specific geometry contract explicitly defines another shape.

The profile does not invent one universal Oteryn world size. Concrete per-world bounds are authored Game data and must be carried by the World revision/export manifest. A consumer must reject coordinates outside the declared envelope.

## 6. Floor identity, domain and vertical order

Canonical floor identity is a signed 16-bit semantic integer `FloorId`.

Rules:

- no universal semantic meaning is assigned to `floor = 0`;
- larger `FloorId` values are geometrically **higher / above** smaller values;
- smaller values are geometrically **lower / below** larger values;
- a world may designate one floor as a display/reference/surface label, but that label does not redefine numeric floor identity.

Every world revision MUST declare a finite, strictly increasing set of valid floor IDs:

```text
floors: [i16, ...]
```

The set may be sparse; consumers must not invent an undeclared intermediate floor. A `WorldTilePosition` is valid only when its `floor` is a member of the declared set.

Display labels such as `surface`, `level 2`, `underground`, or migration labels such as `Z7` are presentation/provenance metadata and are not alternate floor authority.

## 7. Rectangles, points and inclusion

A tile position identifies one logical tile cell at `(x, y, floor)`.

Rectangular spatial bounds use half-open horizontal coordinates:

```text
RectBounds {
  min_x: i64,
  min_y: i64,
  max_x_exclusive: i64,
  max_y_exclusive: i64,
  floor: i16
}
```

A tile belongs to a rectangle exactly when:

```text
min_x <= i64(x) < max_x_exclusive
min_y <= i64(y) < max_y_exclusive
position.floor == bounds.floor
```

Multi-floor selections are represented as an explicit set/range of declared floor IDs plus per-floor geometry; a rectangle does not implicitly span floors.

This avoids inclusive-max ambiguity and makes adjacent rectangular partitions share boundaries without overlapping tile identity.

## 8. Same-position presentation ordering

Presentation ordering is explicit Game-owned data. Consumers MUST NOT infer it from serializer order, content IDs, legacy numeric IDs, filenames or browser insertion order.

Every static presentation record that can coexist with another record at the same canonical tile position MUST expose a semantic order key equivalent to:

```text
PresentationOrderKey {
  plane: i32,
  order: u32
}
```

Semantics:

- smaller `plane` renders earlier / farther back;
- larger `plane` renders later / farther front;
- inside one plane, smaller `order` renders earlier;
- `(plane, order)` MUST be unique among presentation records at one canonical tile position;
- duplicate order keys at one position are invalid producer data and MUST fail validation rather than be repaired by a consumer tie-breaker.

Named plane categories are deliberately not frozen in v1. The canonical World compiler/exporter owns assignment from authored semantic content into these explicit order keys.

Presentation order does **not** by itself define interaction priority, movement legality, collision ownership, use/target selection or server mutation order. Those require their owning gameplay contracts.

## 9. Stable placement identity

A placed canonical/static record that requires independent addressability MUST use a stable Game-owned identity or a deterministic producer-owned placement key derived from canonical inputs.

Placement identity and presentation order are distinct:

- changing semantic identity must not be hidden by an unchanged order key;
- reordering must not require changing canonical content identity;
- Atlas/browser-generated random IDs are not authoritative placement identity.

## 10. Anchor, footprint and displacement semantics

A presentation record has one explicit canonical **anchor tile position**. No corner, center or legacy sprite origin is implied by convention.

When an appearance occupies more than one tile, its footprint MUST be expressed as explicit signed tile offsets relative to the anchor, or by equivalent canonical metadata that expands deterministically to such offsets:

```text
footprint_offset = (dx_tiles, dy_tiles)
```

with:

- positive `dx_tiles` = east;
- positive `dy_tiles` = south;
- `(0, 0)` = the anchor tile.

The owning appearance/profile must therefore say which footprint tile is the anchor; a consumer must not assume north-west, south-east or any Tibia-specific origin.

Sub-tile visual displacement is presentation-only data. The owning appearance/asset profile MUST declare a positive integer `units_per_tile` and signed displacement values:

```text
PresentationDisplacement {
  dx_units: i32,
  dy_units: i32
}

AppearanceSpatialUnitProfile {
  units_per_tile: u32  // MUST be > 0
}
```

Semantics:

- positive `dx_units` moves the visual draw origin east/right;
- positive `dy_units` moves the visual draw origin south/down;
- one tile equals exactly `units_per_tile` displacement units for the bound appearance profile revision;
- displacement affects rendering only;
- displacement MUST NOT change canonical world position, footprint membership, collision, navigation or gameplay authority.

The coordinate profile therefore freezes direction and meaning but not sprite pixel resolution or one permanent sub-tile precision. A Game export/asset catalog must bind the exact appearance profile revision and `units_per_tile` needed for deterministic rendering.

## 11. Canonical ordering outside one tile

When deterministic serialization or iteration requires a total spatial order, the canonical spatial key is:

```text
(floor, y, x)
```

ascending, followed by the record family/key defined by the owning schema.

This is a canonicalization rule, not a rendering rule. Same-position rendering still uses `PresentationOrderKey`.

Consumers must not treat hash-map/database/filesystem enumeration order as semantic input.

## 12. Legacy import mapping

OTBM/Tibia/Canary/Crystal or other legacy coordinates are migration inputs only.

A legacy importer MUST bind an explicit versioned conversion profile that states how source coordinates, source floor identifiers, source stack/order semantics and source presentation displacement map to `oteryn-world-spatial-v1`.

Rules:

- no direct cast from a legacy `z`, stack position or numeric item ID becomes canonical authority merely because values fit the target integer type;
- source values and mapping revision must remain available in bounded conversion provenance/diagnostics where needed;
- ambiguous mapping remains `AMBIGUOUS`, `UNRESOLVED` or `UNKNOWN` under the Game -> Atlas evidence vocabulary;
- a conversion that cannot preserve required ordering/anchor semantics must fail or emit an explicit unsupported record; it must not guess.

This contract intentionally does **not** define a Tibia `z=7` -> canonical `FloorId` mapping. That mapping belongs to a pinned importer/source profile and can vary without redefining native Oteryn spatial semantics.

## 13. Game -> Atlas binding

A Game Atlas export conforming to semantic revision 1 may declare:

```text
coordinate_profile = "oteryn-world-spatial-v1"
```

only when all exported spatial records satisfy this profile.

The Atlas consumer must validate at minimum:

- supported coordinate profile ID/revision;
- world bounds and declared floors;
- every position within bounds and on a declared floor;
- half-open rectangle validity;
- unique same-position presentation order keys;
- footprint offsets and displacement values without unchecked arithmetic;
- a bound appearance/asset spatial unit profile wherever sub-tile displacement is present;
- stable identity/reference integrity required by the selected export capability.

An unknown required coordinate profile is a fail-closed incompatibility.

The label `Thais Z7` may be retained as pinned source/provenance selection terminology for a migration proof, but the semantic Atlas projection must expose the canonical Game position/floor produced by the explicit conversion/export profile. Atlas must not reinterpret legacy `Z7` itself.

## 14. Chunking and physical encoding remain non-authoritative

A chunk, shard, file path, JSON array order, FlatBuffers table order, compression block or GPU atlas page cannot redefine world coordinates or presentation ordering.

Logical chunk addressing may derive from canonical coordinates, but changing chunk dimensions or floor packing must not change canonical tile positions, floor identities, placement identities or presentation order.

`32x32`, `64x64`, per-floor and packed-floor remain benchmark candidates rather than semantic decisions.

## 15. Validation and resource safety

Future executable implementations must add numeric resource ceilings to `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` for any externally controlled world spans/counts, records per position, footprint cardinality, serialized sizes and equivalent allocation risks before implementation acceptance.

This semantic profile itself does not invent production resource ceilings.

Required negative behavior includes rejection of:

- coordinates outside world bounds;
- undeclared floors;
- invalid/empty bounds;
- duplicate same-position presentation order keys;
- arithmetic overflow while translating/indexing bounds or offsets;
- malformed footprint/displacement fields or `units_per_tile = 0`;
- unsupported required coordinate-profile revision.

## 16. Consequences for DYN-ATLAS-001

Once this exact profile is merged to protected `main`, the prior missing **canonical Game spatial/coordinate authority** is closed for the bounded semantic proof.

DYN-ATLAS-001 may then use a pinned Game-owned/exporter-owned fixture or conversion profile to define the exact Thais source selection and emit `oteryn-world-spatial-v1` positions/order. It still must not:

- infer legacy floor/stack semantics in the browser;
- treat OTBM as Atlas runtime authority;
- freeze the production serializer/chunk size from the proof;
- publish proprietary assets without separate provenance/rights evidence;
- claim a physical `Oteryn-Atlas` repository exists when it does not.

## 17. Explicit non-decisions

This profile does not decide:

- concrete per-world coordinate extents or floor list;
- legacy source coordinate/floor mapping;
- World Project or World Bundle physical encoding;
- Game -> Atlas physical serializer/compression/container;
- permanent chunk dimensions or vertical packing;
- pathfinding/collision representation;
- isometric/perspective camera projection;
- sprite pixel dimensions, displacement precision or GPU texture layout;
- asset licensing/distribution;
- live-state coordinates/privacy transport;
- editor/runtime implementation language beyond already accepted repository architecture.

Those decisions require their own evidence/authority.

## 18. Acceptance effect

After merge to protected `main`, this document is the canonical Oteryn-Game spatial coordinate profile v1 for native World/Content and Game -> Atlas semantic compatibility until explicitly superseded.

Historical evidence that previously reported `EVIDENCE_GAP` remains historically correct for its inspected revision; newer readiness evidence should reference this accepted profile rather than rewriting that history.
