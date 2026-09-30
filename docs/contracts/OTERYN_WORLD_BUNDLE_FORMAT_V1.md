# Oteryn World Bundle format v1

- Format ID: `OTERYN_WORLD_BUNDLE/v1`
- Owner: `Oteryn/Oteryn-Game`, MAP-BUNDLE-1 (ADR-0021 §5)
- Status: **CANDIDATE.** ADR-0005 §3 requires an accepted schema contract before a runtime
  treats a serializer as a permanent format. This document is accepted together with
  MAP-BUNDLE-1: it is reviewed with MAP-BUNDLE-1a (this layout) and becomes the contract
  when MAP-BUNDLE-1b closes the OPEN items in §10. MAP-BUNDLE-1b lands in two PRs: 1b-1 (the
  families, the teleport split and the parity report, §10) and 1b-2 (key resolution against
  the Item, Terrain and WorldObject registries and the end-to-end compile of the real map).
- Implementation: `tools/world-bundle-compiler` (writer, reader, compiler, key resolver and
  the `parity` and `compile` commands).
- Runtime reader: none yet (MAP-LOAD-1).
- Governing: ADR-0021 §4.2-§4.6 and §4.8 (D188-D196); ADR-0005 §3; DUR-04 §9;
  `OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1`; `OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1`.

## 1. Scope

A server World Bundle is the one file the game server reads for a World's base map
(ADR-0021 §4.2). The compiler produces it from the World Project: the B3 placements
(`OTERYN_WORLD_REGION_B3/v1`) and the World, Transition.Teleport and House families and the
minimap draft areas (§10). Floor changes come from the catalogue's `floor_change` fact of the
placed object, not from a family of their own (#1170). The bundle is immutable and content-addressed by its digest (§6).

This document fixes the byte layout, the manifest, the checksums and digest, the placement key,
`build_class`, versioning and the reader limits. It does not decide the in-memory model, the
loader or the overlay (MAP-LOAD-1, MAP-OVERLAY-1).

## 2. File layout

All integers are little endian unless the payload grammar (§5) says varint.

| Offset | Size | Field | Rule |
|---|---|---|---|
| 0 | 4 | magic | `"OTWB"` |
| 4 | 2 | `format_version` u16 | `1` |
| 6 | 2 | reserved u16 | `0` |
| 8 | 4 | `manifest_length` u32 | at most `MAP01-BUNDLE-MANIFEST-BYTES` |
| 12 | 4 | `sector_count` u32 | at most `MAP01-BUNDLE-SECTOR-COUNT` |
| 16 | `manifest_length` | manifest | JSON, written canonically (§3) |
| … | 50 × `sector_count` | sector table | §4 |
| … | Σ `compressed_length` | sector frames | back to back, in table order |
| end − 32 | 32 | digest | §6 |

Nothing may follow the digest, and no byte may lie between the table and the first frame or
between two frames. The whole file is at most `MAP01-BUNDLE-FILE-BYTES`.

## 3. Manifest

The manifest is one UTF-8 JSON object. The writer emits it in the field order below without
whitespace (a writer-only rule: the reader accepts any valid JSON of this shape, in any field or
whitespace order, and byte identity is guaranteed by the digest, §6). A
reader rejects unknown fields at every level (fail closed); a new field needs a new
`format_version`.

| Field | Meaning |
|---|---|
| `format` | `"OTERYN_WORLD_BUNDLE/v1"` |
| `min_reader_version` | lowest reader `format_version` able to read the file; `1` |
| `projection_class` | `"server"` (ADR-0021 §4.2). A client projection is not part of v1. |
| `compiler_version` | set by the compiler, never by its caller: `oteryn-world-bundle-compiler/<crate version> zstd/<library version>` (§6) |
| `build_class` | `"production"` or `"non-production"`, §8 |
| `identity` | the compiler inputs copied unchanged: `project_format_version`, `world_schema_version`, `content_revision`, `content_lock_digest`, `min_runtime_version`, `required_capabilities` (list), `ruleset_compatibility` (list), `provenance_summary` (ADR-0005 §3, DUR-04 §9) |
| `world` | `min_x`, `min_y`, `max_x`, `max_y` (native, half-open) and `floors`: native floors strictly ascending within `-15..0` (ADR-0021 §4.3) |
| `palette` | list of `{key, family, id}`; the list index is the bundle palette index used by the payloads |
| `draft_areas` | keys of the draft areas compiled in, sorted and unique; empty in a production bundle |
| `skipped_provisional_keys` | provisional keys skipped in this build, sorted and unique; empty in a production bundle |
| `dropped_teleports` | placement keys (§7, JSON numbers) of the top-level entries whose zero-destination `teleport` attribute the compiler dropped (§10, OPEN-3), strictly ascending; each must name a top-level entry of the bundle. Such an entry is never materialized (ADR-0021 §4.4). Allowed in a production bundle |

`palette[i].key` is the stable World Project key, `family` is `item` or `terrain` (§10,
OPEN-1), and `id` is the compact id of that key in `identity.content_revision`: the index of the
key among all keys of its family in that revision (the Item registry for `item`, the Terrain
catalogue for `terrain`), in ascending byte order. A reader of the same revision derives the
same ids; they are not stable across revisions. The palette holds only
entries that some kept item uses, in ascending order of their World Project palette index, so
two builds of the same input produce the same palette.

## 4. Sector table

One 50-byte row per non-empty sector (at least one tile), strictly ascending by
`(floor, sy, sx)`. The row's floor must be one of the manifest's `world.floors`:

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `floor` i8, native |
| 1 | 1 | reserved, `0` |
| 2 | 2 | `sx` u16, sector x = `x / 32`, below 2048 |
| 4 | 2 | `sy` u16, sector y = `y / 32`, below 2048 |
| 6 | 4 | `offset` u32 of the frame from the start of the file |
| 10 | 4 | `compressed_length` u32, non-zero |
| 14 | 4 | `raw_length` u32, the exact payload length |
| 18 | 32 | SHA-256 of the frame bytes |

Sectors are 32x32 tiles, the B3 sector size (ADR-0021 §4.2). A bundle has no region level: the
sector coordinate is absolute.

## 5. Sector payload

Each frame is exactly one zstd frame (the writer uses level 3): no skippable or second frame, the content
checksum flag set, and a declared content size equal to `raw_length`. It decompresses to exactly
`raw_length` bytes of the B3 sector grammar
(`world_region_codec.py`, `OTERYN_WORLD_REGION_B3/v1`), unchanged except for two meanings:

- the item palette index points into the bundle `palette` (§3), not the World Project
  `index.json`;
- the `teleport` floor byte holds the native floor as an `i8`, not legacy `z`.

The grammar in short (varints are LEB128): `tile_count`, then per tile ascending by `(y, x)` the
position delta, `control = item_count << 3 | zones << 2 | house << 1 | flags`, the present
fields, and the items. An item is `palette << 1 | has_mask`, then the attribute mask
(bit 0 depth, then count, action, unique, door, text, charges, description, teleport, depot)
and the present values in bit order. Container contents follow their container one depth
deeper. A present attribute with value 0 or an empty text stays present.

Varints must be canonical: no redundant zero high group and no bits past 64. With the frame
rules above, the raw payload of a given content has exactly one byte encoding. The frame bytes
themselves are not unique across zstd versions or levels; `compiler_version` fixes them (§6). The writer decodes every payload it
produced and writes it only if it reads back to the same tiles within the same limits, so it
never writes what a reader rejects. The B3 reader applies the same varint rule; the B3 encoder
already writes canonical varints.

## 6. Checksums and digest

- **Per sector.** The table row holds the SHA-256 of the stored frame. A reader checks it before
  it decompresses the frame, so a corrupt frame never reaches the decompressor.
- **Bundle digest.** `SHA-256("OTERYN_WORLD_BUNDLE/v1" || 0x00 || file[0 .. len − 32])`, stored
  as the last 32 bytes. It covers the header, the manifest (including `build_class` and the
  content revision), the table and every frame. It is the bundle identity used for pinning
  (ADR-0021 §4.2), the Ground `map_revision` (§4.4) and `MapItemMaterialization`.
- **Byte identity.** The same inputs and the same `compiler_version` produce the same bytes. The
  compiler derives `compiler_version` from its own crate version and the linked zstd library
  version, because a different zstd may compress the same payload into different bytes.

## 7. Placement key

A placement key names one top-level base entry of the bundle (ADR-0021 §4.2 and §4.4):

```text
placement_key = x << 32 | y << 16 | (−floor) << 8 | ordinal        (u64)
```

`ordinal` is the index of the entry among the top-level entries (depth 0) of its tile in
payload order, below 64 (`MAP01-TILE-BASE-ENTRIES`). A floor outside `-15..0` or an ordinal of
64 or more has no key. Contents of a container have no key of
their own; they belong to their top-level entry.

The key is not stored. The compiler emits it by fixing the tile order and the ordinal, and
every reader derives the same value. It is bound to the bundle digest: it names an entry only
together with the digest of the bundle it came from, and `MapItemMaterialization` and the wire
carry both. It is not a canonical identity and is not stable across bundles (import profile
§8). A provisional entry skipped in a non-production build takes no ordinal. A dropped
zero-destination teleport inside a container is listed under its top-level entry's key; one
inside a skipped provisional entry has no key and is only reported.

## 8. Build class

- `production`: every palette key resolved, no draft area and no skipped provisional key. The
  writer refuses such a manifest otherwise, and the reader rejects it.
- `non-production`: may carry draft areas and skipped provisional keys, each listed in the
  manifest.
- A missing `build_class` and any other string value read as `non-production`. A value that is
  not a string (for example `null`) makes the manifest malformed, and the bundle is rejected. A World deployed as
  production refuses every bundle that is not `production` (ADR-0021 §4.2, §4.6). That check
  belongs to the loader (MAP-LOAD-1).

## 9. Reader rules and limits

A reader rejects the whole bundle on the first failure; there is no partial load. In order:
file size, header, digest, manifest length and sector count, manifest, then per row the
reserved byte, contiguity, the raw and ratio limits, the running raw total, the row floor
against `world.floors`, ascending order, the frame checksum, the single canonical frame,
decompression into exactly `raw_length` bytes, the sector coordinate range, the payload grammar
with the per-bundle tile and entry budget, a non-empty sector, palette indices, tile positions
and teleport destinations inside the World extent, and the top-level entry limit; after the last
frame, every `dropped_teleports` key against the decoded entries. Every size is checked before
memory is reserved for it.

| Limit | Hard maximum |
|---|---|
| `MAP01-BUNDLE-FILE-BYTES` | 1 GiB file |
| `MAP01-BUNDLE-MANIFEST-BYTES` | 16 MiB manifest |
| `MAP01-BUNDLE-SECTOR-COUNT` | 1,048,576 sectors |
| `MAP01-BUNDLE-SECTOR-RAW-BYTES` | 16 MiB per sector payload, at most 1,024 times its frame |
| `MAP01-BUNDLE-TOTAL-RAW-BYTES` | 1 GiB of payloads per bundle |
| `MAP01-BUNDLE-TILES` | 33,554,432 decoded tiles per bundle |
| `MAP01-BUNDLE-ENTRIES` | 67,108,864 decoded entries per bundle, container contents included |
| `MAP01-TILE-BASE-ENTRIES` | 64 top-level entries per tile (ADR-0021 §4.4) |
| `MAP01-TILE-ENTRIES` | 4,096 entries per tile, container contents included |
| `MAP01-ITEM-TEXT-BYTES` | 4,096 bytes per `text` or `description` |

The rows are in `RESOURCE_LIMITS_REGISTRY.json`. The per-tile, text and per-bundle tile and entry
limits were set before the real map was read. The MAP-BUNDLE-1b parity run on the #1170 map
(head `98ba6938`) confirms them: 19,373,519 tiles, 24,983,331 entries, at most 26 entries and 26
top-level entries on a tile, and at most 3,859 bytes in one `text` or `description`. All are
inside the limits, so the values stay.

The compiler fails closed as well (ADR-0021 §4.3, §4.5). It resolves every entry, including the
contents of a provisional entry it skips, and stops on an unknown key, a position or teleport
destination outside the declared World, a legacy `z` above 15, a sector given twice, a
provisional key in a production build, and any limit above. Key resolution fails closed as
§10 OPEN-1 says: a key that is neither an Item key, a Terrain key without an Item record nor a
flagged provisional key; a Terrain or WorldObject identity given twice; an `item_pointer` whose
whole typed reference (key and revision) names no Item record, or that names an Item another
record already names; and an Item `routed_to` that no `item_pointer` confirms or that names
another record. It also stops when a placement
disagrees with a family (§10): a teleport with a real destination and no Transition.Teleport
record from its tile, or a record to another destination (a (0,0,0) attribute included); a
Transition.Teleport record whose
tile carries no teleport attribute; and a tile house id that no House record has as its engine
house id.

## 10. Open items

OPEN-1 to OPEN-3 were answered by the Sol Supervising Architect on #162 (5910173902, with
5915258560 for the route); the answers are recorded below. MAP-BUNDLE-1b-1 implements OPEN-3
and closes OPEN-4; MAP-BUNDLE-1b-2 implements OPEN-1 and OPEN-2 (key resolution).

- **OPEN-1, palette key families. Answered (Q1b) and implemented (1b-2).** A palette entry is
  the id's canonical A12 key:
  - An Item key resolves to `family` `item` and its Item compact id. Its route to a Terrain or
    WorldObject record is the catalogue record whose `item_pointer` names it (ruling 5915258560
    (b)). The pointers must be one-to-one and name existing Item records by their whole typed
    reference, revision included. Catalogue identities are unique across shards. When an Item carries
    `routed_to` (WO-2b), it must name that same record; a disagreement, or a `routed_to` that no
    pointer confirms, fails compilation. WO-2b can add `routed_to` later without changing the
    compiled result. An Item that no record points at is a plain Item.
  - A Terrain key resolves to `family` `terrain` only when its record has no `item_pointer`, the
    id without an Item record of Q2a. A Terrain or WorldObject key whose record points at an
    Item is rejected: the Item key is the palette key. A WorldObject is reached only through
    its Item, so there is no `world-object` family and no ADR amendment.
  - A key the placements index flags `provisional` is skipped (non-production) or fails
    (production), §8. A registry key always resolves, even when flagged.

  On `main` (33,567 Item records; 8,548 Terrain and 12,782 WorldObject records, every one with
  an `item_pointer`), every pointer is one-to-one and no Item carries `routed_to` yet.
- **OPEN-2, appearance-only terrain keys. Answered (Q2a).** The World Project generator mints
  `oteryn:terrain.tibia.i<id>` for the 5,949 ids without an Item key; `oteryn:terrain.aNNNNNN` is
  not used. Only manifest keys change, not the layout. The resolver accepts such a key once
  its Terrain record, without an `item_pointer`, is in the catalogue (1b-2). Until the #1170
  palette is regenerated with them, those ids stay provisional donor keys (below).
- **OPEN-3, orphan teleports. Answered (Q3 split) and implemented (1b-1).** A teleport
  attribute is compared with the Transition.Teleport record from its tile (project frame):
  - a record to the same destination: kept, with the destination floor mapped to native;
  - destination (0,0,0): not a teleport. The attribute is dropped with a diagnostic, and the
    entry's placement key goes into `dropped_teleports` (§3). The tile keeps its `action` and
    `unique` bindings. The list is a production release parity report, not a release blocker;
  - a real destination with no record from its tile, or a record to another destination:
    compilation fails. So does a record whose tile has no teleport attribute.

  **Counts** (`oteryn-world-bundle-compiler parity`, #1170 head `98ba6938` with the #1160
  Transition.Teleport family, 872 records, stacked on it; the House catalogue of `main`):

  | Class | Count | Result |
  |---|---:|---|
  | teleport attributes on the map | 2,455 | |
  | matched by a Transition.Teleport record | 872 | kept |
  | without a record, destination (0,0,0) | 1,577 | dropped with a diagnostic |
  | without a record, real destination | 6 | compilation fails |
  | record to another destination | 0 | compilation fails |
  | record without a teleport attribute | 0 | compilation fails |

  The two orphan classes sum to 1,583. The earlier 1,576 was an estimate before the gap fills
  of #1170. The six real destinations, as legacy `(x, y, z)` from and to, are
  `(30880, 32520, 8) → (30910, 32517, 7)`, `(30948, 32592, 8) → (30976, 32633, 7)`,
  `(33082, 31045, 6) → (195, 61836, 7)`, `(33708, 32375, 15) → (1041, 1008, 7)`,
  `(33733, 32359, 15) → (1081, 989, 7)` and `(33744, 31065, 8) → (988, 126, 9)`. The two counts
  use different references. Against the declared World extent, the last four destinations lie
  outside it and the first two inside. Against the source map, as the #1160 capture counts,
  one destination (`y` 61836) lies outside the map and the other five land on tiles the map
  does not have. The real map therefore does not compile until content marks the four
  outside the World as not teleports and, for the other two, the Transition generator adds
  records or content marks them too (rulings Q3 and 5915258560). That is content work, not a
  compiler change. No tile carries a house id missing from the House catalogue.
- **Real-map compile and equivalence (1b-2).** `oteryn-world-bundle-compiler compile` reads the
  placements, the World, Transition.Teleport and House families and the content registry of a
  repository root, compiles, then proves the bundle against its source authorities before it
  writes it, never against the bundle's own claims. Each manifest palette entry must be the
  `(family, id)` the resolver gives its key. Tile by tile, every source tile must be at its
  native position with the same flags, house and zones, and every entry must keep its depth,
  attributes and palette key. The only exceptions are the rules above: subtrees under keys the
  resolver calls provisional are skipped, (0,0,0) teleports are dropped, and teleport floors are
  native. The bundle holds no other tile, and `skipped_provisional_keys` is exactly the set of
  provisional keys met. The run on #1170 head `2ffba017` (with #1160 `ee19179e` merged in) stops, as it
  must, at the first real-destination orphan. In a scratch copy with only those six
  attributes removed, as the content fix will do, it compiles a non-production bundle in 37 s
  with a peak RSS of 3.8 GB:
  - 19,373,519 tiles and 24,168,528 entries proven equivalent;
  - 814,803 entries skipped under 5,995 donor keys that the #1170 palette flags provisional.
    None of these ids has an Item record; OPEN-2 counted 5,949 such ids. ADR-0021 §4.5
    expects five provisional keys, so a production build waits on the OPEN-2 regeneration;
  - 1,577 teleports dropped;
  - all 19,989 other palette keys resolved as Item keys;
  - 23,721,569 bytes, the same bytes on a second run.

  A production build of the same input fails on the first provisional key.
- **OPEN-4, draft marker. Closed (1b-1).** There is no per-tile draft marker in v1. The drafted
  tiles are already merged into the B3 regions, and a tile does not record that it was
  drafted. The compiler reads the draft areas from the placements index
  (`source.minimap_draft.areas[].name`; 7 on #1170) and lists them in `draft_areas`. A
  production build fails while the World Project declares any draft area (§8), so a production
  World never loads drafted tiles, and the drafts stay a release blocker (ADR-0021 §4.6). Tile
  gating inside a non-production World is not needed: such a World loads the whole bundle.

## 11. Versioning

- `format_version` in the header and `format` in the manifest name the layout. A change of the
  layout, the manifest fields, the payload grammar or the digest rule is a new version.
- A reader accepts only versions it knows and a manifest whose `min_reader_version` it meets.
- Content changes do not change the format: a new content revision or map produces a new bundle
  with a new digest, activated only at a planned world reset (ADR-0021 §4.7).
