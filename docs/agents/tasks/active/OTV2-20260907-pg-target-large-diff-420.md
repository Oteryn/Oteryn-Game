# OTV2-20260907-pg-target-large-diff-420

```yaml
task_id: OTV2-20260907-pg-target-large-diff-420
title: Repair exact PostgreSQL target qualification for large diffs
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/wp3-pg-target-420
pr: 422
base_sha: 4bc27844ffde2a645b5df85c7268babae283d866
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-worker
created_at: 2026-09-07T22:36:36.324Z
updated_at: 2026-09-07T23:30:00Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - docs/agents/tasks/active/OTV2-20260907-pg-target-large-diff-420.md
public_contracts: []
depends_on:
  - protected PR #421 / 4bc27844ffde2a645b5df85c7268babae283d866
blocks:
  - WP3 #351/#356 hosted PostgreSQL qualification
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Make the required PR PostgreSQL classifier qualify a legitimate 803-file WP3 candidate through authenticated immutable exact-target observations at validated base/head commits, without weakening fail-closed routing, checkout parity, race/ABA controls, PostgreSQL 17.6 execution, or the stable `game-gate`.

## Architecture and source of truth

- PROVEN: protected allocation `docs/agents/programs/OTV2_WP3_PG_TARGET_LARGE_DIFF_ALLOCATION_20260907.md`, blob `2b82bedd7e3315e23bf9d6e3d0cd5434d21652d0`, is integrated at `main@4bc27844ffde2a645b5df85c7268babae283d866`.
- PROVEN: #420 records the active Work application and exact sole lease.
- PROVEN: #257/#258 overlap only on workflow custody and their integration is serialized while this repair is active.
- PROVEN: #356 retains separate WP3 source/Cargo/include-only test-target custody and must not write this task's paths.
- UNKNOWN until hosted execution: corrected protected classifier qualification of the actual 803-file #356 candidate.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: ImmutableCommitTargetEvidence_x_PostgreSQLClassifier_x_RequiredGate
authority_invariants:
  - authenticated same-repository open PR with validated immutable base/head commits
  - exact canonical target path and expected file payload type
  - API target state agrees with verified exact-head checkout
  - pre/post PR identity and changed-file count remain stable
consumer_boundaries:
  - required Linux PostgreSQL target selection
  - repository policy digest and final game-gate propagation
mutation_operators:
  applicable:
    - missing target
    - removal or rename away
    - introduced target
    - malformed or mismatched payload
    - authorization transport rate-limit and server errors
    - head base repository count or state movement
    - checkout target mismatch
    - A-to-B-to-A observation race
  considered_not_applicable:
    - runtime gameplay authority mutation
one_invariant_per_negative_case: required
independent_current_fact_sources:
  - GitHub exact commit contents API
  - verified exact-head checkout
record_derived_matching_helper:
  allowed_for_positive_happy_path: false
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: target present absent introduced removed renamed
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: PR classifier and Linux checkout parity
  fenced_durable_writes: NOT_APPLICABLE
  restart_retry_replay_concurrency_pg_reload: API race ABA and PostgreSQL target execution
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - depth-1 regression depended on unavailable historical Git object
    - target-path 404 could conceal unavailable commit or authorization
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - require valid API blob SHA and exact-head checkout blob parity
    - strictly validate post-inspection changed-files count before comparison
    - require successful complete immutable tree evidence before classifying target absence
    - PRRT_kwDOT8SzxM6gITQh: reject malformed paths and illegal type/mode pairs in every returned tree entry
```

## Acceptance criteria

- [x] Preserve focused RED showing the protected classifier rejects a valid 803-file target-present candidate before qualification.
- [x] GREEN exact-target cases cover large-diff present, removed, renamed-away, introduced and genuine both-absent history.
- [x] Malformed/type/path payloads and auth/transport/rate-limit/server failures fail closed.
- [x] Head/base/repository/count/state movement, checkout mismatch and immutable A-to-B-to-A controls fail closed.
- [x] No mutable PR-file pagination and no raised enumeration cap are used for target authority.
- [x] Matching Linux evidence-job digest and mandatory PostgreSQL invocation validate; stale digest or removed invocation fails.
- [x] Target failure reaches the existing required aggregate; no success/skip masks failure.
- [x] Focused repository-policy and PG/SIM regressions pass (the combined cross-platform runner additionally requires `pwsh`, unavailable locally).
- [ ] Independent exact-head CONTROL review is clean, canonical CI passes, normal FULL MQ integrates, and protected readback succeeds.
- [ ] The actual 803-file WP3 candidate executes the corrected hosted PostgreSQL 17.6 target before WP3 qualification is claimed.

## Excluded scope

No WP3 vendor/Cargo/source, Foundation/WP2, SQL/migration/shared PostgreSQL test target, WP4/WP5, Atlas, registry, production, external repository, ruleset, required-status, Merge Queue semantics, permissions, dependency bump, `merge-group-gate.yml`, `rust.yml`, `merge-authority-audit.yml` or unrelated workflow/job changes.

## Implementation / findings

Replaced the downstream PostgreSQL classifier's immutable compare enumeration with authenticated Git commit/tree observations of the one canonical path at exact base and head SHAs. Absence requires a successful identity-matched, complete, non-truncated tree response whose every entry has a valid relative path, object SHA and legal Git type/mode pair; ambiguous contents 404s are not target evidence. A present target must be a regular or executable blob with a valid SHA matching the exact-head checkout. Removal/rename-away, unavailable or malformed tree evidence, malformed post-inspection counts and checkout/API blob disagreement fail closed. The pre/post PR identity, state and count fence remains unchanged, while valid large changed-file counts no longer block target qualification.

The actual inline workflow harness preserves the protected 803-file target-present RED as a self-contained fixture that does not require historical Git objects in a depth-1 checkout. It is GREEN for the allocated state/error/race matrix, including ambiguous/auth-masked 404s, successful both-absent immutable trees, deletion/rename, missing commits, malformed/mismatched/truncated trees, malformed paths and type/mode pairs, legitimate non-target trees/symlinks/submodules, stricter target object modes, checkout mismatch and strict post-count controls. The Linux job digest and mandatory invocation mutations are both negative controls.

## Validation

### Focused

- command/run: `python tools/repository/validate_pr_gate_pg_sim.py`; direct execution of all 20 inline PG/SIM regression functions in `tools/repository/test_validate_pr_gate_pg_sim.py`
- result: PASS

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`; `python tools/agents/validate_governance.py`; `python tools/agents/tests/test_governance_lifecycle_discovery.py`
- result: repository policy PASS; governance validation PASS after PR metadata update; lifecycle discovery PASS

### E2E

- scenario: NOT_APPLICABLE — this task repairs CI target authority; actual hosted PostgreSQL qualification of #356 is a required downstream acceptance step.
- result: NOT_APPLICABLE locally; downstream hosted execution on #356 remains required and unclaimed

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: CONTROL / FULL
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent, adversarial whole-diff inspection plus focused mutation regressions
- material findings: corrected an initially misplaced `urllib.error` import before evidence capture; no unresolved material finding
- verdict: locally coherent; independent review and exact-head canonical CI remain pending

## Independent review

- required: YES — CONTROL change to required PostgreSQL evidence routing
- exact head: pending
- method/auditor: genuinely independent non-author reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly the four allocated paths; no source/Cargo/dependency/permission/status/MQ changes
- unresolved review threads: pending
- related/superseded PRs: #421 allocation protected; #257/#258 held
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: exact-target classifier implementation and local RED/GREEN evidence complete
status: implementing
branch: coord/wp3-pg-target-420
head_sha: null
pr: 422
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish the implementation commit on Draft PR #422, then await genuinely independent CONTROL review and exact-head canonical CI
```
