# OTV2-20260929-appearances-outfits-effects-missiles-staging

```yaml
task_id: OTV2-20260929-appearances-outfits-effects-missiles-staging
title: Stage 15.30 appearances outfit, effect and missile tables as source observations
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/appearances-outfits-effects-missiles
issue: 162
lane_id: content population (reference source staging)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 50d75c65
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "appearances staging worker (claude-code-session-01MnSvpbKjAZEdzEaFrwiu7D)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - imports/cipsoft-appearances/**
  - tools/content-census/stage_appearance_tables.py
  - tools/content-census/stage_appearance_tables_self_test.py
  - docs/agents/tasks/archive/OTV2-20260929-appearances-outfits-effects-missiles-staging.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Top-level fields 2, 3 and 4 of the pinned appearances file
(`content/assets/files/appearances-2dfa943b...dat`, sha256 pinned in `stage_proficiencies.py`) are
staged under `imports/cipsoft-appearances/{outfits,effects,missiles}/` (new directory; no closer
`imports/` convention existed for appearances): 1480, 243 and 76 records with id, verbatim flags,
frame groups reduced to counts and sprite-id lists, and a per-record SHA-256, plus per-directory
`manifest.json` and one README. Field 1 (43516 objects) is untouched; field 5 (1 record) is only hashed.

## Source verification

- PROVEN: exact record counts; every record has fields 1-3 only; frame groups have fields 1-3; sprite
  info fields are within 1-9; no duplicate ids; the reused wire reader is `stage_proficiencies.parse`.
- DERIVED/inferred (marked in the README): table meanings, flag meanings, sprite-info field meanings.
- UNKNOWN: real meaning of field 5.

## Validation (local)

- `stage_appearance_tables.py --check`: ok (17 files). Self-test passes: tampered input, unknown
  top-level and record field, bad wire type, duplicate id, wrong count, truncation, tampered output.
- Governance, repository policy, ruff and `git diff --check`: see PR.
- Review: none required (reference-only staging data).
