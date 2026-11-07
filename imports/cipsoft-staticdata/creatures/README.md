# CipSoft client staticdata: Creatures (bestiary races) (15.30)

Source observations of the 833 creature races in the owner-provided 15.30 client file
`staticdata-62d3f5f7...dat` (`content/assets/files/`, pinned by SHA-256 in `manifest.json`).
Nothing here is an Oteryn identity or gameplay definition; no content directory is populated.

Each record has `source_id` (unique client id), `source_index`, `staticdata_sha256` (SHA-256 of its
raw protobuf record), `name`, `look` (raw look sub-message kept verbatim as hex, not decoded) and raw integers `f4`..`f7`. The script rejects unknown fields, wrong wire types, duplicate ids and
count mismatches.

**Inferred, not source-labelled:** top-level field 1 is the bestiary race table. `f4` (0-5, mostly 3/4/2) looks like a difficulty or stars tier; `f5` (0-3, 668 zeros) looks like an occurrence or rarity tier; `f6` is constant 1; `f7` (0/1, 629 ones) looks like a boolean flag. The raw values are kept unnamed.

Regenerate or verify with `python tools/content-census/stage_staticdata_creatures_bosses_quests.py [--check]` (one script for `creatures/`,
`bestiary-classes/`, `bosses/`, `quest-lines/`; it reuses the wire reader of
`tools/content-census/stage_staticdata_houses_achievements.py`).
