# NOQUEUE-1

```yaml
task_id: NOQUEUE-1
title: Run the full merge-group gate post-merge on push to main
mode: REPAIR
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-11
pr: null
base_sha: 37bcdf52771a4f68992aec4c3ca5b0cdb28936ce
owner: Claude integrator
created_at: 2026-10-23
updated_at: 2026-10-23
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-group-gate.yml
  - .github/workflows/merge-authority-audit.yml
  - docs/agents/BUILD_TEST_MATRIX.md
  - tools/repository/test_validate_merge_group_pg_sim.py
  - tools/repository/validate_repository_policy.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/tasks/NOQUEUE-1.md
public_contracts: []
depends_on: [FLOW-1]
blocks: []
external_repositories: []
registry: no
plan_row: owner decision to drop the merge queue
```

## Problem

The owner turned the GitHub merge queue off for good. The full gate in
`merge-group-gate.yml` runs only on `merge_group`, so the coverage that only it
has no longer runs anywhere. That coverage is the full workspace tests, the
Windows client, node boot, the server seam, the Atlas full world and the full
PostgreSQL harness.

## Fix

- Add a `push` trigger on `main` to `merge-group-gate.yml` and keep
  `merge_group`, so restoring the queue needs no workflow change.
- Resolve the head, base and ref for both events: the `merge_group` fields
  when present, else `github.sha`, `github.event.before` and `github.ref`.
- Cancel in progress only on `merge_group`. On push every main commit keeps
  its own verdict.
- Accept `push` in the event check. `merge_group` still requires
  `checks_requested`.
- Rotate `EXPECTED_MERGE_GROUP_GATE_BLOB` in `merge-authority-audit.yml`,
  `validate_repository_policy_core.py` and `test_validate_merge_group_pg_sim.py`.
- Document the post-merge run in `BUILD_TEST_MATRIX.md`.

This is a post-merge verdict, not a PR requirement. The ruleset keeps
`game-gate` as the only required check. No job is removed and no test is
skipped.

## Validation

- `python3 tools/repository/validate_repository_policy.py`.
- `python3 -m unittest discover -s tools/repository -p 'test_*.py'`.
- `python3 tools/agents/validate_governance.py`, `git diff --check`.

## Owner action

`merge-authority-audit.yml` refuses any PR that modifies it, so its
`protected-base validate` check is expected to be red, as on CI-PG-LIB-2.
