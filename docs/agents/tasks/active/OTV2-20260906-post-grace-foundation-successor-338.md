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
head_sha: ceccd130b50c871236b096723754c6a26f1c11bf
final_head_sha: null
final_head_frozen_at: null
owner: foundation_audit (Work custody rebind 5559848749)
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
  - apps/game-server/src/foundation/control_loss_durability_tests.rs
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

- [x] Private live successor requires newly verified reauthenticated recovery with correct source scope/provenance/deadlines and independently owning current actor/continuity source. Raw DTO/history/caller flag/reconnect proof cannot select it.
- [x] Explicit closed timing discriminant preserves SameSession/V1/V2 behavior. Missing/unknown/mixed variant rejects; old failures/history cannot upgrade into live post-grace privilege.
- [x] At preparation and final authority now is strictly after exact historical grace and within frozen finite deadline derived from accepted credential/security/trust bounds with checked arithmetic. Refresh cannot extend the same attempt; equality/early-terminal legacy behavior remains unchanged.
- [x] Candidate session differs from terminal predecessor and starts connection generation exactly1; predecessor generation is separate. Same uncontrolled actor/placement/state, account/world, lease/scope/revisions and absent controller remain current prerequisites.
- [x] Retained loss epoch/grace and complete actor-bound attempt budget/dispositions span replacement. Eight/nine and exact retries qualified; missing compacted state fails closed, no new empty budget inferred on restart.
- [x] Protection entitlement/consumption/activation/rearm evidence is retained; no mint/reset/rearm/new epoch/respawn/heal/relocation. Existing eligible once-only activation and retirement/restoration finality preserved.
- [x] Additive PREPARE/final/completion/reconcile/adoption surfaces use exact immutable operation and independently current facts. Later source deny/fence change rejects; historical committed outcome cannot reinstall stale controller.
- [x] Owner-sealed successor claim effects bind exact successor operation and current predecessor/claims; standalone publication cannot apply them. Existing B-consumed claim APIs and enum variants remain compatible.
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
last_progress: owning loss plus adoption-continuity and bounded-field review repairs pass; full package339 and25doctests, strict lint and B source-inclusion rerun; Work final review/publication remains
status: running
admission_state: ADMITTED
branch: agent/post-grace-foundation-successor-338
head_sha: null
pr: 343
execution_budget_minutes: 60
execution_window_number: 4
execution_window_started_at: 2026-09-06T13:49:00Z
execution_window_elapsed_minutes: 40
execution_windows_completed: 3
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
repair_cycles_for_current_gate: 9
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


## Checkpoint 5 — remaining semantic matrix

Same window3 and branch; parentf2ccc7de74ed58bd47c7c71c0d1e62c8a5fce331 has independent/root P0/P1/P2=0 and canonical CI success recorded by Work3435559531198/5559562859. Additive qualification expands both PREPARE/final locked boundaries across12 actor,8 source,8 canonical mutations plus stale/missing claims, unavailable source and queue delay. Thirteen authentically re-signed credential negatives cover profile/purpose/issuer/audience/protocol/transport/revisions/time/generation. A full8-entry retained mixed history with already consumed protection successfully commits/adopts without reset; dropping an old entry rejects. Adoption-source forgery and receipt-to-controller conversion add compile-fail checks. No runtime semantics changed after checkpoint4; two rustdoc anti-forgery examples are the only runtime-source edit.

Acceptance evidence is mapped directly in the plan; implemented semantic items are checked, while actual B-integrated compile, selected exact-head integration/MQ and Work release remain open. Signed nbf+1 was a mistaken test expectation because accepted tolerance is+5; corrected to nbf+6 without changing runtime policy. Repair counter3 and priorUNKNOWN remain. Unexpected-control-loss amendment348 is not active in this checkpoint; no new path written.

Checkpoint5 qualification at 2026-09-06T13:37:35+00:00:34 focused family cases pass in full locked package library324; all integration targets and20 compile-fail doctests pass. Strict all-target Clippy passes6.05s; fmt/diff/governance pass. Explicit protected-source durability_postgres --no-run passes; this is current checkout source compatibility, not future B-integrated source or SQL execution proof. No runtime behavior changes beyond checkpoint4.


Checkpoint5 independent/root review P2 accepted: the eight canonical predecessor negatives reused actor source revision11 while changing canonical content, so same-revision contradiction masked their intended fences. The repaired matrix starts from coherent independent actor revision12/accepted12/new decision/time101, first proves unchanged canonical facts succeed at BOTH PREPARE/final locked boundaries, and then changes exactly one canonical predicate per negative. Focused repaired matrix passes. Runtime semantics did not change; this is evidence-strength repair, not a claimed runtime RED. Cumulative known repair counter is4; priorUNKNOWN preserved. Full package324/20-doctest evidence immediately above predates this test-only repair; repaired focused and strict Clippy evidence are fresh.


## Checkpoint 6 — owning unexpected-control-loss window4

Work custody rebind [5559848749](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5559848749) resumes the paused/unstarted window4 under sole writer `foundation_audit`. Existing admission `4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee`, branch/PR343, completed windows3, rotation1 and prior UNKNOWN history remain unchanged. Protected348 `d9d1b566acb57b537ff901d9765c32a95110c259` adds the seventh test path; its owning-loss addendum governs this checkpoint. A normal merge-up to that exact main is staged with both original parents preserved, not committed/published by the worker. Work retains native publication/integration. Paused time is not charged as productive time.

Implemented a separate owning unexpected-loss source, private authorization, single-take split request, pure final effect, sealed completion, immutable receipt and history-only reconciliation flow. Exact scope plus ownership generation and existing owner-issued loss epoch identify the source/decision; complete original origin/grace, canonical session, controller/transport, connection generation, actor placement, Account presence, lease and retained prior budget/protection remain bound. Fresh-origin continuity is an explicit sealed-owner assertion, never inferred from a missing row. Resumed history retains the completed budget and consumed/rearm evidence. Post-grace generation-one sessions remain eligible for a later independently proven loss. The effect makes only that session reconnectable and removes only its exact controller; no claim, actor, lease or protection mutation is included.

The later durable adapter must call the final predicate using independently current owning facts under the same atomic fences as its mutation. Historical reports classify persistence only; they cannot produce another live request/effect or restore controller authority. Exact committed retry returns its original receipt; conflicting original operation/time/disposition rejects. Actual SQL atomicity, runtime source registration and production liveness/rearm policy remain outside this semantic allocation.

Actual missing-API REDs precede initial loss and split flow. Twelve focused cases cover first and resumed loss, post-grace generation1, six non-authoritative causes at both authorization/final, nineteen independently changed final predicates with newer coherent positive controls, ten missing/provenance/time negatives at both boundaries, absent source, immutable retry/completion conflicts, closed versions/terminal dispositions and restart without a live capability. Five additional compile-fail examples deny external loss-source/completion-source registration, raw authorization/effect construction and receipt-to-request conversion.

Self-review accepted two predicate findings with actual 9-pass/2-fail RED: protection must validate at immutable loss origin, not a later observation time; resumed retained committed transport must match the current lost controller. Both repaired, then eleven focused cases passed. Independent review additionally identified unbounded new String identities in the supposedly bounded predicate. Replaced them with existing fixed-size `RuntimeScopeRefV1` and `ControlLossEpochRefV1`, retaining exact equality and complete immutable origin/grace; no Platform byte cap, new entity/wire identity, hashing or truncation. The only variable-sized fields are the existing canonical UUID account claim and at-most-eight retained budget entries. Known repair cycles now6 plus priorUNKNOWN (self-review cycle5, boundedness cycle6).

Final material validation and independent exact-tree disposition are recorded below before publication. Actual B-integrated source inclusion, selected exact-head CI/Merge Queue/protected closeout remain outstanding. This is not Server Seam release.

Next action: Work reviews and native-publishes the exact staged two-parent candidate on existing PR343, then qualifies canonical CI and the remaining integrated source/adapter dependencies without resetting custody/history.


Checkpoint6 final local evidence at 2026-09-06T14:43Z: complete locked package passes336 library tests and25 compile-fail doctests; all selected integration targets pass. The unconfigured PostgreSQL124 cases are skip/compile compatibility only, not SQL execution. Strict all-target Clippy passes4.02s; formatting/whitespace, governance26 policy documents/9 lanes and workspace architecture check pass. Dedicated semantic workflow remains a canonical-CI check; its exact staged seven-file scope does not match a local dedicated semantic profile. No local authored commit was manufactured for that check.

Work separately authorized qualification-only composition from exact remote B `0b47fff257902bdcc71c4ee68a6123fd1d9585c1` with the five Foundation runtime/test blobs from staged tree `9ebb421b518b0b29e45d597362b2329c5cc879c5`. Disposable archive `/workspace/scratch/c123eb18789e/foundation-b-qualification` passed `cargo test --locked -p oteryn-game-server --test durability_postgres --no-run` in19.91s using the Foundation-exclusive target. This proves actual B-candidate source-inclusion/API compatibility, not protected integration or database execution. Canonical B branch/worktree was untouched. Runtime bytes remain those of this tested tree; subsequent evidence-only task/plan updates require final review rebind.

Window4 retains its original13:49 allocation timestamp and owner pause. Productive continuation is under5559848749; normal merge-up first mutation is observed locally14:23:26Z. The25-minute checkpoint charge conservatively includes preceding read-only re-entry/qualification work; no stopped time is charged and no window is reset. Completed windows3/rotation1/known repairs6+priorUNKNOWN remain. Work must preserve the remaining window budget across publication/review.


## Checkpoint6 independent whole-diff repairs (supersedes prior candidate freeze)

The independent reviewer accepted a test-only P2: after typed loss decision binding, the reused-epoch negative changed the epoch but left decision identities incoherent, masking the epoch reuse predicate. Fixed both decision identities together with the epoch; the final changed-epoch case additionally proves fresh authorization of the coherent new event succeeds before the old immutable operation rejects it. Focused12 PASS; this is evidence repair, not a runtime RED claim (cycle7).

Verified P1: post-grace adoption did not require the canonical successor session to retain loss epoch/original grace. The old positive fixture itself omitted both fields. Actual RED reproduced controller adoption from missing continuity. The repair independently compares current successor epoch to the retained prior budget epoch and its grace to the original predecessor grace. Direct and reconciled isolated missing/changed epoch/grace negatives now reject and clear the projection. A true adopted canonical snapshot then successfully authorizes a later generation1 owning loss without budget/protection reset. Existing adoption fixtures now establish their complete retained session continuity (cycle8).

Verified sibling resource finding: new post-grace actor/provenance/audit/claim paths could compare or clone unbounded source strings. Accepted resource decision341 §4 `DFR-OPERATION-BYTES` covers complete claim lifecycle operations/effects including nested provenance. Therefore a single retained field cannot exceed its65,536-byte complete operation. Apply that necessary conservative per-field check before owned new raw actor, audit, provenance, completion and claim-row comparisons/clones; fixed two-row and existing bounded reconciliation/budget collections keep local work finite. Actual source-deadline RED precedes repair. Independent inclusive65,536/65,537 controls now cover source provenance, new actor authorization and claim transitions. Keep exact external provenance strings and all existing V1 representations/validators unchanged (cycle9).

This necessary per-field guard is not a complete canonical encoded-size check: sums, framing/escaped encoding, retained copies and the active4MiB execution envelope remain mandatory at the later owning codec/adapter/source boundaries. No new smaller provenance cap, Platform wire mapping, truncation, identity hashing, serializer or resource registration is introduced. No claim of full SQL/producer memory accounting follows.

Runtime/test candidate `bac051bc5e784abb955fb9099a923bd68d77a8f6` passes full locked game-server package339 library tests and25 compile-fail doctests; strict all-target Clippy4.49s; source/actor/claim inclusive-bound tests; post-grace38 cases. These supersede prior336/335 candidate counts. All seven allocated paths now contain authored material; upstream merge-only paths remain exact protected main. Final B composition compatibility and independent review rebind follow below. Window4 conservative cumulative productive charge40minutes, completed3/rotation1/known repairs9+priorUNKNOWN; no budget reset.

Final reviewed-material composition: exact remote B0b47fff257902bdcc71c4ee68a6123fd1d9585c1 plus Foundationbac051bc5e784abb955fb9099a923bd68d77a8f6 five runtime/test blobs again passes source-included durability_postgres --no-run20.67s. Full local validation above binds these same runtime bytes. Formatting/whitespace and governance26/9 rerun PASS after evidence updates. Actual SQL execution and canonical protected integration remain unclaimed.
