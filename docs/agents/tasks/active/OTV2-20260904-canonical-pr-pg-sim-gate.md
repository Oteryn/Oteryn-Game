# OTV2-20260904-canonical-pr-pg-sim-gate

```yaml
task_id: OTV2-20260904-canonical-pr-pg-sim-gate
title: Canonicalize PR PostgreSQL and simulation exact-head gates
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/canonical-pr-pg-sim-279
pr: 287
issue: 279
parent_issue: 277
base_sha: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
head_sha: null
final_head_sha: null
final_head_frozen_at: 2026-09-04T14:24:00Z
owner: ChatGPT GPT-5.6 Pro implementation worker
created_at: 2026-09-04T13:56:00Z
updated_at: 2026-09-04T14:24:00Z
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

Every Rust/workspace-relevant pull-request head executes deterministic Windows simulation inside canonical `game-gate`. The same required Linux job owns a pinned PostgreSQL 17.6 service and runs the real `durability_postgres` target whenever that target exists on the exact candidate. A PR cannot turn existing PostgreSQL evidence into a skip by deleting or renaming the target; revisions predating its allocation report an explicit `NOT_APPLICABLE` rather than a false E2E PASS.

## Architecture and source of truth

- `PROVEN` — protected `main` at admission is `d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d`.
- `PROVEN` — ruleset `20991995` requires stable `game-gate`; this task does not change the ruleset or status name.
- `PROVEN` — `durability_postgres.rs` is absent from protected main and exists only in still-unmerged PR #252 at the current implementation stage.
- `PROVEN` — adjacent `rust.yml` PG/SIM results are not canonical child predicates of PR `game-gate`.
- `PROVEN` — protected-base merge-authority audit pins `merge-group-gate.yml`; this task leaves both the gate and pin byte-identical.
- `DERIVED` — PG in the already-required Linux job and SIM in the already-required Windows job strengthen evidence while preserving the existing scope, validate and final fan-in implementations.
- `DERIVED` — exact-PR file-status classification is required to distinguish historical non-allocation from candidate removal/rename without trusting a stale base assumption.

## High-risk authority/recovery qualification

```yaml
applicable: CONTROL_PLANE_REQUIRED_CHECK_COMPOSITION
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - canonical_windows_job_runs_simulation
  - allocated_postgresql_target_runs_in_canonical_linux_job
  - candidate_cannot_remove_or_rename_postgresql_target_into_skip
  - historical_absence_is_not_misreported_as_postgresql_pass
consumer_boundaries:
  - pull_request_game_gate
mutation_operators:
  - remove_simulation_contract
  - remove_postgresql_contract
  - remove_or_rename_postgresql_test_target
independent_current_fact_sources:
  - exact_pull_request_head_resolved_by_scope
  - exact_head_checkout
  - exact_live_pull_request_changed_file_statuses
record_derived_matching_helper:
  allowed_for_positive_happy_path: NOT_APPLICABLE
  forbidden_for_negative_authority_or_provenance_cases: NOT_APPLICABLE
finding_family_sweep:
  sibling_apis: pull_request_gate_only_in_this_task
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: linux_postgresql_windows_simulation_and_existing_aggregate
  restart_retry_replay_concurrency_pg_reload: durability_postgres_target_when_allocated
  evidence:
    - tools/repository/validate_pr_gate_pg_sim.py
    - RED 95812aaffe88974958b73803760e070e8c2abe2b
    - RED 891adbf70723ef5f558e15aa69e58ce1a6c957a1
finding_dispositions:
  p0_p1_verified_repair_or_rejection: pending_exact_head_green_and_independent_review
  p2_fixed_accepted_or_deferred: pending_exact_head_green_and_independent_review
```

## Acceptance criteria

- [x] Deterministic validator names the exact PG, SIM and deletion-safe routing contracts.
- [x] Original validator RED failed before PG/SIM were added.
- [x] Initial runtime GREEN attempt proved exact Windows SIM success and exposed the missing-target compatibility defect.
- [x] A distinct second RED requires exact-head changed-file classification and deletion-safe conditional execution.
- [x] Required Linux job starts pinned PostgreSQL 17.6 and verifies exact PR SHA.
- [x] If `durability_postgres.rs` exists, the required Linux job runs it; if the PR removes/renames it, the job fails closed; if it was never allocated on that revision, the job records `NOT_APPLICABLE`.
- [x] Required Windows job verifies exact PR SHA and runs simulation golden fixtures.
- [x] Existing Linux, Windows client, dependency review, CodeQL, governance and supply-chain work remains intact.
- [x] Existing scope, validate and `game-gate` fan-in implementations remain unchanged.
- [x] `merge-group-gate.yml`, protected audit pin and `rust.yml` remain unchanged.
- [ ] Exact final-head canonical `game-gate` passes with inspectable target-classification, PostgreSQL disposition and simulation steps.
- [ ] One independent deep review has no unresolved actionable finding.

## Excluded scope

No runtime/test-source edit, no #252-owned path, no merge-group gate, protected audit, ruleset, branch protection, risk classifier, lane skipping, deduplication, production, secret or external-repository change. Merge-group PG/SIM remains staged through #284 and #285.

## Implementation / findings

### RED 1 — missing canonical PG/SIM contracts

- Exact head `95812aaffe88974958b73803760e070e8c2abe2b`.
- Agent Governance run `33881045461`, job `101049506434`, verified exact checkout.
- Existing governance passed; repository policy failed only on the expected missing PG/SIM job fragments.
- Architecture run `33881045529` passed.

### Initial GREEN attempt — root-cause evidence

- Exact head `fe8e76c617472b6281e519647cc099ebc7b7d1ad` added unconditional PG and exact-head SIM.
- Merge Gate run `33881858954`: Windows job `101052200457` passed, including exact SHA and simulation golden execution.
- Linux job `101052200653` failed only at the new PG command: Cargo reported no `durability_postgres` test target; the target is not present on protected main.
- Root cause is target allocation state, not PostgreSQL service startup or a test failure.

### RED 2 — deletion-safe target routing

- Exact head `891adbf70723ef5f558e15aa69e58ce1a6c957a1` extends the deterministic validator only.
- Agent Governance run `33883182869`, job `101056524350`, verified exact checkout; all existing governance checks passed and repository policy failed only on the 17 newly required target-routing fragments.
- Architecture run `33883182811` passed.

### Minimal GREEN repair

- `rust_linux` receives `pull-requests: read` only, then revalidates the exact live PR head and enumerates every changed file before classifying removal/rename of `apps/game-server/tests/durability_postgres.rs`.
- API failure, moved head, invalid/over-cap count or enumeration mismatch fails the required job.
- Exact candidate file present → execute real PostgreSQL E2E; target removed/renamed → fail; historical non-allocation → explicit `NOT_APPLICABLE`.
- `rust_windows` retains its exact-SHA and simulation golden proof.
- No scope, aggregate, lane-selection, merge-group or protected-setting semantics are changed.
- The final exact commit SHA is established by authoritative GitHub branch/PR readback after this already-known metadata is committed.

## Validation

### Focused

- command/run: `python tools/repository/validate_pr_gate_pg_sim.py`
- result: pending exact final head; validator design proven RED on `891adbf7...`
- command/run: repository-policy protected block hashes
- expected: current protected `scope` and `validate` SHA-256 values unchanged
- command/run: workflow YAML syntax and forbidden-token review
- expected: parse PASS; no `continue-on-error`

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`; canonical hosted `game-gate`
- result: pending exact final head

### E2E

- scenario: deterministic Windows simulation on final head
- result: pending exact final head
- scenario: PostgreSQL target on this pre-#252 candidate
- result: expected explicit `NOT_APPLICABLE`, not PASS; real execution becomes mandatory when the exact candidate contains the target

### Exact-head CI

- final head: established by authoritative GitHub readback after publication
- trigger source: Draft PR #287 synchronize
- workflow/run/job: pending
- runner assignment: GitHub-hosted Linux and Windows
- classification: material control-plane required-check strengthening
- result: pending

## Self-review

- exact head: established after publication
- method/reviewer: implementing agent whole-diff review
- material findings: pending hosted evidence
- verdict: pending

## Independent review

- required: YES — required-check composition and workflow control plane
- exact head: pending stable GREEN readback
- method/auditor: one independent deep review under current META policy
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly five allocated paths
- unresolved review threads: pending
- related/superseded PRs: #284 and #285 are staged merge-group work
- protected auto-merge: disabled/not requested
- merge commit/result: pending control-plane integration
- ownership release: after protected-main terminal readback

## Context checkpoint

```yaml
last_progress: two exact RED generations proved missing contracts and missing-target routing; minimal deletion-safe GREEN is frozen for publication
status: validating
branch: ci/canonical-pr-pg-sim-279
head_sha: null
pr: 287
final_head_sha: null
final_head_frozen_at: 2026-09-04T14:24:00Z
ci_trigger_source: pull_request_synchronize_after_final_green
ci_check_generation: final_green_pending
ci_checks_for_current_head: 0
ci_run_ids:
  - 33881045461_RED1_POLICY_FAILURE_EXPECTED
  - 33881045529_RED1_ARCHITECTURE_SUCCESS
  - 33881858954_INITIAL_GREEN_RUNTIME_FAILURE
  - 33883182869_RED2_POLICY_FAILURE_EXPECTED
  - 33883182811_RED2_ARCHITECTURE_SUCCESS
ci_job_ids:
  - 101049506434_RED1
  - 101052200457_INITIAL_GREEN_WINDOWS_SIM_SUCCESS
  - 101052200653_INITIAL_GREEN_LINUX_MISSING_TARGET_FAILURE
  - 101056524350_RED2
runner_assignment_state: github_hosted_final_green_pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish the deletion-safe GREEN, verify exact branch/PR head and require repository policy, explicit PostgreSQL disposition, Windows simulation, canonical game-gate, whole-diff self-review and one independent deep review before READY_FOR_INTEGRATION; do not merge
```
