# CipSoft client map data (15.30)

Source observations of the owner-provided 15.30 client file `map-c54dfeb8...dat`
(`content/assets/files/`, pinned by SHA-256 in each `manifest.json`). Nothing here is an
Oteryn identity, map or gameplay definition. Regenerate or verify with
`python tools/content-census/stage_map_areas.py [--check]`.

The file is a protobuf read with the strict wire reader shared from `stage_proficiencies.py`
(varint and length-delimited; the single fixed64 field is split off the record tail). Unknown
fields or wire types, duplicate ids, dangling ids and wrong counts are rejected.

| Directory | Top-level field | Records | Record keys |
| --- | --- | --- | --- |
| `areas/` | 1 | 465 | `source_id` (field 1, unique), `name` (2), `flag` (3; 1 or 2), `subarea_ids` (repeated 4), `position` (5), `field_6`, `field_7_text`, `subarea_image` |
| `markers/` | 2 | 1270 | `name` (1), `position` (2), `icon_id` (3); no id, `source_index` is the identity |
| `subareas/` | 3 | 1157 | `layer_kind` (1), `position` (2), `image` (3), `field_4`, `field_5`, `area_id` (6), `scale` (7) |

`bounds.json` holds top-level fields 4 and 5 (two position messages: min and max corner).
Every record also carries `source_index` (order in the file) and `source_record_sha256` (of its
raw wire bytes).

Position messages: PROVEN shape. Every marker, area, layer and bounds position is exactly varint
fields 1, 2, 3 in that order, staged as `{x, y, z}` with `z` in 0..15. The axis names are
inferred from the value ranges (x about 31000-34000, y about 30000-33000, z the floor), not from
a schema. 158 of 465 areas carry a position, the rest `null`.

Layers (`subareas/`): `layer_kind` 0 = 209 subarea overlays (`subarea-NNNN-*.bmp.lzma`, no scale,
`area_id` = field 6, unique, equal to the file-name number), 1 = 741 `satellite-*` tiles, 2 = 207
`minimap-*` tiles. Kind and file-name prefix agree in every record (the kind names are inferred
from that). Kinds 1 and 2 carry `scale` (0.0625, 0.03125 or 0.015625, a fixed64 double).
`field_4` and `field_5` look like pixel width and height (256/256, 512/512, 1024/1024, 112/86 ...)
and are kept raw. Images are joined by file name only: `image.file`, `image.file_sha256` (SHA-256
of the stored `.lzma` bytes) and `image.name_hash` (the 64-hex token inside the name, which is not
the digest of the stored file). No image is decoded.

Areas: every `subarea_ids` entry (field 4, present on 28 areas) resolves to an area id,
so a "subarea" is an area id. Areas with a matching kind 0 layer have a `subarea_image`, the rest
`null`. `field_6` (0 or 1, 28 areas; only on areas that list subareas) and `field_7_text` (a second string on 53 areas,
for example "Werelion", "Hero Cave") are UNKNOWN in meaning and kept raw. `flag` values 1 and 2
are unexplained.

Inferred names (not from a schema): `position` axes, `flag`, `subarea_ids`, `icon_id`,
`layer_kind`, `scale`, `field_4`/`field_5` as sizes. UNKNOWN: icon id meaning, `flag`, `field_6`,
`field_7_text`, how `subarea_ids` relate spatially to the overlay images.
