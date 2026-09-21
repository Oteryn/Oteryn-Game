# OTV2-20260905-risk-scoped-test-lanes

```yaml
task_id: OTV2-20260905-risk-scoped-test-lanes
title: Trusted-base dependency-aware fail-closed PR test lanes
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/risk-scoped-lanes-283
pr: 297
issue: 283
parent_issue: 277
base_sha: e9584d1466f7021b5c542fac4ef0fec6291ff20e
head_sha: null
final_head_sha: null
owner: Codex sole writer
created_at: 2026-09-05T08:30:00Z
updated_at: 2026-09-05T08:30:00Z
execution_budget_minutes: 120
large_budget_reason: Two dependent protected integrations and representative hosted benchmark
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/rust.yml
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260905-risk-scoped-test-lanes.md
public_contracts:
  - Exact-head game-gate with conservative trusted-base test-lane selection
depends_on:
  - issue: 279
    state: completed
  - issue: 285
    state: completed
external_repositories: []
```

## Contract and acceptance

Issue #283 owns the concrete contract and staged benchmark allocation. Dependency #285 is proven on protected main e9584d1466f7021b5c542fac4ef0fec6291ff20e, queue33954927967 with actual PG17.6/117 and Windows SIM7.

The classifier and Cargo metadata execute from exact protected base. Immutable changed-file evidence is rechecked against PR identity. All normal/dev/build/optional/target-specific local dependency edges enter reverse closure. Proven server-only changes retain Linux/PG, policy and supply chain and may omit Windows/SIM only if Windows consumers are unaffected and the reviewed protected-base input snapshot matches. Cargo metadata alone does not describe all include/runtime inputs: any changed consumer tree, symlink, unknown graph or build/dependency input makes the snapshot fail and selects FULL until a reviewed update adopts it.

Control-plane/root/manifests/toolchain/build inputs, unknown/mixed/incomplete enumeration and classifier failure select FULL. Neutral Markdown is limited to approved root/docs paths with AGENTS/migration/control-plane exclusions. No labels/body selection. Strict aggregate output booleans and selected predicate success are mandatory. Existing SHA-bound PG target check remains fail-closed even if incomplete enumeration selects FULL. MQ blob and ruleset remain unchanged. Remove only rust.yml PR trigger; preserve main/manual qualification.

Fresh RED precedes workflow edits. Validate classifier fixtures, actual aggregate result cross-product, identity races, policy/governance, whole-diff self-review, exact-head full CI and one independent deep review. Implementation itself selects FULL. After protected implementation integration, 3–5 allocated nonmerged hosted probes must prove actual lane decisions and runner/wall-time metrics before Issue283 closes. Historical timings are baseline only, never substitute acceptance.

## Execution and review

One writer/worktree; independent read-only design analysis completed. Coupled implementation is serial to preserve classifier/workflow/validator authority. No runtime, ruleset, protected audit or MQ edits. No post-freeze bookkeeping commit. Advisory findings must be reproduced before repair/re-review. Final exact-head evidence lives on Issue/PR.

## RED evidence

Fresh tests fail because trusted-base classifier is absent, and actual current aggregate rejects a proven server-only control with Windows skipped. No production behavior edited before these observations.

## Context checkpoint

Next action: preserve RED, implement classifier and fail-closed workflow wiring, then GREEN and hosted qualification. No owner action required.

## GREEN and self-review refinement

RED653b8289d27cb972b5c32bfc50931df28d5e01bb is preserved. Classifier, workflow wiring and aggregate fixtures pass. Self-review reproduced an additional docs-only input-assumption gap: a future server include could consume Markdown. Fresh failing assertion preceded a separate full-workspace document-input snapshot. Both stale consumer and stale server snapshots now select FULL. The Windows snapshot binds63 inputs; the document snapshot binds120 inputs at admission. Classifier job mutations and original PR/queue regression families remain mandatory. Exact final source head, hosted observation/review and benchmark evidence will be published on PR297/Issue283.
