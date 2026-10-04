# MAP-KIND-CLASS-0: classification of the 50 Terrain records with UNKNOWN kind

Status: architect ruling and implementation packet, accepted when this merges.
Date: 2026-10-04. Base: `main` `42f0a0f8`.
Owner of the ruling: Sol Supervising Architect. Owner of the follow-up: the content lane, through
the control plane.

## 1. Problem

MAP-BUNDLE-2 (#1751) found 50 palette records routed to Terrain whose `kind` is UNKNOWN.
`tools/world-bundle-compiler/src/resolve.rs` (`terrain_of`) stops on an UNKNOWN kind, so the real
compile stops until they are classified. MAP-CUTOVER-1 cannot be allocated before that.

All 50 records share the same facts:

- Route `{owner: Terrain, reason: primarytype_world_object}`. The items.xml `primarytype` is
  `artificial tiles` or `natural tiles`, which `WORLD_OBJECT_PRIMARYTYPES`
  (`tools/content-schema/item-authoring/engine_items.py`) sends to Terrain.
- Their keys are already minted as `oteryn:terrain.tibia.i<ID>`.
- Not one of them carries `flags.bank`, `flags.fullbank`, `flags.clip` or `flags.bottom` in the
  pinned appearances (`content/assets/files/appearances-2dfa943b….dat`, read with
  `engine_items.load_appearance_objects`). None has a `magicfield` type or a name containing
  "roof".
- So no `terrain_kind` rule in `tools/content-schema/world-object-authoring/world_objects.py`
  applies, and `kind` stays UNKNOWN. That is correct: an unknown fact is never defaulted (WO-0 §4.3).

65 Terrain records in total have UNKNOWN kind. 50 of them are in the palette.

## 2. Evidence

### 2.1 None of them is ground

WO-0 §4.2 takes `ground_speed` from the bank waypoints, and format v2
(`docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md` §12) needs a speed for every ground, at least 1
when it is walkable. None of the 50 has a bank, so none has a speed, and a speed is never
defaulted. 29 of them also carry `unpass`.

Without a bank or a clip, bottom or top flag, the client draws these appearances in the common
item layer of a tile: wooden planks, mosaics, leaves and sand drifts. They are tile layers that are
not the tile's ground.

### 2.2 Placements: 342 tiles have one of these ids as their first item

This scan decodes all 1208 B3 region files of `content/world/placements/`, top-level entries only.
These ids appear 1083 times in total:

- **First item, 342 tiles.** Of these, 571 has 7, 589–592 have 24, 628 has 2 and 18566–18577 have
  309. On every one of these 342 tiles, no later top-level entry is a `ground` record, so the
  tile has no ground item. On 252 of them the id is the only top-level entry.
- **Later item, 741 placements.** The id lies on a tile that already has a ground or another layer
  below it.

On 333 of the 342 tiles the first item is `unpass` (589–592, 18566–18577), so they block movement
under any kind. The other 9 are listed here:

| Id | Positions (x, y, z) |
|---|---|
| 571 | (32331, 32721, 5), (32332, 32721, 5), (32320, 31129, 6), (32321, 31129, 6), (32322, 31129, 6), (32332, 32722, 6), (32333, 32722, 6) |
| 628 | (33865, 32247, 6) with 35046 above it, and (32146, 32830, 14) |

Each of these 9 tiles holds a plank or edge piece with no ground.

## 3. Ruling (no identity migration)

**R1. The family is unchanged.** All 65 records keep their Terrain route and their minted
`oteryn:terrain.tibia.i<ID>` keys. WO-0 §4.1 never moves a key between families, so no identity
retirement, successor key or contract amendment is needed.

**R2. The kind is `border`.** One rule is added to `terrain_kind`, after every existing rule.
A Terrain record from the `artificial tiles` or `natural tiles` primarytype that no other rule
matched is a `border`, meaning a non-ground Terrain layer.

- `border` is the existing kind for a tile layer that lies over, or in place of, a ground and is
  not one itself. Format v2 gives every non-ground kind the same shape, `walkable` and
  `ground_speed` both `null`.
- The format, the compiler and the reader are unchanged.
- The record keeps its KNOWN `walkable` (from `unpass`), `blocks_sight` and `automap` facts in
  the Terrain catalogue. A later collision slice reads them there. The bundle does not carry them
  in v2.
- `wall` was considered and rejected. It is the kind for `bottom` and `walls` items, and none of
  these ids has either.

**R3. The placements are unchanged.** The palette uses Item keys (`oteryn:item.tibia.iNNN`), which
resolve through their routed pointer. That pointer does not change.

**R4. The 342 first-item tiles are classified with the same rule.**

- Each of these tiles has a `border` and no ground. Under MAP-LOAD-PACKET-1 §1.3, a tile with no
  ground item is not walkable and has ground speed 0.
- This is the only result that no-migration allows. `ground` would need a speed the evidence
  does not have.
- It matches the Reference engines (`OTS_HYPOTHESIS_ONLY`). There, a tile whose first item is not
  a ground tile has no ground, and creatures cannot enter it.
- There is no explicit hold. A hold would leave the kind UNKNOWN and keep the compile stopped.
  On 333 of the tiles the item is `unpass` in any case.

**R5. The teleports.** Ids 628 and 878 carry `type teleport`. They stay Terrain `border` and carry
the Interaction behaviour `teleport` (WO-2c 2a, `TERRAIN_BEHAVIORS`). This packet does not decide
their destination or behaviour.

### 3.1 Per-record classification

| Source ids | Name | Appearance flags | Kind |
|---|---|---|---|
| 571, 572 | wooden floor | unmove, automap 129 | border |
| 589, 590, 591, 592 | wooden floor | unpass, unmove | border |
| 628, 878 | wooden floor (`type teleport`) | unmove | border (Interaction `teleport`) |
| 12651 | branches | unmove | border |
| 18400–18405 | sand | unpass, unmove, unsight | border |
| 18566–18577 | teal leaves | unpass, unmove | border |
| 20652 | parquet floor | unmove, light 4/138 | border |
| 20727 | parquet floor | unpass, unmove, automap 186 | border |
| 30655–30660, 30670–30673 | mosaic | unmove | border |
| 31298, 31311 | black marble floor | unpass, unmove | border |
| 31299, 31313 | stone tile | unpass, unmove | border |
| 31300, 31312 | white marble floor | unpass, unmove | border |
| 31317, 31319 | stone floor | unmove | border |
| 36081–36083 | mosaic | unmove | border |

## 4. Follow-up packet: MAP-KIND-CLASS-1

- **Mode:** ordinary implementation (`oteryn-impl-worker`), content lane.
- **Owned paths:** `tools/content-schema/world-object-authoring/world_objects.py` and its tests,
  the regenerated `content/world/terrain/` (regenerate it with the repository tools, never by
  hand), and the task record.
- **Change:** add the R2 rule as the last `terrain_kind` rule, with tests for these cases:
  - a walkable tile, an `unpass` tile and a `type teleport` tile all become `border`;
  - a bank, clip, bottom, field or roof record keeps its kind;
  - a non-tile primarytype stays unmatched.
- **Acceptance (real map, `parity .`):**
  1. `unknown_kind` 0 and `refused` 0.
  2. Placed entries by kind: ground 2016, border 3503 (3453 + 50), wall 1819, roof 198, field 102.
  3. `null` unchanged at 7719 and 4673.
  4. No Item, Terrain or palette key changes.
  5. Every Terrain record already carrying a kind keeps it.
  6. The real compile succeeds.
  7. Once the map-backed `GroundSpeedSource` (MAP-LOAD-1) exists, it returns 0 for the 342 tiles
     of §2.2.
- **Review:** no protocol, persistence, identity or authority change, so ordinary review applies.
- **Order:** MAP-KIND-CLASS-1 comes before MAP-CUTOVER-1.

## 5. Rejected options

- **Reroute to WorldObject.** WO-0 §4.1 forbids moving minted Terrain keys to another family. The
  alternative is an identity migration, which needs retirement, a contract amendment and an
  independent identity review, for no gain in v2 semantics.
- **Ground with an assumed speed.** This defaults an unknown fact, which WO-0 forbids, and it makes
  an `unpass` tile a ground.
- **A new Terrain kind `overlay`.** v2 readers reject an unknown kind, so this would need a new
  format version and a compiler change for the same `null` semantics.
- **Hold the 342 first-item tiles.** This keeps the real compile blocked, and the evidence already
  decides that these tiles have no ground.

## 6. Decision test

1. **Must this be decided now?** Yes. The real compile stops on these records, and MAP-CUTOVER-1
   waits on that.
2. **What work is blocked?** MAP-CUTOVER-1, and the real compile with a parity run.
3. **Is it harder later?** Yes. Once a bundle is cut over, a kind change needs a new bundle at a
   planned World reset (format §12).
4. **What evidence would supersede this?** Two kinds of evidence:
   - a Reference proof (tibia.com or TibiaWiki) that one of the 9 walkable-flagged first-item
     tiles of §2.2 can be walked on, together with a speed source for it;
   - a Terrain field in a later format version that distinguishes layer kinds.
5. **What is not decided?** Teleport destinations and behaviour (Interaction), light behaviour,
   collision from the record facts, and any change to format v2.
