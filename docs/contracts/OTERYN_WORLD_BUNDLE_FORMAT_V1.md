# Oteryn World Bundle format v1

- Format ID: `OTERYN_WORLD_BUNDLE/v1`
- Owner: `Oteryn/Oteryn-Game`, MAP-BUNDLE-1 (ADR-0021 §5)
- Status: **CANDIDATE.** ADR-0005 §3 requires an accepted schema contract before a runtime
  treats a serializer as a permanent format. This document is accepted together with
  MAP-BUNDLE-1: it is reviewed with MAP-BUNDLE-1a (this layout) and becomes the contract
  when MAP-BUNDLE-1b closes the OPEN items in §10.
- Implementation: `tools/world-bundle-compiler` (writer, reader and compiler skeleton).
- Runtime reader: none yet (MAP-LOAD-1).
- Governing: ADR-0021 §4.2-§4.6 and §4.8 (D188-D196); ADR-0005 §3; DUR-04 §9;
  `OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1`; `OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1`.

## 1. Scope

A server World Bundle is the one file the game server reads for a World's base map
(ADR-0021 §4.2). The compiler produces it from the World Project: the B3 placements
(`OTERYN_WORLD_REGION_B3/v1`) and, in MAP-BUNDLE-1b, the World, FloorChange, Transition, House
and area families. The bundle is immutable and content-addressed by its digest (§6).

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
| 16 | `manifest_length` | manifest | canonical JSON (§3) |
| … | 50 × `sector_count` | sector table | §4 |
| … | Σ `compressed_length` | sector frames | back to back, in table order |
| end − 32 | 32 | digest | §6 |

Nothing may follow the digest, and no byte may lie between the table and the first frame or
between two frames. The whole file is at most `MAP01-BUNDLE-FILE-BYTES`.

## 3. Manifest

The manifest is one UTF-8 JSON object written in the field order below, without whitespace. A
reader rejects unknown fields at every level (fail closed); a new field needs a new
`format_version`.

| Field | Meaning |
|---|---|
| `format` | `"OTERYN_WORLD_BUNDLE/v1"` |
| `min_reader_version` | lowest reader `format_version` able to read the file; `1` |
| `projection_class` | `"server"` (ADR-0021 §4.2). A client projection is not part of v1. |
| `build_class` | `"production"` or `"non-production"`, §8 |
| `identity` | the compiler inputs copied unchanged: `project_format_version`, `world_schema_version`, `content_revision`, `content_lock_digest`, `compiler_version`, `min_runtime_version`, `required_capabilities` (list), `ruleset_compatibility` (list), `provenance_summary` (ADR-0005 §3, DUR-04 §9) |
| `world` | `min_x`, `min_y`, `max_x`, `max_y` (native, half-open) and `floors`: native floors strictly ascending within `-15..0` (ADR-0021 §4.3) |
| `palette` | list of `{key, family, id}`; the list index is the bundle palette index used by the payloads |
| `draft_areas` | keys of the draft areas compiled in; empty in a production bundle |
| `skipped_provisional_keys` | provisional keys skipped in this build, sorted and unique; empty in a production bundle |

`palette[i].key` is the stable World Project key, `family` is `item` or `terrain` (OPEN-1),
and `id` is the compact id of that key in `identity.content_revision`. The palette holds only
entries that some kept item uses, in ascending order of their World Project palette index, so
two builds of the same input produce the same palette.

## 4. Sector table

One 50-byte row per non-empty sector, strictly ascending by `(floor, sy, sx)`:

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

Each frame is one zstd frame at level 3 with the content checksum and content size flags set.
It decompresses to exactly `raw_length` bytes of the B3 sector grammar
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

## 6. Checksums and digest

- **Per sector.** The table row holds the SHA-256 of the stored frame. A reader checks it before
  it decompresses the frame, so a corrupt frame never reaches the decompressor.
- **Bundle digest.** `SHA-256("OTERYN_WORLD_BUNDLE/v1" || 0x00 || file[0 .. len − 32])`, stored
  as the last 32 bytes. It covers the header, the manifest (including `build_class` and the
  content revision), the table and every frame. It is the bundle identity used for pinning
  (ADR-0021 §4.2), the Ground `map_revision` (§4.4) and `MapItemMaterialization`.
- **Byte identity.** The same inputs and the same `compiler_version` produce the same bytes. The
  zstd library version is part of `compiler_version`, because a different zstd may compress the
  same payload into different bytes.

## 7. Placement key

A placement key names one top-level base entry of the bundle (ADR-0021 §4.2 and §4.4):

```text
placement_key = x << 32 | y << 16 | (−floor) << 8 | ordinal        (u64)
```

`ordinal` is the index of the entry among the top-level entries (depth 0) of its tile in
stacking order, below 64 (`MAP01-TILE-BASE-ENTRIES`). Contents of a container have no key of
their own; they belong to their top-level entry.

The key is not stored. The compiler emits it by fixing the tile order and the ordinal, and
every reader derives the same value. It is bound to the bundle digest: it names an entry only
together with the digest of the bundle it came from, and `MapItemMaterialization` and the wire
carry both. It is not a canonical identity and is not stable across bundles (import profile
§8). A provisional entry skipped in a non-production build takes no ordinal.

## 8. Build class

- `production`: every palette key resolved, no draft area and no skipped provisional key. The
  writer refuses such a manifest otherwise, and the reader rejects it.
- `non-production`: may carry draft areas and skipped provisional keys, each listed in the
  manifest.
- A missing `build_class` and any other value read as `non-production`. A World deployed as
  production refuses every bundle that is not `production` (ADR-0021 §4.2, §4.6). That check
  belongs to the loader (MAP-LOAD-1).

## 9. Reader rules and limits

A reader rejects the whole bundle on the first failure; there is no partial load. In order:
file size, header, digest, manifest length and sector count, manifest, then per row the
reserved byte, contiguity, the raw and ratio limits, the running raw total, the sector
coordinate range, ascending order, the frame checksum, decompression into exactly `raw_length`
bytes, the payload grammar, palette indices, tile positions and teleport destinations inside the
World extent, and the top-level entry limit. Every size is checked before memory is reserved for it.

| Limit | Hard maximum |
|---|---|
| `MAP01-BUNDLE-FILE-BYTES` | 1 GiB file |
| `MAP01-BUNDLE-MANIFEST-BYTES` | 16 MiB manifest |
| `MAP01-BUNDLE-SECTOR-COUNT` | 1,048,576 sectors |
| `MAP01-BUNDLE-SECTOR-RAW-BYTES` | 16 MiB per sector payload, at most 1,024 times its frame |
| `MAP01-BUNDLE-TOTAL-RAW-BYTES` | 1 GiB of payloads per bundle |
| `MAP01-TILE-BASE-ENTRIES` | 64 top-level entries per tile (ADR-0021 §4.4) |
| `MAP01-TILE-ENTRIES` | 4,096 entries per tile, container contents included |
| `MAP01-ITEM-TEXT-BYTES` | 4,096 bytes per `text` or `description` |

The rows are in `RESOURCE_LIMITS_REGISTRY.json`. The per-tile and text limits are initial
values; MAP-BUNDLE-1b confirms them on the real map before the format is accepted.

The compiler fails closed as well (ADR-0021 §4.3, §4.5): an unknown key, a position or teleport
destination outside the declared World, a legacy `z` above 15, a sector given twice, a
provisional key in a production build and any limit above all stop compilation.

## 10. Open items

These depend on the Sol Supervising Architect's answers on #162 (5909595542) and are **not
decided** here. MAP-BUNDLE-1b closes them before this format is accepted.

- **OPEN-1, palette key families.** ADR-0021 §4.5 admits only Item and Terrain keys; after
  #1319 about 7,699 palette ids are WorldObject keys. Either `family` gains `world-object`, or
  the compiler follows the A12 §4.6 Item pointer and `family` stays as it is.
- **OPEN-2, appearance-only terrain keys.** 5,949 ids have no Item key and #1170 keys them
  `oteryn:terrain.aNNNNNN`. Whether they are minted as `oteryn:terrain.tibia.i<id>` or keep
  that scheme changes only the manifest keys, not the layout.
- **OPEN-3, orphan teleports.** 1,576 map teleport attributes have no Transition record, most
  with destination (0,0,0). Either the compiler drops them with a diagnostic and the tile is not
  a teleport, or compilation fails until the content is fixed. Until then the skeleton keeps
  every teleport attribute and fails when its destination lies outside the World.
- **OPEN-4, draft marker.** v1 lists draft areas in the manifest by key. Whether the runtime also
  needs a per-tile draft marker, and how draft tiles are gated, is decided with the area family
  in MAP-BUNDLE-1b.

## 11. Versioning

- `format_version` in the header and `format` in the manifest name the layout. A change of the
  layout, the manifest fields, the payload grammar or the digest rule is a new version.
- A reader accepts only versions it knows and a manifest whose `min_reader_version` it meets.
- Content changes do not change the format: a new content revision or map produces a new bundle
  with a new digest, activated only at a planned world reset (ADR-0021 §4.7).
