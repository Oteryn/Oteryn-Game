# OTV2-20260929-house-authoring-schema-candidate

```yaml
task_id: OTV2-20260929-house-authoring-schema-candidate
title: HOUSES-2 - House authoring schema candidate v1, House tiles, CrystalServer and TibiaWiki BR cross-checks
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/tender-mendel-06tjg2
issue: 162
lane_id: content population (house authoring schema)
pr: null   # recorded in the FREEZE_SHA packet
base_sha: 3eac57c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "house schema worker (claude-code-session-01AGaDHCuMKQ95cqTJBs5XRQ)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/house-authoring/**
  - .github/workflows/house-authoring-schema.yml
  - .github/workflows/house-tibiawiki-br-capture.yml
  - imports/cipsoft-staticdata/houses/README.md
  - docs/agents/tasks/archive/OTV2-20260929-house-authoring-schema-candidate.md
public_contracts: []
depends_on: [OTV2-20260929-staticdata-houses-achievements-staging]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

`tools/content-schema/house-authoring/` holds a CANDIDATE authoring schema for static House
definitions including each House's tile area, a validator, positive/negative cases and a converter that turns all 995 staged
15.30 client houses (HOUSES-1) plus CrystalServer `world-house.xml` into a valid candidate
catalog in CI. `content/houses/` stays READY_UNPOPULATED; no runtime, identity registry or
Item identity is touched. Runtime House state stays out by the tree contract.

## Source verification

- PROVEN: CrystalServer `summer-update` @ `00ce02a57ca5a12e48f32a3476e37471167e4c3f`
  `data-global/world/world-house.xml` (sha256 `36044bf9...e90b`) joins 1:1 on
  `clientid` == client house id (995/995). Rent, beds and guildhall agree for all houses.
- PROVEN: `guildhall` and `shop` are never both set; engine `townid` maps 1:1 to client towns.
- DERIVED: client staticdata field 6 (staged as `entrance`) is at or next to the layout
  bounding-box centre for all 995 houses, so it is modeled as `map_marker`; the entry tile
  comes from the engine.
- CONFLICT (reported, official wins): `size` differs for 812 houses, one name differs,
  10 engine entries fall outside the client footprint (`samples/conversion-report.json`).
- PROVEN: staticmapdata cell order is ascending z, then x, then y, `skip` after the cell:
  best of 24 candidates against the pinned `world.otbm` (sha256 `dcb73554...d8d7`) House
  tiles, 104,748 ground matches vs 96,981 next; 98.4% of engine House tiles covered, 442
  identical houses (`samples/otbm-tile-check.json`). Engine entry is next to a House tile
  for 975 houses, i.e. the tile in front of the door.
- DERIVED: a local run of the wiki comparison on four Fandom pages agreed on rent, size,
  beds (supports official `size_sqm` over the engine value).
- PENDING: TibiaWiki BR challenges agent containers; `house-tibiawiki-br-capture.yml`
  captures it on the runner. Facts are committed after its first run.

## Owner decisions (2026-09-29)

1a House key from name slug. 2a Town as city `Area` ref. 3a Engine entry tile as
`entrance`. 4 a+c: House tiles from the official layout (done), then the TibiaWiki BR capture.

## Validation (local)

- `otbm_tile_check.py --check` against the pinned map: ok. `wiki_br_houses.py self-test`: ok.
- `verify_formal_schema.py`: 24/24 cases; `validate_houses.py synthetic-valid-house.json`: ok.
- `convert_houses.py convert --check`: 995 houses valid, report unchanged.
- `convert_houses.py extract-crystal --check` against the pinned checkout: ok.
- `ruff check` / `ruff format --check`: pass.
- Review: none required (candidate schema, no runtime or public contract activation).
