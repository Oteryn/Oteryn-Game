# OTV2-20260930-pg-coverage-1

```yaml
task_id: OTV2-20260930-pg-coverage-1
title: Run every PostgreSQL target's cases in the CI lane (PG-COVERAGE-1)
mode: IMPLEMENT
priority: P1
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/pg-coverage-1
issue: 162
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: null   # origin/main after #1340; exact base is in the FREEZE_SHA packet
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
owner: "pg-coverage-1 worker (claude-code-session-019diAMFnxeyiZii2J4EZEvj)"
created_at: 2026-09-30
updated_at: 2026-09-30
owned_paths:
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/postgres_target_aggregation.rs
  - docs/agents/tasks/archive/OTV2-20260930-pg-coverage-1.md
public_contracts: []
jira: null   # sync pending (coordinator batch)
```

## Problem

CI runs four PostgreSQL targets (durability, character_authority, runtime_scope_assignment,
native_admission_source). Every other `tests/*_postgres.rs` target runs only in the workspace
`cargo test`, without `OTERYN_TEST_POSTGRES_ADMIN_URL`, so its cases return early. Five targets'
cases were not aggregated into a CI-run target: `item_fee_burn`, `chest_use`, `combat_bestiary`,
`combat_death_reward` and `combat_pickup`. That let #1316 break `item_fee_burn_postgres` unnoticed
(fixed by #1340).

## Outcome

- `character_authority_postgres.rs` now includes the five missing `support/*_cases.rs` files, with
  the path-loaded modules they need (`combat`, `combat_pickup`, the `content` test shim,
  `interaction`, `interaction_chest_use`), in the existing PRIV-GUARD-1 pattern. The standalone
  targets were already thin wrappers and are unchanged.
- Guard `support/postgres_target_aggregation.rs`, included by `character_authority_postgres`: reads
  the PostgreSQL targets `.github/workflows/rust.yml` runs, and fails when any other
  `tests/*_postgres.rs` target has inline tests, includes no cases file, or includes a
  `support/*_cases.rs` file that no CI-run target includes. It needs no database, so it also runs in
  the workspace `cargo test`. Checked to fail with the `item_fee_burn` include removed.
- No workflow, `tools/repository/` or product code changed.

## Validation (local, PostgreSQL 17.6 pinned image)

Filled in the FREEZE_SHA packet on #162 (per-target counts, fmt, clippy).

## Product bugs found

None: every newly aggregated case passes against PostgreSQL 17.6 (item_fee_burn after #1340).
