# OTV2-20260929-staticdata-creatures-bosses-quests-staging

```yaml
task_id: OTV2-20260929-staticdata-creatures-bosses-quests-staging
title: Stage 15.30 staticdata creature, bestiary-class, boss and quest-line source observations
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/staticdata-creatures-bosses-quests
issue: 162
lane_id: content population (reference source staging)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 50d75c65
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "staticdata staging worker (claude-code-session-01MnSvpbKjAZEdzEaFrwiu7D)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - imports/cipsoft-staticdata/creatures/**
  - imports/cipsoft-staticdata/bestiary-classes/**
  - imports/cipsoft-staticdata/bosses/**
  - imports/cipsoft-staticdata/quest-lines/**
  - tools/content-census/stage_staticdata_creatures_bosses_quests.py
  - tools/content-census/stage_staticdata_creatures_bosses_quests_self_test.py
  - docs/agents/tasks/archive/OTV2-20260929-staticdata-creatures-bosses-quests-staging.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

833 creature races, 21 bestiary classes, 447 bosses and 102 quest lines of the pinned 15.30 client
`staticdata` (`content/assets/files/`) are staged as exact source observations under
`imports/cipsoft-staticdata/`, by a sibling script that reuses the strict wire reader of the
houses/achievements script (which is untouched; its outputs stay byte-identical). Each record keeps
its source id, index and raw-record SHA-256; look sub-messages are kept verbatim as hex; the
remaining numeric fields keep raw names (`f4`..`f7`). Each directory has `manifest.json` with
per-file SHA-256 and a README.

## Source verification

- PROVEN: input pinned by SHA-256; exact field shapes per table asserted (unknown field, wire type,
  duplicate id, count mismatch all rejected).
- DERIVED: meanings of creature `f4`/`f5`/`f7` and boss `f4` are inferred from value
  distributions only and marked as such in the READMEs; `f6` is constant 1.
- UNKNOWN: look sub-message semantics and any link from creatures to bestiary classes.

## Validation (local)

- New script `--check`: ok (12 files); houses/achievements `--check`: ok (7 files, unchanged).
- New self-test: pass (synthetic staging, unknown field, duplicate ids, wrong wire type, truncation,
  missing field, wrong counts, tampered input, committed bytes regenerate).
- `validate_governance.py`, `validate_repository_policy.py`, `ruff`, `git diff --check`: see PR.
- Review: none required (reference-only staging data).
