# SPELL-CLOSURE-LAND-1

```yaml
task_id: SPELL-CLOSURE-LAND-1
title: Land spell source-closure reference data r27 and r28 from #1755
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-closure-land-1-20261004
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: impl worker for control plane session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04T00:00:00Z
updated_at: 2026-10-04T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/reference/spells/r27-source-closure/**
  - docs/reference/spells/r28-source-closure/**
  - docs/reference/spells/README.md
  - docs/agents/tasks/archive/SPELL-CLOSURE-LAND-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: 1622
external_repositories: []
```

## Outcome

The r27 and r28 source-closure reference data from the owner's draft PR #1755 (`codex/spells-source-closure-20261003`) is on `main`, so later spell workers no longer need the draft branch. The rest of #1755 is already on `main` under new migration numbers.

## Architecture and source of truth

- PROVEN: both directories are taken byte-identical from the #1755 head with `git checkout <head> -- <dir>`; the PR body records the tree-hash comparison.
- PROVEN: the data is reference evidence only (Canary/Crystal sources); it grants no authority.
- PROVEN: no `.gitattributes` or LFS requirement; the repository has no LFS and already carries larger reference files.

## Acceptance criteria

- [x] r27 and r28 trees identical to the #1755 head.
- [x] README entry marks both as reference evidence only.
- [x] No code, migration, workflow or `.gitattributes` change.

## Excluded scope

The owner's `codex/` branch is not written. Other r29+ directories of #1755 are not landed here.

## Implementation / findings

Data copied unchanged. Added one README section and this record.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
