# OTV2-20261011-deploy-stage-loot-1

```yaml
task_id: OTV2-20261011-deploy-stage-loot-1
title: DEPLOY-STAGE-LOOT-1 stage the loot_tables pin in the Synology deploy
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/deploy-stage-loot-1-20261011
pr: 1992
base_sha: 4ab138b90
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: DEPLOY-STAGE-LOOT-1 writer (control plane session_01CwP6d84eCPvpgoEuyci8Tx)
created_at: 2026-10-11T00:00:00Z
updated_at: 2026-10-11T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - deploy/synology-game/stage_gameplay.py
  - tools/qualification/spells/prepare_test_manifest.py
  - tools/qualification/spells/test_qualification_tools.py
  - docs/agents/tasks/OTV2-20261011-deploy-stage-loot-1.md
  - docs/agents/tasks/archive/OTV2-20261011-deploy-stage-loot-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The deploy stager copies the `loot_tables` file that the v5 gameplay manifest pins, so the staged gameplay directory is complete for the server.

## Architecture and source of truth

- `apps/game-server/src/content/native_gameplay.rs` loads every pinned file and refuses a missing one. PROVEN by reading the loader.
- `content/spells.manifest.json` pins 14 files including `loot_tables`; `stage_gameplay.py` listed 13. CONFIRMED.
- `tools/qualification/node_boot/run.sh` already listed `loot_tables`, so `StagingTests` already failed on main.

## Acceptance criteria

- [x] `stage_gameplay.py` stages `loot_tables`.
- [x] A guard test stages the canonical manifest and fails when any pinned file is not staged.

## Excluded scope

No workflow, server or content change; the workflow uses the script unchanged.

## Implementation / findings

- Added `loot_tables` to `FIELDS` in `stage_gameplay.py` and to `PINS` in `prepare_test_manifest.py`.
- Added `test_stager_stages_every_file_the_canonical_manifest_pins`; it fails on the old stager.

## Validation

### Focused

- `python -m unittest test_qualification_tools` (tools/qualification/spells): OK, 15 tests
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK

### Component/integration

- NOT_APPLICABLE: no deploy is run.

### E2E

- NOT_APPLICABLE: no deploy is run.

### Exact-head CI

- final head: see PR
- result: pending

## Self-review

- exact head: see PR
- method/reviewer: implementing agent
- verdict: no findings

## Independent review

- required: control plane decides

## PR and closeout

- PR: 1992
- ownership release: after merge

## Context checkpoint

```yaml
last_progress: fix and guard test written
status: completed
branch: claude/deploy-stage-loot-1-20261011
head_sha: null
pr: 1992
final_head_sha: null
next_action: control plane review and freeze
```
