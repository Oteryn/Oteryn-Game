# World metadata authoring (City Area, House, teleport Transition)

This package populates the world tree from the pinned CrystalServer map. Steps 1 and 2
cover **metadata**: towns, houses and teleports. Step 3 covers the **base map** (19.3 M
tiles, 24.9 M items) in a binary region format selected by measurement (see "Base map
(step 3)" below).

| Family | Path | Records | Shard schema |
|---|---|---:|---|
| `Area.City` | `content/world/areas/cities/` | 35 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
| `House` | `content/houses/` | 995 | `OTERYN_HOUSE_AUTHORING_SHARD/v1` |
| `Transition.Teleport` | `content/world/transitions/` | 872 | `OTERYN_TRANSITION_AUTHORING_SHARD/v1` |

Each directory has an `OTERYN_FAMILY_INDEX/v1` `index.json`, plus canonical JSON shards of
500 records named `<stem>-<start>-<end>.json`. Every family index, shard and record shape
is defined in `world-metadata.schema.json`.

Records follow the existing `{declaration, source_bindings}` shape (see
`content/npcs/definitions/`). Positions use the same `global-target-2026-09-27` frame that
`content/services/travel/` already uses for the same CrystalServer coordinates. Mapping
those into the native `WorldTilePosition` profile
(`docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md`) is a separate adapter step,
not part of this package.

## Source

The source is pinned in `convert_world_metadata.py` (`SOURCE`) and copied into every index
and `samples/source-capture-v1.json`:

- `zimbadev/crystalserver`, branch `summer-update`, commit `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.
- `data-global/world/world.otbm`, gzip, OTBM v4, 35143×34812, floors 0–15.
- `data-global/world/world-house.xml`.

This source is migration evidence (`OtsHypothesisOnly`). Only normalized facts are
committed. Map, sprite and asset bytes never are.

## What is imported and what is not

Keys are stable across source updates: when the family files are already committed, the
converter reuses the existing key for the same source id (`crystalserver/house-id`,
`crystalserver/town-id`) even if the source renames the house or town, and only mints a slug
key for a new id (failing closed if it collides with any committed key). Teleport keys are
position-based.

- **City:** one record per OTBM town, holding the name and temple position. All 35 towns
  are kept as the source declares them, including `Dawnport Tutorial`, `Island of
  Destiny`, `Targuna` and `Home`. TibiaWiki (Cidades) enrichment comes later.
- **House:** holds identity, name, city, entry, rent, guildhall flag and beds from
  `world-house.xml`. It also holds the footprint from OTBM house tiles (per-floor tile
  counts, bounding box) and doors (`door_id` and position).
  - `declared_size` (XML `size`) and `footprint.tile_count` (OTBM tiles) differ for 979 of
    995 houses, so both are recorded.
  - Ownership, rent payment, ACL and inventory are runtime state and never appear here.
- **Teleport:** holds a from/to position, and the teleport `Item` resolved through
  `imports/crystalserver/bindings/items.json`. Only teleports whose destination is set,
  inside the map and on an existing tile are imported. The capture summary counts those
  rejected:
  - 1573 with an unset destination, which are script-driven;
  - 1 with a destination outside the map;
  - 5 with a destination on an absent tile.
- **Not here yet:**
  - the World record (bounds and floors). Its `worlds/` directory shares space with a
    legacy locator.
  - floor changes through stairs, ladders or holes. These are derived from item types
    together with terrain.
  - hunting places, islands and streets.
  - the 18 editor waypoints.
  - the `data-global/world/15.30/` fragment maps.

## Coexistence with the legacy WorldProject package

`content/world/` is still the legacy WorldProject package root. The family shards in
`areas/cities/` and `transitions/` are not WorldProject locators:

- `validate_materialized_game_tree.py` accepts a populated family index there only when its
  shards stay in that directory, the directory holds no other file, and no legacy locator
  shares it.
- `--print-world-successor-files` lists markers and shards. The package-seed workflow
  removes them before its exact `diff -r`.
- `content_world_project_repository.rs` admits them by the same rule.

## Commands

```bash
pip install -r requirements.txt -r requirements-dev.txt
python validate_world_metadata.py          # committed families: schema + semantics
python test_world_authoring.py             # synthetic OTBM fixtures, converter, validator
# Regenerate (needs the pinned crystalserver checkout, not fetched by CI):
python convert_world_metadata.py --crystal-root /path/to/crystalserver [--check]
```

## Base map (step 3)

`WorldPlacement.Base` holds every tile and item of `world.otbm` (19,325,129 tiles,
24,925,845 items, floors 0-15) in `content/world/placements/`. JSON is not used: the same
data is 3.25 GB as JSON sectors and about 22 MB in the format below.

- **Layout:** `index.json` (`OTERYN_FAMILY_INDEX/v1`, family `WorldPlacement.Base`) plus one
  file per non-empty 256x256 region, `region-z{z:02}-x{rx:03}-y{ry:03}.b3`
  (`rx = x // 256`). The index lists the shards, a `regions` table with each file's
  `path`, `sha256`, `tiles` and `items`, and the item `palette`.
  `samples/world-base-capture-v1.json` records the source, extent, totals, per-floor
  tiles, bytes on disk, the zstd build, the palette counts and the count of rejected
  attributes (must be 0).
- **Format** (`OTERYN_WORLD_REGION_B3/v1`, spec in the `world_region_codec.py` docstring): a
  12-byte header (`OTRB`, version, floor, `rx`, `ry`, sector count), a sector table
  (index 0-63, offset, length) and one zstd level-3 frame per non-empty 32x32 sector. A
  sector lists its tiles sorted by (y, x) with delta-coded positions and varints. Items
  are in stacking order, container contents flattened behind their container with a depth.
- **Carried per tile:** flags, house id, tile zone ids (Canary `OTBM_TILE_ZONE`, 476 tiles
  on floor 10). **Per item:** the palette index and, when present, count,
  charges, action id, unique id, text, description, teleport destination, depot id, house
  door id. Presence is exact, so a present zero or empty text stays present.
- **Item identity (palette):** a region file never names an item. It stores, per item, an
  index into `palette` in `index.json`. The palette holds the distinct server item
  ids the map uses (and any retired ones) as `{"key", "source_item_id", "provisional"}`. A fresh build orders it by
  ascending id, and the palette is then **append-only** (see below):
  - an id bound in `imports/crystalserver/bindings/items.json` (`ots/item_server_id`) takes
    that binding's target key, either `oteryn:item.registry.iNNNNNNNN` or a named key such
    as `oteryn:item.currency.gold_coin`, with `provisional: false`;
  - any other id takes `donor:crystalserver@00ce02a5:item/<id>` (the donor-census key
    form) with `provisional: true`. This covers appearance-only terrain ids that are not in
    `items.xml` and new items that no binding covers yet.

  An entry the map stops using is never removed: it is flagged `"retired": true`, which is
  allowed only while no region references it.

  Later identity work (admitting the provisional items, a Terrain identity path) rewrites
  palette entries in `index.json` only. The 22 MB of region files do not change. The
  capture summary counts provisional entries and occurrences, split into ids that
  `items.xml` declares at the pinned revision and appearance-only ids. Current counts:
  25,963 palette entries, 5,987 provisional, 811,579 provisional item occurrences.
  The converter still fails closed, and never guesses, on an item or tile attribute
  outside the carried set.
- **Measured** (prototype of this codec on the same map, one thread): full load 1.7 s into
  a naive model, one sector read about 30 us, compact in-memory base about 170 MB (a
  shared immutable base for all channels). An edit rewrites one sector but changes the
  region file as a whole, so git stores it as a binary diff (`.b3` is marked `binary`).
  Runtime loading and the per-channel overlay are out of scope here.
- **Speed:** `convert_world_base.py` takes about 2.2 minutes. `validate_world_base.py`
  decodes all 1,208 regions in about 13 s on four cores (about 50 s of CPU).

```bash
python validate_world_base.py              # committed family: sha256, decode, counts, palette
python test_world_base.py                  # codec round trips, converter, validator, negatives
# Regenerate (needs the pinned crystalserver checkout, not fetched by CI):
python convert_world_base.py --crystal-root /path/to/crystalserver [--check]
```

Region files are compressed by libzstd through the pinned `zstandard` package. The capture
summary records the versions, and `--check` compares bytes, so run it with the same pins.

To update from a newer CrystalServer revision, change the pin in `convert_world_metadata.py`
and the `items.xml` digest in `convert_world_base.py`, then rerun the converter. Region
files whose tiles did not change stay byte-identical, so only the changed regions differ,
plus `index.json` and the summary. The palette is append-only, so a new id never renumbers
anything: when `content/world/placements/index.json` is committed, the converter keeps every existing entry
at its index (refreshing only its key if the binding changed, for example provisional to an
Oteryn key), appends new server ids at the end in ascending id order, and keeps an id the
new map no longer uses as `"retired": true`. Untouched region files stay byte-identical.
The validator requires unique ids and keys, every non-retired entry referenced and every
retired entry unreferenced. Oteryn edits authored on top of the base map will later need a
separate patch layer that survives a regeneration. That layer is not implemented.
