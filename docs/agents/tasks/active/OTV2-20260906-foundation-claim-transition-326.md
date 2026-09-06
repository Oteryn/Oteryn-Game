# OTV2-20260906-foundation-claim-transition-326

```yaml
task_id: OTV2-20260906-foundation-claim-transition-326
title: Pair atomic claim transitions with admission and lifecycle effects
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/foundation-claim-transition-326
issue: 326
pr: null
allocation_source_main_sha: 93f31ba05972d3b96afb0d9ea08e2c6753507d8c
admission_main_sha: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated Foundation claim-transition worker
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-06
updated_at: 2026-09-06
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/src/foundation/admission_authority_publication.rs
  - apps/game-server/src/foundation/fresh_admission_durability.rs
  - apps/game-server/src/foundation/fresh_admission_durability_tests.rs
  - docs/agents/tasks/active/OTV2-20260906-foundation-claim-transition-326.md
public_contracts: [FND-DUR-FRESH-CLAIM-PUBLICATION-V1]
depends_on: [324, 325]
blocks: [Child_B_allocation, 247]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Durability can consume exact owner-authored conditional claim effects with a matching canonical session operation, without inventing source provenance. Fresh completion/reconciliation preserves the complete immutable operation; independently current adoption remains required.

## Architecture and source of truth

- PROVEN: #325 accepted the bounded decision at protected merge `93f31ba05972d3b96afb0d9ea08e2c6753507d8c`; original A #318/#321 and its closeout #323 remain complete/released.
- Governing decision: `docs/architecture/reviews/OTERYN_GAME_ATOMIC_FRESH_CLAIM_PUBLICATION_DECISION_2026-09-06.md` and its unchanged parent FND-DUR-FRESH-ADMISSION-V1 (#313/#317).
- Plan: `docs/superpowers/plans/2026-09-06-foundation-claim-transition.md`.
- This prospective lease activates only after this allocation's protected integration and Work readback. Work creates the canonical worker branch from that actual allocation merge, which becomes immutable `admission_main_sha` and `base_sha`; the worker records that verified SHA in its first material implementation checkpoint. The source SHA above is not worker admission.
- One worker owns this exact four-path allocation and writable worktree. Steps share the sealed API and are serial; independent analysis/review is read-only. Work owns PR creation/integration and shared programme documents. No other writer may edit these paths concurrently.
- Open PR #243 is historical read-only evidence under completed #250/#252/#290, not a competing live lease. Preserved Server Seam #247 has no ownership of these three runtime paths.
- Actual source/bootstrap/readiness remains UNKNOWN under #319; this task establishes semantic capability correctness only.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [owner_seal, exact_operation_binding, current_predecessor, current_session, source_and_CAS_monotonicity, immutable_remote_provenance, atomic_claim_effects, current_adoption]
consumer_boundaries: [owner_preparation, request_pairing, final_L, historical_restore, retry_reconcile, adoption, claim_preserving_lifecycle, terminal_replacement, terminal_release, standalone_publication]
mutation_operators:
  applicable: [missing_or_extra_guard, wrong_owner_key_purpose, stale_source_CAS, equal_revision_changed_decision, wrong_candidate_replay_transport, wrong_holder_lease_generation, expired_future_time, source_provenance_substitution, overflow, historical_capability_forgery, changed_transition_replay, stale_release]
  considered_not_applicable: [SQL_atomicity_and_restart_enforcement_belong_to_B, production_source_connectivity_belongs_to_C]
one_invariant_per_negative_case: required
independent_current_fact_sources: [sealed_fixture_owner, independent_locked_guards, independent_current_session_and_transport]
record_derived_matching_helper:
  allowed_for_positive_happy_path: historical_identity_classification_only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required_fresh_publication_replacement_release
  protocol_versions: preserve_reconnect_V1_V2
  direct_and_reconciled_paths: required
  fenced_durable_writes: pure_semantic_predicates_here_SQL_in_B
  restart_retry_replay_concurrency_pg_reload: semantic_history_here_real_PG_in_B
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [ ] Registered owner-sealed fresh transition prepares exactly Account/Character successors from independent accepted predecessors, bound to full authorization/replay/session/transport and strict source/CAS advancement.
- [ ] Paired production request and final-L predicate require both live capabilities; no raw DTO/receipt path creates one. No pre-COMMIT activation or source high-water consumption.
- [ ] Lossless historical operation envelope, receipt, retry/completion and reconciliation bind exact transition decisions/effects; restoration is historical-only. No original-binding replacement on ambiguity.
- [ ] Ordinary publication cannot acquire/release/change claims independently of matching session effects; nested Platform provenance remains unchanged except its permitted local publication wrapper.
- [ ] Explicit narrow lifecycle siblings cover claim-preserving reconnect/control loss, already accepted terminal replacement, and fenced terminal release. They consume existing public authorization/current-session getters, expose inert exact effects and historical binding, and never authorize arbitrary transfer/takeover policy.
- [ ] Wrong owner/binding/predecessor/current session/lease/source/CAS/time/provenance cases fail closed with one invariant mutated per case. Source decision reuse across transactions is explicitly B's durable obligation, not a stateless-proof claim.
- [ ] Existing equal-revision, verifier/reconnect, ambiguity/restart and independent adoption regressions remain green; actual fixture acquisition applies the submitted owner-prepared transition rather than handwritten post-hoc increments.
- [ ] Focused RED/GREEN, component/doctests, fmt/strict Clippy, whole-diff adversarial self-review and required independent exact-head review pass; all material findings dispositioned.
- [ ] Canonical exact-head CI and normal Merge Queue pass; Work verifies protected integration, archives task and releases custody.

## Excluded scope

No SQL/migrations, Cargo/lockfile, workflow/protection, listener, facade/verifier/export edits, arbitrary lifecycle policy, production/bootstrap/key/live-data changes, external repositories, B/C allocation or source-readiness claims. No reopening #318 or modification of its archive. A concrete additional path need is reported before mutation; only Work can amend the exact allocation.

## Implementation / findings

Work's independent pre-allocation audit found existing public APIs sufficient for the planned sibling helpers inside the publication file; no extra Foundation file is currently justified. All flow-begin/receipt restoration callers requiring adjustment are in the allocated split test file. This is an implementation plan, not a claim of passing behavior.

Prepare known task metadata before final freeze. Review/check evidence after freeze belongs on the exact PR head; no timestamp/no-op commits or counter resets. Worker rotation is nonterminal and preserves the same branch/history and one next action.

## Validation

### Focused

Run Rust1.94.0 locked fresh_admission and admission_authority_publication tests for each RED/GREEN step; record actual expected failure and corrected result.

### Component/integration

Run `cargo +1.94.0 test --locked -p oteryn-game-server`, doctests, `cargo +1.94.0 fmt --all --check`, and package all-target strict Clippy. Governance validator covers task metadata. Required hosted checks may select wider workspace lanes.

### E2E

NOT_APPLICABLE to this semantic-only child: no SQL adapter, transport or physical journey is added. Existing configured PostgreSQL regressions still run in required CI; new atomic SQL/source-readiness evidence remains B/C's obligation.

### Exact-head CI

Final head is recorded after publication in immutable PR/check evidence. Require current selected Merge gate and full Merge Queue game-gate; no earlier-head substitution.

## Self-review

Required whole-diff adversarial review by the writer, with invariant/caller/finding-family sweep; result pending implementation.

## Independent review

Required YES: high-risk admission/provenance/session lifecycle semantics. Separate non-author exact-head reviewer; result pending stable candidate. External review remains advisory; protected controls own integration.

## PR and closeout

Worker returns one exact-head candidate within the four paths. Work creates/updates its sole implementation PR, integrates through protected controls, then archives this task and releases custody. Allocation PR is not the implementation PR. No self-merge or shared-document ownership is granted.

## Context checkpoint

```yaml
last_progress: prospective bounded followup allocation prepared after accepted claim-publication decision
status: waiting
branch: agent/foundation-claim-transition-326
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
blocker: allocation_not_yet_protected_integrated
next_action: Work qualifies and integrates this exact allocation, binds its actual merge SHA, then dispatches the sole worker.
```
