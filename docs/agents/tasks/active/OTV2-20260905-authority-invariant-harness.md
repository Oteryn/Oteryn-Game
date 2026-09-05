# OTV2-20260905-authority-invariant-harness

```yaml
task_id: OTV2-20260905-authority-invariant-harness
title: Independent authority invariant harness
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: test/authority-invariant-harness-281
pr: null
issue: 281
parent_issue: 277
base_sha: 9631cbfe718e75d6bc530352fb811e08a444b6b0
head_sha: null
final_head_sha: null
owner: Programme continuation sole writer
created_at: 2026-09-05T10:00:00Z
updated_at: 2026-09-05T10:00:00Z
execution_budget_minutes: 120
large_budget_reason: Owner-authorized sequential completion of 281 and 282
owned_paths:
  - apps/game-server/tests/authority_invariants.rs
  - apps/game-server/tests/support/authority_matrix.rs
  - apps/game-server/tests/durability_postgres.rs
  - docs/agents/tasks/active/OTV2-20260905-authority-invariant-harness.md
depends_on:
  - issue: 280
    state: completed
external_repositories: []
```

## Outcome and authority

Issue281 governs allocation and acceptance under programme277. Independent live facts must be constructed without consuming a prepared record. The expected record may only be passed at the final production API binding. Keep all existing positive/E2E assertions. No production, schema, workflow, dependency or ruleset changes.

## High-risk authority/recovery qualification

Use AuthorityInvariant × ConsumerBoundary × MutationOperator. The registry enumerates identity/binding, current liveness and temporal/provenance classes; each mutation changes one raw source field. Every applicable boundary runs a positive control and every declared mutation. Explicit N/A reasons distinguish values not consumed by terminal PREPARE from COMMIT/reconciliation predicates. The same independent source feeds real PostgreSQL-reloaded committed snapshots. No record-derived matching helper is used by the new negative matrix.

## Execution and validation

One writer/exclusive checkout. Independent read-only Foundation analysis is complete; coupled registry/builders and PG wiring are serial. Fresh RED requires actual executed coverage at each declared authority boundary before implementing the registry. Then run focused matrix, all server tests, strict Clippy, repository/governance validators, exact-head PG17.6 and canonical gate; whole-diff/finding-family self-review and independent deep review precede protected Merge Queue/readback.

## Context checkpoint

Next action: preserve missing-boundary RED, implement independent source and executable matrix, then qualify the material candidate. Final SHA and review/CI evidence live on Issue/PR; no post-freeze bookkeeping commit. Issue282 remains blocked until protected-main readback.
