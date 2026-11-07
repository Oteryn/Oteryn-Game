# MIGLOCK-BUDGET-1 — scale migration-lock retry budget with the ledger

```yaml
task_id: MIGLOCK-BUDGET-1
title: Scale migration-lock interruption retry budget with migration count
mode: REPAIR
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-miglock
pr: null
owner: task worker (owner-allocated worktree)
created_at: 2026-10-24
updated_at: 2026-10-24
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/tests/support/postgres.rs
  - docs/agents/tasks/MIGLOCK-BUDGET-1.md
public_contracts: []
depends_on: []
blocks: []
```

## Delivered

The migration-lock interruption test's fresh retry receives a base budget of
5 seconds plus 50 milliseconds per entry in `GAME_MIGRATOR`, the same embedded
ledger applied by `MigrationExecutor::apply_embedded_ledger`. Named duration
constants and a one-line comment explain the allowance for ledger growth on
loaded CI runners. The 250 ms interruption timeout, lock and DDL assertions,
operation ordering, error expectations and final compatibility assertion remain
unchanged.

## Validation

- `cargo fmt --all --check`: passed.
- `cargo clippy -p oteryn-game-server --all-targets --locked --offline --quiet -- -D warnings`: passed; existing vendored tokio/sqlx dependency warnings remain.
- `cargo test -p oteryn-game-server --test durability_postgres --locked --offline --quiet -- migration_lock --nocapture`: passed against local PostgreSQL, 1 test, 570 filtered out, 5.90 seconds. No missing-URL early return was reported.
- `python3 tools/agents/validate_governance.py`: passed.
- `python3 -m unittest discover -s tools/agents/tests`: passed, 129 tests.
- `git diff --check`: passed.

Only the requested retry budget changes; existing assertions and error
propagation remain intact. No public functions, enum variants, struct fields or
trait methods were added.

## Unfinished

None locally. PR CI pending.
