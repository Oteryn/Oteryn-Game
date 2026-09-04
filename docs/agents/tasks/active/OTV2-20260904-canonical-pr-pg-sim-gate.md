# OTV2-20260904-canonical-pr-pg-sim-gate

```yaml
task_id: OTV2-20260904-canonical-pr-pg-sim-gate
title: Canonicalize PR PostgreSQL and simulation exact-head gates
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/canonical-pr-pg-sim-279
pr: 287
issue: 279
parent_issue: 277
base_sha: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol implementation worker
created_at: 2026-09-04T13:56:00Z
updated_at: 2026-09-04T17:14:00Z
execution_budget_minutes: 60
large_budget_reason: required-check composition with hosted RED/GREEN and review-repair evidence
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_repository_policy.py
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260904-canonical-pr-pg-sim-gate.md
public_contracts:
  - canonical PR game-gate composition
depends_on:
  - issue: 277
  - issue: 278
    state: completed
    protected_main_readback: 68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705
blocks:
  - issue: 283
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Every Rust/workspace-relevant pull-request head executes deterministic Windows simulation inside canonical `game-gate`. The same required Linux job owns a pinned PostgreSQL 17.6 service and runs the real `durability_postgres` target whenever that target exists on the exact candidate. Removal/rename fails closed; revisions predating allocation are explicit `NOT_APPLICABLE`; and the actual PG/SIM evidence steps cannot be converted into non-failing skips while repository-policy validation remains green.

## Architecture and source of truth

- `PROVEN` — ruleset `20991995` uses the stable canonical `game-gate`; this task does not change the ruleset or required status name.
- `PROVEN` — governance convergence #286 is integrated at protected `main@68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705` and now governs this repair.
- `PROVEN` — PR #252 is integrated, so `apps/game-server/tests/durability_postgres.rs` is now present and current exact-head PR qualification can execute the real PostgreSQL harness.
- `PROVEN` — adjacent `rust.yml` results are not canonical PR child predicates; C1 therefore places PG/SIM inside already-required Merge Gate Rust jobs.
- `PROVEN` — protected merge-group gate remains unchanged in this task; #284/#285 own that separate control-plane transition.
- `DERIVED` — evidence-step structure must itself be validated, because command-presence checks alone do not prove GitHub Actions will execute a step.

## High-risk authority/recovery qualification

```yaml
applicable: CONTROL_PLANE_REQUIRED_CHECK_COMPOSITION
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - canonical_windows_job_runs_simulation
  - allocated_postgresql_target_runs_in_canonical_linux_job
  - candidate_cannot_remove_or_rename_postgresql_target_into_skip
  - applicable_postgresql_and_simulation_steps_are_unconditional
  - historical_absence_is_not_misreported_as_postgresql_pass
consumer_boundaries:
  - pull_request_game_gate
mutation_operators:
  - remove_simulation_contract
  - remove_postgresql_contract
  - remove_or_rename_postgresql_test_target
  - add_false_condition_to_postgresql_evidence_step
  - add_false_condition_to_simulation_evidence_step
  - add_continue_on_error_to_evidence_step
independent_current_fact_sources:
  - exact_pull_request_head_resolved_by_scope
  - exact_head_checkout
  - exact_live_pull_request_changed_file_statuses
record_derived_matching_helper:
  allowed_for_positive_happy_path: NOT_APPLICABLE
  forbidden_for_negative_authority_or_provenance_cases: NOT_APPLICABLE
finding_family_sweep:
  sibling_apis: both_canonical_PR_evidence_steps
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: linux_postgresql_windows_simulation_and_existing_aggregate
  fenced_durable_writes: NOT_APPLICABLE_control_plane_only
  restart_retry_replay_concurrency_pg_reload: durability_postgres_target_when_allocated
  evidence:
    - tools/repository/validate_pr_gate_pg_sim.py
    - tools/repository/test_validate_pr_gate_pg_sim.py
    - RED 95812aaffe88974958b73803760e070e8c2abe2b
    - RED 891adbf70723ef5f558e15aa69e58ce1a6c957a1
    - RED 8636afc54da0c9a900aca1a37a490432cf764c87
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - review_comment_3936176055_step_skip_gap
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] Deterministic validator names the exact PG, SIM and deletion-safe routing contracts.
- [x] RED 1 proves the pre-change PR gate lacked canonical PG/SIM contracts.
- [x] Initial runtime GREEN proves Windows SIM execution and exposed historical target-allocation compatibility.
- [x] RED 2 proves deletion/rename routing contracts were missing before their repair.
- [x] Required Linux job starts pinned PostgreSQL 17.6, verifies exact PR SHA and runs real `durability_postgres` when allocated.
- [x] Required Windows job verifies exact PR SHA and runs deterministic simulation golden fixtures.
- [x] Exact current-head PostgreSQL execution is proven after #252 integration.
- [x] Independent review finding `3936176055` is accepted: whole-job substring validation could not prevent `if: false` on evidence steps.
- [x] RED 3 at `8636afc54da0c9a900aca1a37a490432cf764c87` independently proves both PostgreSQL and simulation skipped-step mutations were accepted by the old validator.
- [ ] Structural evidence-step validation rejects any step-level `if:` and `continue-on-error` for canonical PG/SIM execution while preserving the required job-level Rust applicability condition.
- [ ] Focused regression suite and repository-policy validator pass on the repaired candidate.
- [ ] Exact final-head canonical `game-gate` passes with real PostgreSQL and Windows SIM evidence.
- [ ] Whole-diff family sweep finds no sibling bypass.
- [ ] One independent deep re-review after the P1 repair has no unresolved actionable finding.

## Excluded scope

No Game runtime/test-source edit, no merge-group gate or protected audit mutation, no ruleset/branch-protection change, no risk-scoped lane omission, no `rust.yml` deduplication, no production/secret/external-repository mutation.

## Implementation / findings

### RED 1 — missing canonical PG/SIM contracts

- exact `95812aaffe88974958b73803760e070e8c2abe2b`;
- Agent Governance `33881045461` failed only on expected missing PG/SIM contracts;
- Architecture `33881045529` passed.

### Initial GREEN and compatibility discovery

- `fe8e76c617472b6281e519647cc099ebc7b7d1ad` proved exact Windows SIM in Merge Gate `33881858954` but Linux exposed that pre-#252 main had no durability test target.

### RED 2 / deletion-safe routing

- exact `891adbf70723ef5f558e15aa69e58ce1a6c957a1`;
- Agent Governance `33883182869` failed only on the newly required exact-head changed-file/removal/rename contracts.

### First stable GREEN generation

- `2ac8ac57d75310510f56e4426cf3cd5e5cfc7113`, later non-force reconciled to `10f21fb0...` and `f2d5bf340e3e5c256424f017275bcac66be33460` as protected main advanced.
- On `f2d5bf340...`, Linux exact-head job executed real PostgreSQL 17.6 durability successfully and Windows exact-head qualification was progressing under the new governance.

### Accepted review P1 — evidence step may be skipped

- Independent review `5115755399` produced P1 comment `3936176055`: inserting a false step-level `if:` preserves all required strings yet GitHub marks the step skipped/non-failing, allowing the job/aggregate to pass without evidence.
- The current-head review summary later identified `f2d5bf3` as the reviewed generation; the finding therefore applies to the material candidate and is accepted rather than dismissed as historical noise.

### RED 3 — structural step condition

- regression-only exact head `8636afc54da0c9a900aca1a37a490432cf764c87` adds no production/workflow change;
- `python tools/repository/test_validate_pr_gate_pg_sim.py` exits 1 with `AssertionError: validator accepted a skipped PostgreSQL evidence step`;
- an independent invocation of `test_simulation_evidence_step_cannot_be_skipped()` exits 1 with `AssertionError: validator accepted a skipped simulation evidence step`;
- `python tools/repository/validate_repository_policy.py` remains 0 on the unmutated real gate, proving the RED isolates the mutation-detection gap.

### Minimal GREEN design

- extract exactly one named step block from each already-required Rust job;
- require the canonical evidence command inside that step;
- reject any step-local `if:` (quoted or unquoted) and `continue-on-error`;
- preserve job-level `if: needs.scope.outputs.rust == 'true'`, exact SHA checks, PG target classifier and aggregate fan-in unchanged.

## Validation

### Focused

- `python tools/repository/test_validate_pr_gate_pg_sim.py`: pending GREEN
- `python tools/repository/validate_pr_gate_pg_sim.py`: pending GREEN
- `python tools/repository/validate_repository_policy.py`: pending GREEN

### Component/integration

- canonical hosted PR Merge Gate with PostgreSQL 17.6 and Windows SIM: pending repaired exact head

### E2E

- real PostgreSQL durability target: required on repaired exact head
- deterministic Windows simulation: required on repaired exact head

### Exact-head CI

- final head: established by authoritative GitHub readback after repair publication
- trigger source: PR #287 synchronize
- workflow/run/job: pending repaired generation
- runner assignment: GitHub-hosted Linux and Windows
- classification: material control-plane required-check repair
- result: pending

## Self-review

- exact head: pending repaired generation
- method/reviewer: implementing agent whole-diff + finding-family sweep
- material findings: accepted P1 `3936176055` under repair
- verdict: pending

## Independent review

- required: YES — required-check composition; accepted material P1 invalidates prior review as final evidence
- exact head: pending stable repair generation
- method/auditor: one independent deep Codex re-review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly six allocated paths maximum
- unresolved review threads: P1 `PRRT_kwDOT8SzxM6fV1h3` remains unresolved until GREEN + exact-head qualification + re-review
- related work: #284/#285 separately stage merge-group PG/SIM; #283 remains blocked
- protected auto-merge: disabled/not requested
- merge commit/result: pending
- ownership release: after protected-main readback

## Context checkpoint

```yaml
last_progress: independent P1 converted into two precise skipped-evidence RED regressions on exact 8636afc5
status: implementing
branch: ci/canonical-pr-pg-sim-279
head_sha: null
pr: 287
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: review_repair_red3
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: github_hosted
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish minimal structural evidence-step GREEN, then require focused regressions, repository policy, exact-head PostgreSQL/SIM game-gate, whole-diff family sweep and one independent re-review before integration
```
