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
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: sol server seam lead"
created_at: 2026-09-04T19:27:00+02:00
updated_at: 2026-09-04T19:27:00+02:00
execution_budget_minutes: 120
large_budget_reason: high-risk TCP/TLS plus Foundation admission/reconnect integration, bounded-resource proof, real production-path seam integration and independent exact-head review
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
  - merged_pr: 290
blocks:
  - native Client allocation/readiness
  - physical gameplay Tier 1/Tier 2 QA sequence
external_repositories: []
```

## Outcome

Implement the smallest accepted production gameplay server/client-entry seam on top of merged Foundation, FND-04 verifier/consumer and Durability authorities: bounded TCP + TLS 1.3 with ALPN `oteryn-game/1`, FND-02 framing/validation, canonical fresh-admission/reconnect verification, current GameSession/CharacterLease/fencing authority, durable reconnect journal consumption, bounded backpressure/drain, and real local production-path TCP/TLS integration evidence.

This lane makes the later ADR-0007 physical Tier 1 journey executable but does **not** own or declare QA Tier 1/Tier 2 `PROVEN`. Formal QA Tier 1/Tier 2 remain `NOT_EVALUATED` until a separate QA allocation after the Server Seam is merged.

Unregistered gameplay remains fail-closed. This task allocates no gameplay command/state/event/capability/stable numeric ID and does not create gameplay domain semantics.

## Architecture and source of truth

- `PROVEN` — write authority is not active while allocation PR #294 is unmerged. This task remains `waiting`/read-only until Work reads the allocation merge SHA from protected `main` and creates the worker branch from exactly that SHA.
- `PROVEN` — accepted Server Seam architecture is `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`, merged through PR #117.
- `PROVEN` — Issue #115 implementation is terminal: archived task `OTV2-20260825-fnd04-verifier-consumer` records PR #151 merged as `2d0e951ce37c2e28773c22966bb816c00bebaa0a` with released ownership.
- `PROVEN` — Issue #116 is closed and the current Resource Limits Registry contains the accepted NET03 Server Seam hard maxima.
- `PROVEN` — current Durability implementation/journal/reconnect terminal-replacement work is merged and ownership released; this lane consumes it and does not redesign it.
- `PROVEN` — current Foundation protocol constants include protocol major `1`, TCP/TLS13 transport profile `1`, ALPN `oteryn-game/1`, the 1,048,576-byte wire-frame hard maximum and message-specific bounds.
- `PROVEN` — current `foundation/protocol.rs` validates inbound wire envelopes but the accepted #117 decision still allocates the minimum typed bootstrap/resume extraction plus registered-only server encoders to this lane.
- `PROVEN` — current `apps/game-server/src/main.rs` remains fail-closed for ordinary gameplay startup and must not be changed into a hard-coded production endpoint.
- `DERIVED` — the accepted seam can be implemented without a new stable ID, resource maximum, duplicate authority or production deployment decision by keeping configuration/TLS material caller-supplied and using non-shipping loopback fixtures for implementation/integration evidence.

All prerequisite facts are re-read from protected `main` at worker release. A changed/conflicting prerequisite blocks the affected mutation rather than becoming an assumption.

Governing order: protected-main governance -> merged allocation -> accepted #117 architecture -> current FND/DUR contracts/registries -> live implementation.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - authenticated transport identity/binding is independently verified before admission or reconnect authority is granted
  - fresh admission GameSession/CharacterLease/world/runtime ownership facts are current and match canonical FND-04 verifier output
  - reconnect candidate identity, connection generation, runtime scope, ownership generation and control-loss continuity are current at every authority-consuming boundary
  - durable PREPARE/COMMIT/reconciliation evidence defines expected persisted binding but is never its own source of live current authority
  - final reconnect COMMIT/controller installation revalidates current authority/fence facts rather than trusting an earlier snapshot
  - stale connection generation cannot send accepted post-admission work or regain controller authority
consumer_boundaries:
  - pre-admission TLS connection and bounded FND-02 bootstrap/resume decode
  - FND-04 fresh admission verification and canonical GameSession commit
  - reconnect/resume verification and durable PREPARE/reconciliation
  - final reconnect current-authority revalidation/controller installation
  - admitted read/write dispatch and connection-generation fencing
  - shutdown/drain of bounded transport work including already-authoritative reserved work
mutation_operators:
  applicable:
    - accept one pre-admission connection within the registered connection budget
    - begin one bounded TLS handshake/authentication unit
    - commit a fresh GameSession only through canonical Foundation authority
    - prepare or reconcile reconnect only through the current durable adapter
    - authorize final reconnect COMMIT/controller replacement only from independently current facts
    - attach or replace the transport generation after canonical authority succeeds
    - enqueue bounded server output/pending writes for the current admitted generation
    - close/drain transport-local work without mutating foreign gameplay authority
  considered_not_applicable:
    - gameplay command/domain mutation: no gameplay command IDs are allocated
    - Movement/Combat/Ability/Interaction/AI mutation: foreign lanes
    - production certificate/key/port/deployment mutation: explicitly excluded
one_invariant_per_negative_case: required
independent_current_fact_sources:
  - production FND-04 verifier/consumer output plus independently current authoritative evidence
  - current Foundation GameSession/CharacterLease/runtime authority at the consuming boundary
  - current connection-generation/runtime-scope/ownership-generation facts resolved independently of immutable reconnect records
record_derived_matching_helper:
  allowed_for_positive_happy_path: only where current protected-main governance explicitly permits a test-only convenience that does not claim negative/current-authority proof
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required across fresh admission, resume/reconnect and server acknowledgement/error bridge APIs
  protocol_versions: v1 only unless protected main allocates another version; no compatibility path may weaken v1 authority
  direct_and_reconciled_paths: required
  fenced_durable_writes: required where reconnect PREPARE/COMMIT/reconciliation is invoked
  restart_retry_replay_concurrency_pg_reload: required where the current durable adapter boundary is exercised; existing Durability evidence may be reused only if this diff has not invalidated it
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

Immutable prepared/persisted reconnect evidence may define expected durable binding but never proves current session, lease, runtime scope, connection generation or ownership generation.

## Acceptance criteria

### Transport and framing

- [ ] TCP + TLS 1.3 profile 1 and ALPN exactly `oteryn-game/1`; no plaintext, downgrade, legacy or Canary fallback.
- [ ] BE32 frame length is checked before peer-sized body allocation/read; 0 and >1,048,576 reject fail-closed; exact max and truncation are tested.
- [ ] Wrong protocol major/transport profile, malformed/duplicate/unknown/over-limit fields, invalid direction/phase/generation and unknown message types fail before admission/domain mutation.
- [ ] No `unsafe`; current workspace lint policy remains intact.

### Foundation bridge and authority consumption

- [ ] `foundation/protocol.rs` adds only the minimum crate-internal typed bootstrap/resume bridge and registered-only encoders for `ServerAccepted`, `ServerResumeAccepted`, `ProtocolError`; no new ID/schema/field meaning/limit.
- [ ] Outbound encoder evidence includes canonical/golden bytes and an independent non-self-referential cross-oracle/wire check; self encode->self decode alone is insufficient.
- [ ] New externally public Foundation API/schema/wire semantics require `ARCHITECTURE_ESCALATION_REQUIRED` before mutation.
- [ ] Fresh admission uses the production FND-04 verifier/consumer and canonical Foundation commit; invalid/expired/replayed/wrong-binding/stale-evidence cases fail independently.
- [ ] Concurrent/replayed fresh admission cannot create two sessions.
- [ ] Reconnect consumes current Durability/Foundation replacement flow and independently current facts; no record-derived helper substitutes for live authority.
- [ ] Stale/missing/mismatched GameSession, CharacterLease, world/runtime scope, ownership generation, connection generation, transport binding or provenance fails before the affected authority grant/mutation.

### Resource/lifecycle bounds

- [ ] Hard maxima are enforced with checked accounting before partial mutation: pre-admission 256, handshake/auth 64, outbound entries 64/session, outbound bytes 1,048,576/session, pending writes 8/session, drain tasks 256/batch.
- [ ] Each applicable limit has max-accepted, max+1 rejected/backpressured and relevant overflow/early-rejection proof.
- [ ] Slow clients cannot create unbounded task/channel/retry growth or consume another session's authority budget.
- [ ] Shutdown/drain is bounded/cancellation-safe, does not transfer/resurrect stale controller authority, and does not silently drop already-authoritative reserved work; such work is drained or explicitly resolved according to canonical Foundation lifecycle semantics.

### Composition / unsupported gameplay

- [ ] `lib.rs` composes exactly one Server Seam while preserving Foundation/Durability/content and existing high-risk regression coverage.
- [ ] `main.rs` preserves `--smoke` and uses only explicit valid configuration; no production bind address/port/certificate/key/secret/deployment topology is selected here.
- [ ] Missing/incompatible runtime configuration remains fail-closed; no plaintext fallback.
- [ ] After admission, unsupported/unregistered `ClientCommand` fails closed with zero command reservation/domain mutation and no invented gameplay ID.

### TDD and physical seam evidence

- [ ] Fresh RED -> minimal GREEN covers Foundation bridge/encoders, framing, TLS/ALPN/profile, malformed/oversized input, FND-04 invalid/expired/replay/wrong-binding, concurrent fresh admission, stale reconnect facts, all resource boundaries, unsupported post-admission gameplay, backpressure and bounded shutdown/drain.
- [ ] `apps/game-server/tests/gameplay_server_seam.rs` traverses the actual production listener/composition path on loopback using non-shipping TLS material for bootstrap/admission and resume/reconnect.
- [ ] Physical assertions observe canonical GameSession/current-generation outcomes, not merely socket success.
- [ ] Server Seam physical integration may be reported `PROVEN` for the exact candidate when tests pass; ADR-0007 QA Tier 1 and Tier 2 remain `NOT_EVALUATED` for this lane.

### Qualification and handoff

- [ ] Focused/package tests, fmt, strict Clippy and applicable workspace validation pass on the coherent candidate.
- [ ] Exact-head repository CI including current Linux/Windows/supply-chain/merge-gate composition as applicable is green.
- [ ] Whole-diff self-review completes the required finding-family sweep and leaves no unresolved P0/P1/P2 disposition gap.
- [ ] One genuinely independent exact-head deep review covers protocol/session/admission/reconnect/fencing/TLS/resource/evidence-ownership risk.
- [ ] Zero unresolved required review threads and no material head movement after qualifying review.
- [ ] Worker returns canonical SERVER_SEAM `READY_FOR_INTEGRATION` handoff without self-merging.

## Excluded scope

- No gameplay command/state/event/capability/stable numeric ID allocation or gameplay formula/state implementation.
- No Movement, Combat, Ability, Interaction, AI, Channel or Analytics implementation.
- No permanent Content/world-bundle format decision or Reference-parity claim.
- No production address/port/DNS/certificate/private key/secret/environment/deployment/live-account/session/data mutation.
- No QUIC activation/registration or alternate protocol stack.
- No new persistence semantics, migration redesign or durable value/item semantics.
- No `workspace-boundaries.toml`, workflow, ruleset, repository-protection, stable registry or architecture-contract writes.
- No Platform, Atlas, META or other external-repository writes.
- No QA Tier 1/Tier 2 completion claim from this lane.

A legitimate need outside exact owned paths is `SHARED_LEASE_REQUIRED`. A material public API/schema/protocol/trust/fencing/persistence/resource/production/evidence-ownership decision is `ARCHITECTURE_ESCALATION_REQUIRED` before mutation.

## Implementation / findings

Follow `docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md` after allocation merge/readback. The plan is subordinate to this task and accepted #117; it cannot widen authority.

Do not begin with a listener. Begin with fresh RED around the Foundation wire bridge and untrusted framing boundary. Before shared Cargo mutation and final integration, re-read #259/#260/#261 and all active non-Dependabot ownership.

## Validation

### Focused

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam`
- result: not run — mutation authority waits on allocation merge/readback

### Component/integration

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server`
- result: not run — mutation authority waits on allocation merge/readback

### Physical Server Seam integration

- scenario: real local loopback TCP/TLS production listener -> FND-02 bootstrap/resume -> production FND-04 verifier/current authority -> canonical Foundation admission/reconnect/Durability -> bounded registered server response; stale generation and unsupported gameplay fail closed
- result: not run — mutation authority waits on allocation merge/readback
- QA Tier 1: `NOT_EVALUATED` — separate post-merge QA allocation owns the ADR-0007 evidence envelope
- QA Tier 2: `NOT_EVALUATED`

### Exact-head CI

- final head: null
- trigger source: pull_request after implementation candidate exists
- workflow/run/job: not started
- runner assignment: unknown
- classification: high-risk protocol/session/admission/reconnect/fencing/TLS Server Seam
- result: not started

## Self-review

- exact head: null
- method/reviewer: `Oteryn: sol server seam lead` whole-diff adversarial review
- material findings: not evaluated
- verdict: not evaluated

## Independent review

- required: `YES`
- exact head: null
- method/auditor: genuinely independent exact-head deep review under current protected-main policy
- material findings: not evaluated
- verdict: not evaluated

## PR and closeout

- changed-file review: exact owned-path allowlist only
- unresolved review threads: not evaluated
- protected integration: worker does not merge its own PR
- merge/result: Work control-plane responsibility after truthful `READY_FOR_INTEGRATION`
- ownership release: only after protected-main merge/readback and worker-task archival

## Context checkpoint

```yaml
last_progress: allocation PR #294 is Draft; self-review corrected QA evidence ownership and expanded the child plan to literal #117 negative/golden/shutdown requirements; runtime authority remains withheld until the allocation qualifies, merges and is read back from protected main
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
next_action: remain read-only until Work proves PR #294 merged and reads the exact merge SHA from protected main, then create the worker branch from exactly that SHA and run the first focused Foundation-bridge RED before any production transport implementation
```
