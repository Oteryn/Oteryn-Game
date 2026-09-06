# OTV2-20260906-post-grace-foundation-successor-338

Work admitted this allocation at protected PR340 merge `4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee` (comment5558312039). Resumed under Work comment5558970336, same branch and immutable admission; previous unpublished local work receives no completion credit.

```yaml
task_id: OTV2-20260906-post-grace-foundation-successor-338
title: Add sealed post-grace recovery timing and authority successor
mode: IMPLEMENT
status: running
admission_state: ADMITTED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/post-grace-foundation-successor-338
issue: 338
pr: null
allocation_source_main_sha: 1bcdc951e90a56310d24dfb5f3953ec0f86e1695
admission_main_sha: 4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee
base_sha: 4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated post-grace Foundation worker
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-06
updated_at: 2026-09-06
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/foundation/fnd04_verifier.rs
  - apps/game-server/src/foundation/admission_authority_publication.rs
  - apps/game-server/src/foundation/post_grace_recovery_tests.rs
  - docs/agents/tasks/active/OTV2-20260906-post-grace-foundation-successor-338.md
  - docs/superpowers/plans/2026-09-06-post-grace-foundation-successor.md
public_contracts: [FND-DUR-POST-GRACE-TIMING-V1]
depends_on: [332, 334]
blocks: [post_grace_Durability_successor, complete_Server_Seam_recovery_qualification]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Additive closed recovery timing/authorization/flow enables truthful semantic post-grace recovery of the same uncontrolled actor into a distinct generation-one GameSession. Preserve immutable historical grace/epoch, frozen verified deadline, current authority at each boundary and retained budget/protection. B329 and existing V1/V2 remain independently useful and unchanged.

## Architecture and source of truth

- Work verified #334 accepted at protected `1bcdc951e90a56310d24dfb5f3953ec0f86e1695`, Merge Queue `34022052840` PASS, #332 closed completed 2026-09-06T08:39:48Z. This is accepted architecture, not this child's admission.
- Governing `docs/architecture/reviews/OTERYN_GAME_POST_GRACE_RECOVERY_TIMING_DECISION_2026-09-06.md` preserves its parent recovery/reconnect/terminal replacement contracts and #326 sealed claim ownership.
- Plan: `docs/superpowers/plans/2026-09-06-post-grace-foundation-successor.md`. Issue #338 governs this exact prospective allocation; Work binds the actual protected allocation merge before dispatch.
- GitHub #162 is sole mutating control plane. Work checks live main/overlap, creates issue/allocation/branch/PR and records actual allocation merge as immutable admission. No worker authority follows before this exact allocation is protected and read back.
- One writer owns the four semantic/test surfaces plus exactly allocated task/plan. No concurrent writes with another owner; B keeps its separate SQL/harness custody. Additive APIs minimize compile coupling but do not eliminate final merge-up validation.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [sealed_verified_reauth, current_actor_presence_placement, terminal_predecessor, exact_account_character_world, current_lease_runtime_revisions, generation_one_new_session, immutable_timing_deadline, source_freshness_antirollback, retained_epoch_budget_protection, exact_claim_operation, current_adoption]
consumer_boundaries: [verified_evidence, owning_current_resolution, preparation, final_authorization, claim_effect_validation, completion, historical_restore, reconcile, adoption]
mutation_operators:
  applicable: [missing_source, forged_history_flag, reconnect_proof_substitution, wrong_scope_key_profile, stale_provenance, denied_source, active_predecessor, controller_present, absent_changed_actor_placement, wrong_binding_generation, expired_future_nonmonotonic_time, overflow, changed_variant_deadline_replay, budget_reset_exhaustion, missing_continuity, protection_reset, stale_claim_CAS, postcommit_supersession]
  considered_not_applicable: [actual_SQL_atomicity_requires_later_Durability_child, production_source_readiness_is_separate, physical_listener_journey_requires_Server_Seam_allocation]
one_invariant_per_negative_case: required
independent_current_fact_sources: [sealed_authenticated_recovery_fixture, sealed_current_actor_continuity_fixture, independently_current_claim_session_transport]
record_derived_matching_helper:
  allowed_for_positive_happy_path: historical_identity_classification_only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: same_session_legacy_early_terminal_post_grace_claims
  protocol_versions: preserve_V1_V2_add_closed_successor
  direct_and_reconciled_paths: required
  fenced_durable_writes: semantic_predicates_only_later_SQL_required
  restart_retry_replay_concurrency_pg_reload: semantic_history_now_actual_PG_later
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: [fixed_exact_source_replay_local_publication_advance]
```

## Acceptance criteria

- [ ] Private live successor requires newly verified reauthenticated recovery with correct source scope/provenance/deadlines and independently owning current actor/continuity source. Raw DTO/history/caller flag/reconnect proof cannot select it.
- [ ] Explicit closed timing discriminant preserves SameSession/V1/V2 behavior. Missing/unknown/mixed variant rejects; old failures/history cannot upgrade into live post-grace privilege.
- [ ] At preparation and final authority now is strictly after exact historical grace and within frozen finite deadline derived from accepted credential/security/trust bounds with checked arithmetic. Refresh cannot extend the same attempt; equality/early-terminal legacy behavior remains unchanged.
- [ ] Candidate session differs from terminal predecessor and starts connection generation exactly1; predecessor generation is separate. Same uncontrolled actor/placement/state, account/world, lease/scope/revisions and absent controller remain current prerequisites.
- [ ] Retained loss epoch/grace and complete actor-bound attempt budget/dispositions span replacement. Eight/nine and exact retries qualified; missing compacted state fails closed, no new empty budget inferred on restart.
- [ ] Protection entitlement/consumption/activation/rearm evidence is retained; no mint/reset/rearm/new epoch/respawn/heal/relocation. Existing eligible once-only activation and retirement/restoration finality preserved.
- [ ] Additive PREPARE/final/completion/reconcile/adoption surfaces use exact immutable operation and independently current facts. Later source deny/fence change rejects; historical committed outcome cannot reinstall stale controller.
- [ ] Owner-sealed successor claim effects bind exact successor operation and current predecessor/claims; standalone publication cannot apply them. Existing B-consumed claim APIs and enum variants remain compatible.
- [ ] Complete single-invariant matrix, compile-fail anti-forgery, direct/reconciled positive post-grace and existing V1/V2/early-terminal regressions pass. Source-included B integration target compile is proven on the integrated source before compatible handoff.
- [ ] Focused RED/GREEN, package/doctest/fmt/strict Clippy/architecture/governance, whole-diff self-review and independent exact-head review pass; findings dispositioned.
- [ ] Canonical selected CI and Merge Queue pass; Work verifies protected readback/archive/releases custody. Semantic completion makes no actual PostgreSQL or source/listener readiness claim.

## Excluded scope

No SQL/migrations or future migration number, B329 worktree/harness edits, Cargo/workflow, facade/export/lib edits, actual provider registration/bootstrap/production, external repository, transport/listener or source-readiness claims. No weakening V1 constraints, downcast, new resource/timing number or general recovery-policy redesign. Additional concrete paths require Work amendment before mutation.

## Implementation / findings

Checkpoint 1: additive sealed recovery-scoped source/verifier and signed negative fixtures implemented. Existing V1 connection fence requires predecessor+1; continuity caps prepared deadline by original grace. Existing verified recovery facts lack authenticated source deadlines/provenance and current evidence lacks present-uncontrolled placement/retained-continuity proof. Existing replacement claim capability embeds V1 record. The four proposed paths address these concrete additive gaps without borrowing B paths.

## Validation

### Focused

Actual local RED: `recovery-red.log`, missing recovery deadline API E0425 (five errors). GREEN: `cargo +1.94.0 test --locked -p oteryn-game-server --lib post_grace`, 5 passed after review repair. Independent signed fixture covers exact recovery scope/deadline, unavailable source, new denial, same-revision contradictory decision and backwards time. Scope metadata is validated as ExistingActorRecovery, never rewritten from FreshAdmission. This credential capability alone grants no actor or flow authority. Strict all-target Clippy passed after replacing test-only expect calls with propagated errors; fmt passed. Full package/doctest and final review are still pending.

Run focused post-grace RED/GREEN and independent invariant matrix, retaining valid controls. Include equality, strict-after-grace, generation1, exact frozen deadline, actor disappearance/controller return, attempts8/9, consumed entitlement and historical forgery cases.

Review P2 fixed: exact immutable source replay must permit a newer local publication wrapper. Actual `recovery-wrapper-red.log` failed with RecoverySecurityEvidenceStale; repair normalizes only publication_revision in equality. Five focused tests now pass, retaining wrapper rollback and source content/decision/time/uncertainty/authority contradiction negatives. Source provenance is never re-aged.

### Component/integration

Rust1.94.0 locked game-server tests/doctests, fmt, strict all-target Clippy, architecture/governance and existing recovery regressions. Compile configured/source-included `--test durability_postgres --no-run` against actual integrated B where available; record concrete remaining integration dependency otherwise, not an invented passing claim. Preserve full canonical gate.

### E2E

Actual PostgreSQL post-grace and real Server Seam journey are NOT_PROVEN_BY_THIS_SEMANTIC_CHILD and mandatory later. Fake normalized durability ports establish semantic flows only. Do not bypass local PG restrictions or modify B/workflows to obtain test authority.

### Exact-head CI

Native remote exact head is canonical; affected checks/review rerun after material repairs/merge-up. Require full selected CI and Merge Queue; package compilation alone is insufficient.

## Self-review and independent review

NOT_STARTED. Whole changed-content adversarial review and genuinely independent exact-head high-risk review required. Accepted findings repair test-first and sweep siblings; rejected findings retain exact evidence. Review never grants merge authority.

## PR and closeout

Work owns PR creation and binds protected admission; worker publishes only allocated paths. One 60-minute window, bounded checkpoint and one next action; Work grants continuation. Rotation preserves failure/repair/CI counters, branch history and frozen candidate. Work owns final integration/archive/release, later SQL allocation and actual source/Server Seam readiness.

## Context checkpoint

```yaml
last_progress: sealed recovery evidence verifier first material checkpoint; actor continuity and flow remain open
status: running
admission_state: ADMITTED
branch: agent/post-grace-foundation-successor-338
head_sha: null
pr: null
execution_budget_minutes: 60
execution_window_number: 2
execution_window_started_at: 2026-09-06T11:43:00Z
execution_window_elapsed_minutes: 0
execution_windows_completed: 1
worker_rotations: 1
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
prior_unpublished_repair_history: UNKNOWN
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Continue the sealed owning actor and retained continuity prerequisites, then exact immutable post-grace flow and claim successor on this branch.
```
