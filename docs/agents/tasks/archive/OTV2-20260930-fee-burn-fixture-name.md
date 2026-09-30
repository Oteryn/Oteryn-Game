# OTV2-20260930-fee-burn-fixture-name

```yaml
task_id: OTV2-20260930-fee-burn-fixture-name
title: Give the GOLD-FEE-1a PostgreSQL fixture root a Character name (0022)
mode: IMPLEMENT
priority: P0   # broken tests on main
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/fee-burn-fixture-name
issue: 162
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 1ae6c698
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
owner: "fee-burn-fixture-name worker (claude-code-session-01EfiFA9LMuUuzoNkizLfR2R)"
created_at: 2026-09-30
updated_at: 2026-09-30
owned_paths:
  - apps/game-server/tests/support/item_fee_burn_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20260930-fee-burn-fixture-name.md
public_contracts: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

The #1318 fixture inserted its `game_character_roots` row without `name`, which CHAR-NAME-1 (migration 0022,
#1316) made NOT NULL, so every PostgreSQL case failed with SQLSTATE 23502. The insert now passes the valid name
`Fee Hero`, as the other fixtures do; the 0022 trigger reserves the name, and each case has its own database, so
no uniqueness conflict is possible. Test-only change; no migration, workflow or production code touched.

## Validation (local, PostgreSQL 17.6 pinned image)

- `item_fee_burn_postgres` before: 666 passed, 5 failed (all 23502). After: 671 passed, 0 failed.
- `cargo fmt --all --check`: pass. `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- CI coverage: no workflow runs this target against PostgreSQL. It is not a registered PostgreSQL target (only
  durability, character_authority, runtime_scope_assignment, native_admission_source are); the workspace
  `cargo test` runs it without `OTERYN_TEST_POSTGRES_ADMIN_URL`, so its cases return early. Reported, not changed.
- Review: none required (test-only fixture repair).
