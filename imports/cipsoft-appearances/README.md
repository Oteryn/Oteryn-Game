# CipSoft appearances non-object tables (15.30)

Source observations from top-level fields 2, 3 and 4 of the pinned client appearances file
`content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat`. Authority: SOURCE_OBSERVATIONS_ONLY; no Oteryn ProductionKeys or gameplay promotion.

| Directory | Top-level field | Records | Inferred meaning |
|---|---|---|---|
| outfits | 2 | 1480 | outfits (inferred) |
| effects | 3 | 243 | magic effects (inferred) |
| missiles | 4 | 76 | distance missiles (inferred) |

Field 1 (objects) is other lanes' scope; field 5 (one record) is only hashed in each manifest.

Per record: `source_id` (field 1), `source_index`, `flags` (field 3 as verbatim field number -> value;
length-delimited flags carry `hex` and, when varint-only, `varint_fields`), `frame_groups` (field 2:
`fixed_frame_group`, `group_id`, `sprite_count`, `sprite_ids`, other sprite-info fields verbatim) and
`source_record_sha256` of the raw record bytes. No image decoding.

Inferred meanings, not proven by the data (all marked inferred): flags 23 looks like light
(intensity, colour), 26 like a draw offset (x, y), the varint-only flags (29, 39, 49-52) like booleans or
small enums; sprite-info fields 1-4 like pattern width, height, depth and layers, 6 like animation
phases, 7 like bounding square, 8 like opaque, 9 like per-direction bounding boxes. Outfit records
with two frame groups are inferred as idle and walking groups.

Regenerate with `python tools/content-census/stage_appearance_tables.py`; `--check` compares bytes.
