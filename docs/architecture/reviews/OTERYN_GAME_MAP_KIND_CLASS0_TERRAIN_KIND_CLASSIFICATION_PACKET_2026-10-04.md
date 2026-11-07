# MAP-KIND-CLASS-0: classification of the 50 Terrain records with UNKNOWN kind

Status: architect ruling and implementation packet, accepted when this merges. It amends
WO-0 §4.2 and format v2 §12 through MAP-KIND-CLASS-1 (§3 R2, §4).
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

**R2. A new Terrain kind, `common`.** WO-0 §4.2 and format v2 §12 gain a sixth kind, `common`.
It is a Terrain tile-layer item with no bank, clip or bottom flag, drawn in the client's common item
layer. It is not a ground, a border, a wall, a roof or a field.

- **Converter.** One rule is added to `terrain_kind`, after every existing rule. A Terrain record
  from the `artificial tiles` or `natural tiles` primarytype that no other rule matched is
  `common`.
- **Border stays reserved for `flags.clip`.** The render disposition keeps borders apart from
  common items, and `common` keeps that apart in the catalogue and the bundle.
- **Bundle.** `common` has the shape of every non-ground kind: `walkable` and `ground_speed` both
  `null`. A `common` entry is never the tile's ground item (MAP-LOAD-PACKET-1 §1.3).
- **Record facts.** The record keeps its KNOWN `walkable` (from `unpass`), `blocks_sight` and
  `automap` facts in the Terrain catalogue, and a later collision slice reads them there.
- **Format and contract.** Adding `common` amends the closed kind set of v2. It does not add a new
  format version, because no v2 bundle has been published or activated and no server reads one on
  `main` (`eead8574`). MAP-BUNDLE-2's compiler and its tests are the only v2 reader and writer.
  - The amendment is made before MAP-CUTOVER-1 publishes the first v2 bundle.
  - Format §11 and §12 still require a planned World reset for any kind change after
    publication.
  - The `format_version` and `min_reader_version` stay at 2, and the digest domain is unchanged.
  - The owning contract change and its independent format review belong to MAP-KIND-CLASS-1
    (§4).
- **Rejected for R2.** `wall` is the kind for `bottom` and `walls` items, and none of these ids has
  either.

**R3. The placements are unchanged.** The palette uses Item keys (`oteryn:item.tibia.iNNN`), which
resolve through their routed pointer. That pointer does not change.

**R4. The 342 first-item tiles are classified with the same rule.**

- Each of these tiles has a `common` entry and no ground. Under MAP-LOAD-PACKET-1 §1.3, a tile with no
  ground item is not walkable and has ground speed 0.
- This is the only result that no-migration allows. `ground` would need a speed the evidence
  does not have.
- It matches the Reference engines (`OTS_HYPOTHESIS_ONLY`). There, a tile whose first item is not
  a ground tile has no ground, and creatures cannot enter it.
- There is no explicit hold. A hold would leave the kind UNKNOWN and keep the compile stopped.
  On 333 of the tiles the item is `unpass` in any case.

**R5. The teleports.** Ids 628 and 878 carry `type teleport`. They stay Terrain `common` and carry
the Interaction behaviour `teleport` (WO-2c 2a, `TERRAIN_BEHAVIORS`). This packet does not decide
their destination or behaviour.

### 3.1 Per-record classification

| Source ids | Name | Appearance flags | Kind |
|---|---|---|---|
| 571, 572 | wooden floor | unmove, automap 129 | common |
| 589, 590, 591, 592 | wooden floor | unpass, unmove | common |
| 628, 878 | wooden floor (`type teleport`) | unmove | common (Interaction `teleport`) |
| 12651 | branches | unmove | common |
| 18400–18405 | sand | unpass, unmove, unsight | common |
| 18566–18577 | teal leaves | unpass, unmove | common |
| 20652 | parquet floor | unmove, light 4/138 | common |
| 20727 | parquet floor | unpass, unmove, automap 186 | common |
| 30655–30660, 30670–30673 | mosaic | unmove | common |
| 31298, 31311 | black marble floor | unpass, unmove | common |
| 31299, 31313 | stone tile | unpass, unmove | common |
| 31300, 31312 | white marble floor | unpass, unmove | common |
| 31317, 31319 | stone floor | unmove | common |
| 36081–36083 | mosaic | unmove | common |

## 4. Follow-up packet: MAP-KIND-CLASS-1

- **Mode:** ordinary implementation (`oteryn-impl-worker`), content and map lanes, one writer. It
  needs an independent format review because it amends a public contract.
- **Owned paths:**
  - `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md`: §12 gains `common` in the kind set and the
    fail-closed lists, and gets an "Amended by MAP-KIND-CLASS-0" line;
  - `docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md`:
    an Amends line, and §4.2 `kind` gains `roof` (WO-2c 1a) and `common`;
  - `tools/content-schema/world-object-authoring/terrain.schema.json`, `world_objects.py` and
    their tests;
  - `tools/world-bundle-compiler/src/resolve.rs`, `src/bundle.rs` and their tests;
  - the regenerated `content/world/terrain/` (regenerate it with the repository tools, never by
    hand);
  - the task record.
- **Change:**
  1. Add the R2 rule as the last `terrain_kind` rule.
  2. Add `common` to the schema enum and to `TerrainKind`, with `walkable` and `ground_speed`
     both `null`.
  3. The reader and writer accept `common` with that shape and refuse it with any other shape.
- **Tests:**
  - a walkable tile, an `unpass` tile and a `type teleport` tile all become `common`;
  - a bank, clip, bottom, field or roof record keeps its kind;
  - a non-tile primarytype stays unmatched;
  - the compiler accepts `common`, and the reader refuses `common` with a non-null member;
  - a tile whose only top-level entry is `common` has no ground item.
- **Acceptance (real map, `parity .`):**
  1. `unknown_kind` 0 and `refused` 0.
  2. Placed entries by kind: ground 2016, border 3453, wall 1819, roof 198, field 102, common 50.
  3. `null` unchanged at 7719 and 4673.
  4. No Item, Terrain or palette key changes.
  5. Every Terrain record already carrying a kind keeps it.
  6. The real compile succeeds, with `format_version` 2.
  7. Once the map-backed `GroundSpeedSource` (MAP-LOAD-1) exists, it returns 0 for the 342 tiles
     of §2.2.
- **Review:** an independent format review for the contract amendment. There is no protocol,
  persistence, identity or authority change.
- **Order:** MAP-KIND-CLASS-1 comes before MAP-CUTOVER-1. It must merge before the first v2 bundle
  is published.

## 5. Rejected options

- **Reroute to WorldObject.** WO-0 §4.1 forbids moving minted Terrain keys to another family. The
  alternative is an identity migration, which needs retirement, a contract amendment and an
  independent identity review, for no gain in v2 semantics.
- **Ground with an assumed speed.** This defaults an unknown fact, which WO-0 forbids, and it makes
  an `unpass` tile a ground.
- **Label them `border`.** `border` is reserved for `flags.clip`, and the render disposition keeps
  borders apart from common items. The label would write a false fact into 65 records (#1756 P1
  4177466160).
- **A new format version for `common`.** No v2 bundle exists yet, so amending v2 before its first
  publication costs no reader and no migration.
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
   - a v2 bundle published before MAP-KIND-CLASS-1 merges, which would make `common` need a new
     format version.
5. **What is not decided?** Teleport destinations and behaviour (Interaction), light behaviour,
   collision from the record facts, and the render order inside the common layer.
