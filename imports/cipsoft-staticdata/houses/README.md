# CipSoft client staticdata: Houses (15.30)

Source observations of the 995 houses in the owner-provided 15.30 client files
`staticdata-62d3f5f7...dat` (house table) and `staticmapdata-0967af2e...dat`
(per-house layout), both under `content/assets/files/` and pinned by SHA-256 in
`manifest.json`. `content/houses/` stays unpopulated: no canonical authoring
schema exists yet, and nothing here is an Oteryn identity or gameplay definition.

Each record keeps `source_id` (the client house id, joined 1:1 between the two
files), `source_index` (order in the file) and the SHA-256 of its raw protobuf
records (`staticdata_sha256`, `layout.staticmapdata_sha256`).

The files are protobuf without a shipped schema. Field names are **inferred from
value distributions** and known houses (for example id 10004 = Dark Mansion,
Thais, 361 sqm, 17 beds, guildhall):

| staticdata house field | name | note |
| --- | --- | --- |
| 1 | `source_id` | unique |
| 2 | `name` | |
| 3 | `restrictions` | free text, usually empty |
| 4 | `rent_gold` | |
| 5 | `beds` | |
| 6 | `entrance` | x, y, z; at or next to the layout centre, so a map marker rather than the door (see `tools/content-schema/house-authoring/`) |
| 7 | `size_sqm` | |
| 8 | `guildhall` | 0/1 |
| 9 | `town` | |
| 10 | `shop` | 0/1; every observed shop name carries "(Shop)" |

`layout` comes from staticmapdata: `origin` and `dimensions` (width, height,
floors) plus `cells`. Each cell lists `items` (client appearance ids; not Oteryn
Item identities) and an optional `skip` count. Verified for all 995 houses:
`len(cells) + sum(skip) == width*height*floors`, so `skip` is a run of empty
tiles, and the script rejects any house where it does not hold. The mapping of
cell order to x/y/z is not documented and is not derived here. Item sub-fields
101 and 102 (rare) are kept verbatim as `item_extra_fields` hex.

Regenerate or verify (inputs default to `content/assets/files/`):

```sh
python tools/content-census/stage_staticdata_houses_achievements.py
python tools/content-census/stage_staticdata_houses_achievements.py --check
```

The script rejects an input digest mismatch, unknown or missing fields, wrong
wire types, non-boolean flags, non-UTF-8 text, duplicate ids, house id sets that
differ between the two files, and any count other than 995.
