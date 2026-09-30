# PRIV-GUARD-1

```yaml
task_id: PRIV-GUARD-1
title: PRIV-GUARD-1 - runtime role can execute every CHECK function
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/priv-guard-1-runtime-check-privileges
issue: 162
lane_id: durability
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: b3be8ddd
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "PRIV-GUARD-1 worker (claude-code-session-01MnSvpbKjAZEdzEaFrwiu7D)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/tests/**
  - docs/agents/tasks/archive/PRIV-GUARD-1-runtime-check-function-privileges.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

0016 (fixed by 0018 in #1278) and 0020 (open PR #1307) each revoked EXECUTE from PUBLIC on a function used by a
CHECK without granting it to `oteryn_game_runtime`; PostgreSQL checks it as the inserting role (42501), and tests
run as the table owner. `check_function_privileges_postgres_cases.rs` walks `pg_constraint` (contype 'c') to
`pg_depend` to `pg_proc` for user tables the runtime role can INSERT/UPDATE and asserts
`has_function_privilege` for every function, listing each offender. Controls: the walk must find the two 0016
blessing CHECKs, and a rolled-back REVOKE must be reported. Registered through `character_authority_postgres.rs`
(already in the protected lane) plus a standalone wrapper; no workflow, migration or production change.

## Validation (local, PostgreSQL 17.6 pinned image)

- New test: pass on main; RED with a scratch (uncommitted) migration revoking the blessing function: fails naming
  both 0016 CHECKs.
- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`,
  governance and repository-policy validators, `git diff --check`: see PR.
- Review: none required (test-only).
