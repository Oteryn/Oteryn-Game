# OTV2-20260905-merge-group-pg-sim-activation

```yaml
task_id: OTV2-20260905-merge-group-pg-sim-activation
title: Activate preapproved synthetic-head PostgreSQL and simulation gate
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/merge-group-pg-sim-285
pr: null
issue: 285
parent_issue: 277
base_sha: be708dc5be5290274f635d534d83f62b2f14b732
head_sha: null
final_head_sha: null
owner: Codex sole writer
created_at: 2026-09-05T08:00:00Z
updated_at: 2026-09-05T08:00:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - .github/workflows/merge-group-gate.yml
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260905-merge-group-pg-sim-activation.md
public_contracts:
  - canonical synthetic-head game-gate PostgreSQL and Windows simulation
depends_on:
  - issue: 284
    state: completed
blocks:
  - issue: 283
external_repositories: []
```

## Outcome

Activate exactly blob e3291fe8fca8fcf70166d5652b43d5a26fa0d762, already approved by protected-main #284/#291. Historical #288 is superseded. Do not change the audit, ruleset, PR classification or runtime. One writer; a read-only analysis lane prepares the dependent #283 contract. Activation and integration are serial because #283 requires protected-main readback.

## Acceptance criteria

- Exact preapproved blob with all existing predicates and new PG/SIM evidence.
- Fresh test RED then minimal GREEN; mutation tests reject skips/tolerance/early exit and aggregate rejects all non-success results.
- Full policy/governance, current-head hosted PR checks, whole-diff review and one independent deep review.
- Protected Merge Queue executes real PostgreSQL17.6 and Windows SIM on its synthetic head, followed by main/tree readback.

## Validation

RED: test_validate_merge_group_pg_sim.py fails on the existing gate with queue PG/SIM activation absent or altered. The suite is wired into the existing canonical regression driver, preserving all12 prior regression functions. Final exact-head evidence belongs on Issue/PR after publication, not a bookkeeping-only commit.

## High-risk authority/recovery qualification

NOT_APPLICABLE to runtime authority APIs; control-plane risk is qualified through immutable workflow identity, current synthetic checkout, evidence-job mutation family and every aggregate predicate's failure/skip/cancel/missing-result behavior. No runtime record/current-fact semantics change.

## Independent review

Required once deterministic GREEN is stable, under current META AI review policy. Suggestions require verification; only accepted material findings require repair/re-review.

## Context checkpoint

Next action: preserve RED, activate exact approved gate, update executable policy and run GREEN. Owner authorizes protected integration then immediate #283 continuation. No force/rebase/reset or protection bypass.
