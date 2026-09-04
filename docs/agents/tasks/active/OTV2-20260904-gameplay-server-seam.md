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
large_budget_reason: high-risk TCP/TLS plus Foundation admission/reconnect integration, bounded-resource evidence, real Tier 1 proof and independent exact-head review
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
  - merged_pr: 252
  - merged_pr: 290
blocks:
  - native Client allocation/readiness
  - physical gameplay Tier 1/Tier 2 sequence
external_repositories: []
```

## Outcome

Implement the smallest accepted production gameplay server/client-entry seam on top of the already-merged Foundation and Durability authorities: bounded TCP + TLS 1.3 transport using ALPN `oteryn-game/1`, FND-02 framing and protocol validation, FND-04 admission/reconnect verification, current GameSession/CharacterLease/fencing ownership, current durable reconnect journal, bounded backpressure/drain behavior, and a real local Tier 1 journey through the production boundary.

This task does not make gameplay commands available. Unregistered gameplay remains fail-closed until later owning lanes allocate their protocol/state/event identities and runtime semantics.

## Architecture and source of truth

- `PROVEN` — write authority is **not active** while allocation PR #294 has not merged. This task remains `waiting`/read-only until Work reads the allocation merge SHA from protected `main` and creates the worker branch from exactly that SHA.
- `PROVEN` — accepted Server Seam architecture is `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`, merged through PR #117.
- `PROVEN` — current Foundation protocol constants include `PROTOCOL_MAJOR_V1=1`, `TRANSPORT_PROFILE_TCP_TLS13_V1=1`, `ALPN_OTERYN_GAME_V1="oteryn-game/1"`, a 1,048,576-byte FND-02 wire-frame hard maximum and message-specific bounds.
- `PROVEN` — current `WireEnvelopeView` exposes message type/generation/sequence/raw payload and `decode_wire_envelope` validates current inbound envelopes; the accepted decision requires the missing typed bootstrap/resume consumer bridge and outbound Foundation acknowledgement/error encoding.
- `PROVEN` — current `apps/game-server/src/main.rs` is intentionally fail-closed for normal gameplay and must not be converted into a hard-coded production endpoint.
- `PROVEN` — current Durability terminal-replacement implementation is merged and ownership released; this task consumes it but does not redesign it.
- `PROVEN` — Server Seam hard maxima are registered, including pre-admission connections 256, concurrent handshake/auth work 64, outbound queue entries 64/session, outbound queue bytes 1,048,576/session, pending writes 8/session and drain tasks 256/batch.
- `DERIVED` — the seam can be implemented without new gameplay IDs, new resource maxima, a second admission/session authority or a production deployment decision by keeping configuration/TLS material caller-supplied and using local test-only fixtures for physical Tier 1.

Governing source order is protected-main governance -> merged allocation -> accepted #117 architecture -> current FND/DUR contracts/registries -> live implementation. Any material conflict blocks only the affected mutation and is escalated rather than guessed through.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - authenticated transport identity/binding is independently verified before admission or reconnect authority is granted
  - fresh admission GameSession/CharacterLease/world/runtime ownership facts are current and match the accepted FND-04 verifier result
  - reconnect candidate identity, connection generation, runtime scope, ownership generation and control-loss continuity are current at every authority-consuming boundary
  - durable PREPARE/COMMIT/reconciliation evidence defines expected persisted binding but is never used as its own source of current live authority
  - final reconnect COMMIT/controller installation revalidates current authority/fence facts rather than trusting a stale earlier snapshot
  - stale connection generation cannot send accepted post-admission work or receive controller authority
consumer_boundaries:
  - pre-admission TLS connection acceptance and FND-02 bootstrap decode
  - FND-04 fresh admission verification and GameSession commit
  - reconnect/resume verification and durable PREPARE/reconciliation
  - final reconnect authority revalidation/controller installation
  - admitted connection read/write dispatch and generation fencing
  - shutdown/drain path that releases connection/session-local transport work without transferring authority
mutation_operators:
  applicable:
    - accept one pre-admission connection within the registered connection budget
    - begin one bounded TLS handshake/authentication unit
    - commit a fresh admitted GameSession through the canonical Foundation authority
    - prepare or reconcile a reconnect attempt through the current durable adapter
    - authorize final reconnect COMMIT/controller replacement only from independently current facts
    - attach or replace the admitted transport generation after canonical authority succeeds
    - enqueue bounded server output and pending writes for the current admitted generation
    - close/drain transport-local work without mutating foreign gameplay authority
  considered_not_applicable:
    - gameplay command/domain mutation: no gameplay command IDs are allocated to this lane
    - Movement/Combat/Ability/Interaction/AI state mutation: owned by later or separate lanes
    - production certificate/key/port deployment mutation: explicitly outside task authority
one_invariant_per_negative_case: required
independent_current_fact_sources:
  - current FND-04 evidence verifier/consumer output for authentication/admission/reconnect evidence
  - current Foundation GameSession/CharacterLease/runtime authority state at the exact consuming boundary
  - current connection generation/runtime-scope/ownership-generation facts resolved independently of immutable reconnect records
record_derived_matching_helper:
  allowed_for_positive_happy_path: only when current protected-main governance explicitly permits a test-only convenience and the case does not claim negative/current-authority proof
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: required across fresh admission, resume/reconnect and server acknowledgement/error bridge APIs that consume authority facts
  protocol_versions: v1 only unless protected main allocates another version; no compatibility version may weaken v1 authority
  direct_and_reconciled_paths: required
  fenced_durable_writes: required where this seam invokes reconnect PREPARE/COMMIT/reconciliation
  restart_retry_replay_concurrency_pg_reload: required where the current durable adapter boundary is exercised; PostgreSQL reload may rely on already-merged Durability tests only when the Server Seam change does not invalidate that behavior, otherwise add focused physical evidence within owned paths or escalate for a missing test lease
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

Immutable prepared/persisted reconnect evidence may define the expected durable binding but never proves that a session, lease, runtime scope, connection generation or ownership generation is current. Every authority-consuming production boundary must receive independent current facts sufficient for that boundary.

## Acceptance criteria

### Transport and framing

- [ ] Normal Server Seam transport uses TCP with TLS 1.3 profile 1 and ALPN exactly `oteryn-game/1`; no plaintext gameplay path or legacy/Canary protocol path is introduced.
- [ ] Four-byte big-endian frame length is validated before body allocation/read; 0 and >1,048,576 are rejected fail-closed, truncation is deterministic, and all FND-02 message-specific bounds remain authoritative.
- [ ] Unknown/malformed/direction-invalid/phase-invalid messages are rejected without admission, domain mutation or unbounded retained work.
- [ ] Transport implementation has no `unsafe` and preserves workspace lint policy.

### Foundation authority consumption

- [ ] `foundation/protocol.rs` exposes only the minimum typed, crate-internal consumer bridge required to obtain already-validated `ClientBootstrap`/`ClientResume` fields and encode already-registered `ServerAccepted`, `ServerResumeAccepted` and `ProtocolError`; no new message ID/capability/state-domain/stable ID is allocated.
- [ ] If implementation would require a new externally public Foundation API/schema/wire semantic rather than a crate-internal accepted consumer bridge, stop with `ARCHITECTURE_ESCALATION_REQUIRED` before that change.
- [ ] Fresh admission is committed only through canonical FND-04/Foundation admission authority after trusted evidence verification; no transport-local duplicate session authority exists.
- [ ] Resume/reconnect consumes the current Durability/Foundation replacement flow and final current-authority revalidation; no record-derived helper substitutes for live facts.
- [ ] Stale/missing/mismatched generation, GameSession, CharacterLease, world/runtime scope, ownership generation, transport binding and authority/provenance cases fail independently before the affected authority grant/mutation.

### Resource and lifecycle bounds

- [ ] Registered hard maxima are enforced with checked accounting before allocation or partial mutation: pre-admission connections 256, handshake/auth work 64, outbound queue entries 64/session, outbound queue bytes 1,048,576/session, pending writes 8/session, drain tasks 256/batch.
- [ ] Each applicable registered Server Seam limit has max-accepted, max+1 rejected and checked-overflow/relevant early-rejection evidence consistent with the registry.
- [ ] Backpressure never converts queue saturation into unbounded task spawning, hidden retry or cross-session starvation authority.
- [ ] Shutdown/drain is bounded, cancellation-safe and does not retain or transfer stale controller/session authority.

### Composition and configuration

- [ ] `apps/game-server/src/lib.rs` composes exactly one Server Seam implementation while preserving existing Foundation/Durability/content modules and current high-risk regression tests.
- [ ] `apps/game-server/src/main.rs` keeps `--smoke` behavior and may enter gameplay serving only from explicit caller/configuration input; it must not choose or hard-code a production bind address, port, certificate, private key, secret location or deployment topology.
- [ ] Library/test composition accepts local caller-supplied endpoint/TLS material sufficient for real loopback Tier 1 without production credentials.
- [ ] Missing/incompatible runtime configuration remains fail-closed rather than silently opening gameplay.

### TDD and physical evidence

- [ ] Fresh RED is captured before implementation for typed Foundation extraction/encoding, framing boundaries, malformed/oversized/unknown input, TLS/ALPN mismatch, pre-admission/handshake limits, authority-before-mutation, stale generation/reconnect facts, queue/pending-write saturation and bounded shutdown.
- [ ] Minimal GREEN implements only the accepted seam; repair cycles do not broaden into gameplay or architecture redesign.
- [ ] `apps/game-server/tests/gameplay_server_seam.rs` proves a real local loopback TCP/TLS production-path Tier 1 bootstrap/admit flow and reconnect/resume path using non-shipping test certificate/evidence fixtures.
- [ ] Direct-domain/synthetic success is not reported as physical Tier 1.
- [ ] Real Tier 1 records the exact candidate head, local topology, negotiated TLS/ALPN, bounded input path and authoritative admission/reconnect result.

### Qualification and handoff

- [ ] Focused/package tests, strict Clippy, fmt and applicable whole-workspace tests pass on the stable candidate.
- [ ] Exact-head repository CI, including current Linux/Windows/supply-chain/merge-gate composition as applicable, is green for the exact final head.
- [ ] Whole-diff self-review finds no unresolved P0/P1/P2 disposition gap and completes the finding-family sweep required above.
- [ ] One genuinely independent exact-head deep review covers protocol/session/admission/fencing risk; green CI alone does not satisfy review.
- [ ] Zero unresolved required review threads and no material head movement after final qualifying review.
- [ ] Worker returns the canonical SERVER_SEAM handoff with `READY_FOR_INTEGRATION`; worker does not merge its own PR.

## Excluded scope

- No gameplay command/state/event/capability/stable numeric ID allocation.
- No Movement, Combat, Ability, Interaction, AI, Channel, Analytics or gameplay formula implementation.
- No permanent Content/world-bundle format decision.
- No production bind address, port, DNS, certificate, private key, secret, environment, deployment or live-account/session/data mutation.
- No QUIC activation or alternate protocol stack.
- No new persistence semantics, migration redesign or durable value/item semantics.
- No `workspace-boundaries.toml`, workflow, ruleset, repository-protection or architecture-contract writes.
- No Platform, Atlas, META or other external-repository writes.
- No Reference-parity or production-readiness claim from this seam alone.

A required change outside the exact owned paths is reported to Work as `SHARED_LEASE_REQUIRED`. A material public API/schema/protocol/trust/fencing/persistence/resource/production decision is `ARCHITECTURE_ESCALATION_REQUIRED` and stops only the affected mutation.

## Implementation / findings

The implementation follows `docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md` after the allocation merge/readback. The plan is subordinate to this task and accepted contracts; it cannot authorize a path or semantic not listed here.

Do not begin by creating a listener. Begin with RED tests around the already-accepted Foundation consumer bridge and untrusted framing boundary, because these establish the only safe inputs the listener may later consume.

Dependency changes in root/game-server Cargo files are restricted to exact dependencies/features needed by the accepted TCP/TLS implementation. Do not absorb open Dependabot #259/#260/#261 upgrades as convenience. Re-read those PRs before shared Cargo mutation and before final integration.

## Validation

### Focused

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam`
- result: not run — mutation authority is waiting on allocation merge

### Component/integration

- command/run: `cargo +1.94.0 test --locked -p oteryn-game-server`
- result: not run — mutation authority is waiting on allocation merge

### E2E

- scenario: local loopback TCP/TLS production listener -> FND-02 bootstrap/resume -> FND-04/Foundation admission/reconnect -> bounded server acknowledgement/error path using caller-supplied test TLS material
- result: not run — mutation authority is waiting on allocation merge

### Exact-head CI

- final head: null
- trigger source: pull_request after implementation candidate exists
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

- required: `YES` — protocol/session/admission/reconnect/fencing plus production transport boundary
- exact head: null
- method/auditor: one genuinely independent exact-head deep review under current protected-main policy
- material findings: not evaluated
- verdict: not evaluated

## PR and closeout

- changed-file review: exact owned-path allowlist only
- unresolved review threads: not evaluated
- related/superseded PRs: none for Server Seam at allocation admission
- protected auto-merge: worker does not enable or perform terminal integration
- merge commit/result: control-plane responsibility after READY_FOR_INTEGRATION
- ownership release: after protected-main merge/readback and worker-task archival

## Context checkpoint

```yaml
last_progress: allocation PR #294 is open Draft and records this worker task; runtime authority remains withheld until the allocation itself qualifies, merges and is read back from protected main
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
next_action: remain read-only until Work proves PR #294 merged and reads the exact merge SHA from protected main, then create the worker branch from exactly that SHA and run the first focused Server Seam RED before any production transport implementation
```
