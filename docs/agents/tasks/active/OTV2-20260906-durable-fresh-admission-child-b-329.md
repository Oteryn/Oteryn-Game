# OTV2-20260906-durable-fresh-admission-child-b-329

Work admitted this allocation after protected PR #333 integration and readback at `b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd` (tree `d7f5a1a07d2a27b81de75ad8c3264848981d4279`); issue #329 admission comment `5557972460` precedes worker mutation.

```yaml
task_id: OTV2-20260906-durable-fresh-admission-child-b-329
title: Persist atomic fresh admission and owner-authored claim effects
mode: IMPLEMENT
status: in_progress
admission_state: ADMITTED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/durable-fresh-admission-child-b-329
issue: 329
pr: 335
allocation_source_main_sha: f69e9c12c8b69b625a7ce9d911bf3132c141ada6
admission_main_sha: b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd
base_sha: b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated Durability Child B worker
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-06
updated_at: 2026-09-06
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/src/durability/admission_authority_guards.rs
  - apps/game-server/src/durability/admission_journal.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/migrations/0002_fresh_admission_authority.sql
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/postgres.rs
  - docs/agents/tasks/active/OTV2-20260906-durable-fresh-admission-child-b-329.md
  - docs/superpowers/plans/2026-09-06-durable-fresh-admission-child-b.md
public_contracts: [FND-DUR-FRESH-ADMISSION-V1, FND-DUR-FRESH-CLAIM-PUBLICATION-V1]
depends_on: [326, 331]
blocks: [Child_C_composition, 247]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Asynchronous fresh COMMIT, typed publication and original-operation reconciliation atomically persist owner-authored claims, canonical session, immutable receipt and global transport reservation. Existing reconnect V1/V2, current-authority fencing and truthful lifecycle continuity remain valid. Real PostgreSQL17.6 evidence proves this boundary; it does not prove production producer readiness.

## Architecture and source of truth

- Issue #329 and parent #162 remain authoritative. This prospective allocation follows verified #331 integration; no worker is admitted until its own protected merge and Work readback.
- Accepted decisions: `docs/architecture/reviews/OTERYN_GAME_FRESH_ADMISSION_DURABILITY_AUTHORITY_DECISION_2026-09-05.md` (#313/#317) and `docs/architecture/reviews/OTERYN_GAME_ATOMIC_FRESH_CLAIM_PUBLICATION_DECISION_2026-09-06.md` (#324/#325).
- Foundation reference #326 native `14389fe41e8d3053e5143bdeee2acc7dd97eff00`, tree `b1348af0baecc10e2f54eba5766d45b3060e3208`. Its four authored blobs match local `925387f70bc4b8ab0ae2bf70058b89e03c8c8792` (tree `9c423133b73099289adcfdfc391af3d34ba5c79e`); final integration additionally includes accepted #330 documents. Protected #331 merge is `f69e9c12c8b69b625a7ce9d911bf3132c141ada6` after Merge Queue `34019841021` PASS. This is allocation source provenance, not B worker admission.
- Plan: `docs/superpowers/plans/2026-09-06-durable-fresh-admission-child-b.md`.
- After protected allocation integration, Work records the actual immutable admission/base in the issue before first worker mutation and creates the canonical branch from it. The worker replaces NOT_ADMITTED values in its first material checkpoint using that verified SHA; no self-referential admission commit is required. A proposed branch name alone grants no authority. Work owns PR creation, shared LIVE documents and #326/#328 terminal archives.
- One worker/branch/worktree; shared schema and lock/lifecycle dependencies require serial mutation. Independent read-only analysis/review may run in parallel. No custody overlap at rotation.
- C/#319 actual sources/readiness remain separately required. The inherited post-grace limitation is not a B blocker and is not new policy authority.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [complete_operation_identity, owner_provenance, source_and_publication_high_water, current_guard_CAS, account_character_world_binding, account_global_exclusion, character_lease, runtime_scope_and_independent_revisions, transport_nonreuse, original_L_time, canonical_session_lifecycle, current_adoption]
consumer_boundaries: [publication_CAS, final_L, atomic_SQL_effects, retry_reconcile, restart_decode, first_control_loss, reconnect_V1_V2, terminal_replacement, terminal_release, normalized_completion_adoption]
mutation_operators:
  applicable: [missing_guard, wrong_key_owner_purpose, stale_source_CAS, equal_revision_contradiction, decision_reuse, changed_transition_replay, wrong_candidate_transport, wrong_holder_lease_scope, expiry_during_pre_L_acquisition, future_nonmonotonic_time, provenance_substitution, concurrent_incumbent, cross_origin_collision, lost_commit_response, process_restart, stale_release, overflow, mirror_corruption, migration_conflict]
  considered_not_applicable: [new_wire_protocol_no_wire_change, production_source_connectivity_belongs_to_C, new_post_grace_policy_not_authorized]
one_invariant_per_negative_case: required
independent_current_fact_sources: [sealed_test_crate_owner, independently_published_locked_DB_guards, independently_current_session_and_transport]
record_derived_matching_helper:
  allowed_for_positive_happy_path: historical_identity_classification_only
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: fresh_publication_claim_preserving_replacement_release
  protocol_versions: preserve_reconnect_V1_V2
  direct_and_reconciled_paths: required
  fenced_durable_writes: all_guard_claim_session_receipt_reservation_effects
  restart_retry_replay_concurrency_pg_reload: required_hosted_PostgreSQL17_6
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [ ] Four typed current guard domains, owner provenance, persistent high-water/tombstones, exact CAS and decision/effect consistency; migration invents no authority.
- [ ] Forward-only 0002 preserves 0001 and existing reconnect rows; canonical session table, account/character nonterminal uniqueness, truthful absent fresh continuity, full-u64 and strict mirrors/decoding.
- [ ] Fresh request consumes complete `operation()` and exact returned owner-authored successors. Claim/high-water/session/receipt/reservation effects commit atomically after valid L; no fresh PREPARE or source metadata fabrication.
- [ ] All competing publication/fresh/V1/V2/lifecycle/mutating-reconcile paths use documented deterministic absent-key/row ordering. Actual unique/FK/table waits are inventoried and resolved before trusted DB time; unexpected post-decision semantic contention aborts/revalidates.
- [ ] Exact/conflicting replay includes transition decisions/effects; lost/ambiguous response and process restart reconcile original operation. Receipt/current snapshot share one fence; historical evidence never grants current authority.
- [ ] One permanent cross-origin transport namespace; all collision directions, exact owner replay and migrated reservations qualified.
- [ ] Fresh first control loss populates real continuity; claim-preserving reconnect retains metadata; accepted replacement and terminal release consume #326 sealed effects with matching session transaction. Stale release cannot clear successor.
- [ ] Release preserves #326 structural-freshness repair: old valid unchanged nested Platform evidence is allowed for relinquishment, fresh Game successor still required; no provenance substitution/re-aging or new post-grace policy.
- [ ] Bounded queue/normalized completions keep SQL/database wait outside FND-03 writer; independently current postcommit adoption remains required.
- [ ] Compiler-verified sealed source-identical integration harness exposes no production forging route. All required real PostgreSQL17.6 matrix families in the plan execute through existing hosted `--test durability_postgres`, with exact head/run/job/test evidence.
- [ ] Migration/role/checksum/locking/reload/corruption/full-u64 negatives and V1/V2 regressions pass. No unconfigured local skip is SQL proof.
- [ ] Focused RED/GREEN, component/doctests, fmt/strict Clippy, architecture/governance, whole-diff adversarial review and independent exact-head high-risk review pass; every material finding dispositioned.
- [ ] Full canonical selected CI and Merge Queue game-gate pass; Work verifies protected readback, archives/releases task. Completion does not release Server Seam without C readiness.

## Excluded scope

No Foundation, Cargo/lockfile, workflow/protection, 0001 edits, production DB/source/bootstrap/key/live-account work, external repositories, listener/deployment, arbitrary transfer/takeover or new post-grace policy. No local PostgreSQL privilege/root-check bypass. Missing concrete path needs go to Work before mutation. No automatic source readiness or B/C release claims.

## Implementation / findings

Window 1 started at `2026-09-06T08:17:20Z` on the verified clean canonical branch and dedicated worktree. Initial material checkpoint adds the source-identical integration Foundation module/alias and a test-only sealed claim owner. The focused SQL schema case requires the fresh receipt table and truthful nullable initial continuity; its real hosted RED is pending. No schema or adapter success is claimed. Early compiler proof and SQL RED must use the existing hosted configured PostgreSQL route for actual DB assertions. Local PostgreSQL cannot start under the current user-namespace/unprivileged-identity constraints; continue useful local compilation and hosted qualification without weakening gates.

The plan's milestones preserve full acceptance and are not separate task/PR allocations. At each authorized 60-minute window boundary, checkpoint exact durable head, changed paths, evidence, unresolved findings and one next action. Work controls continuation/custody rotation. A frozen failing candidate remains frozen/failing until dispositioned; rotation does not reset failure, repair or CI counters. No metadata/no-op commits for counter resets.

### Checkpoint 2 — forward schema implementation; SQL GREEN pending

- Canonical first checkpoint: PR #335 head `a251d94b4a23b7ae0b889a0a6dfbb93dfb1574f3`, tree `d3855e4e18bd308952eb7058ef3810dc69a72432`; normal fast-forward preserved one branch/history.
- Actual configured PostgreSQL17.6 RED: gate `34021756196`, Linux job `101455557352`, `2026-09-06T08:28:13Z`; target reported 295 passed / 1 failed. Sole failure `fresh_admission_forward_schema_supports_truthful_atomic_session`: `fresh atomic admission requires its immutable receipt table`. Compiler and lint succeeded. This is schema prerequisite RED, not full atomic adapter proof.
- Forward 0002 adds immutable complete-operation receipt storage, canonical fresh-origin continuity, account uniqueness, four typed guard domains and permanent source/decision history, exact fresh/reconnect reservation ownership, and lifecycle receipt storage. Migration does not bootstrap source truth. SQL execution/constraints and all adapter acceptance remain pending until configured hosted evidence.
- Independent test-only source now verifies a signed fresh grant, prepares paired owner effects and captures the request through the existing bounded port. Local focused fixture check passed 1/1, including independently missing current guard rejection. Compiler RED for the initially absent fixture was observed separately; logs `b329-fixture-red.log` and `b329-fixture-green.log`.
- Work explicitly dispositioned existing isolated-admin reservation corruption fixtures: disable only the named immutability trigger and inject corruption inside one transaction, then re-enable before commit; error/drop rolls back both changes. Existing reload assertions remain. A separate restricted test runtime role is granted normal DML only and must fail both reservation deletion (23514) and trigger disable (42501). Its role/grants are themselves transactionally rolled back. No production bypass is added.
- Shared pre-L inventory: V1 prepare session/attempt/actor continuity/transport/pending FK; V1 commit unique nonce and session/attempt/protection state; mutating reconciliation protection activation; V2 broad predecessor attempts and candidate replacement/session/continuity/reservation/pending FK. New receipt/guard/history/account uniqueness must join one protocol. Common locks and lossless codecs are not implemented yet.

## Validation

### Focused

Checkpoint 1 compiler verification: `CARGO_TARGET_DIR=/workspace/scratch/ec4cc99115b7/game-live/target /root/.cargo/bin/cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres --no-run` passed (28.81s); log `/workspace/scratch/ec4cc99115b7/b329-harness-compile.log`. Source-identical Foundation/Durability types and the test-crate owner seal compile without production changes. `cargo +1.94.0 fmt --all -- --check` passed. Focused strict Clippy initially caught fixture placement after an internal test module; moving that fixture before the module repaired it, and `cargo +1.94.0 clippy --locked -p oteryn-game-server --test durability_postgres -- -D warnings` passed (10.20s). This was a pre-publication compiler repair, not a hosted SQL RED. Focused case `fresh_admission_forward_schema_supports_truthful_atomic_session` is prepared for configured hosted execution; expected RED is the missing immutable receipt table. This assertion proves the missing schema prerequisite only; it does not claim an atomic transaction test or SQL GREEN.

Record intended RED, observed failure, minimal GREEN and affected-family sweep on each risk-bearing change. Compiler/harness RED and actual configured SQL RED are separate evidence. Never count a skipped PostgreSQL case as passed SQL qualification.

### Component/integration

Run Rust1.94.0 locked game-server tests/doctests, fmt, package strict all-target Clippy, architecture and task governance. Preserve full canonical selected build/workspace/platform/security/supply-chain checks; focused commands do not narrow the gate.

### E2E

Required real hosted isolated PostgreSQL17.6 target: `cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres`. Record exact native head/run/job/cases and configured DB evidence. Use independent fixture source mutation and deterministic lock barriers. Do not claim production readiness or physical Server Seam journey.

### Exact-head CI

Canonical remote head is PR/check authority. Preserve native/local SHA/tree mapping if Work publishes through an authorized native path. Exact-head selected CI and full Merge Queue game-gate remain mandatory. Reconcile upstream normally without force/reset or repeating unchanged failing CI.

## Self-review

NOT_STARTED. Required full changed-content adversarial review and AuthorityInvariant x ConsumerBoundary x MutationOperator coverage; no receipt-derived current authority in negative cases.

## Independent review

Required YES: durable schema/admission/provenance/lifecycle. Genuinely independent exact-head non-author review; accepted material findings require focused repair/family sweep, rejected findings exact evidence. Review does not grant merge authority.

## PR and closeout

Work creates/identifies the sole implementation PR after protected allocation; worker updates only its authorized branch/paths and returns a reviewable exact-head candidate. Work controls protected integration/readback/archive/release and shared programme documents. Allocation PR is not implementation completion.

## Context checkpoint

```yaml
last_progress: hosted schema RED proven; forward schema and independent sealed fixture implemented; adapter pending
status: in_progress
admission_state: ADMITTED
branch: agent/durable-fresh-admission-child-b-329
head_sha: null
pr: 335
execution_budget_minutes: 60
execution_window_number: 1
execution_window_started_at: 2026-09-06T08:17:20Z
execution_window_elapsed_minutes: 0
execution_windows_completed: 0
worker_rotations: 0
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: [34021756196]
ci_job_ids: [101455557352]
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Work publishes forward schema checkpoint for configured SQL qualification; worker continues lossless operation codecs and shared pre-L locking under unchanged acceptance.
```
