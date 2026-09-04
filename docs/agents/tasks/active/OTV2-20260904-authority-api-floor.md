# OTV2-20260904-authority-api-floor

```yaml
task_id: OTV2-20260904-authority-api-floor
title: Enforce authority API contract floor
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: refactor/authority-api-floor-280
pr: null
issue: 280
parent_issue: 277
base_sha: b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol implementation worker
created_at: 2026-09-04T16:41:00Z
updated_at: 2026-09-04T16:41:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/tests/support/postgres.rs
  - docs/agents/tasks/active/OTV2-20260904-authority-api-floor.md
public_contracts:
  - reconnect current-authority construction boundary
depends_on:
  - issue: 250
    state: completed
    protected_main_readback: b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3
blocks:
  - issue: 281
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Production authority-granting paths cannot use a public record-derived constructor as current candidate/current authority evidence. Immutable record data may still construct an internal expected binding for comparison, while callers supply live current facts independently.

## Architecture and source of truth

- `PROVEN` — PR #252 is integrated as protected `main@b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3`; Issue #250 is closed and its task branch is deleted.
- `PROVEN` — `ReconnectCurrentAuthorityV1::from_record(...)` and `GameSessionAuthoritySnapshot::new(...)` are already `#[cfg(test)]` on protected main.
- `PROVEN` — `ReconnectCandidateBindingV1::from_record(...)` remains production-public and derives candidate identity/attempt/generation/transport/deadline from immutable `ReconnectDurabilityRecordV1`.
- `PROVEN` — V1/V2 committed reconciliation consumes complete `ReconnectCurrentAuthorityV1`; historical terminal outcomes do not install a controller.
- `DERIVED` — the minimal remaining API-floor repair is to make record-derived candidate construction an internal expected-binding helper, retain a test-only convenience wrapper, and mechanically adapt external PostgreSQL test support to explicit `ReconnectCandidateBindingV1::new(...)`.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - current_candidate_provenance
  - complete_current_authority_provenance
consumer_boundaries:
  - direct_commit_authorization
  - committed_v1_reconciliation
  - committed_v2_reconciliation
  - terminal_replacement_prepare_authorization
mutation_operators:
  applicable:
    - provenance_substitution
    - stale_candidate_binding
    - missing_candidate_binding
  considered_not_applicable:
    - historical_terminal_live_authority_equality
one_invariant_per_negative_case: true
independent_current_fact_sources:
  - public_ReconnectCurrentAuthorityV1_from_current_facts
record_derived_matching_helper:
  allowed_for_positive_happy_path: test_only_or_internal_expected_binding
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required
  protocol_versions: V1_and_V2
  direct_and_reconciled_paths: required
  fenced_durable_writes: NOT_APPLICABLE_no_new_write_semantics
  restart_retry_replay_concurrency_pg_reload: existing_behavior_must_remain_green
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [ ] A distinct test-only RED proves `ReconnectCandidateBindingV1::from_record(...)` is production-public on the admission baseline.
- [ ] Production source exposes no record-derived current-authority or current-candidate convenience constructor.
- [ ] Internal validation may derive expected candidate binding from immutable record data without presenting it as current evidence.
- [ ] External PostgreSQL test support builds matching candidate bindings explicitly via `ReconnectCandidateBindingV1::new(...)`.
- [ ] Existing V1/V2 complete-current-authority reconciliation and historical-terminal behavior remain unchanged.
- [ ] Focused/package tests, strict Clippy, real PostgreSQL and exact-head `game-gate` pass.
- [ ] Whole-diff family sweep and one independent deep review have no unresolved actionable finding.

## Excluded scope

No workflow/ruleset/branch-protection change, no schema semantic change, no new authority token/control plane, no refactor outside the four allocated paths, no production/live data.

## Implementation / findings

TDD is mandatory. Publish a distinct RED in `apps/game-server/src/durability/schema.rs` before any production edit. GREEN is limited to the visibility/helper repair in Foundation and mechanical test-support adaptation.

## Validation

### Focused

- command/run: pending RED
- result: pending

### Component/integration

- command/run: pending
- result: pending

### E2E

- scenario: real isolated PostgreSQL durability harness
- result: pending

### Exact-head CI

- final head: pending
- trigger source: pull_request synchronize
- workflow/run/job: pending
- runner assignment: GitHub-hosted
- classification: high-risk authority API hardening
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent whole-diff family sweep
- material findings: pending
- verdict: pending

## Independent review

- required: YES — authority/reconciliation boundary
- exact head: pending
- method/auditor: one independent deep review under current META policy
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly four allocated paths
- unresolved review threads: pending
- related/superseded PRs: #252 merged dependency
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: after protected-main readback

## Context checkpoint

```yaml
last_progress: dependency released and exact post-252 API-floor allocation established
status: implementing
branch: refactor/authority-api-floor-280
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
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
next_action: publish a distinct schema contract RED proving production-public ReconnectCandidateBindingV1::from_record before any production edit
```
