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

- [x] Verified fresh result preserves AccountId and every final evidence fence without changing authentication/classification precedence.
- [x] Private owning-source capability/publication API prevents grant, caller fact struct or old receipt from seeding current guard truth; no default-forged evidence.
- [x] Typed bootstrap/CAS/source-order/publication completion semantics keep readiness closed before acknowledged or reconciled publication; exact replay cannot re-age source time.
- [x] Fresh bounded submit/yield/completion/reconciliation flow distinguishes committed, existing committed, replay conflict, incumbent rejection, stale authority and ambiguous/unavailable.
- [x] Same-key retry preserves original immutable binding and decision evidence; ambiguity only reconciles the original candidate/transport and cannot mint another session.
- [x] Strict NumericDate and conservative source-age semantics use checked math; final authority is decided at L, not BEGIN or COMMIT acknowledgment.
- [x] Committed or historical receipt alone cannot install a controller; stale or replaced current facts, reconnectable/terminal lifecycle and higher connection generation fail closed.
- [x] Synchronous compatibility remains explicitly non-production and cannot be the production SQLx path.
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
- result: PASS locally on the Task3/4 material candidate: Foundation 158/158 and doctests 7/7; exact published head and canonical CI remain Work-owned qualification.

### Component/integration

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test authority_invariants`; existing verifier/reconnect regression suite; formatting and strict all-target Clippy
- result: PASS locally on the Task3/4 material candidate: authority_invariants 4/4, cargo fmt and strict all-target Clippy; canonical PR CI also runs selected locked workspace/PG checks.

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
- material findings: independently reproduced and repaired equal-source Game state substitution, equal-Platform-revision security decision substitution, historical deadline extension, absent original physical transport and non-monotonic adoption time; signing guard/source revision disagreement and retained owned request API also repaired test-first
- verdict: PASS_LOCAL_CANDIDATE; independent exact-head review and canonical CI remain outstanding

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
last_progress: coherent verifier and typed publication checkpoint locally qualified; Task3 remains; controlled rotation without completion
status: implementing
branch: agent/foundation-fresh-admission-318
head_sha: null
pr: 321
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: [33985311543, 33985663094, 33985944120, 33986616753]
ci_job_ids: [101357665714, 101358627615, 101359375408, 101361296047, 101361296064]
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: implement Task3 retained authorization/outcome/reconcile/adoption using separate historical DTOs and a journal-independent durable projection
```

Source readiness prerequisite is tracked separately in Issue #319 for Child C. It does not block Child A semantic implementation or widen this allocation.

## Historical rich verifier checkpoint (7141845)

- Executed behavioral RED: `776fcf3ba47b66f660f9f0d56717239ef1011ee4`, run `33985944120`, Linux job `101359375408`: build/strict Clippy passed; independently published positive control failed at the expected `is_ok` assertion, 233 passed / 1 failed. Separate formatting failure was not RED evidence. Work recorded comment `5554135439`.
- Local tool discovery corrected the initial PATH-only assessment: `/root/.cargo/bin/cargo` and `rustfmt +1.94.0` are available. Exclusive clone `foundation-worktree-318` was clean at exact remote `776fcf3` before edits; native GitHub publication preserves parent history.
- Current candidate implements private verified facts, authenticated source provenance and accepted revision/decision checks, conservative source-age arithmetic, original credential times, proposed acquired lease, independent current-field classification and pre-commit revalidation. No production source is registered.
- Executed local `cargo +1.94.0 test --locked -p oteryn-game-server --lib foundation::fnd04_verifier`: 27 passed, 0 failed. Owned-file rustfmt applied. An earlier unqualified focused invocation also linked the unrelated `durability_postgres` integration target and failed with undefined hidden Tokio `Runtime::block_on` symbols; no baseline conclusion is claimed. Canonical workspace/PG CI remains required.
- Remaining: typed publication/bootstrap/CAS/activation, immutable fresh authorization and six outcomes, split-phase reconciliation, independently current adoption, capability doctests and full Child A qualification. This checkpoint is not integration-ready.
- Deadline accessor is the greatest accepted integer NumericDate second. A future durable adapter must preserve the accepted strict credential predicate for its actual trusted timestamp precision; truncating a subsecond L cannot relax expiration. Original receipt audit/deadline values remain immutable once committed.

## Controlled rotation checkpoint — 2026-09-05 19:37Z

Status remains **IN_PROGRESS** (`implementing`); `integration_ready=false`. No task completion, lease release, production source registration or background execution is claimed. The original 60-minute foreground allocation began at 18:44Z and is not reset by this checkpoint.

Implemented since `7141845d2742d10b8fdeae8a8166fa4aa3defb0c`:

- Closed Account/Character/Runtime/SigningTrust publication guards; a sealed owning publisher prepares an immutable atomic request. Raw state/provenance fields cannot register a publisher.
- Independent bootstrap/high-water and locked CAS comparisons, exact idempotent replay preserving source time, source identity/order/decision checks, security/lease/runtime high-water retention and conflicting tombstone/bootstrap rejection.
- Account security retains its own subject and Platform provenance separately from Game presence publication. Advancing only the accepted Game guard wrapper preserves the Platform source revision, decision and observation time; that wrapper must equal the accepted Account guard revision.
- Bounded submit/reconcile ports yield; exact receipt completion additionally checks a separate sealed independently-current source before activation. An older receipt cannot activate after a newer deny. There is no SQL, production adapter or source registration.
- Capability misuse compile-fail doctests, plus edition-2024 formatting of owned source files. The earlier standalone edition-2021 formatting caused signature differences; `cargo +1.94.0 fmt --all --check` now passes.

Executed local evidence:

- Publication bootstrap RED: independently owned positive control failed its expected `is_ok` assertion before implementation (0 passed / 1 failed).
- Account wrapper RED: preserving the same Platform source at a new Game guard wrapper initially returned `Stale` (0 passed / 1 failed), then GREEN after the narrow wrapper comparison fix.
- Independent Platform authority substitution and observation-time rollback RED: both isolated tests returned `Ok(())` instead of `Err(Stale)` (0 passed / 2 failed), then GREEN after identity/time monotonic checks.
- `/root/.cargo/bin/cargo +1.94.0 test --locked -p oteryn-game-server --lib foundation::`: **138 passed, 0 failed**, including 17 publication tests and preserved Foundation/reconnect behavior.
- `/root/.cargo/bin/cargo +1.94.0 test --locked -p oteryn-game-server --doc`: **6 passed, 0 failed**, including both new capability misuse doctests.
- `/root/.cargo/bin/cargo +1.94.0 fmt --all --check`: passed.
- `/root/.cargo/bin/cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: passed. Three local test-fixture `expect` lint findings were repaired with explicit propagated fixture errors.
- Canonical CI for this material checkpoint is not yet observed. Previous `7141845` run `33986616753` had two ordinary observations by Work, last observed running; this worker did not repeat them. Earlier canonical runs and outcomes remain historical evidence, not current-head qualification.

Counters and custody: prior `7141845` CI generation ordinary observations = 2; no identical-failure retry, no CI recovery action, no no-op/rerun trigger, no force/rebase/reset, no second PR. Formatting repair count for the current gate = 3. New material head opens a new CI generation; its first observation belongs to Work. Local test/lint fixes do not convert an expected RED into a completion claim.

Remaining work is substantial: Task3 immutable retained authorization/receipt audit DTOs, six durable outcomes, same-key exact/conflicting replay, ambiguity-only original-binding reconciliation, independently-current adoption and stale projection clearing; a production durable entry independent of the synchronous journal generic bound; full direct/reconciled/restart/transport-collision qualification and independent exact-head review. The persisted historical binding must remain distinct from the private current verified capability so B can restore receipts without forging live authority. Task2 still needs whole-boundary independent review and canonical current-head qualification; physical PostgreSQL semantics belong to B. Child C/source #319 and Server Seam #247 remain unavailable and unallocated here.

One next action: implement Task3 retained authorization/outcome/reconcile/adoption on this same allocated branch after coordinator-controlled continuation, preserving the existing budget/counter history and ownership.


## Task3/4 material candidate — 2026-09-05T20:58Z

The owner explicitly resumed foreground continuation through qualification and integration; Work recorded this on PR #321 comment `5554630920`. This continuation preserves the immutable admission SHA, original 18:44Z dispatch and 60-minute allocation history, existing counters and sole writer/branch custody. It does not reset a budget or declare background execution. Work independently reported prior checkpoint `33f0324` canonical run `33987697014` and `game-gate` SUCCESS; that is historical evidence, not qualification of this new candidate.

Implemented in the seven allocated Foundation paths:

- Separate historical audit binding/receipt DTOs and private verified authorization/request capabilities. Historical restore validates retained NumericDate/source deadlines and original expected/acquired lease bindings; it never constructs live authorization. Owned immutable requests can be retained in a bounded queue for later L validation.
- Four exact typed accepted guard bindings retain source authority/purpose/revision/decision/time, authenticated Platform security and signing trust, account/character/world/channel, every independent runtime/protocol/transport/gameplay revision and original credential times. The pure final-L helper requires independently locked exact rows and trustworthy in-bound time, after all potentially blocking acquisitions; it performs no SQL.
- Six required durable dispositions plus explicit proven candidate/session and shared transport collision classifications. Exact same-key replay returns the original immutable receipt/decision time; changed candidate, transport, account, deadline, credential or guard binding conflicts. Submit and reconciliation only enqueue/yield; wrong, duplicate and out-of-order inputs reject. Missing/ambiguous reconciliation retains the original binding and yields for bounded re-read.
- Direct completion and atomic reconciliation retain history without controller installation. Adoption independently checks owning current guards, actual current session/lifecycle, exact acquired lease/scope, generation one and original transport, plus the original authenticated physical transport mapping. Every failed adoption clears the process projection without erasing historical receipt. Reconnectable/terminal/new-generation/different-scope sources cannot revive the initial controller.
- The narrow core projection and `DurableFreshAdmissionAuthorityV1` have no synchronous journal generic bound or synchronous persistence call. Existing compatibility remains explicitly non-production; reconnect V1/V2 source and behavior are preserved.

Executed RED/GREEN evidence in this foreground continuation:

1. Missing module/protocol accessor and owned-request/reconciliation/final-L APIs produced expected compilation REDs before their implementation. These are API-shape evidence, not behavioral coverage.
2. Independently sourced direct/reconciled adoption control returned `StaleAuthority` against the fail-closed stub (0 passed / 1 failed); checked installation made the positive control GREEN.
3. Equal Game source revision with acquired state, equal Platform revision with changed still-allowed security minimum, extended historical deadline and absent original physical transport each returned success instead of rejection (10 passed / 4 failed), then GREEN after narrow repairs.
4. Adoption at time 99 before the original L=100 returned success (0 passed / 1 failed), then GREEN. A separate delayed completion control at 200 succeeds only with independently newer current sources while retaining original L=100/deadline=103 and occupied same-session presence/acquired lease.
5. Signing guard revision changed independently from verified trust source returned successful authorization (0 passed / 1 failed), then GREEN after exact precommit source agreement.

Finding-family/negative sweep:

- 28 individually mutated owning-guard cases and 9 individually mutated current-session cases, each exercised on both direct and restart-reconciled adoption with a separately constructed current source and positive adoption control first. Guard cases cover presence/holder, account, security minimum/deny/provenance/age/order, character/world/eligibility/lease, runtime readiness/scope/route/runtime/rules/content/map/policy/offer/transport, and trust deny/key/source time/order. Session cases cover terminal/reconnectable lifecycle, generation, missing/different transport, lease, missing world eligibility, same-world different-channel scope and newer scope generation.
- Initial authorization includes all four missing guard domains, protocol mismatch, occupied presence, accepted publication mismatch and independently mismatched signing-source revision. Existing verifier cases preserve authentication/classification order and independent signed revisions.
- Original-binding ambiguity, restart without source floor, missing physical mapping, exact/conflicting replay, both identity collision categories, immutable audit time, strict/inclusive trusted-L bounds, missing time, overflow and one outstanding reconciliation are exercised. Physical same-account/character winner races, fresh/reconnect reservation uniqueness, PostgreSQL locking/reload/WAL and migrations belong to B; actual source registration/readiness and producer mutation inventory belong to C, as already allocated.

Final local commands on these runtime bytes (all exit 0):

- `/root/.cargo/bin/cargo +1.94.0 test --locked -p oteryn-game-server --lib foundation::`: **158 passed, 0 failed**, including 20 Task3/4 tests and preserved publication/verifier/reconnect behavior.
- `/root/.cargo/bin/cargo +1.94.0 test --locked -p oteryn-game-server --test authority_invariants`: **4 passed, 0 failed**.
- `/root/.cargo/bin/cargo +1.94.0 test --locked -p oteryn-game-server --doc`: **7 passed, 0 failed**, including the historical-DTO/live-capability misuse negative.
- `/root/.cargo/bin/cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: PASS. Three local collapsible-if findings were repaired without suppression.
- `/root/.cargo/bin/cargo +1.94.0 fmt --all --check`: PASS; no unowned formatting changes.
- `git diff --check`: PASS.

Full logs are retained for Work in `/workspace/scratch/1a8583297af8/foundation-validation-318/` during this foreground execution. New local PostgreSQL execution is NOT_RUN; no physical durability, producer availability or production readiness claim is made.

Counters remain preserved: prior CI-generation observations and formatting repair count 3 are historical, no identical-failure CI retry, CI recovery action, no-op/rerun trigger, force/rebase/reset, second PR or scope expansion occurred here. Root will publish the tested tree natively because local GitHub credentials are absent; no unauthenticated push is attempted. Native publication establishes the final material SHA and new canonical CI generation.

One next action: Work publishes this exact tested tree to the existing PR #321, obtains canonical exact-head checks and genuinely independent authority/security review, repairs only material findings as required, then performs protected integration/readback. The worker remains available for authorized repairs; no Task3 checkpoint stop or whole-task completion is claimed. Child B/C and Server Seam remain dependency-held.
