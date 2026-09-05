# OTV2-20260905-fresh-admission-architecture-313

```yaml
task_id: OTV2-20260905-fresh-admission-architecture-313
title: Resolve durable fresh-admission GameSession boundary
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/fresh-admission-authority-313
pr: null
base_sha: 5639dc28c3ac27b7da2772778f71d797cfd60537
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: sol supervising architect
created_at: 2026-09-05T19:30:00+02:00
updated_at: 2026-09-05T19:30:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_FRESH_ADMISSION_DURABILITY_AUTHORITY_DECISION_2026-09-05.md
  - docs/agents/tasks/active/OTV2-20260905-fresh-admission-architecture-313.md
public_contracts:
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_DURABILITY_RECONNECT_AUTHORITY_BOUNDARY_DECISION_2026-08-26.md
  - docs/architecture/reviews/OTERYN_GAME_DURABILITY_TOPOLOGY_DECISION_PACKET_2026-08-24.md
depends_on:
  - Issue #313
  - Issue #247
blocks:
  - Foundation fresh-admission durability implementation allocation
  - Durability fresh-admission/session adapter allocation
  - Server Seam Task 3 resume
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Produce one bounded, ownership-correct architecture decision for Issue #313 that defines the asynchronous Foundation -> Durability fresh-admission / initial GameSession boundary without granting runtime implementation authority. The candidate must preserve FND-03 enqueue/yield, FND-04A final authority, accepted reconnect V1/V2 semantics and DUR-02 forward-only migration history.

## Architecture and source of truth

- `PROVEN`: protected `main@5639dc28c3ac27b7da2772778f71d797cfd60537` is the admission base for this task.
- `PROVEN`: Issue #313 is open and is the current critical-path architecture blocker for Issue #247.
- `PROVEN`: Issue #247 is `WAITING_ARCHITECTURE`; its preserved worker checkpoint is `9370b254c6ac4f6529e069c1968ae6bfa1e1750e` and has no implementation PR.
- `PROVEN`: current `apps/game-server/src/foundation/admission_facade.rs` exposes synchronous `ReconnectAttemptJournal<T>::commit_fresh` and lifecycle methods.
- `PROVEN`: current `apps/game-server/src/durability/**` supplies asynchronous reconnect persistence/reconciliation, not a production fresh-admission adapter.
- `PROVEN`: released `apps/game-server/migrations/0001_admission_reconnect_journal.sql` is reconnect/control-loss oriented and is immutable under accepted migration policy.
- `PROVEN`: FND-04A requires final revalidation plus one atomic fresh-admission authority commit; current trusted security/trust evidence uses authenticated source provenance and a <=5 second accepted source-age ceiling.
- `PROVEN`: accepted Durability topology requires bounded persistence submission, yield of the FND-03 writer, asynchronous PostgreSQL work, then normalized completion/reconciliation.
- `DERIVED`: a production SQLx implementation behind the synchronous compatibility trait would violate the accepted writer boundary.
- `DERIVED`: one forward migration can evolve the existing current-session row for truthful fresh origin while a separate immutable receipt provides replay/lost-response identity.
- `UNKNOWN`: final receipt retention/archival policy; deliberately deferred because it does not block first safe implementation.
- `CONFLICT`: none found in current accepted authority.

Candidate decision:

`docs/architecture/reviews/OTERYN_GAME_FRESH_ADMISSION_DURABILITY_AUTHORITY_DECISION_2026-09-05.md`

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - one-time FreshAdmissionReplayKey / GrantNonce consumption
  - AccountId -> CharacterId before CharacterId -> WorldId
  - account-global incumbent exclusion
  - exact CharacterLease generation
  - exact RuntimeScope ownership generation
  - current security/trust provenance/freshness/anti-rollback
  - exact independent protocol/transport/revision bindings
  - candidate GameSessionId and AuthenticatedTransportRefV1 binding
  - first connection_generation = 1
  - current GameSession lifecycle/controller authority
consumer_boundaries:
  - Foundation final authorization construction
  - Durability atomic fresh-admission COMMIT
  - Durability retry/lost-response/restart reconciliation
  - Foundation post-commit independently-current adoption
  - first control-loss/reconnect transition from a fresh-origin session
mutation_operators:
  applicable:
    - missing required fact
    - stale generation or revision
    - mismatched identity or binding
    - expired/future/non-monotonic time evidence
    - same-key exact replay
    - same-key conflicting replay
    - same-account concurrent admission
    - same-character concurrent admission
    - session-id or transport-ref collision
    - lost DB response after commit
    - process restart before completion consumption
    - runtime-scope replacement during in-flight commit
    - replay after reconnectable or terminal lifecycle transition
    - PostgreSQL reload/reconnect
  considered_not_applicable:
    - reconnect predecessor replacement design changes; existing DUR-RECONNECT-AUTHORITY-V1 remains authoritative
one_invariant_per_negative_case: required_for_implementation_children
independent_current_fact_sources:
  - FND-04A verifier current Platform-security/trust authority
  - current AccountId/CharacterId/World eligibility authority
  - current CharacterLease authority
  - current RuntimeScope ownership authority
  - current GameSession/controller authority
record_derived_matching_helper:
  allowed_for_positive_happy_path: only exact idempotent replay classification
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required
  protocol_versions: Foundation fresh v1 plus preserved reconnect v1/v2
  direct_and_reconciled_paths: required
  fenced_durable_writes: required
  restart_retry_replay_concurrency_pg_reload: required
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] Fresh GitHub readback proves #313 remains the current material architecture blocker.
- [x] Candidate answers the exact ownership, split-phase execution, durable linearization, retry/lost-response, current-authority and migration questions from #313.
- [x] Existing reconnect V1/V2 semantics are preserved rather than redesigned.
- [x] No runtime, migration, Cargo, workflow, resource-value, stable-ID, production or cross-repository authority is granted by the architecture candidate.
- [x] Exact future Foundation and Durability implementation surfaces and ordering are named.
- [x] Synchronous `commit_fresh` compatibility disposition is explicit and forbids production SQLx blocking.
- [ ] Exact-head repository/governance checks are green.
- [ ] Required independent high-risk architecture review is complete with all findings explicitly disposed.
- [ ] Candidate is integrated through normal protected controls and read back from protected `main` before implementation allocation.

## Excluded scope

No runtime Rust implementation, SQL migration execution, database mutation, Cargo/lockfile change, Server Seam worker mutation, workflow/ruleset/protection change, resource-limit number, protocol/gameplay ID, production/deployment/secret, Platform write, Atlas write, external-repository write or self-merge.

## Implementation / findings

Selected candidate: `FND-DUR-FRESH-ADMISSION-V1`.

Key design result:

- Foundation performs complete final FND-04A revalidation and emits a versioned `FreshAdmissionCommitAuthorizationV1` carrying the exact durable binding and existing accepted current evidence fences/deadline.
- Persistence submission is bounded and asynchronous; FND-03 writer yields.
- Durability performs one PostgreSQL COMMIT transaction that consumes the typed replay key via immutable receipt and creates the canonical ACTIVE GameSession generation 1 under account/character single-winner constraints.
- Ambiguous/lost response reconciles by the same replay key and cannot mint another winner.
- Foundation consumes completion as a new normalized input and independently resolves current authority before installing/binding the physical transport.
- `ReconnectAttemptJournal<T>` stays test/in-memory compatibility; SQLx must not hide behind the synchronous surface.
- migration `0001` remains immutable; later implementation uses only forward `0002_fresh_admission_authority.sql`.

No material architecture conflict was found that requires an owner product decision. Independent review and protected integration remain mandatory because this is a high-risk authority/persistence boundary.

## Validation

### Focused

- command/run: repository-source/live-state semantic readback against protected main and Issues #313/#247
- result: PASS for architecture evidence collection; implementation validation is not part of this docs-only candidate

### Component/integration

- command/run: `NOT_APPLICABLE` for runtime because no executable/runtime code changes
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` for this architecture-only candidate; real PostgreSQL and Server Seam E2E are mandatory in later implementation children
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending PR creation/freeze
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: docs/governance architecture candidate
- result: pending

## Self-review

- exact head: pending final candidate head
- method/reviewer: `Oteryn: sol supervising architect`
- material findings: pending final changed-file readback
- verdict: pending

## Independent review

- required: YES — material admission/session/persistence authority decision under root and game-server governance
- exact head: pending final candidate head
- method/auditor: governing Work/control-plane authorized independent deep review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none found for #313 at admission readback
- protected auto-merge: forbidden for this architecture role
- merge commit/result: pending control-plane integration
- ownership release: after terminal protected-main integration/readback or explicit supersession

## Context checkpoint

```yaml
last_progress: architecture decision candidate authored from current protected main and exact #313 evidence
status: validating
branch: arch/fresh-admission-authority-313
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
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
blocker: independent review and protected integration are owned by the governing Work/control plane, not this architecture role
next_action: open the bounded architecture PR and hand the exact candidate to Oteryn: work coordinator for independent review, checks and protected integration
```
