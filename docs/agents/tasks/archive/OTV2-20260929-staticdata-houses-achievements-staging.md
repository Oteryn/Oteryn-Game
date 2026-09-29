# OTV2-20260929-staticdata-houses-achievements-staging

```yaml
task_id: OTV2-20260929-staticdata-houses-achievements-staging
title: HOUSES-1 and ACHIEVEMENTS-1 - stage 15.30 staticdata house and achievement source observations
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/staticdata-houses-achievements
issue: 162
lane_id: content population (reference source staging)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 73b5d604
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "staticdata staging worker (claude-code-session-01MnSvpbKjAZEdzEaFrwiu7D)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - imports/cipsoft-staticdata/houses/**
  - imports/cipsoft-staticdata/achievements/**
  - tools/content-census/stage_staticdata_houses_achievements.py
  - tools/content-census/stage_staticdata_houses_achievements_self_test.py
  - docs/agents/tasks/archive/OTV2-20260929-staticdata-houses-achievements-staging.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

The 995 houses and 368 achievements of the pinned 15.30 client `staticdata` and
`staticmapdata` files (`content/assets/files/`) are staged as exact, reproducible source
observations under `imports/cipsoft-staticdata/` (the packet's paths; the alternative
`imports/official/client-assets/15.30/` holds only the checksum manifest). Each record keeps its
source id, source index and a SHA-256 of its raw protobuf record; each directory has a
`manifest.json` with per-file SHA-256. `content/houses/` and `content/achievements/` stay
READY_UNPOPULATED; no schema, runtime, or Item identity is touched.

## Source verification

- PROVEN: both inputs are pinned by SHA-256 in the script. Houses: 995 in staticdata joined 1:1 by
  id with 995 in staticmapdata. Achievements: 368, exactly four fields each.
- DERIVED: protobuf has no shipped schema; field names are inferred from value distributions and
  known houses (documented in the READMEs). Layout `skip` is a run of empty tiles: verified
  `len(cells) + sum(skip) == width*height*floors` for all 995 houses and enforced by the script.
- UNKNOWN: cell order to x/y/z mapping is not derived; item sub-fields 101/102 are kept as raw hex.

## Validation (local)

- `stage_staticdata_houses_achievements.py --check`: ok (7 files, regeneration equals committed bytes).
- `stage_staticdata_houses_achievements_self_test.py`: 4 tests pass, including truncated, unknown-field,
  duplicate-id, id-set-mismatch, cell-identity and tampered-input rejections.
- `validate_governance.py`, `validate_repository_policy.py`: pass. `ruff check` on the new tools: pass.
  `git diff --check`: clean.
- Review: none required (reference-only staging data).
