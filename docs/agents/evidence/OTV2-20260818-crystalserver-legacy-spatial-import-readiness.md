# CrystalServer legacy spatial import readiness — 2026-08-18

## Verdict

**READY_FOR_EXACT_HEAD_REVIEW** for the bounded semantic importer profile.

This evidence does not authorize asset redistribution, Game exporter runtime, Atlas runtime, physical serialization, or production use.

## Trusted revisions

- Game authority: `blakinio/Oteryn-v2@16c665223c1256cc7e4a8a97cf2bc34cd278423c`
- Native spatial contract: `docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md`
- Game -> Atlas export contract: `docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md`
- Legacy reference: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`
- CrystalServer content revision: `zimbadev/crystalserver@5e89bf8329ea406cb4ea8f4a18f32954f13e5418`
- legacy world SHA-256: `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`
- Otheryn semantic parser blob: `a11343a472145aee4d9cf65c6ce28b3e4a71a2b3`
- Otheryn renderer blob: `9d66d9ff2128663eda9eb07943c60ee4ef79c9e3`
- independent OTClient floor-direction evidence: `opentibiabr/otclient@dd5641492a71e966b96b8a91398b44bb3df67d88`, `modules/game_cyclopedia/tab/map/map.lua`, blob `15cc5423d2fece71159c6395c54cd94b098760c4`

## Evidence findings

### Horizontal axes

The pinned legacy renderer places `(x - min_x) * 32` on the horizontal image axis and `(y - min_y) * 32` on the vertical image axis. This is consistent with the accepted native +X east/right and +Y south/down semantic orientation.

**Result:** identity x/y conversion is evidence-supported for this source family.

### Vertical order

The accepted native profile requires larger `FloorId` values to be geometrically higher/above. Independent OTClient map code defines `SEA_FLOOR = 7`, describes floors `0..7` as surface/above and `8..15` as underground, and explicitly describes `z < 7` as floors above Z7. This directly establishes that the legacy Z numerical direction is opposite the native vertical-order direction.

Selected mapping:

```text
FloorId = -legacy_z
```

Negation is therefore a representation adapter that preserves vertical order without promoting legacy surface numbering or assigning universal meaning to native floor zero.

For legacy populated `Z=0..15`, the native valid floor set is `[-15..0]` expressed strictly increasing.

**Result:** legacy Thais `Z=7` maps to native `FloorId=-7`.

### Bounds

Otheryn durable handover records the mandatory historical Thais region:

- `X=32280..32440` inclusive;
- `Y=32155..32305` inclusive;
- `Z=7`.

Native profile rectangles are half-open, therefore the exact mapped selection is:

```text
x=[32280,32441)
y=[32155,32306)
floor=-7
```

This preserves the historical 161 x 151 tile-position rectangle.

### Visible same-position order

Pinned `semantic.py` constructs a tile as:

- optional compact ground;
- remaining compact items;
- top-level child ITEM nodes in preserved source order;
- nested item children retained inside their parent item.

Pinned `render.py` renders optional ground followed by `tile.items` and explicitly states that nested item children are container contents, not visible map stack entries.

**Result:** the importer can preserve the evidenced legacy visible sequence by assigning `PresentationOrderKey { plane: 0, order: sequence_index }` per tile. Atlas consumers receive the explicit native order key and do not infer legacy order themselves.

## Negative boundaries

The evidence does **not** establish:

- canonical stable Game identities for every legacy item ID;
- collision, interaction, targeting, movement or mutation ordering;
- native appearance anchor/footprint/displacement semantics for the Tibia asset set;
- redistribution rights for Tibia/CipSoft assets;
- a permanent physical Game -> Atlas serializer/chunk profile.

Those remain separate gates.

## DYN-ATLAS effect

After merge, DYN-ATLAS-001 may treat the historical Thais rectangle as a reproducible legacy source selection and map it into native coordinates/floor/presentation order through the Game-owned profile.

It still must not:

- parse OTBM in browser runtime;
- infer canonical identities from legacy IDs;
- publish proprietary asset pixels without a separate rights gate;
- claim complete Game -> Atlas compatibility without a bounded Game-owned semantic export fixture/digest and any required appearance profile.

## Classification

- coordinate orientation: **PROVEN**
- legacy Z numerical direction: **PROVEN** by independent OTClient floor semantics
- Thais legacy bounds: **PROVEN**
- parser-visible top-level sequence: **PROVEN**
- `FloorId=-legacy_z`: **DERIVED DESIGN CHOICE preserving proven legacy/native order**, now proposed as Game-owned importer authority
- presentation plane `0` + order index: **DERIVED DESIGN CHOICE preserving pinned legacy draw order**, now proposed as Game-owned importer authority
- asset redistribution: **UNKNOWN / NOT AUTHORIZED**
- appearance spatial profile: **UNKNOWN**
- Game semantic fixture/export artifact: **NOT IMPLEMENTED**
