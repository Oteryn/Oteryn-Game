# OTV2-20260905-foundation-fresh-admission-318

```yaml
task_id: OTV2-20260905-foundation-fresh-admission-318
title: Foundation fresh-admission durability semantics
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/foundation-fresh-admission-318
planned_worker_branch: agent/foundation-fresh-admission-318
issue: 318
parent_issue: 162
pr: 321
base_sha: 8fd0a40928c4089b453556edbf0a5abebe46986d
admission_main_sha: 8fd0a40928c4089b453556edbf0a5abebe46986d
allocation_preparation_main_sha: a8678d4a94e479a9aa2a92920379a4b32f95143b
admission_base_rule: exact protected merge SHA of the coordinator allocation containing this task; resolve and record in GitHub before first worker mutation
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn Foundation fresh-admission worker
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-05
updated_at: 2026-09-05
execution_budget_minutes: 60
large_budget_reason: null
allocation_state: protected_integrated
allocation_pr: 320
allocation_merge_sha: 8fd0a40928c4089b453556edbf0a5abebe46986d
allocation_merge_queue_run: 33984473923
worker_dispatched_at: 2026-09-05T18:44:00Z
owned_paths:
  - apps/game-server/src/foundation/fresh_admission_durability.rs
  - apps/game-server/src/foundation/admission_authority_publication.rs
  - apps/game-server/src/foundation/admission.rs
  - apps/game-server/src/foundation/admission_facade.rs
  - apps/game-server/src/foundation/fnd04_verifier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/foundation/fresh_admission_durability_tests.rs
  - docs/agents/tasks/active/OTV2-20260905-foundation-fresh-admission-318.md
public_contracts:
  - docs/architecture/reviews/OTERYN_GAME_FRESH_ADMISSION_DURABILITY_AUTHORITY_DECISION_2026-09-05.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_DURABILITY_RECONNECT_AUTHORITY_BOUNDARY_DECISION_2026-08-26.md
depends_on:
  - issue: 313
    architecture_pr: 317
    protected_merge_sha: a8678d4a94e479a9aa2a92920379a4b32f95143b
  - allocation_pr: 320
    state: merged
    protected_merge_sha: 8fd0a40928c4089b453556edbf0a5abebe46986d
blocks:
  - Child B Durability fresh-admission adapter allocation
  - Child C owning producer and composition integration
  - Server Seam Issue 247 resume
cross_repository_coordination_id: null
external_repositories: []
```

This is Child A of accepted decision `FND-DUR-FRESH-ADMISSION-V1`, allocated by protected PR #320. Work created and the worker independently read back `agent/foundation-fresh-admission-318` at immutable admission base `8fd0a40928c4089b453556edbf0a5abebe46986d`. Governing dispatch is Issue #318 comment `5553991516`. Only the exact allowlist is active. Work owns implementation PR creation and integration; no worker self-allocation or self-merge is authorized.

## Outcome

Expose a Foundation-owned verified fresh authorization with complete provenance and typed current guard expectations; bounded owning-source publication and fresh persistence submission; exact completion/reconciliation; independently-current controller adoption. Preserve synchronous compatibility as non-production and reconnect V1/V2 behavior. This child delivers no PostgreSQL implementation or production source/readiness claim.

Implementation plan: `docs/superpowers/plans/2026-09-05-foundation-fresh-admission-durability.md`.

## Architecture and source of truth

- `PROVEN`: architecture PR #317 integrated through successful protected Merge Queue run `33983003548` as `a8678d4a94e479a9aa2a92920379a4b32f95143b`, tree `d7224da9885fd1b55406c3f64d48f2c239508df8`; reviewed material candidate `ca8d69b60fec79f0a3525439ced5bf110833af9e`.
- `PROVEN`: current fresh verifier returns narrow `FreshAdmissionFacts`, while `Fnd04EvidenceAuthority` returns key bytes/generation without the required source provenance. Both repair surfaces are owned here.
- `PROVEN`: existing exported `AuthenticatedTransportRefV1`, `RuntimeScopeRefV1`, `AuthorityEvidenceFenceV1`, `CharacterLease` and `GameSessionAuthoritySnapshot::from_current_facts` can be consumed without editing their source.
- `DERIVED`: all seven Child A surfaces suffice; no additional path is currently necessary.
- `UNKNOWN`: actual production owning-source registration/readiness. Child C must establish it and keep unsupported sources closed; a test fixture cannot prove it.
- Ownership reconciliation: #208 and #280 are closed; #281 is closed and releases its allocation despite stale task/index prose. Existing #240/#243 concerns Durability, not Child A. Refresh this overlap classification before dispatch and integration.
- Server Seam `agent/otv2-gameplay-server-seam-01@9370b254c6ac4f6529e069c1968ae6bfa1e1750e` remains preserved. A+B+C protected integration and C readiness proof precede Work's resume decision.

## Execution and concurrency

One mutating Foundation worker owns one branch and one isolated writable worktree. Functional steps are serial because verifier, capability, flow and projection APIs share these exact surfaces. Independent read-only analysis/review may run when useful without a branch/path lease. Child B and C are not allocated by this task. Do not turn the 60-minute foreground budget into a completion claim or automatic reset; use the applicable bounded continuation policy and one concrete next action.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - replay_key_and_exact_candidate_binding
  - account_character_before_character_world
  - account_global_incumbent_exclusion
  - expected_and_acquired_character_lease_generation
  - runtime_scope_ownership_and_readiness
  - independent_protocol_transport_route_runtime_gameplay_revisions
  - authenticated_security_trust_source_age_order_and_decision
  - accepted_credential_time_and_authorization_deadline
  - initial_connection_generation_one
  - current_session_lifecycle_and_controller
consumer_boundaries:
  - verified_fresh_result_and_final_authorization
  - owning_source_publication_submission_and_receipt_activation
  - direct_fresh_completion
  - fresh_reconciliation_and_current_adoption
mutation_operators:
  applicable:
    - missing_current_fact_or_source
    - stale_generation_or_revision
    - mismatched_identity_or_binding
    - expired_future_non_monotonic_or_uncertain_time
    - provenance_substitution
    - exact_same_key_replay
    - changed_binding_same_key_replay
    - unavailable_or_ambiguous_submission_completion
    - wrong_duplicate_or_out_of_order_completion
    - independently_changed_authority_after_commit
    - restart_without_nonrollback_floor
    - conflicting_bootstrap_stale_CAS_or_equal_revision_contradiction
    - candidate_or_transport_collision_classification
  considered_not_applicable:
    - PostgreSQL physical locking_atomicity_WAL_and_migration_qualification belongs to Child B
    - production_source_connectivity_and_mutation_entrypoint_inventory belongs to Child C
one_invariant_per_negative_case: true
independent_current_fact_sources:
  - separately controlled test owning-source state built before expected authorization or receipt
record_derived_matching_helper:
  allowed_for_positive_happy_path: test_only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required_before_freeze
  protocol_versions: existing_reconnect_V1_V2_preserved
  direct_and_reconciled_paths: required
  fenced_durable_writes: semantic_port_only_SQL_not_allocated
  restart_retry_replay_concurrency_pg_reload: semantic_cases_here_existing_PG_regressions_in_CI_new_PG_cases_Child_B
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [ ] Verified fresh result preserves AccountId and every final evidence fence without changing authentication/classification precedence.
- [ ] Private owning-source capability/publication API prevents grant, caller fact struct or old receipt from seeding current guard truth; no default-forged evidence.
- [ ] Typed bootstrap/CAS/source-order/publication completion semantics keep readiness closed before acknowledged or reconciled publication; exact replay cannot re-age source time.
- [ ] Fresh bounded submit/yield/completion/reconciliation flow distinguishes committed, existing committed, replay conflict, incumbent rejection, stale authority and ambiguous/unavailable.
- [ ] Same-key retry preserves original immutable binding and decision evidence; ambiguity only reconciles the original candidate/transport and cannot mint another session.
- [ ] Strict NumericDate and conservative source-age semantics use checked math; final authority is decided at L, not BEGIN or COMMIT acknowledgment.
- [ ] Committed or historical receipt alone cannot install a controller; stale or replaced current facts, reconnectable/terminal lifecycle and higher connection generation fail closed.
- [ ] Synchronous compatibility remains explicitly non-production and cannot be the production SQLx path.
- [ ] Focused RED -> GREEN, affected regressions, full-diff/finding-family review, genuinely independent exact-head review and canonical CI pass before Work integration.

## Excluded scope

No SQLx/SQL/schema/migration, Cargo/lockfile, workflow/registry/stable-ID, listener/composition, reconnect redesign, public protocol or external-repository mutation. No `admission_recovery_inner.rs` edit. No production source fixture, deployment, secret/live-data or production readiness claim. An unowned-path requirement must be reported to Work for exact amendment before editing.

## Implementation / findings

Historical compile-time RED: `e88c106a41e130f90cd9d6c41b8a8ab237ade18e`, canonical run `33985311543`, Linux job `101357665714`, six expected E0433/E0425 missing-API errors; PR321 comment5554057753 retains exact evidence. The separate rustfmt findings were not RED.

Initial unavailable-entry GREEN: `4e0ce78479efa7fb17dc541695297704b7564f27`, run `33985663094`, Linux job `101358627615` completed SUCCESS, including strict Clippy, workspace tests and existing PostgreSQL regression. Policy/formatting passed. This proves only the initial unavailable-source boundary, not full Child A.

The next behavioral RED adds sealed owning-source traits and raw observation payloads without a successful verification implementation. A real signed grant plus independently published current source must succeed; the existing closed entry still rejects it. Source traits cannot be implemented outside the Game crate; raw payloads, grants or receipts cannot register a capability. Child B may follow the existing test-target path-inclusion convention to exercise crate-owned sources without a public fixture constructor or Cargo/workflow change. No production producer registration is claimed.

Complete verified evidence, publication, durable flow/adoption and qualification remain in progress. Preserve assertions and independently controlled sources; use exact PR evidence for frozen heads.

## Validation

### Focused

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server foundation::` and `cargo +1.94.0 test --locked -p oteryn-game-server --doc`
- result: NOT_RUN; future worker evidence required

### Component/integration

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test authority_invariants`; existing verifier/reconnect regression suite; formatting and strict all-target Clippy
- result: NOT_RUN; canonical PR CI also runs selected locked workspace/PG checks

### E2E

- scenario: new production fresh PostgreSQL/listener journey is NOT_APPLICABLE to Child A semantic-only implementation; existing canonical PostgreSQL regressions remain required where selected
- result: no new physical E2E or production readiness claimed

### Exact-head CI

- final head: null; recorded on allocated PR after publication
- trigger source: canonical allocated pull_request lifecycle
- workflow/run/job: pending worker PR
- runner assignment: unknown
- classification: repository-selected server paths plus applicable governance
- result: NOT_RUN

## Self-review

- exact head: pending worker material head
- method/reviewer: implementing worker whole-diff/adversarial/finding-family review
- material findings: not evaluated
- verdict: NOT_EVALUATED

## Independent review

- required: YES; admission/security/current-authority and recovery boundary
- exact head: pending worker material head
- method/auditor: genuinely independent non-author under current root review policy
- material findings: not evaluated
- verdict: NOT_EVALUATED

## PR and closeout

- changed-file review: exact eight-path worker allowlist above
- unresolved review threads: must be zero before integration
- related/superseded PRs: architecture #317 integrated; no replacement or reopening
- protected auto-merge: Work control plane only through normal protection/Merge Queue
- merge commit/result: pending worker integration
- ownership release: Work verifies protected readback and performs bounded archive/release
- future dependency: B only after A integration; C only after A+B; Server Seam resume only after A+B+C and producer readiness

## Context checkpoint

```yaml
last_progress: initial unavailable-entry canonical GREEN verified; sealed-source positive behavioral RED prepared on the same branch/PR
status: waiting
branch: agent/foundation-fresh-admission-318
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: [33985311543, 33985663094]
ci_job_ids: [101357665714, 101358627615]
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
next_action: observe the independently published source positive control fail in canonical CI before implementing successful verification
```

Source readiness prerequisite is tracked separately in Issue #319 for Child C. It does not block Child A semantic implementation or widen this allocation.
