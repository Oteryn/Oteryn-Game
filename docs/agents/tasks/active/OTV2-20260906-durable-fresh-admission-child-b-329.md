# OTV2-20260906-durable-fresh-admission-child-b-329

## Current prospective351 hosted-test lease — Work162

Work162 [comment5560691505](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5560691505) authorizes this five-document allocation package on `coord/sqlx-351-test-lease`, based on protected main `b61f9d8cc1c0a7289ffdaf1bf4e42b851d2c0f9a`. The new test-target lease is **NOT_ACTIVE** until independent qualification, protected integration/readback and an explicit Work grant. Existing351 implementation admission continues; this is not a new worker admission or budget reset.

Current custody overrides older prospective/NOT_ADMITTED prose below. B329 retains immutable admission `b8ae4c965cc7f686b89b4d5c0ba2bc04af6e07fd`, branch `agent/durable-fresh-admission-child-b-329`/PR335, window6/completed5/rotation1 under [329 comment5560373810](https://github.com/Oteryn/Oteryn-Game/issues/329#issuecomment-5560373810). Native `6a2cccb5f448fc9f3b8ca07e1e4a66dc7aadec29` is the actual restart qualification checkpoint recorded in [329 comment5560643661](https://github.com/Oteryn/Oteryn-Game/issues/329#issuecomment-5560643661), not full B acceptance. Newer canonical B head `834db1d7118d751e31287715d3eaac7780a0c7b9`, tree `b11f20a35e4c205c7e3320469616ccd4aaa96bc5`, is the independently reviewed sealed-completion checkpoint in [329 comment5560718303](https://github.com/Oteryn/Oteryn-Game/issues/329#issuecomment-5560718303); its hosted CI is pending, not covered by the earlier366/0 result. At this checkpoint B reports50 productive minutes used in window6, approximately10 remaining, with waiting paused; this amendment adds no minutes. Driver351 retains immutable admission `53c6bdf06a2282d893035a995c46052c88f935b4`, branch `agent/sqlx-driver-budget-351`/draft PR356 and window2 under [351 comment5560554622](https://github.com/Oteryn/Oteryn-Game/issues/351#issuecomment-5560554622), following native `1363c9b5b238f4922615eda9b502866c305e83bf`. Window1 remains55m14s productive/4m46s unused, completed1/repair1/rotation0. These immutable checkpoints do not replace later canonical branch heads or cumulative findings. Preserve all branch/task history and subsequent windows/repairs through normal merge-up; old zero counters below are historical allocation evidence.

### Exact prospective transfer

After the activation gate, temporarily remove `apps/game-server/tests/durability_postgres.rs` from B329's active write scope and lease it exclusively to the sole351 writer **only** to add this module inclusion:

```rust
#[path = "../../../vendor/sqlx-postgres-0.9.0/tests/oteryn_resource_budget.rs"]
mod oteryn_resource_budget;
```

The included `vendor/sqlx-postgres-0.9.0/tests/oteryn_resource_budget.rs` stays within351's existing vendor subtree. No other change to the shared target is authorized: preserve every existing B test, import, fixture, gate and assertion; no reformatting or test suppression. No workflow, Cargo feature/dependency, production B, Foundation or source scope is added. Driver retains its separately protected two-crate Cargo lease and exclusions.

B keeps every other owned path and its canonical branch/worktree. Work verifies exact overlap before granting the lease and before integration. While active, B must not write the shared target or integrate overlapping target changes;351 may not use the lease for any additional edits. Work serializes ordinary merge-up and reviews the resulting exact delta, retaining prior B material. Return the target to B only after protected351 delivery/integration/readback and Work's explicit release/readmission for this file; no concurrent writer or automatic lease inheritance. Earlier14-path B lists remain historical during the active transfer.

The existing canonical PostgreSQL17.6 target must actually execute the included tests on the pinned root dependency graph. Vendor-only test results and successful compilation do not establish hosted SQL execution. Keep all existing workflows and tests intact. If the service is plaintext, it supplies no TLS-positive evidence:351 must separately qualify actual TLS without security/feature downgrade or treating skipped/unconfigured tests as success. This amendment alone proves no TLS/driver/B acceptance and does not release Server Seam247.

### B effective owned paths during the activated lease

Only after activation, B retains these13 paths from its canonical14-path scope; the shared target is excluded until Work returns it:

- `apps/game-server/src/bin/oteryn-game-migrate.rs`
- `apps/game-server/src/durability/fresh_admission.rs`
- `apps/game-server/src/durability/admission_authority_guards.rs`
- `apps/game-server/src/durability/admission_journal.rs`
- `apps/game-server/src/durability/db.rs`
- `apps/game-server/src/durability/mod.rs`
- `apps/game-server/src/durability/schema.rs`
- `apps/game-server/migrations/0002_fresh_admission_authority.sql`
- `apps/game-server/tests/support/postgres.rs`
- `apps/game-server/tests/support/authority_matrix.rs`
- `apps/game-server/tests/support/authority_recovery.rs`
- `docs/agents/tasks/active/OTV2-20260906-durable-fresh-admission-child-b-329.md`
- `docs/superpowers/plans/2026-09-06-durable-fresh-admission-child-b.md`

### B329 task-record handoff

This is a coordinated ownership correction under Work162 comment5560691505. This task's earlier metadata and all historical evidence remain intact. It grants no reset/replacement of the existing admitted worker. Before activation, existing runtime custody remains unchanged; after activation, the exact shared-target exception above supersedes earlier conflicting path lists only. Preserve the worker branch's newer task evidence when normally merging this coordinator amendment.

Next action: Work qualifies/protects the amendment and verifies exact branch overlap before granting the target lease.

This exact prospective allocation activates only after protected integration and Work readback. The actual allocation merge becomes immutable worker admission/base before first mutation.

```yaml
task_id: OTV2-20260906-durable-fresh-admission-child-b-329
title: Persist atomic fresh admission and owner-authored claim effects
mode: IMPLEMENT
status: waiting
admission_state: NOT_ADMITTED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/durable-fresh-admission-child-b-329
issue: 329
pr: null
allocation_source_main_sha: f69e9c12c8b69b625a7ce9d911bf3132c141ada6
admission_main_sha: NOT_ADMITTED
base_sha: NOT_ADMITTED
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

NOT_STARTED. This draft contains no implementation/test success claim. Early compiler proof and SQL RED must use the existing hosted configured PostgreSQL route for actual DB assertions. Local PostgreSQL cannot start under the current user-namespace/unprivileged-identity constraints; continue useful local compilation and hosted qualification without weakening gates.

The plan's milestones preserve full acceptance and are not separate task/PR allocations. At each authorized 60-minute window boundary, checkpoint exact durable head, changed paths, evidence, unresolved findings and one next action. Work controls continuation/custody rotation. A frozen failing candidate remains frozen/failing until dispositioned; rotation does not reset failure, repair or CI counters. No metadata/no-op commits for counter resets.

## Validation

### Focused

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
last_progress: prerequisite331 integrated; exact prospective B allocation prepared; no worker admitted
status: waiting
admission_state: NOT_ADMITTED
branch: agent/durable-fresh-admission-child-b-329
head_sha: null
pr: null
execution_budget_minutes: 60
execution_window_number: 0
execution_window_started_at: null
execution_window_elapsed_minutes: 0
execution_windows_completed: 0
worker_rotations: 0
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
blocker: NOT_ADMITTED
next_action: Work qualifies this exact allocation, then binds its actual protected merge and dispatches the sole worker.
```
