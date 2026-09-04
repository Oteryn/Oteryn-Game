# OTV2-20260904-authority-api-floor

```yaml
task_id: OTV2-20260904-authority-api-floor
title: Enforce authority API contract floor
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: refactor/authority-api-floor-280
pr: 289
issue: 280
parent_issue: 277
base_sha: b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol implementation worker
created_at: 2026-09-04T16:41:00Z
updated_at: 2026-09-04T16:55:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/mod.rs
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
- `PROVEN` — RED `600a8139778923136e03632ef661849806c07b58` fails specifically because that candidate constructor remains production-public.
- `PROVEN` — attempted minimal GREEN exposed separately compiled all-target/binary test call sites in `durability/db.rs` and `durability/mod.rs`; a `#[cfg(test)]` wrapper on the library is unavailable to those external-library contexts. No GREEN was published from the blocked attempt.
- `PROVEN` — historical PR #243 is superseded read-only provenance and does not retain current mutating ownership after #252 integration.
- `DERIVED` — the minimal repair therefore needs two additional same-repository test paths for mechanical explicit candidate construction; production semantics remain limited to constructor visibility/internal expected-binding separation.

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

- [x] A distinct test-only RED proves `ReconnectCandidateBindingV1::from_record(...)` is production-public on the admission baseline.
- [ ] Production source exposes no record-derived current-authority or current-candidate convenience constructor.
- [ ] Internal validation may derive expected candidate binding from immutable record data without presenting it as current evidence.
- [ ] Separately compiled test helpers in `durability/db.rs`, `durability/mod.rs`, `durability/schema.rs` and PostgreSQL support use explicit `ReconnectCandidateBindingV1::new(...)` where the test-only wrapper is unavailable.
- [ ] Existing V1/V2 complete-current-authority reconciliation and historical-terminal behavior remain unchanged.
- [ ] Focused/package/all-target tests, strict Clippy, real PostgreSQL and exact-head `game-gate` pass.
- [ ] Whole-diff family sweep and one independent deep review have no unresolved actionable finding.

## Excluded scope

No workflow/ruleset/branch-protection change, no persistence-schema semantic change, no new authority token/control plane, no production/live data, and no runtime semantic refactor in the mechanically added test paths.

## Implementation / findings

TDD RED is preserved at `600a8139778923136e03632ef661849806c07b58`. The blocked GREEN attempt made no commit. The six-path allocation is the smallest verified scope that can both hide the production constructor and keep all-target/binary test contexts compiling without weakening the API-floor assertion.

## Validation

### Focused

- command/run: `cargo test -p oteryn-game-server --lib record_derived_candidate_binding_is_internal_and_its_convenience_is_test_only`
- result: expected RED at `600a8139778923136e03632ef661849806c07b58`; failure is the production-public candidate `from_record`

### Component/integration

- command/run: pending GREEN all-target/lib/Clippy
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

- changed-file review: six allocated paths maximum
- unresolved review threads: pending
- related/superseded PRs: #252 merged dependency; #243 historical read-only provenance
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: after protected-main readback

## Context checkpoint

```yaml
last_progress: RED compile evidence proved two additional mechanical test paths are required for a safe GREEN
status: implementing
branch: refactor/authority-api-floor-280
head_sha: established_by_authoritative_remote_readback
pr: 289
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: RED
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish the minimal six-path GREEN that hides the production candidate record constructor and mechanically adapts separately compiled test helpers
```
