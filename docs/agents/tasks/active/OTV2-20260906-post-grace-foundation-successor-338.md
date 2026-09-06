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
pr: 343
allocation_source_main_sha: 1bcdc951e90a56310d24dfb5f3953ec0f86e1695
admission_main_sha: 4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee
base_sha: 4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee
head_sha: 8a72a305330011edb9021abc1f8449d65a377909
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

Checkpoint 2: sealed current actor source and private post-grace prepare/current revalidation capability implemented alongside explicit complete retained budget, epoch finality, protection/rearm history and closed timing. Credential signed attempt UUID is retained independently of the Game attempt; malformed zero source security generation rejects. Checkpoint 3 adds immutable original admission operation and lossless credential audit separate from refreshed live facts, with historical version/timing consistency validation. Split completion/reconcile/adoption flow and successor claim capability remain required. The core admission operation still needs the exact owner-authored claim transition bound by the later flow layer. Existing V1 connection fence requires predecessor+1; continuity caps prepared deadline by original grace. Existing verified recovery facts lack authenticated source deadlines/provenance and current evidence lacks present-uncontrolled placement/retained-continuity proof. Existing replacement claim capability embeds V1 record. The four proposed paths address these concrete additive gaps without borrowing B paths.

## Validation

### Focused

Actual local RED: `recovery-red.log`, missing recovery deadline API E0425 (five errors). GREEN: `cargo +1.94.0 test --locked -p oteryn-game-server --lib post_grace`, 5 passed after review repair. Independent signed fixture covers exact recovery scope/deadline, unavailable source, new denial, same-revision contradictory decision and backwards time. Scope metadata is validated as ExistingActorRecovery, never rewritten from FreshAdmission. This credential capability alone grants no actor or flow authority. Strict all-target Clippy passed after replacing test-only expect calls with propagated errors; fmt passed. Full package/doctest and final review are still pending.

Run focused post-grace RED/GREEN and independent invariant matrix, retaining valid controls. Include equality, strict-after-grace, generation1, exact frozen deadline, actor disappearance/controller return, attempts8/9, consumed entitlement and historical forgery cases.

Review P2 fixed: exact immutable source replay must permit a newer local publication wrapper. Actual `recovery-wrapper-red.log` failed with RecoverySecurityEvidenceStale; repair normalizes only publication_revision in equality. Five focused tests now pass, retaining wrapper rollback and source content/decision/time/uncertainty/authority contradiction negatives. Source provenance is never re-aged.

Checkpoint 2 actual RED/GREEN: missing signed-attempt API; zero security generation assertion; missing retained-budget/timing/current-actor APIs; retained own-attempt deletion assertion. Final focused post_grace family: 14 passed; strict all-target Clippy passed (7.91s). New tests use independent actor/source fixtures, and cover absent/changed placement/presence/protection/revisions/security floor, final epoch closure, 8/9 budget, existing retry, same-revision actor contradiction and frozen deadline despite refreshed evidence. The retained own-attempt deletion finding was repaired by requiring every pre-existing own entry remain exact; peer entries already compare completely. Full local package passed: library304, all selected integration binaries including unconfigured PostgreSQL target124, and16 doctests including actor/source anti-forgery. PostgreSQL target cases skip SQL when unconfigured: this is compile/compatibility evidence, not SQL execution proof.

Checkpoint 3 (window2 before12:43): actual signed generation2/source floor2 to newer floor1 RED repaired with comparative minimum-generation monotonicity. Actual missing FND02/readiness and immutable operation/history APIs REDs followed by 18 focused tests passing. Current actor now supplies independent FND02 reconciliation and runtime readiness; refresh preserves the original complete admission operation while current evidence advances. Unknown stored version and changed derived deadline reject. Strict all-target Clippy passed7.50s and fmt/diff passed. Full package evidence above belongs to checkpoint2; no claim of fresh full-package or PostgreSQL execution for checkpoint3.

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
last_progress: sealed claim-time Recovery authorization and split flow/current adoption implemented; 30 focused and full package tests pass; independent review pending
status: running
admission_state: ADMITTED
branch: agent/post-grace-foundation-successor-338
head_sha: null
pr: 343
execution_budget_minutes: 60
execution_window_number: 3
execution_window_started_at: 2026-09-06T12:49:00Z
execution_window_elapsed_minutes: 32
execution_windows_completed: 2
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
repair_cycles_for_current_gate: 3
prior_unpublished_repair_history: UNKNOWN
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Work native-publishes this staged material checkpoint, obtains independent review and canonical CI, then returns custody for remaining qualification and any separately protected amendment; no SQL or server-ready claim.
```


## Checkpoint 4 — window3 material evidence

Work comment5559305170 authorizes window3 from12:49 through13:49 UTC and the bounded architect interpretation: unchanged canonical nested Fresh history plus separate independently current Recovery authorization and the shared Account floor. Existing V1 validators, exhaustive enums and signatures remain unchanged. Prior windows2/rotation1 and unpublished repair historyUNKNOWN persist. Elapsed32 minutes is the material-checkpoint observation, not a window reset.

The new private claim transition binds original admission operation, exact owning-source predecessor/successor CAS and a separate immutable claim-time Recovery/actor audit. Fresh nested provenance is preserved byte-for-byte except its permitted local publication wrapper. Real review P2 RED: original Recovery7, retained Fresh8 and current Recovery9 was incorrectly rejected by comparing history to the original credential. The repair retains the original operation7 while binding claim-time authorization9, and validates subsequent current decisions against both original and claim-time history. Current Recovery cannot be substituted by Fresh observations or regress below the shared floor.

Split PREPARE/COMMIT/reconcile requests preserve the exact complete operation. Raw stored history can request only reconciliation; a sealed completion source supplies typed historical outcomes. A sealed PREPARED report plus newly verified credential/current owning sources is required to resume a restarted prepared operation. Commit receipts do not install controllers. Direct and reconciled success use the same independent current adoption fence: canonical candidate/session origins, exact controller/transport, actor presence/placement, retained closed budget/protection, same-holder claims, independently current Recovery/trust and shared floor. Late adoption preserves original decision time and does not recheck expired original credential time, but current authority still must pass. Failed adoption clears the local projection. Terminal collision/failure reasons remain typed and cannot reopen the flow.

Actual missing-API REDs cover claim, split flow, adoption, prepared restart and typed terminal outcomes. Self-review actual RED found omitted candidate initial transport in adoption; repair adds exact canonical initial transport/generation and lease/scope origins. Known repair counter is3: prior wrapper1, claim-time review1, canonical-origin self-review1; prior unpublished history remainsUNKNOWN.

Fresh local evidence:30 focused post_grace tests GREEN; full locked game-server package passes library320, all selected integration binaries and18 compile-fail doctests. Strict all-target Clippy passes7.66s; fmt and governance pass. Unconfigured PostgreSQL124 is skip/compile compatibility evidence only, not actual SQL execution. Root still must review exact published content, run canonical selected CI/MQ and later normal merge-up/source-inclusion qualification. Current adoption conservatively requires exact committed claim successor rows; further owner publication prevents this attempt from installing a projection. The later SQL/provider child must implement registered owning sources and atomic current locked boundaries; this semantic child establishes no production registration or server seam readiness.
