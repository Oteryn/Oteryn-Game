# Oteryn Atlas 15.32 Drive Appearance Spatial Profile v1

- Contract ID: `oteryn-atlas-15-32-appearance-spatial-v1`
- Semantic revision: `1`
- Coordination ID: `OTERYN-GAME-ATLAS-V1-APPEARANCE-15-32`
- Canonical owner: `Oteryn-Game` (current source lineage: `blakinio/Oteryn-v2`)
- Source label: `15.32` (repository-pinned project label)
- Status: **PROPOSED until this exact content is merged to protected `main`; ACCEPTED thereafter unless explicitly superseded**
- Runtime implementation status: `NOT_IMPLEMENTED`
- Production status: `NOT_ENABLED`

## 1. Purpose

This profile defines the bounded Game-owned appearance presentation semantics for the exact Google Drive asset bundle selected by the repository's latest Atlas build path under the project label `15.32`.

It converts that pinned source representation into explicit presentation semantics compatible with `oteryn-world-spatial-v1` and `oteryn-game-atlas-export-v1` without making the asset archive, Tibia/CipSoft file formats, Google Drive, or the legacy renderer canonical Oteryn World authority.

It does not define gameplay occupancy, collision, movement legality, interaction priority, a permanent Game -> Atlas serializer, a permanent texture package, CDN policy or GPU packing.

## 2. Exact source identity

The source is identified by repository selection plus immutable content identity, not by a package-version field:

```text
project label: 15.32
drive file id: 1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
zip sha256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
```

The repository selection is pinned by:

- `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`;
- `tools/otbm_atlas/build_latest_local.sh`, whose default `ATLAS_ASSET_LABEL` is `15.32`, Drive file ID is the value above, and expected ZIP SHA-256 is the value above.

A direct one-shot verification in `Oteryn/Oteryn-Atlas` downloaded that exact Drive file and verified the expected ZIP SHA before decoding metadata. Durable metadata-only evidence is committed at:

- `Oteryn/Oteryn-Atlas@0b56d9a95279f1ec02fddd0dfcf8bd6ffd16b539`;
- `docs/evidence/DYN-ATLAS-001-15-32-drive-asset-profile.json`.

The verification run was `32115353176`, job `95643523890`, metadata artifact `9316452985`.

## 3. Version-label semantics

The verified ZIP contains one top-level `assets/` directory and **does not contain `package.json`**.

Therefore:

- `15.32` is the repository-pinned project/source label for this exact Drive ID + digest;
- this contract does **not** claim that an internal package metadata field independently states version `15.32`;
- the immutable ZIP/catalog/appearance digests are the compatibility identity;
- another archive called `15.32` is not compatible unless its exact identity is separately accepted.

This distinction prevents a filename/label from becoming stronger evidence than the bytes actually verified.

## 4. Verified metadata profile

Direct decoding of the exact bundle with the pinned Otheryn appearance decoder produced:

```text
catalog sha256: 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
catalog entries: 5090
appearance file: appearances-dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075.dat
appearance sha256: dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
object appearances: 43514
object frame records: 43514
sprite sheets: 5084
total object sprite refs: 111957
unique object sprite ids: 79269
shift-bearing appearances: 567
height-bearing appearances: 2192
max sprite refs in one object frame: 855
catalog sprite id domain: 0..301200
```

Decoded sheet cell layouts are exactly:

```text
32 x 32
32 x 64
64 x 32
64 x 64
```

These facts are source-profile evidence. They are not gameplay entity counts and source appearance/sprite IDs are not promoted to canonical Game identity.

## 5. Spatial unit

For this exact source profile:

```text
AppearanceSpatialUnitProfile {
  units_per_tile: 32
}
```

One presentation unit equals one decoded source pixel at the source profile's 32-unit tile scale.

This is conversion semantics. It does not require a browser to render at one device pixel per unit and does not freeze final GPU texture resolution.

## 6. Visual anchor and coverage

The pinned reference renderer projects an object's decoded sprite relative to the owning map tile by subtracting the decoded sprite's excess width/height beyond one 32-unit tile, then applying source shift/height adjustments.

For one decoded primitive of `width_units x height_units`:

```text
width_tiles  = width_units  / 32
height_tiles = height_units / 32
```

The owning map tile is the south-east / bottom-right **visual anchor cell** for this source conversion. Visual coverage extends west/north by:

```text
dx_tiles in [-(width_tiles  - 1), 0]
dy_tiles in [-(height_tiles - 1), 0]
```

Requirements:

- dimensions MUST be positive and divisible by 32;
- `(0, 0)` is the visual anchor tile offset;
- visual coverage is presentation-only metadata;
- pixel-derived coverage MUST NOT be used to infer collision, movement blocking, item occupancy, use range, targeting, ownership or other gameplay area semantics.

A gameplay footprint, if one is ever required, remains owned by a separate Game-domain contract.

## 7. Shift/height to explicit presentation displacement

For this source profile, missing source `shift` and `height` values are treated as zero only for conversion of the corresponding optional source fields.

The pinned reference renderer subtracts source shift and source height from the projected draw origin. The Game-owned resolved presentation primitive therefore exposes equivalent explicit 2D displacement:

```text
PresentationDisplacement {
  dx_units = -(shift_x + height)
  dy_units = -(shift_y + height)
}
```

under the `oteryn-world-spatial-v1` axis convention where +X is east/right and +Y is south/down.

Arithmetic is checked. Atlas consumes the explicit converted displacement and must not reconstruct the legacy shift/height rule from raw asset files.

## 8. Concrete frame, phase, pattern and layer ownership

The source metadata contains pattern dimensions, layer counts, animation phase data and sprite IDs. The legacy renderer additionally contains source-specific position, stack-count, hook and fluid pattern selection behavior.

Those are bounded source-conversion concerns, not Atlas authority.

For a Game -> Atlas fixture/export using this profile, Game MUST resolve concrete render primitives before Atlas consumption. The semantic equivalent of each resolved primitive includes at minimum:

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
  source_profile_id
  provenance
}
```

Rules:

- source appearance/sprite IDs are provenance/reference identifiers, not canonical Game entity identities;
- selected frame/phase/pattern/layer values are explicit producer output;
- layer order is deterministic and explicit;
- the tile/placement keeps its Game-owned `PresentationOrderKey` independently of appearance-internal layer order;
- Atlas MUST NOT reimplement source stack-count, hook, fluid, coordinate-modulo pattern or animation-selection heuristics as world truth.

## 9. Bounded static policy for DYN-ATLAS-001

DYN-ATLAS-001 is a static semantic proof rather than the generalized animation contract.

For the selected Thais static fixture, the producer may follow the same bounded parity policy used by the pinned reference renderer provided the resulting choice is exported explicitly:

- first applicable object frame group exercised by the reference path;
- declared default start phase;
- no elapsed-time animation advancement in the canonical static fixture;
- every emitted appearance-internal layer is explicit.

The browser receives resolved primitives and does not derive this legacy policy itself.

## 10. Pixel-content identity and Atlas deduplication

Game owns the exact asset source identity and semantic mapping to resolved appearance primitives. Atlas owns publication-side pixel content deduplication and GPU/runtime packing.

Atlas may assign deterministic content IDs to decoded pixel blobs using a declared domain-separated digest over exact dimensions + exact decoded RGBA bytes.

Pixel content identity:

- MAY deduplicate identical pixels referenced by multiple semantic appearances;
- MUST NOT replace semantic appearance identity;
- MUST NOT replace Game placement/entity identity;
- MUST NOT encode GPU page/texture coordinates as persistent semantic identity.

GPU page placement remains runtime/cache state.

## 11. Validation and failure behavior

A producer/fixture claiming this profile fails closed on:

- Drive/source ZIP digest mismatch;
- catalog digest mismatch;
- appearance digest mismatch;
- malformed catalog or appearance metadata;
- missing referenced sprite source IDs;
- unsupported/non-positive/non-32-integral decoded sprite dimensions;
- displacement arithmetic overflow;
- unresolved concrete frame/phase/pattern/layer/sprite selection when the fixture declares presentation capability;
- missing public-safe source provenance required to reproduce the conversion.

An Atlas consumer must reject inconsistent width/height/visual-coverage/displacement metadata instead of reopening raw source assets to infer or repair semantics.

## 12. Rights/provenance boundary

Technical identity/provenance for the 15.32 Drive bundle is proven by repository configuration and direct digest verification.

This contract does not itself manufacture third-party copyright permission. Public redistribution/publication of exact source pixels requires the project's rights/authorization evidence to explicitly cover this exact ZIP digest `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`.

Until that exact-digest authorization is recorded, producer/consumer semantic work and non-pixel metadata verification may proceed, but a public Atlas publication containing those source pixels remains fail-closed.

## 13. DYN-ATLAS-001 effect

After this profile is accepted, DYN-ATLAS-001 may use the repository-selected 15.32 Drive bundle for appearance conversion without importing the separate 15.25 fixture's identity/counts.

The resulting proof must pin:

```text
appearance profile: oteryn-atlas-15-32-appearance-spatial-v1
source label: 15.32
drive file id: 1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
zip sha256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog sha256: 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
appearance sha256: dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

The remaining Game-side executable dependency is a bounded deterministic Thais semantic export fixture/artifact under:

- `oteryn-game-atlas-export-v1`;
- `oteryn-world-spatial-v1`;
- `oteryn-crystalserver-legacy-spatial-import-v1`;
- this appearance profile.

## 14. Supersession

Revision 1 must be superseded rather than silently edited if the selected source ZIP/catalog/appearance identity, unit mapping, visual anchor/coverage semantics, displacement conversion or static primitive-resolution semantics change materially.

Serializer, compression, chunk dimensions, browser framework or GPU packing changes alone do not supersede this semantic source profile.
