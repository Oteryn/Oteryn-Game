# OTV2-20260905-authority-recovery-matrix

```yaml
task_id: OTV2-20260905-authority-recovery-matrix
title: Independent authority recovery and concurrency matrix
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: test/authority-recovery-matrix-282
pr: null
issue: 282
parent_issue: 277
base_sha: d4c71aebed6431df4759af2ff1ad875a5c17af18
head_sha: null
final_head_sha: null
owner: Programme continuation sole writer
created_at: 2026-09-05T10:45:00Z
updated_at: 2026-09-05T10:45:00Z
execution_budget_minutes: 120
owned_paths:
  - apps/game-server/tests/authority_invariants.rs
  - apps/game-server/tests/support/authority_matrix.rs
  - apps/game-server/tests/support/authority_recovery.rs
  - apps/game-server/tests/durability_postgres.rs
  - docs/agents/tasks/active/OTV2-20260905-authority-recovery-matrix.md
depends_on:
  - issue: 281
    state: completed
external_repositories: []
```

## Authority and scope

Issue282 governs this test-only continuation of programme277. Issue281 is integrated/read back at the admission SHA. Reuse its independent Seed/LiveSource and invariant/operator registry; do not build a competing framework. One writer owns this checkout, with completed independent read-only design analysis. Preserve existing assertions and production/schema/workflow/ruleset behavior.

## Acceptance and execution

Exercise exact retry/lost-response PREPARE and COMMIT, final current-authority revalidation, actual PostgreSQL reload, typed historical terminal reasons and later committed epochs, signed replacement replay and rejection of unsigned/legacy PREPARE, deterministic identical/distinct replacement races with confirmed lock waiters and winner/loser fencing, and fresh child processes that reread a separately seeded resolver table. Every applicable registry mutation changes exactly one source field. Historical/replay-only boundaries explicitly do not grant authority; current equality is N/A there but terminal/no-controller behavior is executable.

Create/migrate before sampling PG time and immediate initial COMMIT. Child observation timestamps are deterministic resolver inputs, distinct from elapsed process time. Parent changes pristine source payload and resolver revision atomically; child reads them from PG, independently constructs expected record from Seed, checks distinct PID/revision/one-field delta and real V1/V2 journal projections. Bound and reap children. Repeat each synchronized race three times and report runtime/counts to assess optional E.

## Validation and checkpoint

Fresh focused RED requires actual execution of every retry revalidation case before implementation. Then focused/full-server tests, strict all-target Clippy, validators, real PG17.6, repeated deterministic races, exact-head canonical gate, whole-diff/family sweep and independent deep review. Integrate only through protected Merge Queue and read back main. Final SHA/CI/review evidence stays on PR/Issue; no post-freeze bookkeeping commit.

Next action: preserve the focused retry-matrix RED, then implement recovery helpers and real PG/process boundaries.
