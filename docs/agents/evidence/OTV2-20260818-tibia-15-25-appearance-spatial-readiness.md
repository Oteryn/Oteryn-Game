# Tibia 15.25 appearance spatial readiness — 2026-08-18

## Verdict

**READY_FOR_EXACT_HEAD_REVIEW** for the bounded `oteryn-tibia-15-25-appearance-spatial-v1` profile.

This evidence does not authorize a different asset package, a permanent Game -> Atlas serializer, production deployment or gameplay semantics.

## Pinned inputs

- Game base: `blakinio/Oteryn-v2@ade3005cdf9ad6daeb87dc20a6546d5c29ee61da`
- native spatial profile: `oteryn-world-spatial-v1`
- legacy spatial importer: `oteryn-crystalserver-legacy-spatial-import-v1`
- Game -> Atlas semantic contract: `oteryn-game-atlas-export-v1`
- legacy reference implementation/evidence: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`
- owner-supplied asset archive SHA-256: `01c45146e2fcec3f4087844e0cbc1817fb1d60b310a35ac5d88c07aab6f73d1a`

## Direct archive verification

Direct inspection of the supplied archive established:

```text
package.json version = 15.25.bd5a04
package.json sha256 = 01c405e8f72bc4bbf2009052cdccc10510a5664ab8b81713b78c804422c3db3b
catalog-content.json sha256 = 93ea5888174ef44b352d7c2b1f8061573a4a260bfaba4b7ec32ea836b9e411ab
catalog entries = 4933
sprite sheets = 4927
appearance file sha256 = aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50
object appearances = 42107
unique object appearance ids = 42107
catalog sprite id domain = 0..289767
unique sprite ids referenced by object appearances = 75623
```

The package is therefore **15.25.bd5a04**, not 15.32.

The object-appearance, sprite-sheet and referenced-sprite counts reconcile with the pinned Otheryn 15.25 legacy Atlas evidence.

## Appearance metadata findings

Bounded deterministic decoding using the pinned Otheryn decoder semantics found:

- 42,107 object appearances;
- 42,107 object frame records in the bounded object category;
- 540 appearances with explicit shift metadata;
- 2,141 appearances with explicit height metadata;
- 105,321 total sprite references across object frames;
- 75,623 unique referenced sprite ids;
- maximum 160 sprite references in one object frame under the source pattern/layer representation.

Catalog sprite cell layouts are 32x32, 32x64, 64x32 and 64x64.

## Pinned renderer semantics

The legacy renderer in `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` uses a 32-pixel map-tile scale and positions sprites by subtracting `(sprite_width - 32)`, `(sprite_height - 32)`, source shift and source height from the tile draw origin.

This supports the bounded source-profile mapping selected by the proposed contract:

- `units_per_tile = 32`;
- owning tile as the south-east/bottom-right **visual anchor cell**;
- decoded visual coverage extending west/north according to sprite dimensions;
- native effective displacement `(-(shift_x + height), -(shift_y + height))`.

Self-review identified and repaired one important terminology risk: decoded sprite dimensions prove a **presentation visual-coverage rectangle**, not gameplay occupancy/collision footprint. The proposed contract now states that collision, movement blocking, item ownership and interaction area MUST NOT be inferred from pixel dimensions.

The mapping preserves pinned visual-reference behavior while converting it into explicit native presentation semantics. Atlas does not inherit the legacy renderer implementation as authority.

## Pattern/layer/animation boundary

The source metadata contains pattern dimensions, layers, animation phases and sprite ids. The legacy renderer also contains stack-count, hangable-hook, fluid and position-dependent pattern selection rules.

Those rules are not delegated to Atlas. The proposed profile requires the Game-owned bounded fixture/export step to resolve concrete frame/phase/layer/sprite primitives and export them explicitly.

For the static DYN-ATLAS-001 fixture, the parity policy remains the pinned reference policy: first applicable object frame group, declared default start phase, no elapsed-time advancement.

## Rights classification

The project owner explicitly attested authorization for the exact archive digest to be decoded, derived, rendered, deduplicated and published for public Oteryn Atlas use.

Classification: **OWNER_ATTESTED / PROJECT_AUTHORIZED** for that exact digest. This is not represented as independently verified third-party copyright ownership and does not extend to another digest.

## DYN-ATLAS effect

After merge:

- asset package version and identity are pinned;
- asset-rights gate is project-authorized for the exact digest;
- appearance spatial unit/visual anchor/visual coverage/displacement semantics are explicit and Game-owned;
- Atlas no longer needs to infer legacy asset-space rules;
- no gameplay occupancy semantics are invented from sprite pixels.

The remaining upstream implementation requirement is a bounded Game-owned Thais semantic export fixture/artifact with a deterministic digest under the accepted export + spatial + legacy-import + appearance profiles.

## Classification

- package version 15.25.bd5a04: **PROVEN** by package metadata
- package/archive/catalog/appearance digests: **PROVEN** by direct hashing
- object/sprite counts: **PROVEN** by deterministic decoding
- 32-pixel tile scale and draw displacement behavior: **PROVEN** by pinned renderer evidence
- selected native visual-anchor/coverage/displacement conversion: **DERIVED DESIGN CHOICE**, proposed as Game-owned source-profile authority
- gameplay occupancy/collision footprint from pixel size: **NOT AUTHORIZED / NOT INFERRED**
- project permission for exact asset digest: **OWNER_ATTESTED / PROJECT_AUTHORIZED**
- bounded Game Thais semantic export artifact: **NOT YET IMPLEMENTED**
