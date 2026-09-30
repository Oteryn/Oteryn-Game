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

On `origin/main` 09ccf36 (with #1340 and LCFA-1 #1330) merged into the branch, every target passes
(passed / failed):

- `character_authority_postgres` (CI-run, aggregating): 867 / 0, including the guard.
- `durability_postgres` 767 / 0; `native_admission_source_postgres` 694 / 0;
  `runtime_scope_assignment_postgres` 702 / 0.
- Newly aggregated standalone targets: `item_fee_burn` 684, `chest_use` 752, `combat_bestiary` 723,
  `combat_death_reward` 729, `combat_pickup` 751; all 0 failed.
- Other standalone targets: `account_achievement` 686, `bestiary_progress` 684,
  `character_death_receipts` 6, `character_progression` 691, `character_stance` 5, `charm_state` 687,
  `check_function_privileges` 1, `corpse_decay` 704, `corpse_transfer` 699, `item_mint` 700,
  `item_transfer` 694, `reward_claim_mint` 686; all 0 failed.
- Before #1340, the aggregated `item_fee_burn` cases failed 5/5 (SQLSTATE 23502), which is the
  regression this lane now catches.
- `cargo fmt --all --check`: pass. `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.

## Product bugs found

None: every newly aggregated case passes against PostgreSQL 17.6 (item_fee_burn after #1340).
