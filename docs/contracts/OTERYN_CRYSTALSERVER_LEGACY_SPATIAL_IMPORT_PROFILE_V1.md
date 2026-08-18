# Oteryn CrystalServer Legacy Spatial Import Profile v1

- Contract ID: `oteryn-crystalserver-legacy-spatial-import-v1`
- Semantic revision: `1`
- Coordination ID: `OTERYN-GAME-ATLAS-V1-LEGACY-IMPORT`
- Canonical owner: `Oteryn-Game` (current source lineage: `blakinio/Oteryn-v2`)
- Status: **PROPOSED until this exact content is merged to protected `main`; ACCEPTED thereafter unless explicitly superseded**
- Runtime implementation status: `NOT_IMPLEMENTED`
- Production status: `NOT_ENABLED`

## 1. Purpose

This profile defines one explicit, versioned mapping from the pinned CrystalServer/Tibia legacy spatial representation used by the historical Otheryn Atlas pipeline into the accepted native Oteryn spatial semantics in `OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md`.

It exists to prevent legacy `x/y/z` and tile item sequence from leaking into Game/Atlas as implicit authority. OTBM remains a legacy importer input. The output of this profile is native semantic data; consumers must not parse OTBM to reconstruct or reinterpret these rules.

This profile does **not** define a physical Game -> Atlas serialization, chunk size, asset encoding, collision semantics, movement legality, interaction priority or asset redistribution right.

## 2. Pinned legacy evidence

The profile is limited to the following evidence family unless explicitly superseded:

- legacy reference repository: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`;
- canonical legacy content revision recorded there: `zimbadev/crystalserver@5e89bf8329ea406cb4ea8f4a18f32954f13e5418`;
- canonical `world.otbm` SHA-256: `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`;
- semantic parser evidence: `tools/otbm_atlas/semantic.py` at Otheryn blob `a11343a472145aee4d9cf65c6ce28b3e4a71a2b3`;
- legacy renderer evidence: `tools/otbm_atlas/render.py` at Otheryn blob `9d66d9ff2128663eda9eb07943c60ee4ef79c9e3`;
- independent legacy floor-direction evidence: `opentibiabr/otclient@dd5641492a71e966b96b8a91398b44bb3df67d88`, `modules/game_cyclopedia/tab/map/map.lua` at blob `15cc5423d2fece71159c6395c54cd94b098760c4`, which defines `SEA_FLOOR = 7`, classifies floors `0..7` as surface/above and `8..15` as underground, and explicitly treats `z < 7` as floors above Z7.

The historical mandatory Thais regression selection recorded by Otheryn is:

```text
legacy X = 32280..32440 inclusive
legacy Y = 32155..32305 inclusive
legacy Z = 7
```

This profile does not make that rectangle or legacy Z numbering canonical World content. It only defines how a producer/importer maps that pinned legacy evidence into native semantics.

## 3. Horizontal coordinates

For this pinned source family:

```text
native.x = legacy.x
native.y = legacy.y
```

Evidence from the legacy renderer projects increasing x to the right/east and increasing y downward/south, matching `oteryn-world-spatial-v1` orientation.

The mapping is checked identity conversion, not inheritance of OTBM coordinate authority. Native validity still requires the target World revision's declared finite bounds.

## 4. Legacy Z to native FloorId

The mapping is:

```text
native.floor = checked_i16(-legacy.z)
```

Rationale:

- the native profile requires larger `FloorId` values to mean geometrically higher/above;
- pinned OTClient map semantics independently confirm that legacy floors above Z7 have lower legacy Z (`z < 7`) while underground floors use larger values (`8..15`);
- therefore legacy Tibia/CrystalServer Z numerical direction is opposite the native vertical-order direction;
- negation preserves vertical order without assigning any universal semantic meaning to native floor zero;
- this avoids special-casing legacy `Z=7` as a canonical surface identity.

For the currently observed populated legacy source domain `Z=0..15`, the mapped native floor IDs are `0..-15`; expressed as the required strictly increasing native valid-floor set:

```text
[-15, -14, -13, -12, -11, -10, -9, -8, -7, -6, -5, -4, -3, -2, -1, 0]
```

A future source whose Z domain cannot be represented safely by this mapping is incompatible with revision 1 and requires a successor/import profile.

## 5. Rectangle conversion

Legacy Otheryn regression bounds are recorded with inclusive maxima. Native rectangles are half-open.

Conversion is:

```text
native.min_x = legacy.min_x
native.min_y = legacy.min_y
native.max_x_exclusive = legacy.max_x_inclusive + 1
native.max_y_exclusive = legacy.max_y_inclusive + 1
native.floor = -legacy.z
```

All `+1` operations use checked widened arithmetic before native bound validation.

Therefore the pinned Thais regression selection maps exactly to:

```text
RectBounds {
  min_x: 32280,
  min_y: 32155,
  max_x_exclusive: 32441,
  max_y_exclusive: 32306,
  floor: -7
}
```

The resulting horizontal dimensions remain 161 x 151 tile positions.

## 6. Visible same-position presentation sequence

For one legacy tile, the importer constructs the **visible legacy presentation sequence** exactly as evidenced by the pinned semantic parser and renderer:

1. optional decoded ground item, when present;
2. top-level decoded tile items, in their preserved source order;
3. nested child items inside an item/container are **not** members of the static visible tile presentation sequence.

The importer MUST NOT flatten nested container contents into visible spatial presentation records.

## 7. Mapping visible sequence to PresentationOrderKey

For each tile independently, let `i` be the zero-based index in the visible legacy presentation sequence from section 6.

The importer emits:

```text
PresentationOrderKey {
  plane: 0,
  order: u32(i)
}
```

Requirements:

- order conversion is checked; overflow is a hard import failure;
- keys are unique by construction for one tile;
- consumers receive explicit order keys and MUST NOT reconstruct them from serializer order, legacy IDs or browser insertion order;
- `plane = 0` is an importer-profile encoding for this bounded legacy source, not a universal named Game presentation category;
- presentation order does not define collision, use/target priority, movement legality or server mutation order.

A future native compiler may classify authored content into richer planes without changing this legacy source's historical draw order; that would require explicit migration/supersession evidence.

## 8. Stable identity and legacy IDs

Legacy server/item IDs remain provenance/migration identifiers only unless another accepted Game contract maps them to stable canonical identities.

This profile does not permit Atlas or an importer to manufacture canonical identities from:

- legacy numeric item IDs;
- display names;
- repository paths;
- source-order indexes.

If canonical identity is unresolved, the Game-owned export uncertainty rules remain in force.

## 9. Anchor, footprint and displacement

This profile intentionally does not promote legacy asset pixel metadata into the native appearance spatial authority.

Before a Game export or Atlas proof claims compatibility for multi-tile appearance footprint, visual displacement or sprite-pixel presentation, the owning appearance/asset profile must explicitly provide the semantics required by `oteryn-world-spatial-v1`, including `units_per_tile`, anchor/footprint interpretation and provenance.

Legacy renderer behavior may be used as reference evidence, but it is not itself the canonical appearance profile.

## 10. Asset rights boundary

The pinned Otheryn evidence records a Tibia client asset subset and its provenance. This contract grants **no redistribution right** for Tibia/CipSoft or other third-party assets.

A public Atlas repository/publication may consume or redistribute appearance pixels only when a separate rights/provenance gate authorizes the exact asset fixture or a legally/public-safe replacement fixture is selected.

## 11. Determinism and failure behavior

For identical pinned legacy bytes, parser/profile revision and importer implementation revision, the semantic mapping defined here must be deterministic.

Fail closed on:

- coordinate overflow;
- rectangle max-edge overflow;
- floor conversion overflow;
- presentation-order overflow or duplicate generated keys;
- malformed legacy structure;
- source digest/revision mismatch when the import invocation claims this pinned profile;
- missing required appearance semantics when a downstream export claims those capabilities.

No consumer-side heuristic repair is permitted.

## 12. DYN-ATLAS-001 mapping

This profile closes only the coordinate/floor/visible-order mapping gate for the historical Thais legacy source.

For DYN-ATLAS-001:

```text
legacy selection: X=32280..32440, Y=32155..32305, Z=7
native selection: x=[32280,32441), y=[32155,32306), floor=-7
coordinate profile: oteryn-world-spatial-v1
legacy import profile: oteryn-crystalserver-legacy-spatial-import-v1
```

DYN-ATLAS-001 remains blocked from claiming a complete sprite/render proof until the required Game-owned bounded semantic fixture/export and appearance/asset rights/profile gates are separately satisfied.

## 13. Supersession

Revision 1 must be superseded rather than silently changed if evidence requires a materially different:

- source coordinate orientation;
- Z/floor mapping;
- visible top-level item ordering rule;
- nested-item visibility rule;
- source family/digest applicability.

Physical serialization, chunking and browser framework changes alone do not supersede these semantic mapping rules.
