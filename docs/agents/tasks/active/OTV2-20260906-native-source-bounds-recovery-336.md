# OTV2-20260906-native-source-bounds-recovery-336

```yaml
task_id: OTV2-20260906-native-source-bounds-recovery-336
title: Bound native source resources and explicit recovery evidence transport
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/native-source-bounds-recovery-336
issue: 336
pr: null
base_sha: b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd
admission_main_sha: b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn Sol Supervising Architect
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-06
updated_at: 2026-09-06
execution_budget_minutes: 30
large_budget_reason: null
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_NATIVE_SOURCE_RESOURCE_ENVELOPE_DECISION_2026-09-06.md
  - docs/architecture/reviews/OTERYN_GAME_RECOVERY_SOURCE_TRANSPORT_AMENDMENT_2026-09-06.md
  - docs/agents/tasks/active/OTV2-20260906-native-source-bounds-recovery-336.md
public_contracts: [NATIVE-SOURCE-RESOURCE-ENVELOPE-V1, FND-RECOVERY-SOURCE-TRANSPORT-V2]
depends_on: [accepted_330_source_contracts]
blocks: [native_source_adapter_acceptance, actual_recovery_source_registration]
cross_repository_coordination_id: OTV2-NATIVE-SOURCE-EVIDENCE
external_repositories: [Oteryn/Oteryn-Platform_read_only]
```

## Outcome

Two candidate decisions propose a finite first-slice resource envelope and explicit versioned recovery evidence transport. Numeric values are justified conservative policy choices, not measurements or production capacity claims. Game acceptance, registry registration, implementation and external counterpart readiness remain separate. Work alone publishes, independently qualifies and integrates.

## Architecture and source of truth

Dispatch: Issue #336 comment `5558030902`; live issue/branch/base verified. Root/docs agent instructions, supervising architect prompt and decision discipline govern. Accepted #330 native evidence/assignment decisions and FND-04 fresh/recovery profiles retain authority. Exact registry readback at the admission SHA proves that fixed identity/crypto encodings exist but complete private HTTP/TLS/JSON/assignment mappings do not.

PROVEN: recovery profile sections 12–13 require source provenance/order/freshness and sections 18–19 current revalidation. DERIVED: semantic typed recovery results follow existing authority, while #330's fresh-only service operations need an explicit compatible recovery extension. UNKNOWN: measured first-slice performance, counterpart certificate sizes, producer acceptance/implementation and actual bootstrap/connectivity. CONFLICT: none. #334 remains an independent candidate at this base, not an assumed accepted dependency.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [bounded_allocation, bounded_work, exact_scope_version, authenticated_source, shared_security_floor, distinct_recovery_trust, nonrollback_restart]
consumer_boundaries: [HTTP_TLS_ingress, decoding, enqueue, publication, assignment_transaction, recovery_verification, final_COMMIT, adoption]
mutation_operators:
  applicable: [max_plus_one, arithmetic_overflow, nested_unknown_JSON, chunk_overflow, excess_certificate_work, slow_peer, cancellation_slot_leak, hidden_retry, scope_substitution, version_downgrade, cross_path_security_rollback, key_namespace_switch, stale_source_time, missing_floor]
  considered_not_applicable: [compression_explicitly_disabled, batching_enumeration_disabled, production_provisioning_unauthorized]
one_invariant_per_negative_case: required_for_implementation
independent_current_fact_sources: [authenticated_owner_observations, persistent_security_and_trust_floors, owning_assignment_authority]
record_derived_matching_helper:
  allowed_for_positive_happy_path: historical_replay_only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: fresh_and_recovery_service_operations_share_total_resource_budget
  protocol_versions: explicit_V1_V2_isolation_and_shared_security_floor
  direct_and_reconciled_paths: bounded_reconcile_no_new_operation_after_ambiguity
  fenced_durable_writes: existing_atomic_owner_rules_preserved
  restart_retry_replay_concurrency_pg_reload: explicit_qualification_matrices
  evidence: [exact_registry_profile_readback_and_whole_candidate_review]
finding_dispositions:
  p0_p1_accepted_and_repaired: [P1_end_to_end_publication_completion_reconcile_resource_escape_fixed]
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: [P2_shared_security_mutual_invalidation_explicitly_accepted_and_clarified]
```

## Acceptance criteria

- [x] Map only identical existing field bounds and propose remaining finite per-boundary policy values with rationale/uncertainty.
- [x] Specify aggregate accounting, early rejection, cancellation, timeout/ambiguity and durable-floor preservation.
- [x] Exclude unnecessary compression/batching/enumeration/automatic retry rather than invent speculative capacity.
- [x] Add explicit recovery V2 operations and preserve source namespaces/crypto/freshness and external acceptance dependency.
- [ ] Independent exact-head review, selected CI and protected integration.

## Excluded scope

Exactly three documentation paths. No registry/JSON/runtime/dependency/schema/external/PKI/provisioning change; no local commit, push, PR/lifecycle/global edit. No production source or capacity claim. B #329/#335 and #334 remain independently qualified; no implementation paths allocated here.

## Implementation / findings

The resource candidate chooses small first-slice native-specific bounds rather than reusing unrelated game frames/session queues. Two evidence exchanges and a bounded queue share one process budget across versions. TLS/HTTP parser and verification work must be demonstrably bounded by the future adapter; defaults are insufficient. Persistent source floors/receipts are not cache entries or subject to volatile eviction. Assignment commands remain single-scope, bounded and independently authorized.

The recovery amendment adds explicit V2 account-security and signing-trust operations. It shares one account-security ordering floor across fresh/recovery but preserves distinct signing trust namespaces and typed purpose-bound observations. Fresh responses and old DTOs cannot create recovery capabilities. Numeric registry integration and Platform counterpart acceptance remain later exact allocations.

Serial authoring is one bounded three-doc packet with shared task custody; independent B/#334 work continues. Final reviewed staged bytes return to Work without a local commit.

## Independent finding repair — first cycle

Work accepted P1: HTTP-only slots left SQL/publication/completion/ambiguity retention unbounded. The repair retains two end-to-end slots, finite pending checkpoint/completion bytes, exact operation identity, durable restart slot recovery and no detached operations; the assignment sibling also retains its sole slot through ambiguity. Fast HTTP, stalled SQL/completion, cancellation and restart cannot multiply outstanding capacity.

P2 disposition: **accepted availability cost and clarified compatibility**, not a weakened revision predicate. A newer shared account-security observation intentionally invalidates a lower revision of either purpose. Only the independently selected eligible Game path requests its typed account-security evidence; there is no automatic two-purpose freshness chase or simultaneous-success promise. Stable selected-path success and both cross-purpose invalidation directions are mandatory qualification. Exact scope seals and newer denial/generation floors remain binding. Full candidate independent re-review is required after this material repair; the budget is not reset.

The immutable admission still records #334 as a candidate at that base. Work subsequently reported its protected integration separately; this packet neither rewrites admission provenance nor depends implicitly on that unrelated delivery.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`; staged `git diff --cached --check`
- result: PASS; governance validated 26 policy documents and 9 lanes; staged whitespace clean

### Component/integration

- NOT_APPLICABLE: documentation only; actual transport/resource/assignment/recovery proof required under future allocations.

### E2E

- NOT_APPLICABLE: no runtime execution claim; later source acceptance must prove actual adapter and producer boundaries.

### Exact-head CI

- final head: pending Work publication
- trigger source: Work-created PR
- workflow/run/job: pending
- runner assignment: pending
- classification: architecture resource/authenticated source compatibility
- result: pending

## Self-review

- exact head: final staged candidate bytes, remote SHA recorded after Work publication
- method/reviewer: author adversarial whole-candidate review against registry, accepted profiles and #330
- material findings: end-to-end pending-slot/completion ownership including restart, assignment sibling sweep, explicit mutual invalidation availability and selected-path qualification; addressed explicitly
- verdict: candidate ready for independent exact-head review; no unresolved author findings

## Independent review

- required: YES; resource exhaustion/authentication/compatibility
- exact head: pending publication
- method/auditor: Work-assigned independent reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly three allocated docs
- unresolved review threads: pending PR
- related/superseded PRs: preserves #330 history, B and #334; only named resource deferral/operation allowlist amended
- protected auto-merge: no architect authority
- merge commit/result: pending Work qualification
- ownership release: author releases writable custody at handoff; Work owns lifecycle
- `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

## Context checkpoint

```yaml
last_progress: independent P1 resource escape repaired and P2 currentness availability explicitly dispositioned
status: validating
branch: arch/native-source-bounds-recovery-336
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
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: independent_review_and_protected_integration_pending
next_action: Work publishes and independently qualifies the staged candidate for protected integration.
```
