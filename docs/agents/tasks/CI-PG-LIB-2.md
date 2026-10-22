# CI-PG-LIB-2

```yaml
task_id: CI-PG-LIB-2
title: Run PostgreSQL-backed library tests in their own harness leg
mode: REPAIR
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-3
pr: null
base_sha: 98b897f848ccfd75c21e48fb53091d5b5f7cd967
owner: Claude integrator
created_at: 2026-10-22
updated_at: 2026-10-22
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-group-gate.yml
  - .github/workflows/pr-postgres-harness.yml
  - .github/workflows/merge-authority-audit.yml
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - docs/agents/tasks/CI-PG-LIB-2.md
public_contracts: []
depends_on: [CI-PG-LIB-1]
blocks: []
external_repositories: []
registry: no
plan_row: integrator-found CI capacity gap
```

## Problem

CI-PG-LIB-1 put the PostgreSQL-backed library tests in the `rest` leg of the
PostgreSQL harness. That leg also runs every unassigned E2E target, and
together the two now reach the 30-minute job timeout. #2304 and #2307 were
dropped from the merge queue on that timeout, and #2306 needed about
25 minutes.

## Fix

- Add a `lib` leg to the harness matrix in `merge-group-gate.yml` and its PR
  early-signal mirror `pr-postgres-harness.yml`.
- In the `lib` leg, skip `Run registered PostgreSQL E2E targets` and run
  `Run PostgreSQL-backed library tests`. Move that step out of the `rest` leg.
- Rotate `EXPECTED_MERGE_GROUP_GATE_BLOB` to the new gate blob in
  `merge-authority-audit.yml`, `validate_repository_policy_core.py` and
  `test_validate_merge_group_pg_sim.py`.

Every registered target still runs exactly once, and the library command is
unchanged. No test is added, skipped or weakened.

## Owner action

`merge-authority-audit.yml` refuses any PR that modifies it. Its
`protected-base validate` check is therefore expected to be red, as on #2300,
and the owner enqueues the PR.

## Validation

- `python3 tools/repository/validate_repository_policy.py`
- `python3 tools/repository/test_validate_merge_group_pg_sim.py`. Without
  `pwsh` installed, run it with the PowerShell canary stubbed.
- `python3 tools/repository/validate_pr_gate_pg_sim.py`
- `python3 tools/agents/validate_governance.py`,
  `python3 -m unittest discover -s tools/agents/tests`, `git diff --check`.
