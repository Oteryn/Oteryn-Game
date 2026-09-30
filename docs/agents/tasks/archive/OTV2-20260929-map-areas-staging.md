# OTV2-20260929-map-areas-staging

```yaml
task_id: OTV2-20260929-map-areas-staging
title: Stage 15.30 client map areas, markers and layer geometry as source observations
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/map-areas-staging
issue: 162
lane_id: content population (reference source staging)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 50d75c65
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "map staging worker (claude-code-session-01MnSvpbKjAZEdzEaFrwiu7D)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - imports/cipsoft-staticdata/map/**
  - tools/content-census/stage_map_areas.py
  - tools/content-census/stage_map_areas_self_test.py
  - docs/agents/tasks/archive/OTV2-20260929-map-areas-staging.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

465 map areas, 1270 named markers and 1157 layer records (209 subarea overlays, 741 satellite and
207 minimap tiles) plus the bounds corners from the pinned 15.30 `map-c54dfeb8...dat` are staged
as exact, reproducible source observations under `imports/cipsoft-staticdata/map/`. The wire
reader is imported from `stage_proficiencies.py`. `content/**` and runtime code are untouched.

## Source verification

- PROVEN: input pinned by SHA-256; position messages are exactly `{x,y,z}` varint fields 1/2/3 in
  every record; area ids unique; every area subarea id and every kind 0 layer id resolves to an
  area id; every layer image file exists; layer kind agrees with the file-name prefix.
- DERIVED: field roles and names inferred from value distributions (see the README).
- UNKNOWN: `flag`, `field_6`, `field_7_text`, icon id meaning; the image name hash is not the
  SHA-256 of the stored file (both recorded, images not decoded).

## Validation (local)

- `stage_map_areas.py --check`: ok (25 files). Self-test passes, incl. tampered-input, unknown-field,
  duplicate-id, dangling-subarea-id, position-shape and count rejection.
- `validate_governance.py`, `validate_repository_policy.py`, `ruff check`, `git diff --check`: see PR.
- Review: none required (reference-only staging data).
