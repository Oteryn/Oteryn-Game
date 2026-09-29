# OTV2-20260929-proficiency-source-staging

```yaml
task_id: OTV2-20260929-proficiency-source-staging
title: Stage 15.30 weapon proficiency definitions and weapon bindings as source observations
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/proficiency-source-staging
issue: 162
lane_id: content population (reference source staging)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: aab7cf74
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "proficiency staging worker (claude-code-session-01MnSvpbKjAZEdzEaFrwiu7D)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - imports/cipsoft-staticdata/proficiencies/**
  - imports/cipsoft-staticdata/weapon-proficiency-bindings/**
  - tools/content-census/stage_proficiencies.py
  - tools/content-census/stage_proficiencies_self_test.py
  - docs/agents/tasks/archive/OTV2-20260929-proficiency-source-staging.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

443 proficiency definitions and 666 weapon to proficiency bindings from the pinned 15.30 client
files are staged as exact, reproducible source observations under `imports/cipsoft-staticdata/`
(same convention as #1277). Owner asked for this directly ahead of the PROFICIENCY-0
architecture decision. `content/**` and runtime code are untouched.

## Source verification

- PROVEN: both inputs pinned by SHA-256; 443 unique definitions with closed key shapes; 666 objects
  carry flags field 61, all ids resolve to a definition, no duplicates.
- DERIVED: appearances field numbers (object 1/3/4, flag 61) inferred from value distributions.
- UNKNOWN: meaning of perk `Type`/`SkillId`/`AugmentType`/`ElementId`/`DamageType` and `Version` codes.
- Observed: 430 of 443 proficiencies are referenced; 13 are unreferenced by any object.

## Validation (local)

- `stage_proficiencies.py --check`: ok (10 files). Self-test: 4 tests pass, incl. tampered-input,
  dangling-id, duplicate-id, unknown-shape and truncation rejection.
- `validate_governance.py`, `validate_repository_policy.py`, `ruff check`, `git diff --check`: see PR.
- Review: none required (reference-only staging data).
