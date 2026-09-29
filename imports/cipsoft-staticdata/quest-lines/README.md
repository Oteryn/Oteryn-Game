# CipSoft client staticdata: Quest lines (15.30)

Source observations of the 102 quest lines in the owner-provided 15.30 client file
`staticdata-62d3f5f7...dat` (`content/assets/files/`, pinned by SHA-256 in `manifest.json`).
Nothing here is an Oteryn identity or gameplay definition; no content directory is populated.

Each record has `source_id` (unique client id), `source_index`, `staticdata_sha256` (SHA-256 of its
raw protobuf record), `name`. The script rejects unknown fields, wrong wire types, duplicate ids and
count mismatches.

**Inferred:** top-level field 6 is the quest-line table (id, name).

Regenerate or verify with `python tools/content-census/stage_staticdata_creatures_bosses_quests.py [--check]` (one script for `creatures/`,
`bestiary-classes/`, `bosses/`, `quest-lines/`; it reuses the wire reader of
`tools/content-census/stage_staticdata_houses_achievements.py`).
