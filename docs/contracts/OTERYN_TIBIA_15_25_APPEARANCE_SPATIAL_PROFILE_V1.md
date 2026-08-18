# Oteryn Tibia 15.25 Appearance Spatial Profile v1

- Contract ID: `oteryn-tibia-15-25-appearance-spatial-v1`
- Semantic revision: `1`
- Coordination ID: `OTERYN-GAME-ATLAS-V1-APPEARANCE`
- Canonical owner: `Oteryn-Game` (current source lineage: `blakinio/Oteryn-v2`)
- Status: **PROPOSED until this exact content is merged to protected `main`; ACCEPTED thereafter unless explicitly superseded**
- Runtime implementation status: `NOT_IMPLEMENTED`
- Production status: `NOT_ENABLED`

## 1. Purpose

This profile supplies the bounded appearance/anchor/displacement semantics required to project the owner-authorized Tibia client asset package used by `DYN-ATLAS-001` into `oteryn-world-spatial-v1` without making the Tibia asset format or legacy renderer canonical World authority.

It does not define Game item identity, gameplay interaction priority, collision/occupancy, a permanent Atlas texture format, a permanent Game -> Atlas serializer, or a general license for unrelated assets.

## 2. Exact asset fixture

The owner-supplied archive is pinned by digest:

```text
archive: assets(1).zip
sha256: 01c45146e2fcec3f4087844e0cbc1817fb1d60b310a35ac5d88c07aab6f73d1a
```

Direct archive inspection records:

```text
package.json version: 15.25.bd5a04
package.json sha256: 01c405e8f72bc4bbf2009052cdccc10510a5664ab8b81713b78c804422c3db3b
assets/catalog-content.json sha256: 93ea5888174ef44b352d7c2b1f8061573a4a260bfaba4b7ec32ea836b9e411ab
appearance file: appearances-aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50.dat
appearance file sha256: aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50
object appearances: 42107
sprite sheets: 4927
catalog sprite id domain: 0..289767
unique sprite ids referenced by object appearances: 75623
```

The package is therefore **15.25.bd5a04**, not 15.32.

The object-appearance, sprite-sheet and referenced-sprite counts reconcile with the pinned legacy Otheryn Atlas evidence for `vendor/map-analysis/tibia-client/15.25.bd5a04/assets/`. That reconciliation supports this bounded proof fixture; it does not make every package with the same version label interchangeable.

## 3. Rights/provenance boundary

For `DYN-ATLAS-001`, the project owner explicitly attested that the exact archive digest above may be decoded, derived, rendered, deduplicated and published for public Oteryn Atlas use.

This is owner-supplied project authorization, not independently verified third-party copyright ownership, and it does not extend to another archive/digest or unrelated proprietary asset set.

## 4. Spatial unit

For this source profile:

```text
AppearanceSpatialUnitProfile {
  units_per_tile: 32
}
```

One native presentation displacement unit equals one decoded source pixel at the proof's 32-pixel tile scale. This does not require the browser to render one screen pixel per unit or freeze a permanent GPU resolution.

## 5. Decoded visual coverage and anchor

The pinned catalog exposes decoded sprite cell layouts of:

```text
32 x 32
32 x 64
64 x 32
64 x 64
```

Every selected decoded sprite dimension MUST be an integral multiple of `units_per_tile` for revision 1.

For `width_px x height_px`:

```text
width_tiles  = width_px  / 32
height_tiles = height_px / 32
```

The source-profile presentation anchor is the tile position owning the presentation record. The decoded sprite visual rectangle extends west/north from that anchor exactly as the pinned reference renderer does.

The **presentation visual-coverage offsets** for one decoded sprite are therefore:

```text
dx_tiles in [-(width_tiles  - 1), 0]
dy_tiles in [-(height_tiles - 1), 0]
```

with `(0, 0)` the south-east / bottom-right anchor cell of that decoded visual rectangle.

This visual-coverage mapping is presentation metadata only. It MUST NOT be interpreted as gameplay occupancy, collision shape, movement blocking, item multi-tile ownership or interaction area. If a canonical Game semantic footprint beyond visual coverage is ever required, that footprint needs its own authoritative content/world evidence and must not be inferred from pixel dimensions.

## 6. Shift and height to presentation displacement

The pinned appearance metadata may carry legacy `shift = (shift_x, shift_y)` and `height`. Missing values are zero for this profile.

The pinned reference renderer subtracts both shift components and subtracts `height` from both projected image axes. The bounded Game conversion therefore emits:

```text
PresentationDisplacement {
  dx_units = -(shift_x + height)
  dy_units = -(shift_y + height)
}
```

where positive native `dx_units` is east/right and positive `dy_units` is south/down under `oteryn-world-spatial-v1`.

This is strictly a 2D presentation offset. Source `height` here does not become canonical floor/Z/elevation authority.

The exporter SHOULD preserve raw source shift/height as bounded public-safe provenance/debug fields, while Atlas consumes the explicit converted displacement rather than re-implementing legacy rules.

## 7. Frames, layers and resolved primitives

The legacy appearance metadata contains pattern dimensions, layers, animation data and sprite IDs. Atlas must not infer a canonical sprite from a legacy appearance ID alone.

For the bounded proof, Game owns resolution of the concrete presentation variant and exports semantic render primitives equivalent to:

```text
ResolvedAppearancePrimitive {
  appearance_source_id
  frame_group
  phase
  layer_index
  sprite_source_id
  width_units
  height_units
  visual_coverage_offsets
  displacement
  provenance
}
```

Rules:

- source appearance/sprite IDs remain provenance identifiers, not canonical Game entity identity;
- layer order is explicit and deterministic;
- appearance-internal layers are not separate tile `PresentationOrderKey` records unless the Game exporter explicitly models them that way;
- the owning tile presentation record retains its Game-owned `PresentationOrderKey` from the spatial/import profile;
- Atlas may deduplicate identical decoded pixel payloads while preserving semantic appearance and placement identity.

## 8. Bounded static phase policy

DYN-ATLAS-001 is a static semantic proof, not the final animation contract.

For parity with the pinned legacy reference pipeline, the producer resolves:

- the first applicable object frame group used by the pinned reference path;
- the declared `default_start_phase` for that frame;
- no elapsed-time animation advancement in the canonical static fixture.

The selected phase is exported explicitly. The browser does not recompute a legacy animation policy.

## 9. Pattern and subtype selection

Pattern/subtype/hangable/fluid/stack-count rules are source conversion concerns. For DYN-ATLAS-001 the Game-side fixture/export step MUST resolve concrete primitive(s) before Atlas consumption.

Atlas MUST NOT reproduce legacy position-modulo-pattern, stack-count bucket, hook-direction or fluid-color rules to discover authoritative presentation state.

Pinned reference code may be used as conversion evidence; the exported result is explicit semantic data with provenance.

## 10. Pixel content identity and deduplication boundary

Game owns the exact asset fixture/revision and semantic mapping from source appearance to resolved primitives. Atlas owns publication-side pixel deduplication and GPU/runtime packing.

For proof measurements, Atlas may assign a deterministic content identity to decoded pixel blobs using a declared domain-separated digest over dimensions plus exact RGBA bytes. That pixel identity MUST NOT replace semantic appearance identity or canonical Game item/entity identity.

GPU atlas/page coordinates are runtime cache state only.

## 11. Validation requirements

Producer/fixture validation under this profile must fail closed on:

- asset archive digest mismatch;
- package version mismatch;
- catalog or appearance digest mismatch;
- unknown sprite source ID;
- unsupported decoded sprite dimensions or dimensions not divisible by 32;
- malformed appearance/frame metadata;
- displacement arithmetic overflow;
- unresolved concrete sprite/phase/layer when declared required;
- missing public-safe provenance required to reproduce conversion.

Atlas consumer validation rejects inconsistent width/height/visual-coverage/displacement metadata instead of reconstructing legacy asset rules.

## 12. DYN-ATLAS-001 effect

After merge, the exact authorized 15.25.bd5a04 fixture has explicit Game-owned presentation conversion semantics:

```text
units_per_tile = 32
anchor = owning tile / south-east visual-coverage cell
visual coverage = explicit west/north offsets derived from decoded sprite dimensions
effective displacement = (-(shift_x + height), -(shift_y + height))
concrete frame/phase/layer/sprite = producer-resolved and exported explicitly
```

This closes the appearance anchor/visual-coverage/displacement authority gate without inventing gameplay occupancy.

DYN-ATLAS-001 still requires a bounded Game-owned Thais semantic export artifact/digest before Atlas runtime may claim the full proof acceptance criteria.

## 13. Supersession

Revision 1 must be superseded rather than silently changed if evidence changes:

- exact asset archive/package identity;
- units-per-tile conversion;
- decoded sprite dimension semantics;
- anchor/visual-coverage mapping;
- shift/height displacement conversion;
- static phase/layer resolution semantics.

Browser framework, texture packing, compression and immutable chunk packaging do not themselves supersede this Game-owned source profile.
