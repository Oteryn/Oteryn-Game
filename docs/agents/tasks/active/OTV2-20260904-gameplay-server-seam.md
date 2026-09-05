# OTV2-20260904-gameplay-server-seam

```yaml
task_id: OTV2-20260904-gameplay-server-seam
title: Implement production gameplay Server Seam
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-gameplay-server-seam-01
pr: null
issue: 247
lane_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
allocation_task_id: OTV2-20260904-gameplay-server-seam-allocation
allocation_pr: 294
allocation_admission_main_sha: 68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705
allocation_integration_main_sha: 187c6b83c6945d79aabef2c5730c3ddba13fcab1
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: sol server seam lead"
created_at: 2026-09-04T19:27:00+02:00
updated_at: 2026-09-05T13:44:00+02:00
execution_budget_minutes: 120
large_budget_reason: high-risk TCP/TLS plus Foundation admission/reconnect integration, bounded-resource proof, production-path integration and independent exact-head review
owned_paths:
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/tcp_tls.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/tests/gameplay_server_seam.rs
  - apps/game-server/src/foundation/protocol.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/main.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md
public_contracts:
  - FND-02 protocol consumer seam
  - FND-03 runtime/resource consumer seam
  - FND-04 admission/GameSession/CharacterLease/reconnect/fencing consumer seam
  - production gameplay TCP/TLS entry seam accepted by PR #117
depends_on:
  - allocation_task: OTV2-20260904-gameplay-server-seam-allocation
  - merged_pr: 117
  - merged_pr: 151
  - merged_pr: 252
  - merged_pr: 289
blocks:
  - native Client allocation/readiness
  - physical gameplay Tier 1/Tier 2 QA sequence
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

After the allocation PR #294 lawfully integrates and Work creates this worker branch from the exact allocation merge SHA, implement the smallest accepted production gameplay server/client-entry seam on top of current Foundation, FND-04 verifier/consumer and Durability authorities.

The seam is bounded TCP + TLS 1.3 with ALPN `oteryn-game/1`, FND-02 framing/validation, canonical fresh-admission/reconnect verification, current GameSession/CharacterLease/fencing authority, durable reconnect journal consumption, bounded backpressure/drain and real local production-path TCP/TLS integration evidence.

This lane makes later ADR-0007 physical QA possible but does **not** own or declare QA Tier 1/Tier 2 `PROVEN`. Formal QA remains `NOT_EVALUATED` until separately allocated after Server Seam integration.

## Architecture and source of truth

- `PROVEN` — this worker has no write authority while allocation PR #294 is unmerged. No worker branch may be created before protected-main allocation readback.
- `PROVEN` — accepted Server Seam architecture is `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`, merged through PR #117.
- `PROVEN` — current protected-main authority API floor from #289 removes production record/identity-derived current-authority convenience constructors; tests #302/#303 further qualify independently sourced current authority and replay/restart behavior without changing production semantics.
- `PROVEN` — Foundation protocol major 1, transport profile 1, ALPN `oteryn-game/1`, FND-02 bounded BE32 framing and registered message semantics remain authoritative.
- `PROVEN` — current `apps/game-server/src/main.rs` remains fail-closed outside `--smoke`; no production listener exists.
- `PROVEN` — FND-04 verifier/consumer and current Durability terminal-replacement/reconnect implementation are merged.
- `PROVEN` — Server Seam hard maxima are registered: pre-admission connections 256, handshake/auth work 64, outbound queue 64 entries/session, outbound queue 1,048,576 bytes/session, pending writes 8/session and drain tasks 256/batch.
- `PROVEN` — no current active non-Dependabot PR owns any path in this task's primary/shared allowlist. Dependabot #259/#260/#261 remain root Cargo/lock candidates only and must be re-read before shared Cargo mutation.
- `DERIVED` — the accepted seam can be implemented without new gameplay IDs, resource maxima, a second admission/session authority or production deployment decision by keeping endpoint/TLS material caller-supplied and using non-shipping loopback test fixtures.

Governing source order is protected-main governance -> merged allocation -> accepted #117 -> current FND/DUR contracts/registries -> live implementation. A lower source cannot widen the allocation.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - authenticated transport identity/binding is independently verified before fresh-admission or reconnect authority is granted
  - fresh admission GameSession/CharacterLease/world/runtime facts are current and match canonical FND-04 verification
  - reconnect candidate identity, connection generation, runtime scope, ownership generation and control-loss continuity are independently current at each consuming boundary
  - immutable prepared/persisted evidence defines expected binding only and is never its own current-authority source
  - final reconnect COMMIT/controller installation revalidates current authority rather than trusting a stale earlier snapshot
  - stale connection generation cannot send accepted post-admission work or regain controller authority
consumer_boundaries:
  - pre-admission TLS connection and FND-02 bootstrap/resume decode
  - FND-04 fresh-admission verification and GameSession commit
  - reconnect PREPARE / direct or reconciled outcome
  - final reconnect current-authority revalidation and controller installation
  - admitted connection read/write dispatch and generation fencing
  - shutdown/drain of connection-local work
mutation_operators:
  applicable:
    - accept one pre-admission connection inside the registered connection budget
    - begin one bounded TLS handshake/auth unit
    - commit fresh admission only through canonical Foundation authority
    - prepare/reconcile reconnect through current durable adapter
    - authorize final reconnect COMMIT/controller replacement only from independently current facts
    - bind admitted transport generation only after canonical authority succeeds
    - enqueue bounded server output / pending writes for the current admitted generation
    - close/drain transport-local work without silently losing already-authoritative reserved work
  considered_not_applicable:
    - gameplay command/domain mutation: no gameplay command/state IDs are allocated to this lane
    - Movement/Combat/Ability/Interaction/AI state mutation: foreign authority
    - production deployment/certificate/secret mutation: excluded
one_invariant_per_negative_case: required
independent_current_fact_sources:
  - current FND-04 verifier/consumer output
  - current Foundation GameSession/CharacterLease/runtime authority at the exact consuming boundary
  - current connection generation/runtime scope/ownership generation resolved independently of immutable reconnect records
record_derived_matching_helper:
  allowed_for_positive_happy_path: only test-only expected-value construction explicitly permitted by current protected-main governance
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required across fresh admission, resume/reconnect and server acknowledgement/error consumers
  protocol_versions: v1 only unless protected main allocates another version
  direct_and_reconciled_paths: required
  fenced_durable_writes: required where reconnect PREPARE/COMMIT/reconciliation is invoked
  restart_retry_replay_concurrency_pg_reload: required where this change invalidates or exercises the durable boundary; leverage current #302/#303 harness only when representative, otherwise add owned-path evidence or request a lease
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

Every negative authority/provenance case changes exactly one applicable invariant while unrelated facts remain valid. Historical terminal outcomes may preserve typed disposition but never reacquire live authority through a weaker path.

## Acceptance criteria

### Transport and framing

- [ ] TCP + TLS 1.3 profile 1 with exact ALPN `oteryn-game/1`; no plaintext/legacy/Canary fallback.
- [ ] Four-byte big-endian frame length is checked before body allocation/read; 0, truncation and >1,048,576 fail closed.
- [ ] Wrong protocol major and transport profile are rejected through the composed production listener path, not only decoder unit tests.
- [ ] Unknown/malformed/direction/phase-invalid input is rejected without admission/domain mutation or unbounded retained work.

### Foundation authority consumption

- [ ] `foundation/protocol.rs` exposes only the minimum crate-internal typed bootstrap/resume consumer bridge and registered `ServerAccepted`, `ServerResumeAccepted`, `ProtocolError` encoders; no new wire ID/schema/capability/state ID.
- [ ] New outbound encoders have independent canonical/golden vectors or another cross-oracle; same-implementation encode/decode round trip alone is insufficient.
- [ ] Fresh admission is committed only after FND-04 verification and canonical current authority.
- [ ] Missing/invalid/expired/replayed/wrong-binding evidence and concurrent same-grant fresh admission fail before a duplicate GameSession/controller is created.
- [ ] Resume/reconnect consumes current Foundation/Durability flow and final current-authority revalidation; no record-derived convenience substitutes for live facts.
- [ ] Stale/missing/mismatched GameSession, account presence, character/world eligibility, generation, runtime scope, ownership generation, transport binding and continuity each fail independently before the affected authority grant/mutation.

### Resources and lifecycle

- [ ] Enforce registered hard maxima before allocation/partial mutation: 256 pre-admission connections, 64 handshake/auth units, 64 outbound entries/session, 1,048,576 outbound bytes/session, 8 pending writes/session, 256 drain tasks/batch.
- [ ] Each applicable resource has max accepted, max+1 rejected/backpressured and checked-overflow/early-rejection evidence.
- [ ] Slow-client saturation cannot create unbounded tasks/channels/retries or cross-session authority starvation.
- [ ] Shutdown/drain is bounded and cancellation-safe; already-authoritative reserved work is completed or durably/reconcilably preserved rather than silently dropped.

### Composition and evidence

- [ ] `lib.rs` composes exactly one Server Seam without changing foreign gameplay authority.
- [ ] `main.rs` preserves `--smoke` and may serve gameplay only from explicit valid configuration; no hard-coded production address/port/cert/key/secret/topology.
- [ ] `apps/game-server/tests/gameplay_server_seam.rs` traverses the actual local production listener/TLS/framing/FND-04/Foundation/Durability path for bootstrap/admission and resume/reconnect using non-shipping test material.
- [ ] The Server Seam integration result is not relabeled as formal ADR-0007 QA Tier 1/Tier 2 proof.

### TDD and qualification

- [ ] Fresh RED precedes implementation for typed extraction/encoding, frame/TLS/ALPN/version/profile negatives, admission evidence/replay/binding/concurrency, reconnect authority family, resource saturation and shutdown preservation.
- [ ] Minimal GREEN stays within the exact allowlist; any required unowned path is `SHARED_LEASE_REQUIRED` rather than implicit expansion.
- [ ] Focused/package/workspace tests, fmt and strict Clippy pass on the stable candidate.
- [ ] Applicable current exact-head repository CI, including canonical PG/SIM behavior selected by protected-base policy, is green.
- [ ] Whole-diff self-review and the complete authority finding-family sweep are clean with explicit P0/P1/P2 disposition.
- [ ] One genuinely independent exact-head deep review covers protocol/session/admission/reconnect/fencing/transport/resource risks.
- [ ] Zero unresolved required threads and no material head movement after final qualifying review.
- [ ] Worker returns `READY_FOR_INTEGRATION`; worker does not merge its own PR.

## Excluded scope

No gameplay command/state/event/capability/stable ID allocation; no Movement/Combat/Ability/Interaction/AI/Channel/Analytics semantics; no permanent Content format; no production bind address/DNS/port/cert/key/secret/environment/deployment; no QUIC; no new persistence semantics; no `workspace-boundaries.toml`, workflow/ruleset/protection/architecture-contract write; no Platform/Atlas/META/external write; no Reference-parity or production-readiness claim.

A material public API/schema/protocol/trust/fencing/persistence/resource/production decision is `ARCHITECTURE_ESCALATION_REQUIRED` before mutation.

## Implementation / findings

Implementation follows `docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md` only after allocation merge/readback. The plan is subordinate to this task and accepted protected-main authority.

Before editing root/game-server Cargo files, re-read active owners and #259/#260/#261. Do not absorb unrelated dependency upgrades. If a non-Dependabot writer acquires a shared path, stop with `SHARED_LEASE_REQUIRED`.

Historical allocation-review findings requiring golden encoding, listener version/profile negatives, replay/binding/concurrent admission and authoritative-work shutdown preservation are already incorporated into the plan/task and must be preserved by implementation.

## Validation

### Focused

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam`
- result: not run — write authority waits on allocation merge/readback

### Component/integration

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server`
- result: not run — write authority waits on allocation merge/readback

### E2E

- scenario: local production-path TCP/TLS listener -> FND-02 bootstrap/resume -> FND-04/Foundation admission/reconnect -> bounded registered server acknowledgement/error path
- result: not run — write authority waits on allocation merge/readback; formal QA Tier 1/Tier 2 is separately owned

### Exact-head CI

- final head: null
- trigger source: future worker pull_request
- workflow/run/job: not started
- runner assignment: unknown
- classification: high-risk protocol/session/admission/fencing Server Seam
- result: not started

## Self-review

- exact head: null
- method/reviewer: `Oteryn: sol server seam lead` whole-diff adversarial review
- material findings: not evaluated
- verdict: not evaluated

## Independent review

- required: `YES` — protocol/session/admission/reconnect/fencing plus production transport/resource boundary
- exact head: null
- method/auditor: genuinely independent exact-head deep review under current policy
- material findings: not evaluated
- verdict: not evaluated

## PR and closeout

- changed-file review: exact owned-path allowlist only
- unresolved review threads: not evaluated
- related/superseded PRs: none for Server Seam implementation at allocation qualification; Dependabot #259/#260/#261 remain non-owning shared-path candidates
- protected integration: worker does not perform terminal merge
- merge/result: control-plane responsibility only after `READY_FOR_INTEGRATION`
- ownership release: after protected-main worker merge/readback and task archival

## Context checkpoint

```yaml
last_progress: current protected main is 187c6b83c6945d79aabef2c5730c3ddba13fcab1; authority-floor #289 is terminal and #302/#303 add independent authority/recovery qualification without changing production semantics; allocation PR #294 has been normally merged up to current main and is undergoing fresh docs-only qualification
status: waiting
branch: agent/otv2-gameplay-server-seam-01
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
blocker: allocation_pr_294_not_merged
next_action: wait for Work to finish exact-head qualification/review and protected integration of allocation PR #294; only after merge-SHA readback may this worker branch be created
```
