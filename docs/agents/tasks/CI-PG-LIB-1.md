# CI-PG-LIB-1

```yaml
task_id: CI-PG-LIB-1
title: Run PostgreSQL-backed library tests against a database in CI
mode: REPAIR
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy
pr: null
base_sha: 97060ec39071db647641843f561be32452b52ffb
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
  - docs/agents/tasks/CI-PG-LIB-1.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
registry: no
plan_row: integrator-found CI coverage gap
```

## Problem

The library tests of `oteryn-gameplay-durability`, `oteryn-gameplay-dispatch`
and `oteryn-game-server` include PostgreSQL tests. They read
`OTERYN_TEST_POSTGRES_ADMIN_URL` and return early when it is unset. The Rust
Linux `lib-*` shards have no database, so in CI these tests pass without
running. The PostgreSQL harness has a database, but it runs only the four
registered integration targets.

## Fix

- Add a `Run PostgreSQL-backed library tests` step to the `rest` leg of the
  Merge Queue harness (`merge-group-gate.yml`) and of its PR early-signal
  mirror (`pr-postgres-harness.yml`). The step runs
  `cargo +1.95.0 test --locked -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --lib`.
- Make that command a required fragment of the `durability_postgres` job in
  `validate_repository_policy_core.py`.
- Rotate `EXPECTED_MERGE_GROUP_GATE_BLOB` to the new gate blob in
  `merge-authority-audit.yml`, `validate_repository_policy_core.py` and
  `test_validate_merge_group_pg_sim.py`.

No test is added, skipped or weakened. Registered targets and leg routing are
unchanged.

## Measurement

The step was run against PostgreSQL 17 on main `9f814e72a` on a loaded
workstation:

| Crate | Passed | Build | Run |
|---|---|---|---|
| `oteryn-gameplay-durability` | 384 | 51 s | 23 s |
| `oteryn-gameplay-dispatch` | 902 (14 ignored) | 125 s | 178 s |
| `oteryn-game-server` | 306 (3 ignored) | 128 s | 34 s |

The `rest` leg took about 10 minutes on the last queue run. With the new step
it stays well inside the 30-minute job timeout.

## Owner action

`merge-authority-audit.yml` refuses any PR that modifies it. Its
`protected-base validate` check is therefore expected to be red, as it was on
#2278, and the owner enqueues the PR.

## Validation

- `python3 tools/repository/validate_repository_policy.py`
- `python3 tools/repository/test_validate_merge_group_pg_sim.py`. Without
  `pwsh` installed, run it with the PowerShell canary stubbed.
- `python3 tools/repository/validate_pr_gate_pg_sim.py`
- `python3 tools/agents/validate_governance.py`,
  `python3 -m unittest discover -s tools/agents/tests`, `git diff --check`.

## Handoff

The integrator publishes this on `claude/happy-davinci-ijz3wy` after ITEM-PIN-1
merges.
