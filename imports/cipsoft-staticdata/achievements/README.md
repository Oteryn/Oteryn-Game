# CipSoft client staticdata: Achievements (15.30)

Source observations of the 368 achievements in the owner-provided 15.30 client
file `staticdata-62d3f5f7...dat` (`content/assets/files/`, pinned by SHA-256 in
`manifest.json`). `content/achievements/` stays unpopulated; nothing here is an
Oteryn identity or gameplay definition.

Each record has `source_id` (unique client id), `source_index`, `name`,
`description`, `grade` and `staticdata_sha256` (SHA-256 of its raw protobuf
record). The source record carries exactly these four fields; the script
rejects any other shape. Names are inferred from content: field 4 takes only
the values 1, 2 and 3 (275/84/9), matching the achievement grade tiers.

Regenerate or verify with
`python tools/content-census/stage_staticdata_houses_achievements.py [--check]`
(shared with `../houses/`; see its README for the rejection rules).
