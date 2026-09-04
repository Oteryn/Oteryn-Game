# OTV2-20260904-canonical-pr-pg-sim-gate

```yaml
task_id: OTV2-20260904-canonical-pr-pg-sim-gate
title: Canonicalize PR PostgreSQL and simulation exact-head gates
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/canonical-pr-pg-sim-279
pr: null
issue: 279
parent_issue: 277
base_sha: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Pro implementation worker
created_at: 2026-09-04T13:56:00Z
updated_at: 2026-09-04T13:56:00Z
execution_budget_minutes: 60
large_budget_reason: required-check composition with hosted RED/GREEN evidence
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_repository_policy.py
  - tools/repository/validate_pr_gate_pg_sim.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260904-canonical-pr-pg-sim-gate.md
public_contracts:
  - canonical PR game-gate composition
depends_on:
  - issue: 277
blocks:
  - issue: 283
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Every currently Rust/workspace-relevant pull-request head must pass real PostgreSQL durability and deterministic Windows simulation tests inside the canonical `game-gate`, without removing any existing lane or changing the protected Merge Queue gate.

## Architecture and source of truth

- `PROVEN` — protected `main` at admission is `d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d`.
- `PROVEN` — ruleset `20991995` requires only the stable `game-gate`; this task does not change the ruleset or status name.
- `PROVEN` — current `merge-gate.yml` requires `rust_linux` and `rust_windows` for the broad existing Rust classifier, but neither job currently owns the real PostgreSQL durability harness or Windows simulation golden test.
- `PROVEN` — adjacent `rust.yml` runs PG/SIM for selected PR paths, but its results are not canonical child predicates of `game-gate`.
- `PROVEN` — protected-base `merge-authority-audit.yml` pins the current merge-group gate blob; this task leaves that file and `merge-group-gate.yml` byte-identical.
- `DERIVED` — adding PG to the already-required Linux job and SIM to the already-required Windows job strengthens evidence without altering the existing exact fan-in implementation or its pinned aggregate digest.

## High-risk authority/recovery qualification

```yaml
applicable: CONTROL_PLANE_REQUIRED_CHECK_COMPOSITION
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - canonical_gate_must_include_real_postgresql_for_rust_surface
  - canonical_gate_must_include_windows_simulation_for_rust_surface
consumer_boundaries:
  - pull_request_game_gate
mutation_operators:
  - remove_postgresql_contract
  - remove_simulation_contract
independent_current_fact_sources:
  - exact_pull_request_head_resolved_by_merge_gate_scope
record_derived_matching_helper:
  allowed_for_positive_happy_path: NOT_APPLICABLE
  forbidden_for_negative_authority_or_provenance_cases: NOT_APPLICABLE
finding_family_sweep:
  sibling_apis: pull_request_gate_only_in_this_task
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: canonical_child_jobs_and_game_gate_fan_in
  restart_retry_replay_concurrency_pg_reload: real_pg_reload_harness_owned_by_existing_test_binary
  evidence:
    - tools/repository/validate_pr_gate_pg_sim.py
finding_dispositions:
  p0_p1_verified_repair_or_rejection: pending_review
  p2_fixed_accepted_or_deferred: pending_review
```

## Acceptance criteria

- [x] A deterministic validator names the exact required PG and SIM contracts.
- [ ] Validator-only RED is executed on an exact branch head and fails because current `merge-gate.yml` lacks both contracts.
- [ ] The already-required Linux job starts pinned PostgreSQL 17.6, verifies the exact PR SHA and runs `durability_postgres`.
- [ ] The already-required Windows job verifies the exact PR SHA and runs simulation golden fixtures.
- [ ] Existing Linux, Windows client, dependency review, CodeQL, governance and supply-chain work remains intact.
- [ ] Existing scope, validate and `game-gate` fan-in semantics remain unchanged.
- [ ] `merge-group-gate.yml`, its protected audit pin and `rust.yml` remain unchanged.
- [ ] Exact-head canonical `game-gate` passes with inspectable PG/SIM steps.
- [ ] One independent deep review has no unresolved actionable finding.

## Excluded scope

No runtime/test-source edit, no #252-owned path, no merge-group gate, protected audit, ruleset, branch-protection, risk classifier, lane skipping, deduplication, production, secret or external-repository change. Merge-group PG/SIM is staged through #284 and #285.

## Implementation / findings

TDD RED candidate adds a fail-closed repository-policy validator and invokes it from the existing wrapper. The current workflow is intentionally not changed in the RED generation, so exact execution must fail specifically on missing canonical PG/SIM job-local contracts.

The planned minimal GREEN adds PostgreSQL service/env/SHA verification/test execution to `rust_linux`, exact SHA verification plus simulation golden execution to `rust_windows`, and updates `BUILD_TEST_MATRIX.md`. No existing job, step or fan-in is removed.

The exact commit SHA is established by authoritative GitHub readback after publication rather than self-embedded.

## Validation

### Focused

- command/run: `python tools/repository/validate_repository_policy.py`
- result: pending hosted RED on validator-only generation

### Component/integration

- command/run: workflow syntax/static contract validation, followed by canonical hosted `game-gate`
- result: pending

### E2E

- scenario: real isolated PostgreSQL durability binary and deterministic Windows simulation golden fixtures
- result: pending GREEN

### Exact-head CI

- final head: pending after RED → GREEN
- trigger source: Draft PR pull_request
- workflow/run/job: pending
- runner assignment: GitHub-hosted Linux and Windows
- classification: material control-plane required-check strengthening
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent whole-diff review
- material findings: pending
- verdict: pending

## Independent review

- required: YES — required-check composition and workflow control plane
- exact head: pending stable GREEN
- method/auditor: one independent deep review under META policy
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: validator-only RED first; five allocated paths on final GREEN
- unresolved review threads: pending
- related/superseded PRs: #284 and #285 are subsequent staged merge-group work
- protected auto-merge: disabled/not requested
- merge commit/result: pending control-plane integration
- ownership release: after protected-main terminal readback

## Context checkpoint

```yaml
last_progress: validator-only TDD RED generation prepared for publication
status: implementing
branch: ci/canonical-pr-pg-sim-279
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request_after_red_publication
ci_check_generation: red_pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: github_hosted_pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish the validator-only RED, open a Draft PR and verify the exact-head repository-policy failure is caused only by missing canonical PostgreSQL and simulation contracts before implementing GREEN
```
