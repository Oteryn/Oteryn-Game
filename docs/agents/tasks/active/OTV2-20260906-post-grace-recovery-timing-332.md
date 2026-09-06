# OTV2-20260906-post-grace-recovery-timing-332

```yaml
task_id: OTV2-20260906-post-grace-recovery-timing-332
title: Separate post-grace attempt timing from predecessor continuity
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/post-grace-recovery-timing-332
issue: 332
pr: null
base_sha: 5412215718d66c743fb78eadc561e6a23b5e2b5f
admission_main_sha: 5412215718d66c743fb78eadc561e6a23b5e2b5f
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
  - docs/architecture/reviews/OTERYN_GAME_POST_GRACE_RECOVERY_TIMING_DECISION_2026-09-06.md
  - docs/agents/tasks/active/OTV2-20260906-post-grace-recovery-timing-332.md
public_contracts: [FND-DUR-POST-GRACE-TIMING-V1]
depends_on: [accepted_FND04B, accepted_DUR_TERMINAL_SESSION_REPLACEMENT_V1]
blocks: [full_Server_Seam_post_grace_recovery_qualification]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A bounded candidate distinguishes historical predecessor grace from the current newly authorized post-grace attempt. It preserves existing same-session expiry, terminality, actor/epoch/protection/replay and current authority. No implementation or acceptance is claimed. Work alone publishes, routes independent review and integrates.

## Architecture and source of truth

Dispatch: Issue #332 comment `5557757008`; live issue and branch/base independently verified. Root/docs instructions, supervising architect prompt and architecture decision discipline govern. Governing accepted sources:

- `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md` §21;
- `docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md` §§17–19;
- `docs/architecture/reviews/OTERYN_GAME_DURABILITY_RECONNECT_AUTHORITY_BOUNDARY_DECISION_2026-08-26.md` §§5–9;
- `docs/architecture/reviews/OTERYN_GAME_TERMINAL_SESSION_REPLACEMENT_COLLISION_RECONCILIATION_DECISION_2026-08-28.md`;
- accepted fresh/claim decisions #317/#325 and current Server Seam task/plan.

PROVEN: current continuity constructor caps prepared deadline by original grace; terminal replacement retains that grace; authorization and V2 PostgreSQL preparation apply the old cap. DERIVED: every newly initiated post-grace candidate is stale despite accepted policy requiring eligible replacement. UNKNOWN: future versioned API/schema spelling and actual consumer implementation. CONFLICT: no conflicting product policy; the inherited representation does not cover the accepted case. This is not a #326/#331 regression.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [timing_variant, historical_grace, fixed_attempt_deadline, terminal_predecessor, same_actor, current_fences, retained_epoch_budget, protection_continuity, exact_replay]
consumer_boundaries: [Foundation_authorization, durable_PREPARE, final_COMMIT, reconciliation, controller_adoption]
mutation_operators:
  applicable: [old_grace_substitution, same_session_after_grace, expired_attempt, changed_variant, proof_substitution, stale_fence, absent_actor, healthy_controller, reset_epoch, exhausted_budget, stale_protection, ambiguous_commit, version_downgrade]
  considered_not_applicable: [new_grace_value_no_value_selected, new_gameplay_policy_no_change]
one_invariant_per_negative_case: required_for_implementation
independent_current_fact_sources: [verified_recovery_evidence, owning_actor_presence_lease_scope, transactionally_current_database_facts]
record_derived_matching_helper:
  allowed_for_positive_happy_path: historical_replay_classification_only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: existing_V1_V2_and_successor_timing_dispatch
  protocol_versions: old_records_keep_original_semantics
  direct_and_reconciled_paths: new_deadline_binding_and_current_adoption_required
  fenced_durable_writes: PREPARE_and_final_COMMIT_qualified_separately
  restart_retry_replay_concurrency_pg_reload: explicit_decision_matrix
  evidence: [read_only_source_proof_and_candidate_self_review]
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] Prove inherited timing gap from current source and accepted post-grace policy.
- [x] Define closed timing successor and deadline derivation without new duration.
- [x] Preserve old records, epoch/budget/protection, exact replay and independent current authority.
- [x] Identify narrow supersession, forward compatibility costs and concrete qualification.
- [ ] Independent exact-head review, selected canonical CI and protected integration.

## Excluded scope

Exactly two documentation paths. No local commit, push, PR/lifecycle/global edit, runtime/schema/migration change, external write, source readiness, credentials, production or new resource values. Foundation #326 and B remain independent. No Server Seam resume or hidden acceptance exclusion.

## Implementation / findings

The new timing variant is constructible only from current verified terminal post-grace recovery authority. Its attempt deadline derives from accepted credential/evidence bounds; historical grace remains exact but does not expire the new candidate. The old timing representation is unchanged. Session replacement cannot reset the retained loss-epoch attempt/protection state.

Serial authoring is bounded to one two-file decision/task packet. Work checked overlapping PR custody before dispatch and retains exact implementation allocation/sequence; shared runtime paths are not allocated by this document. Final staged bytes are handed back without a divergent local commit.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`; staged `git diff --cached --check`
- result: PASS; governance validated 26 policy documents and 9 lanes; staged whitespace clean

### Component/integration

- NOT_APPLICABLE: documentation only; actual Foundation/PostgreSQL proof required under future allocations.

### E2E

- NOT_APPLICABLE: no runtime execution claim; later Server Seam acceptance must explicitly prove its post-grace boundary.

### Exact-head CI

- final head: pending Work publication
- trigger source: Work-created PR
- workflow/run/job: pending
- runner assignment: pending
- classification: architecture session/persistence timing
- result: pending

## Self-review

- exact head: final staged candidate bytes, remote SHA recorded after Work publication
- method/reviewer: author adversarial whole-candidate review against accepted policy and current Foundation/Durability
- material findings: old-session terminality must not reset actor loss-epoch budget; current and historical deadlines must be separate and variant-bound; addressed explicitly
- verdict: candidate ready for independent exact-head review; no unresolved author findings

## Independent review

- required: YES; session timing/authority/persistence
- exact head: pending publication
- method/auditor: Work-assigned independent reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly two allocated docs
- unresolved review threads: pending PR
- related/superseded PRs: preserves #326/#331 work; accepted contract interpretations narrowly superseded in decision §6
- protected auto-merge: no architect authority
- merge commit/result: pending Work qualification
- ownership release: author releases writable custody at handoff; Work owns lifecycle
- `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

## Context checkpoint

```yaml
last_progress: bounded post-grace timing candidate prepared
status: validating
branch: arch/post-grace-recovery-timing-332
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
blocker: independent_review_and_protected_integration_pending
next_action: Work publishes and independently qualifies the staged candidate for protected integration.
```
