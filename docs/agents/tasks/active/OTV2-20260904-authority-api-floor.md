# OTV2-20260904-authority-api-floor

```yaml
task_id: OTV2-20260904-authority-api-floor
title: Enforce authority API contract floor
mode: REPAIR
status: WAITING_DEPENDENCY
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
updated_at: 2026-09-04T20:16:00Z
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

Production authority-granting paths cannot use immutable prepared/persisted identity as current candidate, current account-presence or current character/world-eligibility evidence. Record/identity data may construct only internal expected values for comparison; production callers must supply independently observed current facts.

## Architecture and source of truth

- `PROVEN` — PR #252 is integrated as protected `main@b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3`; Issue #250 is closed and its old implementation branch is deleted.
- `PROVEN` — protected-main readback during the sibling-family repair reached `main@68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705`; nearest `apps/game-server/AGENTS.md` now explicitly requires independent current facts and a finding-family sweep for authority/recovery work.
- `PROVEN` — `ReconnectCurrentAuthorityV1::from_record(...)` and `GameSessionAuthoritySnapshot::new(...)` are `#[cfg(test)]` conveniences, not production authority APIs.
- `PROVEN` — original RED `600a8139778923136e03632ef661849806c07b58` proved production-public `ReconnectCandidateBindingV1::from_record(...)` violated the API floor.
- `PROVEN` — qualification analysis then identified the same authority-provenance family in production-public `AccountPresenceClaimV1::from_identity(...)` and `CharacterWorldEligibilityClaimV1::from_identity(...)`.
- `PROVEN` — fresh sibling-family RED `ddbb44d2644c6f66bf86aba837d7712b01878fac` failed the new API-floor contract before the sibling production surface was hidden.
- `PROVEN` — material GREEN `63357a7e2f8f5ff15f72eb7e7a404a94b0cc906d` hid both sibling conveniences behind `#[cfg(test)]`, retained private `expected_from_identity(...)` helpers, retained private `expected_binding_from_record(...)`, and adapted separately compiled tests to explicit constructors.
- `PROVEN` — formatting-only follow-up `7993a4983374fe4764d51bd8b68593178dca63a6` applied the exact rustfmt diff reported by Merge Gate.
- `PROVEN` — strict-Clippy-only follow-up `67d3d3a9e513ce4c546467bdf5c305ccc47950ae` replaced the test-only `panic!` with an equivalent assertion/fallback and changed no production behavior.
- `PROVEN` — whole-diff changed-file enumeration remains exactly the six allocated Issue #280 paths.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - current_candidate_provenance
  - current_account_presence_provenance
  - current_character_world_eligibility_provenance
  - complete_current_authority_provenance
consumer_boundaries:
  - direct_commit_authorization
  - committed_v1_reconciliation
  - committed_v2_reconciliation
  - terminal_replacement_prepare_authorization
mutation_operators:
  applicable:
    - provenance_substitution
    - account_substitution
    - character_substitution
    - world_substitution
    - stale_candidate_binding
    - missing_candidate_binding
    - restart_retry_replay
    - concurrent_replacement
    - durable_mirror_corruption
  considered_not_applicable:
    - historical_terminal_live_authority_equality
one_invariant_per_negative_case: true
independent_current_fact_sources:
  - public_ReconnectCurrentAuthorityV1_from_current_facts
record_derived_matching_helper:
  allowed_for_positive_happy_path: test_only_or_internal_expected_value
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: complete_preintegration
  protocol_versions: V1_and_V2
  direct_and_reconciled_paths: complete_preintegration
  fenced_durable_writes: unchanged_no_new_write_semantics
  restart_retry_replay_concurrency_pg_reload: complete_preintegration
  evidence:
    - material_head_67d3d3a9e513ce4c546467bdf5c305ccc47950ae
    - postgres_117_of_117
    - merge_gate_33914639361
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - ReconnectCandidateBindingV1_from_record_production_convenience
    - AccountPresenceClaimV1_from_identity_production_convenience
    - CharacterWorldEligibilityClaimV1_from_identity_production_convenience
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] A distinct RED proves `ReconnectCandidateBindingV1::from_record(...)` was production-public on the admission baseline.
- [x] A fresh sibling-family RED proves both identity-derived authority claim conveniences violated the same production API floor.
- [x] Production source exposes no record/identity-derived current-authority, current-candidate, current-account-presence or current-world-eligibility convenience constructor.
- [x] Internal validation may derive expected candidate/account/world values from immutable record data without presenting them as current evidence.
- [x] Separately compiled test helpers use explicit `new(...)` construction where same-crate `#[cfg(test)]` wrappers are unavailable.
- [x] Existing V1/V2 complete-current-authority reconciliation, terminal replacement, retry/replay/restart/concurrency and historical-terminal behavior remain unchanged on the pre-integration material head.
- [x] Pre-integration focused/package/all-target behavior, strict Clippy, real PostgreSQL and exact-head canonical `game-gate` pass on material head `67d3d3a9e513ce4c546467bdf5c305ccc47950ae`.
- [x] Pre-integration whole-diff AuthorityInvariant × ConsumerBoundary × MutationOperator family sweep has P0=0, P1=0, P2=0.
- [ ] After PR #287 and PR #291 are integrated to protected main, perform one normal non-force merge-up and rerun every invalidated exact-head validation/review layer.
- [ ] One independent Codex deep review of the final post-merge-up material candidate has no unresolved actionable finding.
- [ ] Mark READY_FOR_INTEGRATION only after the final post-dependency exact-head qualification has P0=0, P1=0, P2=0.

## Excluded scope

No workflow/ruleset/branch-protection change, no persistence-schema semantic change, no new authority token/control plane, no production/live data, no direct merge, and no mutation of dependency PR branches. PR #287 and PR #291 are read-only dependency state for this task.

## Implementation / findings

TDD lineage:

1. `600a8139778923136e03632ef661849806c07b58` — original candidate-binding RED.
2. `ddbb44d2644c6f66bf86aba837d7712b01878fac` — fresh sibling-family RED for account-presence and character/world-eligibility conveniences.
3. `63357a7e2f8f5ff15f72eb7e7a404a94b0cc906d` — minimal material GREEN across the authority family.
4. `7993a4983374fe4764d51bd8b68593178dca63a6` — exact rustfmt-only repair after deterministic Merge Gate evidence.
5. `67d3d3a9e513ce4c546467bdf5c305ccc47950ae` — exact strict-Clippy-only test repair; production semantics unchanged.

The resulting production boundary is deliberate: `ReconnectCurrentAuthorityV1::from_current_facts(...)` consumes independently supplied current facts, while immutable record/identity derivation is confined to private expected-value helpers and `#[cfg(test)]` conveniences.

## Qualification analyst handoff and disposition

Read-only qualification analysis on the earlier candidate found the two sibling production-public `from_identity(...)` conveniences. The writer re-verified the finding on the then-current exact head, established a fresh sibling-family RED, repaired the complete family and reran the required coverage. The analyst handoff is therefore **accepted and repaired**, not waived.

Pre-integration whole-diff disposition on material head `67d3d3a9e513ce4c546467bdf5c305ccc47950ae`:

- P0: 0
- P1: 0
- P2: 0
- scope drift: 0 paths outside the six-path allocation
- unresolved review threads observed before dependency wait: 0
- runtime authority-source expansion: none

## Validation

### Focused / TDD

- original API-floor RED: `600a8139778923136e03632ef661849806c07b58` — expected failure for production-public candidate `from_record`.
- fresh sibling-family RED: `ddbb44d2644c6f66bf86aba837d7712b01878fac` — expected failure while both sibling `from_identity` conveniences remained production-public.
- GREEN contract: `identity_derived_authority_claim_convenience_is_test_only_across_sibling_family` — PASS on the material head.

### Component / integration

- exact material head: `67d3d3a9e513ce4c546467bdf5c305ccc47950ae`.
- Rust workspace workflow: `33914639397` — SUCCESS.
- Merge Gate Linux workspace: build PASS; strict Clippy PASS; workspace tests PASS; synthetic harness PASS; native game-server bootstrap PASS.
- Merge Gate Windows client: production build PASS; strict client Clippy PASS; visible smoke PASS; synthetic harness PASS (`synthetic-ok`).
- formatting / locked metadata / production closure negatives: PASS.
- supply chain / dependency review / CodeQL / governance: PASS.

### E2E

- real isolated PostgreSQL job: `101158853377` on exact material head.
- `durability_postgres`: **117 passed, 0 failed**.
- coverage includes V1/V2 current-authority revalidation, account/character/world substitutions, terminal replacement, exact receipt binding, restart/replay, concurrent winner, identical replay race, racing reconciliation/commit, rollback, stale/collision/concurrent terminal reasons, durable mirror corruption and actor-epoch attempt budget behavior.

### Exact-head CI

- pre-integration material head: `67d3d3a9e513ce4c546467bdf5c305ccc47950ae`.
- Architecture semantic audit `33914639424` — SUCCESS.
- Agent governance `33914639293` — SUCCESS.
- Rust workspace `33914639397` — SUCCESS.
- Merge Gate `33914639361` — SUCCESS.
- canonical aggregate `game-gate` job `101161048178` — SUCCESS.

These results qualify the **pre-integration material candidate only**. They do not substitute for the required final qualification after dependency integration and protected-main merge-up.

## Self-review

- exact material head: `67d3d3a9e513ce4c546467bdf5c305ccc47950ae`.
- method/reviewer: implementing agent whole-diff AuthorityInvariant × ConsumerBoundary × MutationOperator sweep.
- sibling APIs: candidate binding, account presence, character/world eligibility, current authority.
- consumer boundaries: V1/V2 direct/reconciled and terminal replacement.
- mutation/recovery coverage: stale/missing/substituted current facts, restart/retry/replay/concurrency/PostgreSQL reload/corruption.
- material findings: P0=0, P1=0, P2=0.
- verdict: PREINTEGRATION_CLEAN_WAITING_DEPENDENCY.

## Independent review

- required: YES — material high-risk authority/reconciliation boundary.
- policy: current central META AI review policy.
- pre-integration deep review: intentionally not requested; doing so before the mandatory protected-main merge-up would consume the single deep-review opportunity on a candidate known to be invalidated by dependency integration.
- final exact head: pending dependency integration + merge-up.
- method/auditor: one independent Codex deep review after deterministic final GREEN.
- verdict: pending.

## Dependency gate

`WAITING_DEPENDENCY` is concrete, not discretionary:

- PR #287 is still open/draft on head `445bc91de4c4b0f8c9415e194c6ea2ea06c6b947`; its current exact-head Merge Gate `33899983889` is FAILURE and Agent Governance `33899983882` is FAILURE. Issue #280 has no authority to mutate or merge that branch.
- PR #291 is still open/draft on head `0b134215d60bf9c63ef8406f97043d7b09755bbf`; its current Merge Gate `33900317317` and Agent Governance `33900317448` are SUCCESS, but its documented sequencing waits for #287 integration.
- final #289 qualification therefore cannot truthfully be executed against the required converged protected-main baseline yet.

Required resumption condition: #287 and #291 both have protected-main integration/readback. Then reload protected `main`, Issue #280, PR #289, this task record, root/nearest `AGENTS.md` and current META policy; perform one normal non-force merge-up; rerun focused/package/all-target strict Clippy, real PostgreSQL, canonical exact-head `game-gate`, whole-diff family sweep and exactly one final Codex deep review. Do not merge PR #289.

## PR and closeout

- changed-file review: exactly six allocated paths.
- PR state: Draft / open.
- unresolved review threads: none observed at pre-integration material freeze.
- related dependency chain: #252 integrated; #287 then #291 must integrate before #289 final merge-up.
- protected auto-merge: not enabled by this task.
- direct merge: forbidden by user instruction.
- ownership release: only after protected-main integration/readback of #280; until then #281 remains blocked.

## Context checkpoint

```yaml
last_progress: pre-integration sibling-family repair is deterministic-green with canonical game-gate success on material head 67d3d3a9e513ce4c546467bdf5c305ccc47950ae
status: WAITING_DEPENDENCY
branch: refactor/authority-api-floor-280
pr: 289
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: PREINTEGRATION_GREEN_WAITING_DEPENDENCY
ci_checks_for_current_head: preintegration_material_complete
ci_run_ids:
  - 33914639424
  - 33914639293
  - 33914639397
  - 33914639361
ci_job_ids:
  - 101158853377
  - 101161048178
runner_assignment_state: github_hosted_complete
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: complete
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: PR #287 must integrate to protected main, then PR #291 must integrate; #289 final qualification follows one normal non-force protected-main merge-up
next_action: on both dependency protected-main readbacks, refresh governance and exact heads, merge-up without force/rebase/reset, rerun all invalidated qualification layers, perform one final Codex deep review, and mark READY_FOR_INTEGRATION only if P0=0/P1=0/P2=0; do not merge
```
