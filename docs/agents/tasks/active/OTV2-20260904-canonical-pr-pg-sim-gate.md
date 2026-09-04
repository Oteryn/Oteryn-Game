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
final_head_frozen_at: 2026-09-04T14:00:00Z
owner: ChatGPT GPT-5.6 Pro implementation worker
created_at: 2026-09-04T13:56:00Z
updated_at: 2026-09-04T14:00:00Z
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
- `PROVEN` — ruleset `20991995` requires the stable `game-gate`; this task does not change the ruleset or status name.
- `PROVEN` — adjacent `rust.yml` PG/SIM results are not child predicates of the PR aggregate.
- `PROVEN` — protected-base merge-authority audit pins `merge-group-gate.yml`; this task leaves the gate and pin byte-identical.
- `DERIVED` — putting PG in the already-required Linux job and SIM in the already-required Windows job strengthens canonical evidence while preserving the existing scope, validate and `game-gate` fan-in implementations and their pinned hashes.

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
  direct_and_reconciled_paths: required_linux_and_windows_jobs_plus_existing_aggregate
  restart_retry_replay_concurrency_pg_reload: real_postgresql_reload_harness
  evidence:
    - tools/repository/validate_pr_gate_pg_sim.py
    - hosted RED run 33881045461 job 101049506434
finding_dispositions:
  p0_p1_verified_repair_or_rejection: pending_independent_review
  p2_fixed_accepted_or_deferred: pending_independent_review
```

## Acceptance criteria

- [x] Deterministic validator names the exact required PG and SIM contracts.
- [x] Validator-only RED executed on exact `95812aaffe88974958b73803760e070e8c2abe2b` and failed only on missing canonical PG/SIM contracts.
- [x] Required Linux job starts pinned PostgreSQL 17.6, verifies exact PR SHA and runs `durability_postgres`.
- [x] Required Windows job verifies exact PR SHA and runs simulation golden fixtures.
- [x] Existing Linux, Windows client, dependency review, CodeQL, governance and supply-chain work remains intact.
- [x] Existing scope, validate and `game-gate` fan-in implementations remain unchanged; their protected policy hashes match the current baseline.
- [x] `merge-group-gate.yml`, protected audit pin and `rust.yml` remain unchanged.
- [ ] Exact-head canonical `game-gate` passes with inspectable PostgreSQL and simulation steps.
- [ ] One independent deep review has no unresolved actionable finding.

## Excluded scope

No runtime/test-source edit, no #252-owned path, no merge-group gate, protected audit, ruleset, branch protection, risk classifier, lane skipping, deduplication, production, secret or external-repository change. Merge-group PG/SIM remains staged through #284 and #285.

## Implementation / findings

### TDD RED

- Exact RED head: `95812aaffe88974958b73803760e070e8c2abe2b`.
- Agent Governance run `33881045461`, job `101049506434`, checked out the exact RED SHA.
- All existing governance/prompt checks passed.
- Repository policy then failed only on 15 expected missing job-local fragments: PostgreSQL service/image/user/password/database/port/health/env/SHA/test in `rust_linux`, and exact-SHA/simulation execution in `rust_windows`.
- Architecture run `33881045529` passed on the same RED generation.

### Minimal GREEN

- `rust_linux` retains its existing build, strict Clippy, workspace tests, synthetic harness and server smoke. It adds the already pinned PostgreSQL 17.6 service, exact checkout verification and one explicit `durability_postgres` execution with the admin URL scoped to that step so ordinary workspace tests do not acquire PostgreSQL provenance accidentally.
- `rust_windows` retains client build, strict Clippy, smoke and synthetic harness. It adds exact checkout verification and deterministic simulation golden execution.
- `validate_pr_gate_pg_sim.py` remains the executable regression contract; the wrapper still runs the protected-base audit and existing core validator.
- `BUILD_TEST_MATRIX.md` now distinguishes stable `game-gate`, internal `Merge gate / validate`, canonical PR PG/SIM and the still-pending merge-group work.
- The exact GREEN commit SHA is established by authoritative branch/PR readback after publication and is not self-embedded here.

## Validation

### Focused

- command/run: `python tools/repository/validate_pr_gate_pg_sim.py` against final candidate
- result: local isolated static execution PASS
- command/run: PyYAML syntax parse of final `merge-gate.yml`
- result: PASS; mapping contains 10 jobs
- command/run: repository-policy block hash check
- result: `scope=c4ed68e5e828897500f6fe0cde71f0bbc4de853c585508b893e1c066bb900ab1` MATCH; `validate=c10c941048014cfc8712b0d02eee438a3dabaf6578c212e4c861d36a02d4f11a` MATCH
- command/run: fail-closed scan of modified jobs
- result: no `continue-on-error`

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`; canonical hosted `game-gate`
- result: pending exact GREEN head

### E2E

- scenario: real isolated PostgreSQL durability binary and deterministic Windows simulation golden fixtures on exact PR head
- result: pending hosted GREEN

### Exact-head CI

- final head: established by authoritative GitHub readback after GREEN publication
- trigger source: Draft PR #287 synchronize
- workflow/run/job: pending
- runner assignment: GitHub-hosted Linux and Windows
- classification: material control-plane required-check strengthening
- result: pending

## Self-review

- exact head: established after GREEN publication
- method/reviewer: implementing agent whole-diff review
- material findings: none in local generated candidate; hosted evidence pending
- verdict: PASS_PENDING_EXACT_HEAD_HOSTED_VALIDATION

## Independent review

- required: YES — required-check composition and workflow control plane
- exact head: pending stable GREEN readback
- method/auditor: one independent deep review under current META policy
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: five allocated paths only
- unresolved review threads: pending
- related/superseded PRs: #284 and #285 are subsequent staged merge-group work
- protected auto-merge: disabled/not requested
- merge commit/result: pending control-plane integration
- ownership release: after protected-main terminal readback

## Context checkpoint

```yaml
last_progress: hosted exact-head RED proven and minimal GREEN prepared with unchanged aggregate hashes
status: validating
branch: ci/canonical-pr-pg-sim-279
head_sha: null
pr: 287
final_head_sha: null
final_head_frozen_at: 2026-09-04T14:00:00Z
ci_trigger_source: pull_request_synchronize_after_green_publication
ci_check_generation: green_pending
ci_checks_for_current_head: 0
ci_run_ids:
  - 33881045461_RED_FAILURE_EXPECTED
  - 33881045529_RED_ARCHITECTURE_SUCCESS
ci_job_ids:
  - 101049506434_RED_POLICY_FAILURE_EXPECTED
runner_assignment_state: github_hosted_green_pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish the minimal GREEN, verify branch and PR exact head, then require exact-head repository policy, Linux PostgreSQL, Windows simulation, canonical game-gate, whole-diff self-review and one independent deep review before READY_FOR_INTEGRATION; do not merge
```
