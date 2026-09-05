# OTV2-20260905-authority-recovery-matrix

```yaml
task_id: OTV2-20260905-authority-recovery-matrix
title: Independent authority recovery and concurrency matrix
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: test/authority-recovery-matrix-282
pr: 303
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

## Material candidate evidence

RED `077881c8d5fb0387d8ca3f4c632389a78cc4e62f` fails at missing executed retry revalidation `commit-v1/AccountPresence/Missing`. GREEN locally executes142 isolated retry revalidations and444 changed-source terminal projections (three exact typed reasons, both versions); inherited302 direct authority negatives and28 FND02 regression combinations remain intact. Full game-server tests and strict all-target Clippy pass; real PG execution is still a hosted requirement, not claimed from local skip-capable tests.

The PG module adds six test entry points: lost PREPARE/COMMIT response; signed replacement replay; three typed terminal histories; three repeated rounds each of identical/distinct synchronized replacement; valid later durable epoch; process replacement. Every successful race winner performs actual COMMIT and typed projection checks, with one receipt/attempt/reservation/proof consumption; loser lacks a valid receipt. Recovered Prepared snapshots reach AwaitFinalRevalidation and the whole applicable matrix must fail closed at subsequent COMMIT. Exact replay preserves the complete stored protection row. Process test runs71 fresh children against independent resolver revisions:69 single-field negative cases across both versions and positive controls before/after. Counts are registry-derived and checked at runtime; elapsed PG timings are printed for optional E assessment.

N/A: historical terminal disposition does not require live equality but must return exact V2 reason/V1 Terminal and Terminal phases for every source mutation. PREPARE replay and Prepared reconciliation grant no controller; current facts are consumed at final COMMIT. V1 PREPARE and unsigned V2 replacement requests are rejected; private V1 reconciliation is not represented as a supported replacement recovery API. Later durable epoch is a separately valid scenario, while single-field current-epoch mutation remains separately isolated. Logical observation times in restarted resolver fixtures are not a claim of freshness after arbitrary wall time.

Whole five-file diff/finding-family self-review complete; no runtime/schema/workflow/ruleset change and no existing assertion weakened. Freeze this candidate for canonical exact-head PG17.6 and independent deep whole-diff review, then protected Merge Queue/readback and programme277 closeout. Final evidence remains on Issue/PR.
